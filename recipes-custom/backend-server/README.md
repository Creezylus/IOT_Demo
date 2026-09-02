
# IoT Metric Service

A backend for a seismic/environmental sensor network. Stations report location
and readings from up to 3 edge devices, each with up to 5 sensors
(accelerometer XYZ, humidity, seismic). The service ingests readings, computes
a normal / warning / alert status per reading, and exposes it all over a REST
API with Swagger docs.

## Data model

```

| Table               | Purpose                                                                 |
|---------------------|--------------------------------------------------------------------------|
| `stations`          | One row per station; current lat/lon, name, last-seen.                  |
| `station_locations` | Append-only history of location changes (new row only when lat/lon change). |
| `edges`             | Up to 3 per station, `edge_id` 0–2.                                      |
| `sensors`           | Registry of sensors, up to 5 per edge, `sensor_id` 0–4.                 |
| `sensor_readings`   | Raw time series: accel XYZ, humidity, seismic, per sensor.              |
| `metrics`           | One row per reading with accel magnitude + a computed status per metric (`normal`/`warning`/`alert`) and an overall worst-of `status`. |

`sensor_readings` is deduplicated on `(station_id, edge_id, sensor_id, raw_timestamp)`
so retried/republished packets don't create duplicate rows — or duplicate
`metrics` rows, since a metric is only written when a reading is newly
inserted.

## Setup

Requires PostgreSQL and Rust (stable toolchain).

```bash
./setup_db.sh <your_postgres_username>
```

This creates the `iot_metrics` database and runs, in order:
1. `iot_metric_schema.sql` — stations, edges, sensors, sensor_readings
2. `add_staion_locations.sql` — station_locations
3. `add_metrics.sql` — metrics

Then set environment variables and run the server:

```bash
export DATABASE_URL="postgres://<user>@localhost/iot_metrics"
export SERVER_ADDRESS="0.0.0.0:8080"
cargo run
```

Swagger UI: `http://<SERVER_ADDRESS>/swagger-ui`

## API

| Method | Path                                              | Description                                   |
|--------|----------------------------------------------------|------------------------------------------------|
| POST   | `/api/v1/ingest`                                   | Ingest a station's location + edge/sensor packets. |
| GET    | `/api/v1/stations`                                 | List all stations.                              |
| GET    | `/api/v1/stations/:station_id/locations`           | Location history for a station.                 |
| GET    | `/api/v1/stations/:station_id/edges`               | Edges for a station.                            |
| GET    | `/api/v1/stations/:station_id/edges/:edge_id/sensors` | Sensors for an edge.                         |
| GET    | `/api/v1/readings?station_id=...`                  | Raw sensor readings (filter by edge/sensor/time range). |
| GET    | `/api/v1/metrics?station_id=...`                   | Computed metrics history (same filters as `/readings`). |
| GET    | `/api/v1/status[?station_id=...]`                  | Latest status per sensor — for a dashboard. Omit `station_id` for the whole fleet. |

### Ingest payload

```json
{
  "station_id": "stn-042",
  "latitude": 60.7212,
  "longitude": -135.0568,
  "data": [
    {
      "edge_id": 0,
      "active_flags": [1, 1, 0, 0, 0],
      "sensors": [
        { "id": 0, "timestamp": 1735689600000, "a_x": 0.1, "a_y": 0.2, "a_z": 9.8, "hum": 45.0, "seis": 0.3 },
        { "id": 1, "timestamp": 1735689600000, "a_x": 0.0, "a_y": 0.1, "a_z": 9.7, "hum": 44.5, "seis": 0.2 }
      ]
    }
  ]
}
```

`active_flags[i]` gates whether `sensors[i]` is actually processed — only
flagged sensors get upserted, inserted, and scored. `timestamp` is
milliseconds since epoch, assigned by the station server at packet-receipt
time (not the edge device's own clock).

## Status thresholds

Each reading is scored on three metrics, each independently classified as
`normal`, `warning`, or `alert` against a warning/alert threshold pair. The
reading's overall `status` is the worst of the three.

- **accel** — magnitude of the three axes: `sqrt(a_x² + a_y² + a_z²)`
- **seis** — seismic value, as reported
- **hum** — humidity, as reported

Thresholds are defined as constants in `db/metrics.rs` and should be tuned to
your actual sensor calibration and site requirements before relying on them.

## TODOS 
- [x] Reduce latency in ingest path
- [ ] Add station ID, start time and end time params to `/status`
- [ ] Work on scaling DBs (Distributed dbs and Replicas for read)
\t- [x]  Create Distribued DB
\t- [x]  Create Sync script
\t- [ ]  Sync Dbs via HTTP
- [ ] Add Scaling of Webservers 
- [ ] Add Scaling of LoadBalancer
- [ ] Add Authentication 
- [ ] Work on scaling edge-clients and sensor-clients
- [ ] Make Threshold setting more dynamic.

