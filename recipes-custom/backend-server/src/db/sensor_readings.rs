use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::models::SensorReadingRow;
use crate::SensorReading;

pub async fn insert_reading(
    pool: &PgPool,
    station_id: &str,
    edge_id: i32,
    reading: &SensorReading,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO sensor_readings
        (station_id, edge_id, sensor_id, raw_timestamp, reading_time, a_x, a_y, a_z, hum, seis)
        VALUES ($1, $2, $3, $4, to_timestamp($4 / 1000.0), $5, $6, $7, $8, $9)
        ON CONFLICT (station_id, edge_id, sensor_id, raw_timestamp) DO NOTHING
        "#,
    )
    .bind(station_id)
    .bind(edge_id)
    .bind(reading.id)
    .bind(reading.timestamp)
    .bind(reading.a_x)
    .bind(reading.a_y)
    .bind(reading.a_z)
    .bind(reading.hum)
    .bind(reading.seis)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_readings(
    pool: &PgPool,
    station_id: &str,
    edge_id: Option<i32>,
    sensor_id: Option<i32>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<SensorReadingRow>, sqlx::Error> {
    sqlx::query_as::<_, SensorReadingRow>(
        r#"
        SELECT id, station_id, edge_id, sensor_id, raw_timestamp, reading_time,
               a_x, a_y, a_z, hum, seis, received_at
        FROM sensor_readings
        WHERE station_id = $1
          AND ($2::INTEGER IS NULL OR edge_id = $2)
          AND ($3::INTEGER IS NULL OR sensor_id = $3)
          AND ($4::TIMESTAMPTZ IS NULL OR reading_time >= $4)
          AND ($5::TIMESTAMPTZ IS NULL OR reading_time <= $5)
        ORDER BY reading_time DESC
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
