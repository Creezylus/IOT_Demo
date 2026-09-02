use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use std::net::SocketAddr;
use uuid::Uuid;

#[path = "../../rust_tools/logger/log.rs"]
mod log;

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct SyncPayload {
    pub readings: Vec<SensorReading>,
    pub metrics: Vec<Metric>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&primary_url)
        .await?;

    iotlogger!("Sync Server connected to Primary DB. Starting HTTP server...");

    let app = Router::new()
        .route("/sync", post(handle_sync))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8088));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    iotlogger!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_sync(
    State(pool): State<PgPool>,
    Json(payload): Json<SyncPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    
    // Start a transaction to ensure all or nothing
    let mut tx = pool.begin().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    // 1. Ensure Hierarchy
    for reading in &payload.readings {
        sqlx::query!(
            "INSERT INTO stations (station_id) VALUES ($1) ON CONFLICT (station_id) DO NOTHING",
            reading.station_id
        ).execute(&mut *tx).await.ok();

        sqlx::query!(
            "INSERT INTO edges (station_id, edge_id) VALUES ($1, $2) ON CONFLICT (station_id, edge_id) DO NOTHING",
            reading.station_id, reading.edge_id
        ).execute(&mut *tx).await.ok();

        sqlx::query!(
            "INSERT INTO sensors (station_id, edge_id, sensor_id) VALUES ($1, $2, $3) ON CONFLICT (station_id, edge_id, sensor_id) DO NOTHING",
            reading.station_id, reading.edge_id, reading.sensor_id
        ).execute(&mut *tx).await.ok();
    }

    // 2. Insert Readings
    for row in &payload.readings {
        sqlx::query!(
            r#"
            INSERT INTO sensor_readings (id, station_id, edge_id, sensor_id, raw_timestamp, reading_time, a_x, a_y, a_z, hum, seis)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (station_id, edge_id, sensor_id, raw_timestamp) DO NOTHING
            "#,
            row.id, row.station_id, row.edge_id, row.sensor_id, row.raw_timestamp, row.reading_time, 
            row.a_x, row.a_y, row.a_z, row.hum, row.seis
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // 3. Insert Metrics
    for row in &payload.metrics {
        sqlx::query!(
            r#"
            INSERT INTO metrics (id, station_id, edge_id, sensor_id, reading_id, accel_mag, seis, hum, accel_status, seis_status, hum_status, status, ts)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (id) DO NOTHING
            "#,
            row.id, row.station_id, row.edge_id, row.sensor_id, row.reading_id, 
            row.accel_mag, row.seis, row.hum, row.accel_status, row.seis_status, 
            row.hum_status, row.status, row.ts
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    tx.commit().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok(StatusCode::OK)
}