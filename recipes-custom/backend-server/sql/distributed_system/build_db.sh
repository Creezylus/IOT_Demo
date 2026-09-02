#!/usr/bin/env bash
set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <postgres_username>"
    exit 1
fi

USER="$1"
PRIMARY_DB="iot_metrics_primary"
STATION_DB="iot_metrics_station_1"

echo "Creating databases..."
sudo -u postgres createdb "$PRIMARY_DB" || true
sudo -u postgres createdb "$STATION_DB" || true

# ---------------------------------------------------------
# 1. APPLY PRIMARY DATABASE SCHEMA
# ---------------------------------------------------------
echo "Setting up $PRIMARY_DB..."
psql -U "$USER" -d "$PRIMARY_DB" -h localhost << 'EOF'
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

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

CREATE TABLE station_locations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    station_id      TEXT NOT NULL REFERENCES stations(station_id),
    latitude        DOUBLE PRECISION NOT NULL,
    longitude       DOUBLE PRECISION NOT NULL,
    effective_at    TIMESTAMPTZ NOT NULL DEFAULT now()
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
    FOREIGN KEY (station_id, edge_id, sensor_id) REFERENCES sensors(station_id, edge_id, sensor_id)
);

CREATE INDEX idx_metrics_latest ON metrics (station_id, edge_id, sensor_id, ts DESC);
CREATE INDEX idx_metrics_status_ts ON metrics (status, ts DESC) WHERE status <> 'normal';
CREATE INDEX idx_readings_sensor_time ON sensor_readings (station_id, edge_id, sensor_id, reading_time DESC);
CREATE UNIQUE INDEX uq_readings_dedup ON sensor_readings (station_id, edge_id, sensor_id, raw_timestamp);
EOF

# ---------------------------------------------------------
# 2. APPLY STATION DATABASE SCHEMA (WITH IS_SYNCED FLAGS)
# ---------------------------------------------------------
echo "Setting up $STATION_DB..."
psql -U "$USER" -d "$STATION_DB" -h localhost << 'EOF'
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

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

CREATE INDEX idx_station_locations_unsynced ON station_locations (is_synced) WHERE is_synced = FALSE;
CREATE INDEX idx_readings_unsynced ON sensor_readings (is_synced) WHERE is_synced = FALSE;
CREATE INDEX idx_metrics_unsynced ON metrics (is_synced) WHERE is_synced = FALSE;
CREATE UNIQUE INDEX uq_readings_dedup ON sensor_readings (station_id, edge_id, sensor_id, raw_timestamp);
EOF

echo "Database initialization complete!"