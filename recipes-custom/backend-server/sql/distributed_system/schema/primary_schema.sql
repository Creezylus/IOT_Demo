--
-- PostgreSQL database dump
--

\restrict YkVuZQyjjKHgwqTo8CtsJ2RbuhHsEhOy3fuPkKeRt54SNT6qrltC3EqPAamkq5u

-- Dumped from database version 14.24 (Ubuntu 14.24-0ubuntu0.22.04.1)
-- Dumped by pg_dump version 14.24 (Ubuntu 14.24-0ubuntu0.22.04.1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: edges; Type: TABLE; Schema: public; Owner: creezylus
--

CREATE TABLE public.edges (
    station_id text NOT NULL,
    edge_id integer NOT NULL,
    label text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_seen_at timestamp with time zone,
    CONSTRAINT edges_edge_id_check CHECK (((edge_id >= 0) AND (edge_id <= 2)))
);


ALTER TABLE public.edges OWNER TO creezylus;

--
-- Name: metrics; Type: TABLE; Schema: public; Owner: creezylus
--

CREATE TABLE public.metrics (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    station_id text NOT NULL,
    edge_id integer NOT NULL,
    sensor_id integer NOT NULL,
    reading_id uuid NOT NULL,
    accel_mag real NOT NULL,
    seis real NOT NULL,
    hum real NOT NULL,
    accel_status text NOT NULL,
    seis_status text NOT NULL,
    hum_status text NOT NULL,
    status text NOT NULL,
    ts timestamp with time zone NOT NULL,
    CONSTRAINT metrics_accel_status_check CHECK ((accel_status = ANY (ARRAY['normal'::text, 'warning'::text, 'alert'::text]))),
    CONSTRAINT metrics_hum_status_check CHECK ((hum_status = ANY (ARRAY['normal'::text, 'warning'::text, 'alert'::text]))),
    CONSTRAINT metrics_seis_status_check CHECK ((seis_status = ANY (ARRAY['normal'::text, 'warning'::text, 'alert'::text]))),
    CONSTRAINT metrics_status_check CHECK ((status = ANY (ARRAY['normal'::text, 'warning'::text, 'alert'::text])))
);


ALTER TABLE public.metrics OWNER TO creezylus;

--
-- Name: sensor_readings; Type: TABLE; Schema: public; Owner: creezylus
--

CREATE TABLE public.sensor_readings (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    station_id text NOT NULL,
    edge_id integer NOT NULL,
    sensor_id integer NOT NULL,
    raw_timestamp bigint NOT NULL,
    reading_time timestamp with time zone NOT NULL,
    a_x real NOT NULL,
    a_y real NOT NULL,
    a_z real NOT NULL,
    hum real NOT NULL,
    seis real NOT NULL,
    received_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.sensor_readings OWNER TO creezylus;

--
-- Name: sensors; Type: TABLE; Schema: public; Owner: creezylus
--

CREATE TABLE public.sensors (
    station_id text NOT NULL,
    edge_id integer NOT NULL,
    sensor_id integer NOT NULL,
    sensor_type text DEFAULT 'accel_hum_seis'::text,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_seen_at timestamp with time zone,
    CONSTRAINT sensors_sensor_id_check CHECK (((sensor_id >= 0) AND (sensor_id <= 4)))
);


ALTER TABLE public.sensors OWNER TO creezylus;

--
-- Name: station_locations; Type: TABLE; Schema: public; Owner: creezylus
--

CREATE TABLE public.station_locations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    station_id text NOT NULL,
    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    effective_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.station_locations OWNER TO creezylus;

--
-- Name: stations; Type: TABLE; Schema: public; Owner: creezylus
--

CREATE TABLE public.stations (
    station_id text NOT NULL,
    name text,
    latitude double precision,
    longitude double precision,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_seen_at timestamp with time zone
);


ALTER TABLE public.stations OWNER TO creezylus;

--
-- Name: edges edges_pkey; Type: CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.edges
    ADD CONSTRAINT edges_pkey PRIMARY KEY (station_id, edge_id);


--
-- Name: metrics metrics_pkey; Type: CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.metrics
    ADD CONSTRAINT metrics_pkey PRIMARY KEY (id);


--
-- Name: sensor_readings sensor_readings_pkey; Type: CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.sensor_readings
    ADD CONSTRAINT sensor_readings_pkey PRIMARY KEY (id);


--
-- Name: sensors sensors_pkey; Type: CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.sensors
    ADD CONSTRAINT sensors_pkey PRIMARY KEY (station_id, edge_id, sensor_id);


--
-- Name: station_locations station_locations_pkey; Type: CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.station_locations
    ADD CONSTRAINT station_locations_pkey PRIMARY KEY (id);


--
-- Name: stations stations_pkey; Type: CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.stations
    ADD CONSTRAINT stations_pkey PRIMARY KEY (station_id);


--
-- Name: idx_metrics_latest; Type: INDEX; Schema: public; Owner: creezylus
--

CREATE INDEX idx_metrics_latest ON public.metrics USING btree (station_id, edge_id, sensor_id, ts DESC);


--
-- Name: idx_metrics_status_ts; Type: INDEX; Schema: public; Owner: creezylus
--

CREATE INDEX idx_metrics_status_ts ON public.metrics USING btree (status, ts DESC) WHERE (status <> 'normal'::text);


--
-- Name: idx_readings_sensor_time; Type: INDEX; Schema: public; Owner: creezylus
--

CREATE INDEX idx_readings_sensor_time ON public.sensor_readings USING btree (station_id, edge_id, sensor_id, reading_time DESC);


--
-- Name: uq_readings_dedup; Type: INDEX; Schema: public; Owner: creezylus
--

CREATE UNIQUE INDEX uq_readings_dedup ON public.sensor_readings USING btree (station_id, edge_id, sensor_id, raw_timestamp);


--
-- Name: edges edges_station_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.edges
    ADD CONSTRAINT edges_station_id_fkey FOREIGN KEY (station_id) REFERENCES public.stations(station_id) ON DELETE CASCADE;


--
-- Name: metrics metrics_reading_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.metrics
    ADD CONSTRAINT metrics_reading_id_fkey FOREIGN KEY (reading_id) REFERENCES public.sensor_readings(id) ON DELETE CASCADE;


--
-- Name: metrics metrics_station_id_edge_id_sensor_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.metrics
    ADD CONSTRAINT metrics_station_id_edge_id_sensor_id_fkey FOREIGN KEY (station_id, edge_id, sensor_id) REFERENCES public.sensors(station_id, edge_id, sensor_id);


--
-- Name: sensor_readings sensor_readings_station_id_edge_id_sensor_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.sensor_readings
    ADD CONSTRAINT sensor_readings_station_id_edge_id_sensor_id_fkey FOREIGN KEY (station_id, edge_id, sensor_id) REFERENCES public.sensors(station_id, edge_id, sensor_id);


--
-- Name: sensors sensors_station_id_edge_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.sensors
    ADD CONSTRAINT sensors_station_id_edge_id_fkey FOREIGN KEY (station_id, edge_id) REFERENCES public.edges(station_id, edge_id) ON DELETE CASCADE;


--
-- Name: station_locations station_locations_station_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: creezylus
--

ALTER TABLE ONLY public.station_locations
    ADD CONSTRAINT station_locations_station_id_fkey FOREIGN KEY (station_id) REFERENCES public.stations(station_id);


--
-- PostgreSQL database dump complete
--

\unrestrict YkVuZQyjjKHgwqTo8CtsJ2RbuhHsEhOy3fuPkKeRt54SNT6qrltC3EqPAamkq5u

