-- =========================================================
-- Metrics: computed status (normal / warning / alert) per sensor reading
-- Depends on: iot_metric_schema.sql (sensors, sensor_readings)
-- Run with:  psql -U <user> -d iot_metrics -f add_metrics.sql
-- =========================================================

-- One row per sensor_readings row that was actually inserted (not a
-- deduped retry). Holds the derived accel magnitude plus a status per
-- metric and an overall worst-of status, so dashboards can query this
-- table directly instead of recomputing on read.
CREATE TABLE metrics (
    id              BIGSERIAL PRIMARY KEY,
    station_id      TEXT NOT NULL,
    edge_id         INTEGER NOT NULL,
    sensor_id       INTEGER NOT NULL,
    reading_id      BIGINT NOT NULL REFERENCES sensor_readings(id) ON DELETE CASCADE,

    accel_mag       REAL NOT NULL,   -- sqrt(a_x^2 + a_y^2 + a_z^2)
    seis            REAL NOT NULL,
    hum             REAL NOT NULL,

    accel_status    TEXT NOT NULL CHECK (accel_status IN ('normal', 'warning', 'alert')),
    seis_status     TEXT NOT NULL CHECK (seis_status  IN ('normal', 'warning', 'alert')),
    hum_status      TEXT NOT NULL CHECK (hum_status   IN ('normal', 'warning', 'alert')),
    status          TEXT NOT NULL CHECK (status       IN ('normal', 'warning', 'alert')), -- TODO Add flags instead? 0x7,0x6...

    ts              TIMESTAMPTZ NOT NULL, -- = sensor_readings.reading_time

    FOREIGN KEY (station_id, edge_id, sensor_id)
        REFERENCES sensors(station_id, edge_id, sensor_id)
);

-- "Latest status per sensor" dashboard query (DISTINCT ON station_id, edge_id, sensor_id ORDER BY ts DESC)
CREATE INDEX idx_metrics_latest
    ON metrics (station_id, edge_id, sensor_id, ts DESC);

-- Fast lookup of everything currently in warning/alert
CREATE INDEX idx_metrics_status_ts
    ON metrics (status, ts DESC)
    WHERE status <> 'normal';
