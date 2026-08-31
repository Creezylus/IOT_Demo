#[path = "../../rust_tools/logger/log.rs"]
mod log;

mod db;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use db::models::{EdgeRow, SensorReadingRow, SensorRow, StationLocationRow, StationRow};
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

#[derive(Debug, Deserialize)]
struct ReadingsQuery {
    station_id: String,
    edge_id: Option<i32>,
    sensor_id: Option<i32>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<i64>,
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
        .route("/api/v1/stations", get(list_stations_handler))
        .route("/api/v1/stations/:station_id/locations", get(list_locations_handler))
        .route("/api/v1/stations/:station_id/edges", get(list_edges_handler))
        .route("/api/v1/stations/:station_id/edges/:edge_id/sensors", get(list_sensors_handler))
        .route("/api/v1/readings", get(list_readings_handler))
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

async fn list_stations_handler(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<StationRow>>, (StatusCode, String)> {
    stations::list_stations(&pool)
        .await
        .map(Json)
        .map_err(db_err("list stations"))
}

async fn list_locations_handler(
    State(pool): State<PgPool>,
    Path(station_id): Path<String>,
) -> Result<Json<Vec<StationLocationRow>>, (StatusCode, String)> {
    station_locations::list_locations(&pool, &station_id, 500)
        .await
        .map(Json)
        .map_err(db_err("list locations"))
}

async fn list_edges_handler(
    State(pool): State<PgPool>,
    Path(station_id): Path<String>,
) -> Result<Json<Vec<EdgeRow>>, (StatusCode, String)> {
    edges::list_edges(&pool, &station_id)
        .await
        .map(Json)
        .map_err(db_err("list edges"))
}

async fn list_sensors_handler(
    State(pool): State<PgPool>,
    Path((station_id, edge_id)): Path<(String, i32)>,
) -> Result<Json<Vec<SensorRow>>, (StatusCode, String)> {
    sensors::list_sensors(&pool, &station_id, edge_id)
        .await
        .map(Json)
        .map_err(db_err("list sensors"))
}

async fn list_readings_handler(
    State(pool): State<PgPool>,
    Query(params): Query<ReadingsQuery>,
) -> Result<Json<Vec<SensorReadingRow>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(1000).min(10000);

    sensor_readings::list_readings(
        &pool,
        &params.station_id,
        params.edge_id,
        params.sensor_id,
        params.from,
        params.to,
        limit,
    )
    .await
    .map(Json)
    .map_err(db_err("list readings"))
}

fn db_err(step: &'static str) -> impl Fn(sqlx::Error) -> (StatusCode, String) {
    move |e| {
        iotlogger!("DB step '{}' failed: {}", step, e);
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{step}: {e}"))
    }
}
