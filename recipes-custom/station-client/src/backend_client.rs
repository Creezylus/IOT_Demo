use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::station_client::EdgePacket;
use reqwest::Client;
use serde::Serialize;

// Added payload wrapper to include station_id in the outgoing JSON
#[derive(Debug, Serialize)]
pub struct ApiPayload {
    pub station_id: String,
    pub data: Vec<EdgePacket>,
}

/// Entry point for the backend client's OS thread.
/// Owns a dedicated tokio runtime for API network requests.
pub fn run(shared_data: Arc<Mutex<Vec<EdgePacket>>>, station_id: String) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to build backend runtime");

    rt.block_on(async {
        loop {
            // Buffer time: Wait exactly 1 minute
            tokio::time::sleep(Duration::from_secs(60)).await;

            let buffered_data = {
                let mut lock = shared_data.lock().unwrap();
                std::mem::take(&mut *lock)
            };

            if buffered_data.is_empty() {
                println!("[Backend] No new data in the last minute.");
                continue;
            }

            println!("[Backend] Publishing {} packets to API...", buffered_data.len());

            if let Err(e) = send_payload(&station_id, &buffered_data).await {
                eprintln!("[Backend] Network error publishing to API: {}", e);
            }
        }
    });
}

async fn send_payload(station_id: &str, packets: &[EdgePacket]) -> Result<(), reqwest::Error> {
    let payload = ApiPayload {
        station_id: station_id.to_string(),
        data: packets.to_vec(),
    };

    // Build a client with a 10-second timeout to prevent hanging connections
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client
        .post("http://127.0.0.1:3000/api/v1/ingest")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    if status.is_success() {
        println!("Successfully ingested data for station: {}", station_id);
    } else {
        // Read the body so failures are actually diagnosable -- Axum's JSON
        // rejection includes the exact field/path that failed to deserialize.
        let body = response.text().await.unwrap_or_default();
        eprintln!("Failed to ingest data. Status: {}. Body: {}", status, body);
    }

    Ok(())
}