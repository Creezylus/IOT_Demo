use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct StationRow {
    pub station_id: String,
    pub name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct StationLocationRow {
    pub id: i64,
    pub station_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub effective_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct EdgeRow {
    pub station_id: String,
    pub edge_id: i32,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct SensorRow {
    pub station_id: String,
    pub edge_id: i32,
    pub sensor_id: i32,
    pub sensor_type: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct SensorReadingRow {
    pub id: i64,
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
    pub received_at: DateTime<Utc>,
}

/// Computed status for a single sensor reading: accel magnitude, seis,
/// and hum each checked against thresholds (normal / warning / alert),
/// plus an overall `status` that is the worst of the three.
#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct MetricRow {
    pub id: i64,
    pub station_id: String,
    pub edge_id: i32,
    pub sensor_id: i32,
    pub reading_id: i64,
    pub accel_mag: f32,
    pub seis: f32,
    pub hum: f32,
    pub accel_status: String,
    pub seis_status: String,
    pub hum_status: String,
    pub status: String,
    pub ts: DateTime<Utc>,
}
