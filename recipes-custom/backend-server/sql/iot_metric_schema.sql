-- =========================================================
-- IoT Metric schema: station -> edge (max 3) -> sensor (max 5)
-- Run with:  psql -U <user> -d iot-metric -f iot_metric_schema.sql
-- =========================================================

-- ---------------------------------------------------------
-- 1. STATIONS  (root of the tree)
-- ---------------------------------------------------------
CREATE TABLE stations (
    station_id      TEXT PRIMARY KEY,
    name            TEXT,
    latitude        DOUBLE PRECISION,
    longitude       DOUBLE PRECISION,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at    TIMESTAMPTZ
);

-- ---------------------------------------------------------
-- 2. EDGES  (max 3 per station -> edge_id 0..2)
-- ---------------------------------------------------------
CREATE TABLE edges (
    station_id      TEXT NOT NULL REFERENCES stations(station_id) ON DELETE CASCADE,
    edge_id         INTEGER NOT NULL CHECK (edge_id BETWEEN 0 AND 2),
    label           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at    TIMESTAMPTZ,
    PRIMARY KEY (station_id, edge_id)
);

-- ---------------------------------------------------------
-- 3. SENSORS  (registry only, max 5 per edge -> sensor_id 0..4)
-- ---------------------------------------------------------
CREATE TABLE sensors (
    station_id      TEXT NOT NULL,
    edge_id         INTEGER NOT NULL,
    sensor_id       INTEGER NOT NULL CHECK (sensor_id BETWEEN 0 AND 4),
    sensor_type     TEXT DEFAULT 'accel_hum_seis',
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at    TIMESTAMPTZ,
    PRIMARY KEY (station_id, edge_id, sensor_id),
    FOREIGN KEY (station_id, edge_id)
        REFERENCES edges(station_id, edge_id) ON DELETE CASCADE
);

-- ---------------------------------------------------------
-- 4. SENSOR_READINGS  (time-series, one row per reading)
--    raw_timestamp = milliseconds since epoch, assigned by the STATION
--    SERVER at packet-receipt time (SystemTime::now(), see station_client.rs)
--    -- not the edge device's clock, since the edge RTC isn't trustworthy.
--    All sensors within one EdgePacket share the same raw_timestamp, since
--    it's stamped once per packet, not per sensor reading.
--    reading_time is the same value converted to TIMESTAMPTZ, e.g. via
--    SQL to_timestamp(raw_timestamp / 1000.0) or
--    chrono::DateTime::from_timestamp_millis in Rust -- note the /1000,
--    this is milliseconds, unlike a raw time(NULL) which would be seconds.
-- ---------------------------------------------------------
CREATE TABLE sensor_readings (
    id              BIGSERIAL PRIMARY KEY,
    station_id      TEXT NOT NULL,
    edge_id         INTEGER NOT NULL,
    sensor_id       INTEGER NOT NULL,
    raw_timestamp   BIGINT NOT NULL,      -- original Sensor.timestamp (u64) as received
    reading_time    TIMESTAMPTZ NOT NULL, -- converted wall-clock time
    a_x             REAL NOT NULL,
    a_y             REAL NOT NULL,
    a_z             REAL NOT NULL,
    hum             REAL NOT NULL,
    seis            REAL NOT NULL,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT now(), -- when Postgres got the row
    FOREIGN KEY (station_id, edge_id, sensor_id)
        REFERENCES sensors(station_id, edge_id, sensor_id)
);

-- ---------------------------------------------------------
-- INDEXES
-- ---------------------------------------------------------
-- Fast per-sensor time-range queries (most common access pattern)
CREATE INDEX idx_readings_sensor_time
    ON sensor_readings (station_id, edge_id, sensor_id, reading_time DESC);

-- Fast "all stations, recent window" queries / dashboards
CREATE INDEX idx_readings_time
    ON sensor_readings (reading_time DESC);

-- Prevents duplicate inserts if a packet gets republished/retried.
-- Safe as a UNIQUE constraint because raw_timestamp is now millisecond-
-- resolution, assigned once per packet by the station server at receipt
-- time (see note above) -- not the edge device's own clock. Reads on a
-- given edge's TCP connection are sequential, so two packets from the same
-- (station, edge, sensor) landing on the same millisecond is effectively
-- impossible at realistic sample rates.
CREATE UNIQUE INDEX uq_readings_dedup
    ON sensor_readings (station_id, edge_id, sensor_id, raw_timestamp);

-- ---------------------------------------------------------
-- OPTIONAL: keep last_seen_at fresh automatically on insert
-- Skip this block if you'd rather update these fields yourself
-- from application code.
-- ---------------------------------------------------------
CREATE OR REPLACE FUNCTION touch_last_seen() RETURNS TRIGGER AS $$
BEGIN
    UPDATE sensors    SET last_seen_at = NEW.received_at
        WHERE station_id = NEW.station_id AND edge_id = NEW.edge_id AND sensor_id = NEW.sensor_id;
    UPDATE edges       SET last_seen_at = NEW.received_at
        WHERE station_id = NEW.station_id AND edge_id = NEW.edge_id;
    UPDATE stations    SET last_seen_at = NEW.received_at
        WHERE station_id = NEW.station_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_touch_last_seen
    AFTER INSERT ON sensor_readings
    FOR EACH ROW EXECUTE FUNCTION touch_last_seen();

