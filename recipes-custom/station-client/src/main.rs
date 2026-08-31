mod station_client;
mod backend_client;

use std::env;
use std::sync::{Arc, Mutex};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <station_id> <latitude> <longitude>", args[0]);
        std::process::exit(1);
    }

    let station_id = args[1].clone();
    let latitude: f64 = args[2].parse().expect("Invalid latitude");
    let longitude: f64 = args[3].parse().expect("Invalid longitude");

    let shared_data = Arc::new(Mutex::new(Vec::new()));
    let station_data = Arc::clone(&shared_data);

    std::thread::spawn(move || {
        station_client::run(station_data).expect("Station client failed");
    });

    backend_client::run(shared_data, station_id, latitude, longitude);
}