CREATE TABLE station_locations (
    id BIGSERIAL PRIMARY KEY,
    station_id TEXT NOT NULL REFERENCES stations(station_id),
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    effective_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_station_locations_station_id_effective_at
    ON station_locations (station_id, effective_at DESC);