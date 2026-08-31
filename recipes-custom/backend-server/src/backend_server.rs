use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

// Matches the JSON payload sent by the backend_client
#[derive(Debug, Deserialize)]
pub struct ApiPayload {
    pub station_id: String,
    pub data: Vec<EdgePacket>,
}

// Matches station_client::EdgePacket / Sensor on the wire (max 3 edges, max 5 sensors)
#[derive(Debug, Deserialize)]
pub struct EdgePacket {
    pub edge_id: i32,
    pub active_flags: [i32; 5],
    pub sensors: Vec<SensorReading>,
}

#[derive(Debug, Deserialize)]
pub struct SensorReading {
    pub id: i32,         
    pub timestamp: i64, 
    pub a_x: f32,
    pub a_y: f32,
    pub a_z: f32,
    pub hum: f32,
    pub seis: f32,
}

#[tokio::main]
async fn main() {
    //TODO get this from env
    let database_url = "postgres://creezylus:admin@localhost/iot_metrics";

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    let app = Router::new()
        .route("/api/v1/ingest", post(ingest_data))
        .with_state(pool);

    //TODO get this from env
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn ingest_data(
    State(pool): State<PgPool>,
    Json(payload): Json<ApiPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let station_id = &payload.station_id;

    // 1. Ensure Station Exists
    let _ = sqlx::query("INSERT INTO stations (station_id, name) VALUES ($1, $1) ON CONFLICT (station_id) DO NOTHING")
        .bind(station_id)
        .execute(&pool)
        .await;

    for packet in payload.data {
        // 2. Ensure Edge Exists
        let _ = sqlx::query("INSERT INTO edges (station_id, edge_id) VALUES ($1, $2) ON CONFLICT (station_id, edge_id) DO NOTHING")
            .bind(station_id)
            .bind(packet.edge_id)
            .execute(&pool)
            .await;

        for (i, sensor) in packet.sensors.iter().enumerate() {
            // Skip slots the edge marked inactive. 
            if packet.active_flags.get(i).copied().unwrap_or(0) != 1 {
                continue;
            }

            // 3. Ensure Sensor Exists
            let _ = sqlx::query("INSERT INTO sensors (station_id, edge_id, sensor_id) VALUES ($1, $2, $3) ON CONFLICT (station_id, edge_id, sensor_id) DO NOTHING")
                .bind(station_id)
                .bind(packet.edge_id)
                .bind(sensor.id)
                .execute(&pool)
                .await;

            // 4. Insert Reading

            let insert_result = sqlx::query(
                r#"
                INSERT INTO sensor_readings
                (station_id, edge_id, sensor_id, raw_timestamp, reading_time, a_x, a_y, a_z, hum, seis)
                VALUES ($1, $2, $3, $4, to_timestamp($4 / 1000.0), $5, $6, $7, $8, $9)
                ON CONFLICT (station_id, edge_id, sensor_id, raw_timestamp) DO NOTHING
                "#
            )
            .bind(station_id)
            .bind(packet.edge_id)
            .bind(sensor.id)
            .bind(sensor.timestamp)
            .bind(sensor.a_x)
            .bind(sensor.a_y)
            .bind(sensor.a_z)
            .bind(sensor.hum)
            .bind(sensor.seis)
            .execute(&pool)
            .await;

            if let Err(e) = insert_result {
                eprintln!("Failed to insert reading: {}", e);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
            }
        }
    }

    Ok(StatusCode::OK)
}