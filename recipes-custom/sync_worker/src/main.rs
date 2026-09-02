use dotenvy::dotenv;
use reqwest::Client;

use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::env;
use std::time::{Duration, Instant};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


#[path = "../../rust_tools/logger/log.rs"]
mod log;

const POLL_INTERVAL_SECS: u64 = 5;
const NORMAL_SYNC_INTERVAL_SECS: u64 = 60; 

#[derive(Serialize, Deserialize, Debug)]
pub struct SensorReading {
    pub id: Uuid,
    pub station_id: String,
    pub edge_id: i32,
    pub sensor_id: i32,
    pub raw_timestamp: i64,
    pub reading_time: DateTime<Utc>,
    pub a_x: f32,
    pub a_y: f32,
    pub a_z: f32,
    pub hum: f32,
    pub seis: f32,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Metric {
    pub id: Uuid,
    pub station_id: String,
    pub edge_id: i32,
    pub sensor_id: i32,
    pub reading_id: Uuid,
    pub accel_mag: f32,
    pub seis: f32,
    pub hum: f32,
    pub accel_status: String,
    pub seis_status: String,
    pub hum_status: String,
    pub status: String,
    pub ts: DateTime<Utc>,
}

#[derive(Serialize, Debug)]
pub struct SyncPayload {
    pub readings: Vec<SensorReading>,
    pub metrics: Vec<Metric>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let local_url = env::var("DATABASE_URL").expect("LOCAL_DB_URL must be set");
    let sync_server_url = env::var("SYNC_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/sync".to_string());

    let local_pool = PgPoolOptions::new().connect(&local_url).await?;
    let http_client = Client::new();

    iotlogger!("Connected to local database. Starting dynamic HTTP sync loop...");

    let mut last_normal_sync = Instant::now() - Duration::from_secs(NORMAL_SYNC_INTERVAL_SECS);

    loop {
        let has_alerts = check_for_alerts(&local_pool).await.unwrap_or(false);
        let time_since_last_sync = last_normal_sync.elapsed().as_secs();

        if has_alerts || time_since_last_sync >= NORMAL_SYNC_INTERVAL_SECS {
            iotlogger!(
                "Triggering HTTP sync. (Alert detected: {}, Time since last sync: {}s)",
                has_alerts, time_since_last_sync
            );

            match perform_full_sync(&local_pool, &http_client, &sync_server_url).await {
                Ok(synced_count) => {
                    if synced_count > 0 {
                        iotlogger!("Sync complete. Pushed {} items.", synced_count);
                    }
                    last_normal_sync = Instant::now();
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

async fn perform_full_sync(
    local: &Pool<Postgres>,
    client: &Client,
    server_url: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    
    // 1. Fetch Unsynced Data
    let readings = sqlx::query_as!(
        SensorReading,
        r#"
        SELECT id, station_id, edge_id, sensor_id, raw_timestamp, reading_time, a_x, a_y, a_z, hum, seis 
        FROM sensor_readings 
        WHERE is_synced = false 
        LIMIT 1000
        "#
    )
    .fetch_all(local)
    .await?;

    let metrics = sqlx::query_as!(
        Metric,
        r#"
        SELECT 
            m.id, m.station_id, m.edge_id, m.sensor_id, m.reading_id, m.accel_mag, 
            m.seis, m.hum, m.accel_status, m.seis_status, m.hum_status, m.status, m.ts 
        FROM metrics m
        WHERE m.is_synced = false
        LIMIT 1000
        "#
    )
    .fetch_all(local)
    .await?;

    let total_items = readings.len() + metrics.len();
    if total_items == 0 {
        return Ok(0); // Nothing to sync
    }

    // Capture IDs to mark as synced later
    let reading_ids: Vec<Uuid> = readings.iter().map(|r| r.id).collect();
    let metric_ids: Vec<Uuid> = metrics.iter().map(|m| m.id).collect();

    let payload = SyncPayload { readings, metrics };

    // 2. Send via HTTP
    let response = client.post(server_url)
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        // 3. Update Local DB on Success
        if !reading_ids.is_empty() {
            sqlx::query!("UPDATE sensor_readings SET is_synced = true WHERE id = ANY($1)", &reading_ids)
                .execute(local)
                .await?;
        }

        if !metric_ids.is_empty() {
            sqlx::query!("UPDATE metrics SET is_synced = true WHERE id = ANY($1)", &metric_ids)
                .execute(local)
                .await?;
        }
        Ok(total_items)
    } else {
        Err(format!("Server returned HTTP {}", response.status()).into())
    }
}