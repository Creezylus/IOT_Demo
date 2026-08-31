#[path = "../../rust_tools/logger/log.rs"]
mod log;

mod db;

use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use db::{edges, sensor_readings, sensors, station_locations, stations};

#[derive(Debug, Deserialize)]
pub struct ApiPayload {
    pub station_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub data: Vec<EdgePacket>,
}

#[derive(Debug, Deserialize)]
pub struct EdgePacket {
    pub edge_id: i32,
    pub active_flags: [i32; 5],
    pub sensors: Vec<SensorReading>,
}

#[derive(Debug, Deserialize)]
pub struct SensorReading {
    pub id: i32,
    pub timestamp: i64,
    pub a_x: f32,
    pub a_y: f32,
    pub a_z: f32,
    pub hum: f32,
    pub seis: f32,
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable is not set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    let app = Router::new()
        .route("/api/v1/ingest", post(ingest_data))
        .with_state(pool);

    let server_address = std::env::var("SERVER_ADDRESS").expect("SERVER_ADDRESS environment variable is not set");
    let listener = tokio::net::TcpListener::bind(&server_address).await.unwrap();
    iotlogger!("SERVER_ADDRESS: {}", server_address);

    axum::serve(listener, app).await.unwrap();
}

async fn ingest_data(
    State(pool): State<PgPool>,
    Json(payload): Json<ApiPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let station_id = payload.station_id.as_str();

    upsert_station_location(&pool, station_id, payload.latitude, payload.longitude).await?;

    for packet in &payload.data {
        ingest_edge_packet(&pool, station_id, packet).await?;
    }

    Ok(StatusCode::OK)
}

async fn upsert_station_location(
    pool: &PgPool,
    station_id: &str,
    latitude: f64,
    longitude: f64,
) -> Result<(), (StatusCode, String)> {
    let existing = stations::get_current_location(pool, station_id)
        .await
        .map_err(db_err("lookup station"))?;

    let location_changed = match existing {
        Some((lat, lon)) => lat != latitude || lon != longitude,
        None => true,
    };

    match existing {
        None => {
            stations::create_station(pool, station_id, latitude, longitude)
                .await
                .map_err(db_err("create station"))?;
        }
        Some(_) => {
            stations::update_station_location(pool, station_id, latitude, longitude)
                .await
                .map_err(db_err("update station"))?;
        }
    }

    if location_changed {
        station_locations::record_location_change(pool, station_id, latitude, longitude)
            .await
            .map_err(db_err("record location change"))?;
    }

    Ok(())
}

/// sensor reading in the packet.
async fn ingest_edge_packet(
    pool: &PgPool,
    station_id: &str,
    packet: &EdgePacket,
) -> Result<(), (StatusCode, String)> {
    edges::upsert_edge(pool, station_id, packet.edge_id)
        .await
        .map_err(db_err("upsert edge"))?;

    for (i, sensor) in packet.sensors.iter().enumerate() {
        if packet.active_flags.get(i).copied().unwrap_or(0) != 1 {
            continue;
        }

        ingest_sensor_reading(pool, station_id, packet.edge_id, sensor).await?;
    }

    Ok(())
}

async fn ingest_sensor_reading(
    pool: &PgPool,
    station_id: &str,
    edge_id: i32,
    sensor: &SensorReading,
) -> Result<(), (StatusCode, String)> {
    sensors::upsert_sensor(pool, station_id, edge_id, sensor.id)
        .await
        .map_err(db_err("upsert sensor"))?;

    sensor_readings::insert_reading(pool, station_id, edge_id, sensor)
        .await
        .map_err(db_err("insert reading"))?;

    Ok(())
}

fn db_err(step: &'static str) -> impl Fn(sqlx::Error) -> (StatusCode, String) {
    move |e| {
        iotlogger!("DB step '{}' failed: {}", step, e);
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{step}: {e}"))
    }
}
