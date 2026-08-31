use sqlx::PgPool;

use super::models::StationRow;

pub async fn get_current_location(
    pool: &PgPool,
    station_id: &str,
) -> Result<Option<(f64, f64)>, sqlx::Error> {
    let row = sqlx::query_as::<_, (f64, f64)>(
        "SELECT latitude, longitude FROM stations WHERE station_id = $1",
    )
    .bind(station_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn create_station(
    pool: &PgPool,
    station_id: &str,
    latitude: f64,
    longitude: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO stations (station_id, name, latitude, longitude, last_seen_at) \
         VALUES ($1, $1, $2, $3, now()) \
         ON CONFLICT (station_id) DO NOTHING",
    )
    .bind(station_id)
    .bind(latitude)
    .bind(longitude)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_station_location(
    pool: &PgPool,
    station_id: &str,
    latitude: f64,
    longitude: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE stations SET latitude = $2, longitude = $3, last_seen_at = now() \
         WHERE station_id = $1",
    )
    .bind(station_id)
    .bind(latitude)
    .bind(longitude)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_stations(pool: &PgPool) -> Result<Vec<StationRow>, sqlx::Error> {
    sqlx::query_as::<_, StationRow>(
        "SELECT station_id, name, latitude, longitude, created_at, last_seen_at \
         FROM stations ORDER BY station_id",
    )
    .fetch_all(pool)
    .await
}
