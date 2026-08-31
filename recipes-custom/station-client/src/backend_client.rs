use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::station_client::EdgePacket;
use reqwest::Client;
use serde::Serialize;
use crate::iotlogger;


#[derive(Debug, Serialize)]
pub struct ApiPayload {
    pub station_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub data: Vec<EdgePacket>,
}

pub fn run(shared_data: Arc<Mutex<Vec<EdgePacket>>>, station_id: String, latitude: f64, longitude: f64) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to build backend runtime");

    rt.block_on(async {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;

            let buffered_data = {
                let mut lock = shared_data.lock().unwrap();
                std::mem::take(&mut *lock)
            };

            if buffered_data.is_empty() {
                iotlogger!("[Backend] No new data in the last minute.");
                continue;
            }

            iotlogger!("[Backend] Publishing {} packets to API...", buffered_data.len());

            if let Err(e) = send_payload(&station_id, latitude, longitude, &buffered_data).await {
                iotlogger!("[Backend] Network error publishing to API: {}", e);
            }
        }
    });
}

async fn send_payload(station_id: &str, latitude: f64, longitude: f64, packets: &[EdgePacket]) -> Result<(), reqwest::Error> {
    let payload = ApiPayload {
        station_id: station_id.to_string(),
        latitude,
        longitude,
        data: packets.to_vec(),
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client
        .post("http://127.0.0.1:5000/api/v1/ingest")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    if status.is_success() {
        iotlogger!("Successfully ingested data for station: {}", station_id);
    } else {
        let body = response.text().await.unwrap_or_default();
        iotlogger!("Failed to ingest data. Status: {}. Body: {}", status, body);
    }

    Ok(())
}
