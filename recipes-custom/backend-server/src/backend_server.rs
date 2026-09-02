#[path = "../../rust_tools/logger/log.rs"] // Cheesyyy fix this laterr..
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
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

use db::models::{
    EdgeRow, MetricRow, SensorReadingRow, SensorRow, StationLocationRow, StationRow,
};
use db::{edges, metrics, sensor_readings, sensors, station_locations, stations};

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApiPayload {
    pub station_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub data: Vec<EdgePacket>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EdgePacket {
    pub edge_id: i32,
    #[schema(value_type = Vec<i32>)]
    pub active_flags: [i32; 5],
    pub sensors: Vec<SensorReading>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SensorReading {
    pub id: i32,
    pub timestamp: i64,
    pub a_x: f32,
    pub a_y: f32,
    pub a_z: f32,
    pub hum: f32,
    pub seis: f32,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ReadingsQuery {
    station_id: String,
    edge_id: Option<i32>,
    sensor_id: Option<i32>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct MetricsQuery {
    station_id: String,
    edge_id: Option<i32>,
    sensor_id: Option<i32>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct StatusQuery {
    station_id: Option<String>,
}

/// Aggregated OpenAPI document for the seismic sensor network API.
#[derive(OpenApi)]
#[openapi(
    paths(
        ingest_data,
        list_stations_handler,
        list_locations_handler,
        list_edges_handler,
        list_sensors_handler,
        list_readings_handler,
        list_metrics_handler,
        current_status_handler,
    ),
    components(schemas(
        ApiPayload,
        EdgePacket,
        SensorReading,
        StationRow,
        StationLocationRow,
        EdgeRow,
        SensorRow,
        SensorReadingRow,
        MetricRow,
    )),
    tags(
        (name = "seismic-api", description = "Sensor network ingestion & query API")
    )
)]
struct ApiDoc;

// Cached application state to minimize repetitive existence upserts
struct AppState {
    pool: PgPool,
    station_locations: RwLock<HashMap<String, (f64, f64)>>,
    seen_edges: RwLock<HashSet<(String, i32)>>,
    seen_sensors: RwLock<HashSet<(String, i32, i32)>>,
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

    let state = Arc::new(AppState {
        pool,
        station_locations: RwLock::new(HashMap::new()),
        seen_edges: RwLock::new(HashSet::new()),
        seen_sensors: RwLock::new(HashSet::new()),
    });

    let app = Router::new()
        .route("/api/v1/ingest", post(ingest_data))
        .route("/api/v1/stations", get(list_stations_handler))
        .route("/api/v1/stations/:station_id/locations", get(list_locations_handler))
        .route("/api/v1/stations/:station_id/edges", get(list_edges_handler))
        .route("/api/v1/stations/:station_id/edges/:edge_id/sensors", get(list_sensors_handler))
        .route("/api/v1/readings", get(list_readings_handler))
        .route("/api/v1/metrics", get(list_metrics_handler))
        .route("/api/v1/status", get(current_status_handler))
        .with_state(state)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let server_address = std::env::var("SERVER_ADDRESS").expect("SERVER_ADDRESS environment variable is not set");
    let listener = tokio::net::TcpListener::bind(&server_address).await.unwrap();
    iotlogger!("SERVER_ADDRESS: {}", server_address);
    iotlogger!("Swagger UI available at http://{}/swagger-ui", server_address);

    axum::serve(listener, app).await.unwrap();
}

#[utoipa::path(
    post,
    path = "/api/v1/ingest",
    tag = "seismic-api",
    request_body = ApiPayload,
    responses(
        (status = 200, description = "Data ingested successfully"),
        (status = 500, description = "Internal server error", body = String),
    )
)]
async fn ingest_data(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ApiPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let station_id = payload.station_id.as_str();

    // 1. Station location caching
    let mut location_changed = false;
    let mut needs_station_upsert = false;
    {
        let mut locs = state.station_locations.write().unwrap();
        match locs.get(station_id) {
            Some(&(lat, lon)) if lat == payload.latitude && lon == payload.longitude => {}
            Some(_) => {
                locs.insert(station_id.to_string(), (payload.latitude, payload.longitude));
                location_changed = true;
            }
            None => {
                locs.insert(station_id.to_string(), (payload.latitude, payload.longitude));
                needs_station_upsert = true;
                location_changed = true;
            }
        }
    }

    if needs_station_upsert {
        let existing = stations::get_current_location(&state.pool, station_id)
            .await.map_err(db_err("lookup station"))?;

        if existing.is_none() {
            stations::create_station(&state.pool, station_id, payload.latitude, payload.longitude)
                .await.map_err(db_err("create station"))?;
        } else {
            stations::update_station_location(&state.pool, station_id, payload.latitude, payload.longitude)
                .await.map_err(db_err("update station"))?;
        }
    } else if location_changed {
        stations::update_station_location(&state.pool, station_id, payload.latitude, payload.longitude)
            .await.map_err(db_err("update station"))?;
    }

    if location_changed {
        station_locations::record_location_change(&state.pool, station_id, payload.latitude, payload.longitude)
            .await.map_err(db_err("record location change"))?;
    }

    // 2. Edge/Sensor hierarchy caching
    let mut missing_edges = Vec::new();
    let mut missing_sensors = Vec::new();

    for packet in &payload.data {
        let edge_key = (station_id.to_string(), packet.edge_id);
        {
            if !state.seen_edges.read().unwrap().contains(&edge_key) {
                missing_edges.push(packet.edge_id);
            }
        }
        for (i, sensor) in packet.sensors.iter().enumerate() {
            if packet.active_flags.get(i).copied().unwrap_or(0) != 1 { continue; }
            let sensor_key = (station_id.to_string(), packet.edge_id, sensor.id);
            if !state.seen_sensors.read().unwrap().contains(&sensor_key) {
                missing_sensors.push((packet.edge_id, sensor.id));
            }
        }
    }

    if !missing_edges.is_empty() {
        for edge_id in &missing_edges {
            edges::upsert_edge(&state.pool, station_id, *edge_id).await.map_err(db_err("upsert edge"))?;
        }
        let mut edges_write = state.seen_edges.write().unwrap();
        for edge_id in missing_edges {
            edges_write.insert((station_id.to_string(), edge_id));
        }
    }

    if !missing_sensors.is_empty() {
        for (edge_id, sensor_id) in &missing_sensors {
            sensors::upsert_sensor(&state.pool, station_id, *edge_id, *sensor_id).await.map_err(db_err("upsert sensor"))?;
        }
        let mut sensors_write = state.seen_sensors.write().unwrap();
        for (edge_id, sensor_id) in missing_sensors {
            sensors_write.insert((station_id.to_string(), edge_id, sensor_id));
        }
    }

    // 3. Bulk Insert & Computed Metrics Mapping
    let mut tx = state.pool.begin().await.map_err(db_err("begin transaction"))?;

    let cap = payload.data.len() * 5;
    let mut edge_ids = Vec::with_capacity(cap);
    let mut sensor_ids = Vec::with_capacity(cap);
    let mut raw_tss = Vec::with_capacity(cap);
    let mut axs = Vec::with_capacity(cap);
    let mut ays = Vec::with_capacity(cap);
    let mut azs = Vec::with_capacity(cap);
    let mut hums = Vec::with_capacity(cap);
    let mut seises = Vec::with_capacity(cap);
    let mut accel_mags = Vec::with_capacity(cap);
    let mut accel_statuses = Vec::with_capacity(cap);
    let mut seis_statuses = Vec::with_capacity(cap);
    let mut hum_statuses = Vec::with_capacity(cap);
    let mut statuses = Vec::with_capacity(cap);

    let mut deduplicator = HashSet::new();

    for packet in &payload.data {
        for (i, sensor) in packet.sensors.iter().enumerate() {
            if packet.active_flags.get(i).copied().unwrap_or(0) != 1 { continue; }
            
            // Defend against duplicates within the immediate payload
            if !deduplicator.insert((packet.edge_id, sensor.id, sensor.timestamp)) {
                continue;
            }

            edge_ids.push(packet.edge_id);
            sensor_ids.push(sensor.id);
            raw_tss.push(sensor.timestamp);
            axs.push(sensor.a_x);
            ays.push(sensor.a_y);
            azs.push(sensor.a_z);
            hums.push(sensor.hum);
            seises.push(sensor.seis);

            // Calculate metric logic in Rust before inserting
            let computed = db::metrics::compute(sensor.a_x, sensor.a_y, sensor.a_z, sensor.seis, sensor.hum);
            accel_mags.push(computed.accel_mag);
            accel_statuses.push(computed.accel_status);
            seis_statuses.push(computed.seis_status);
            hum_statuses.push(computed.hum_status);
            statuses.push(computed.status);
        }
    }

    if !edge_ids.is_empty() {
   
        let query = r#"
            WITH new_readings AS (
                SELECT * FROM UNNEST(
                    $1::int[], $2::int[], $3::bigint[],
                    $4::real[], $5::real[], $6::real[], $7::real[], $8::real[],
                    $9::real[], $10::text[], $11::text[], $12::text[], $13::text[]
                ) AS t(edge_id, sensor_id, raw_ts, ax, ay, az, hum, seis, accel_mag, accel_status, seis_status, hum_status, status)
            ),
            inserted_readings AS (
                INSERT INTO sensor_readings (
                    station_id, edge_id, sensor_id, raw_timestamp, reading_time, a_x, a_y, a_z, hum, seis
                )
                SELECT
                    $14, edge_id, sensor_id, raw_ts, to_timestamp(raw_ts / 1000.0), ax, ay, az, hum, seis
                FROM new_readings
                ON CONFLICT (station_id, edge_id, sensor_id, raw_timestamp) DO NOTHING
                RETURNING id, edge_id, sensor_id, raw_timestamp, reading_time
            )
            INSERT INTO metrics (
                station_id, edge_id, sensor_id, reading_id, accel_mag, seis, hum,
                accel_status, seis_status, hum_status, status, ts
            )
            SELECT
                $14, ir.edge_id, ir.sensor_id, ir.id, nr.accel_mag, nr.seis, nr.hum,
                nr.accel_status, nr.seis_status, nr.hum_status, nr.status, ir.reading_time
            FROM inserted_readings ir
            JOIN new_readings nr
              ON ir.edge_id = nr.edge_id
             AND ir.sensor_id = nr.sensor_id
             AND ir.raw_timestamp = nr.raw_ts
        "#;

        sqlx::query(query)
            .bind(&edge_ids)
            .bind(&sensor_ids)
            .bind(&raw_tss)
            .bind(&axs)
            .bind(&ays)
            .bind(&azs)
            .bind(&hums)
            .bind(&seises)
            .bind(&accel_mags)
            .bind(&accel_statuses)
            .bind(&seis_statuses)
            .bind(&hum_statuses)
            .bind(&statuses)
            .bind(station_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err("bulk insert readings and metrics"))?;
    }

    tx.commit().await.map_err(db_err("commit transaction"))?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/v1/stations",
    tag = "seismic-api",
    responses(
        (status = 200, description = "List of all stations", body = Vec<StationRow>),
        (status = 500, description = "Internal server error", body = String),
    )
)]
async fn list_stations_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<StationRow>>, (StatusCode, String)> {
    stations::list_stations(&state.pool)
        .await
        .map(Json)
        .map_err(db_err("list stations"))
}

