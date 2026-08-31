use sqlx::PgPool;

use super::models::SensorRow;

pub async fn upsert_sensor(
    pool: &PgPool,
    station_id: &str,
    edge_id: i32,
    sensor_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sensors (station_id, edge_id, sensor_id) VALUES ($1, $2, $3) \
         ON CONFLICT (station_id, edge_id, sensor_id) DO NOTHING",
    )
    .bind(station_id)
    .bind(edge_id)
    .bind(sensor_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_sensors(
    pool: &PgPool,
    station_id: &str,
    edge_id: i32,
) -> Result<Vec<SensorRow>, sqlx::Error> {
    sqlx::query_as::<_, SensorRow>(
        "SELECT station_id, edge_id, sensor_id, sensor_type, is_active, created_at, last_seen_at \
         FROM sensors WHERE station_id = $1 AND edge_id = $2 ORDER BY sensor_id",
    )
    .bind(station_id)
    .bind(edge_id)
    .fetch_all(pool)
    .await
}
