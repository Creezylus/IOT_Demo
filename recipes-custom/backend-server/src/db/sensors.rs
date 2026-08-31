use sqlx::PgPool;

/// Register a sensor under a station/edge if it hasn't been seen before.
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
