-- Metadata Tables (Populated locally or synced down)
CREATE TABLE stations (
    station_id      TEXT PRIMARY KEY,
    name            TEXT,
    latitude        DOUBLE PRECISION,
    longitude       DOUBLE PRECISION,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at    TIMESTAMPTZ
);

CREATE TABLE edges (
    station_id      TEXT NOT NULL REFERENCES stations(station_id) ON DELETE CASCADE,
    edge_id         INTEGER NOT NULL CHECK (edge_id BETWEEN 0 AND 2),
    label           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at    TIMESTAMPTZ,
    PRIMARY KEY (station_id, edge_id)
);

CREATE TABLE sensors (
    station_id      TEXT NOT NULL,
    edge_id         INTEGER NOT NULL,
    sensor_id       INTEGER NOT NULL CHECK (sensor_id BETWEEN 0 AND 4),
    sensor_type     TEXT DEFAULT 'accel_hum_seis',
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at    TIMESTAMPTZ,
    PRIMARY KEY (station_id, edge_id, sensor_id),
    FOREIGN KEY (station_id, edge_id) REFERENCES edges(station_id, edge_id) ON DELETE CASCADE
);

-- Data Tables with UUID PKs & Local Sync Tracking
CREATE TABLE station_locations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    station_id      TEXT NOT NULL REFERENCES stations(station_id),
    latitude        DOUBLE PRECISION NOT NULL,
    longitude       DOUBLE PRECISION NOT NULL,
    effective_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    is_synced       BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE sensor_readings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    station_id      TEXT NOT NULL,
    edge_id         INTEGER NOT NULL,
    sensor_id       INTEGER NOT NULL,
    raw_timestamp   BIGINT NOT NULL,
    reading_time    TIMESTAMPTZ NOT NULL,
    a_x             REAL NOT NULL,
    a_y             REAL NOT NULL,
    a_z             REAL NOT NULL,
    hum             REAL NOT NULL,
    seis            REAL NOT NULL,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    is_synced       BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (station_id, edge_id, sensor_id) REFERENCES sensors(station_id, edge_id, sensor_id)
);

CREATE TABLE metrics (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    station_id      TEXT NOT NULL,
    edge_id         INTEGER NOT NULL,
    sensor_id       INTEGER NOT NULL,
    reading_id      UUID NOT NULL REFERENCES sensor_readings(id) ON DELETE CASCADE,
    accel_mag       REAL NOT NULL,
    seis            REAL NOT NULL,
    hum             REAL NOT NULL,
    accel_status    TEXT NOT NULL CHECK (accel_status IN ('normal', 'warning', 'alert')),
    seis_status     TEXT NOT NULL CHECK (seis_status  IN ('normal', 'warning', 'alert')),
    hum_status      TEXT NOT NULL CHECK (hum_status   IN ('normal', 'warning', 'alert')),
    status          TEXT NOT NULL CHECK (status       IN ('normal', 'warning', 'alert')),
    ts              TIMESTAMPTZ NOT NULL,
    is_synced       BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (station_id, edge_id, sensor_id) REFERENCES sensors(station_id, edge_id, sensor_id)
);

-- Indexes
CREATE INDEX idx_station_locations_unsynced ON station_locations (is_synced) WHERE is_synced = FALSE;
CREATE INDEX idx_readings_unsynced ON sensor_readings (is_synced) WHERE is_synced = FALSE;
CREATE INDEX idx_metrics_unsynced ON metrics (is_synced) WHERE is_synced = FALSE;
CREATE UNIQUE INDEX uq_readings_dedup ON sensor_readings (station_id, edge_id, sensor_id, raw_timestamp);