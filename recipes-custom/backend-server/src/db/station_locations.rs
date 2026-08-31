use sqlx::PgPool;

use super::models::StationLocationRow;

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

pub async fn list_locations(
    pool: &PgPool,
    station_id: &str,
    limit: i64,
) -> Result<Vec<StationLocationRow>, sqlx::Error> {
    sqlx::query_as::<_, StationLocationRow>(
        "SELECT id, station_id, latitude, longitude, effective_at \
         FROM station_locations WHERE station_id = $1 \
         ORDER BY effective_at DESC LIMIT $2",
    )
    .bind(station_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}
