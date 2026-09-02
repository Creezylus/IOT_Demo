use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::env;
use std::time::{Duration, Instant};

#[path = "../../rust_tools/logger/log.rs"] // Cheesyyy fix this laterr..
mod log;


// Configuration constants
const POLL_INTERVAL_SECS: u64 = 5;
const NORMAL_SYNC_INTERVAL_SECS: u64 = 60; // 5 minutes

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenv().ok();

    let host = env::var("PRIMARY_DB_HOST").expect("PRIMARY_DB_HOST must be set");
    let port = env::var("PRIMARY_DB_PORT").expect("PRIMARY_DB_PORT must be set");
    let name = env::var("PRIMARY_DB_NAME").expect("PRIMARY_DB_NAME must be set");
    let user = env::var("PRIMARY_DB_USER").expect("PRIMARY_DB_USER must be set");
    let pass = env::var("PRIMARY_DB_PASS").expect("PRIMARY_DB_PASS must be set");
    let path = env::var("PRIMARY_DB_PATH").unwrap_or_else(|_| "public".to_string());

    let primary_url = format!(
        "postgres://{}:{}@{}:{}/{}?options=-c%20search_path={}",
        user, pass, host, port, name, path
    );
    let local_url = env::var("DATABASE_URL").expect("LOCAL_DB_URL must be set");

    let local_pool = PgPoolOptions::new().connect(&local_url).await?;
    let primary_pool = PgPoolOptions::new().connect(&primary_url).await?;

    iotlogger!("Connected to both databases. Starting dynamic sync loop...");

    let mut last_normal_sync = Instant::now() - Duration::from_secs(NORMAL_SYNC_INTERVAL_SECS);

    // 2. The Dynamic Loop
    loop {
        let has_alerts = check_for_alerts(&local_pool).await.unwrap_or(false);
        let time_since_last_sync = last_normal_sync.elapsed().as_secs();

        if has_alerts || time_since_last_sync >= NORMAL_SYNC_INTERVAL_SECS {
            iotlogger!(
                "Triggering sync. (Alert detected: {}, Time since last sync: {}s)",
                has_alerts, time_since_last_sync
            );

            match perform_full_sync(&local_pool, &primary_pool).await {
                Ok(_) => {
                    last_normal_sync = Instant::now();
                    iotlogger!("Sync complete.");
                }
                Err(e) => {
                    iotlogger!("Sync failed: {}", e);
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

async fn check_for_alerts(local: &Pool<Postgres>) -> Result<bool, sqlx::Error> {
    let alert_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM metrics 
            WHERE is_synced = false 
            AND status IN ('warning', 'alert')
        )
        "#
    )
    .fetch_one(local)
    .await?;

    Ok(alert_exists.unwrap_or(false))
}

async fn perform_full_sync(local: &Pool<Postgres>, primary: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    // 1. First establish the required station -> edge -> sensor hierarchy
    ensure_hierarchy(local, primary).await?;
    
    // 2. Sync the readings now that parents are guaranteed to exist
    sync_sensor_readings(local, primary).await?;
    
    // 3. Sync the metrics tied to the readings
    sync_metrics(local, primary).await?;
    
    Ok(())
}

async fn ensure_hierarchy(local: &Pool<Postgres>, primary: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    // Fetch unique hierarchy combinations needed by pending readings
    let missing_parents = sqlx::query!(
        r#"
        SELECT DISTINCT station_id, edge_id, sensor_id 
        FROM sensor_readings 
        WHERE is_synced = false
        "#
    )
    .fetch_all(local)
    .await?;

    for row in missing_parents {
        // Insert Station
        let _ = sqlx::query!(
            r#"
            INSERT INTO stations (station_id)
            VALUES ($1)
            ON CONFLICT (station_id) DO NOTHING
            "#,
            row.station_id
        )
        .execute(primary)
        .await;

        // Insert Edge
        let _ = sqlx::query!(
            r#"
            INSERT INTO edges (station_id, edge_id)
            VALUES ($1, $2)
            ON CONFLICT (station_id, edge_id) DO NOTHING
            "#,
            row.station_id, row.edge_id
        )
        .execute(primary)
        .await;

        // Insert Sensor
        let _ = sqlx::query!(
            r#"
            INSERT INTO sensors (station_id, edge_id, sensor_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (station_id, edge_id, sensor_id) DO NOTHING
            "#,
            row.station_id, row.edge_id, row.sensor_id
        )
        .execute(primary)
        .await;
    }
    
    Ok(())
}

async fn sync_sensor_readings(local: &Pool<Postgres>, primary: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let unsynced_rows = sqlx::query!(
        r#"
        SELECT id, station_id, edge_id, sensor_id, raw_timestamp, reading_time, a_x, a_y, a_z, hum, seis 
        FROM sensor_readings 
        WHERE is_synced = false 
        LIMIT 1000
        "#
    )
    .fetch_all(local)
    .await?;

    if unsynced_rows.is_empty() {
        return Ok(());
    }

    let mut synced_ids = Vec::new();

    for row in unsynced_rows {
        let result = sqlx::query!(
            r#"
            INSERT INTO sensor_readings (id, station_id, edge_id, sensor_id, raw_timestamp, reading_time, a_x, a_y, a_z, hum, seis)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (station_id, edge_id, sensor_id, raw_timestamp) DO NOTHING
            "#,
            row.id, row.station_id, row.edge_id, row.sensor_id, row.raw_timestamp, row.reading_time, 
            row.a_x, row.a_y, row.a_z, row.hum, row.seis
        )
        .execute(primary)
        .await;

        match result {
            Ok(_) => synced_ids.push(row.id),
            Err(e) => iotlogger!("Failed to insert reading {}: {}", row.id, e),
        }
    }

    if !synced_ids.is_empty() {
        sqlx::query!(
            "UPDATE sensor_readings SET is_synced = true WHERE id = ANY($1)",
            &synced_ids
        )
        .execute(local)
        .await?;
    }

    Ok(())
}

async fn sync_metrics(local: &Pool<Postgres>, primary: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let unsynced_rows = sqlx::query!(
        r#"
        SELECT 
            m.id, m.station_id, m.edge_id, m.sensor_id, m.reading_id, m.accel_mag, 
            m.seis, m.hum, m.accel_status, m.seis_status, m.hum_status, m.status, m.ts 
        FROM metrics m
        INNER JOIN sensor_readings sr ON m.reading_id = sr.id
        WHERE m.is_synced = false AND sr.is_synced = true
        LIMIT 1000
        "#
    )
    .fetch_all(local)
    .await?;

    if unsynced_rows.is_empty() {
        return Ok(());
    }

    let mut synced_ids = Vec::new();

    for row in unsynced_rows {
        let result = sqlx::query!(
            r#"
            INSERT INTO metrics (id, station_id, edge_id, sensor_id, reading_id, accel_mag, seis, hum, accel_status, seis_status, hum_status, status, ts)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (id) DO NOTHING
            "#,
            row.id, row.station_id, row.edge_id, row.sensor_id, row.reading_id, 
            row.accel_mag, row.seis, row.hum, row.accel_status, row.seis_status, 
            row.hum_status, row.status, row.ts
        )
        .execute(primary)
        .await;

        match result {
            Ok(_) => synced_ids.push(row.id),
            Err(e) => iotlogger!("Failed to insert metric {}: {}", row.id, e),
        }
    }

    if !synced_ids.is_empty() {
        sqlx::query!(
            "UPDATE metrics SET is_synced = true WHERE id = ANY($1)",
            &synced_ids
        )
        .execute(local)
        .await?;
    }

    Ok(())
}