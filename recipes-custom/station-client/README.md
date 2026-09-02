# IoT Station Ingestion Service

A Rust application that acts as an intermediary station server. It receives binary TCP telemetry packets from edge clients, processes and normalizes timestamps, buffers the incoming data, and periodically flushes the buffered payload to a central backend API via HTTP POST every 60 seconds.

---

### Key Features

* **Binary TCP Ingestion:** Binds to a local TCP socket on port `9090` to read binary packed C structures (`EdgePacket`).
* **Concurrency Limits:** Utilizes asynchronous Tokio tasks and semaphores to cap maximum concurrent edge clients to 3.
* **Timestamp Normalization:** Overwrites incoming sensor timestamps with system time to ensure temporal consistency.
* **Periodic Batching:** Aggregates incoming telemetry in a thread-safe shared buffer and posts data to `/api/v1/ingest` every minute.

---

### Environment Variables

The application requires the following environment variables:

* `STATION_IP`: The IP address for the local TCP listener.
* `API_BASE_URL`: Base URL of the backend REST API (e.g., `http://127.0.0.1:8000`).

---

### Usage

**Command-Line Arguments:**
The application takes 3 positional arguments[cite: 2]:
1. `station_id` (String)[cite: 1, 2]
2. `latitude` (Float / f64)[cite: 1, 2]
3. `longitude` (Float / f64)[cite: 1, 2]

```bash
# Set environment variables
export STATION_IP="127.0.0.1"
export API_BASE_URL="http://localhost:<your_port>"

# Run the station application
cargo run -- <station_id> <latitude> <longitude>

# Example
cargo run -- station_01 37.7749 -122.4194