#[utoipa::path(
    get,
    path = "/api/v1/stations/{station_id}/locations",
    tag = "seismic-api",
    params(
        ("station_id" = String, Path, description = "Station identifier"),
    ),
    responses(
        (status = 200, description = "Location history for a station", body = Vec<StationLocationRow>),
        (status = 500, description = "Internal server error", body = String),
    )
)]
async fn list_locations_handler(
    State(state): State<Arc<AppState>>,
    Path(station_id): Path<String>,
) -> Result<Json<Vec<StationLocationRow>>, (StatusCode, String)> {
    station_locations::list_locations(&state.pool, &station_id, 500)
        .await
        .map(Json)
        .map_err(db_err("list locations"))
}

#[utoipa::path(
    get,
    path = "/api/v1/stations/{station_id}/edges",
    tag = "seismic-api",
    params(
        ("station_id" = String, Path, description = "Station identifier"),
    ),
    responses(
        (status = 200, description = "Edges belonging to a station", body = Vec<EdgeRow>),
        (status = 500, description = "Internal server error", body = String),
    )
)]
async fn list_edges_handler(
    State(state): State<Arc<AppState>>,
    Path(station_id): Path<String>,
) -> Result<Json<Vec<EdgeRow>>, (StatusCode, String)> {
    edges::list_edges(&state.pool, &station_id)
        .await
        .map(Json)
        .map_err(db_err("list edges"))
}

