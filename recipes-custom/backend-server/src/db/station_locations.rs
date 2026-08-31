use sqlx::PgPool;

/// Append a row to the station_locations history table. Call this only
/// when the incoming (latitude, longitude) differs from what's on file,
/// so the history table only records actual moves.
pub async fn record_location_change(
    pool: &PgPool,
    station_id: &str,
    latitude: f64,
    longitude: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO station_locations (station_id, latitude, longitude) \
         VALUES ($1, $2, $3)",
    )
    .bind(station_id)
    .bind(latitude)
    .bind(longitude)
    .execute(pool)
    .await?;

    Ok(())
}
