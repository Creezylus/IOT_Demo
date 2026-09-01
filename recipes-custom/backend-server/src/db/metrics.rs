use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::models::MetricRow;

pub const ACCEL_WARN: f32 = 18.0; // magnitude, same units as a_x/a_y/a_z
pub const ACCEL_ALERT: f32 = 20.0;

pub const SEIS_WARN: f32 = 1.0;
pub const SEIS_ALERT: f32 = 3.0;

pub const HUM_WARN: f32 = 50.0; // % relative humidity
pub const HUM_ALERT: f32 = 95.0;

/// Classifies a single value against its warning/alert thresholds.
/// >= alert -> "alert", >= warn -> "warning", otherwise -> "normal".
pub fn classify(value: f32, warn: f32, alert: f32) -> &'static str {
    if value >= alert {
        "alert"
    } else if value >= warn {
        "warning"
    } else {
        "normal"
    }
}

/// Overall status = the worst (most severe) of the given per-metric statuses.
pub fn worst_status(statuses: &[&str]) -> &'static str {
    if statuses.iter().any(|s| *s == "alert") {
        "alert"
    } else if statuses.iter().any(|s| *s == "warning") {
        "warning"
    } else {
        "normal"
    }
}

/// Vector magnitude of the three accelerometer axes.
pub fn accel_magnitude(a_x: f32, a_y: f32, a_z: f32) -> f32 {
    (a_x * a_x + a_y * a_y + a_z * a_z).sqrt()
}

pub struct ComputedMetric {
    pub accel_mag: f32,
    pub accel_status: &'static str,
    pub seis_status: &'static str,
    pub hum_status: &'static str,
    pub status: &'static str,
}

/// Computes accel magnitude and all four statuses for one reading.
pub fn compute(a_x: f32, a_y: f32, a_z: f32, seis: f32, hum: f32) -> ComputedMetric {
    let accel_mag = accel_magnitude(a_x, a_y, a_z);
    let accel_status = classify(accel_mag, ACCEL_WARN, ACCEL_ALERT);
    let seis_status = classify(seis, SEIS_WARN, SEIS_ALERT);
    let hum_status = classify(hum, HUM_WARN, HUM_ALERT);
    let status = worst_status(&[accel_status, seis_status, hum_status]);

    ComputedMetric {
        accel_mag,
        accel_status,
        seis_status,
        hum_status,
        status,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_metric(
    pool: &PgPool,
    station_id: &str,
    edge_id: i32,
    sensor_id: i32,
    reading_id: i64,
    accel_mag: f32,
    seis: f32,
    hum: f32,
    accel_status: &str,
    seis_status: &str,
    hum_status: &str,
    status: &str,
    ts: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO metrics
        (station_id, edge_id, sensor_id, reading_id, accel_mag, seis, hum,
         accel_status, seis_status, hum_status, status, ts)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(station_id)
    .bind(edge_id)
    .bind(sensor_id)
    .bind(reading_id)
    .bind(accel_mag)
    .bind(seis)
    .bind(hum)
    .bind(accel_status)
    .bind(seis_status)
    .bind(hum_status)
    .bind(status)
    .bind(ts)
    .execute(pool)
    .await?;

    Ok(())
}

/// Historical metrics for a station, filterable like `sensor_readings::list_readings`.
pub async fn list_metrics(
    pool: &PgPool,
    station_id: &str,
    edge_id: Option<i32>,
    sensor_id: Option<i32>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<MetricRow>, sqlx::Error> {
    sqlx::query_as::<_, MetricRow>(
        r#"
        SELECT id, station_id, edge_id, sensor_id, reading_id, accel_mag, seis, hum,
               accel_status, seis_status, hum_status, status, ts
        FROM metrics
        WHERE station_id = $1
          AND ($2::INTEGER IS NULL OR edge_id = $2)
          AND ($3::INTEGER IS NULL OR sensor_id = $3)
          AND ($4::TIMESTAMPTZ IS NULL OR ts >= $4)
          AND ($5::TIMESTAMPTZ IS NULL OR ts <= $5)
        ORDER BY ts DESC
        LIMIT $6
        "#,
    )
    .bind(station_id)
    .bind(edge_id)
    .bind(sensor_id)
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_latest_status(
    pool: &PgPool,
    station_id: Option<&str>,
) -> Result<Vec<MetricRow>, sqlx::Error> {
    sqlx::query_as::<_, MetricRow>(
        r#"
        SELECT DISTINCT ON (station_id, edge_id, sensor_id)
               id, station_id, edge_id, sensor_id, reading_id, accel_mag, seis, hum,
               accel_status, seis_status, hum_status, status, ts
        FROM metrics
        WHERE ($1::TEXT IS NULL OR station_id = $1)
        ORDER BY station_id, edge_id, sensor_id, ts DESC
        "#,
    )
    .bind(station_id)
    .fetch_all(pool)
    .await
}
