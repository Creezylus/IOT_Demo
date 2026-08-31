use sqlx::PgPool;

use crate::SensorReading;

/// Insert one sensor reading. Deduplicates on
/// (station_id, edge_id, sensor_id, raw_timestamp) via the unique index,
/// matching the original inline query.
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