#[utoipa::path(
    get,
    path = "/api/v1/stations/{station_id}/edges/{edge_id}/sensors",
    tag = "seismic-api",
    params(
        ("station_id" = String, Path, description = "Station identifier"),
        ("edge_id" = i32, Path, description = "Edge identifier"),
    ),
    responses(
        (status = 200, description = "Sensors belonging to an edge", body = Vec<SensorRow>),
        (status = 500, description = "Internal server error", body = String),
    )
)]
async fn list_sensors_handler(
    State(state): State<Arc<AppState>>,
    Path((station_id, edge_id)): Path<(String, i32)>,
) -> Result<Json<Vec<SensorRow>>, (StatusCode, String)> {
    sensors::list_sensors(&state.pool, &station_id, edge_id)
        .await
        .map(Json)
        .map_err(db_err("list sensors"))
}

#[utoipa::path(
    get,
    path = "/api/v1/readings",
    tag = "seismic-api",
    params(ReadingsQuery),
    responses(
        (status = 200, description = "Sensor readings matching the filter", body = Vec<SensorReadingRow>),
        (status = 500, description = "Internal server error", body = String),
    )
)]
async fn list_readings_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ReadingsQuery>,
) -> Result<Json<Vec<SensorReadingRow>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(1000).min(10000);

    sensor_readings::list_readings(
        &state.pool,
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

#[utoipa::path(
    get,
    path = "/api/v1/metrics",
    tag = "seismic-api",
    params(MetricsQuery),
    responses(
        (status = 200, description = "Computed metrics (accel magnitude, seis, hum, statuses) matching the filter", body = Vec<MetricRow>),
        (status = 500, description = "Internal server error", body = String),
    )
)]
async fn list_metrics_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MetricsQuery>,
) -> Result<Json<Vec<MetricRow>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(1000).min(10000);

    metrics::list_metrics(
        &state.pool,
        &params.station_id,
        params.edge_id,
        params.sensor_id,
        params.from,
        params.to,
        limit,
    )
    .await
    .map(Json)
    .map_err(db_err("list metrics"))
}

#[utoipa::path(
    get,
    path = "/api/v1/status",
    tag = "seismic-api",
    params(StatusQuery),
    responses(
        (status = 200, description = "Latest status per station/edge/sensor -- for a dashboard view", body = Vec<MetricRow>),
        (status = 500, description = "Internal server error", body = String),
    )
)]
async fn current_status_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<StatusQuery>,
) -> Result<Json<Vec<MetricRow>>, (StatusCode, String)> {
    metrics::list_latest_status(&state.pool, params.station_id.as_deref())
        .await
        .map(Json)
        .map_err(db_err("current status"))
}

fn db_err(step: &'static str) -> impl Fn(sqlx::Error) -> (StatusCode, String) {
    move |e| {
        iotlogger!("DB step '{}' failed: {}", step, e);
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{step}: {e}"))
    }
}
