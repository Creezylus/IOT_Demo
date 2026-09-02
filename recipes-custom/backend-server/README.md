
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


## TODOS 
- [x] Reduce latency in ingest path use UNNEST and cache data
- [x] Work on scaling DBs (Distributed dbs)
\t- [x]  Create Distribued DB
\t- [x]  Create Sync script
\t- [x]  Sync Dbs via HTTP
- [ ] Add Scaling of Webservers 
\t- [ ] Add LoadBalancer
\t- [ ] Add Kubernetes
- [ ] Create Replicas for Reading and Visualization
- [ ] Add Authentication 
- [ ] Work on scaling edge-clients and sensor-clients
- [ ] Make Threshold setting more dynamic.
- [ ] Add station ID, start time and end time params to `/status`
