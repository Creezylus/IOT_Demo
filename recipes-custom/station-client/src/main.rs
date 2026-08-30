mod station_client;

use std::thread;

fn main() {
    // Station client runs on its own OS thread with its own tokio runtime,
    // independent of anything else spawned here.
    let station_handle = thread::spawn(|| {
        if let Err(e) = station_client::run() {
            eprintln!("Station client exited with error: {}", e);
        }
    });

    // TODO: spawn a second thread here later for the next piece of work.

    station_handle.join().expect("Station client thread panicked");
}