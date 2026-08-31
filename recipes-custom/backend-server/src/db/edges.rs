use sqlx::PgPool;

/// Register an edge under a station if it hasn't been seen before.
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
