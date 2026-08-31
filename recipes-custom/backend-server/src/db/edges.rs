use sqlx::PgPool;

use super::models::EdgeRow;

pub async fn upsert_edge(
    pool: &PgPool,
    station_id: &str,
    edge_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO edges (station_id, edge_id) VALUES ($1, $2) \
         ON CONFLICT (station_id, edge_id) DO NOTHING",
    )
    .bind(station_id)
    .bind(edge_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_edges(pool: &PgPool, station_id: &str) -> Result<Vec<EdgeRow>, sqlx::Error> {
    sqlx::query_as::<_, EdgeRow>(
        "SELECT station_id, edge_id, label, created_at, last_seen_at \
         FROM edges WHERE station_id = $1 ORDER BY edge_id",
    )
    .bind(station_id)
    .fetch_all(pool)
    .await
}
