mod station_client;
mod backend_client;

use std::thread;
use std::sync::{Arc, Mutex};
use std::env;

fn main() {
    // Read the station ID from the command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <station_id>", args[0]);
        std::process::exit(1);
    }
    let station_id = args[1].clone();

    // Shared state for safely passing EdgePackets across threads
    let shared_buffer = Arc::new(Mutex::new(Vec::new()));

    // Station Thread
    let station_data = Arc::clone(&shared_buffer);
    let station_handle = thread::spawn(move || {
        if let Err(e) = station_client::run(station_data) {
            eprintln!("Station client exited with error: {}", e);
        }
    });

    // Backend Thread
    let backend_data = Arc::clone(&shared_buffer);
    let backend_handle = thread::spawn(move || {
        // Pass the station_id to the backend client
        backend_client::run(backend_data, station_id);
    });

    station_handle.join().expect("Station client thread panicked"); 
    backend_handle.join().expect("Backend client thread panicked"); 
}