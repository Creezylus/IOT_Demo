#!/bin/bash

#Storing here for future ref and convenience.
export DATABASE_URL="postgres://creezylus:admin@localhost/iot_metrics_station_1" #VERY VAD PRACTISE BUT KEEPING THIS FOR READER's UNDERSTANDING
export SERVER_ADDRESS="0.0.0.0:5000" #THESE PORTS and IPs CAN BE CHANGED TO SUIT YOUR MACHINE
export API_BASE_URL="http://127.0.0.1:5000"
export STATION_IP="192.168.1.185"
export PRIMARY_DB_HOST="localhost"
export PRIMARY_DB_PORT="5432"
export PRIMARY_DB_NAME="iot_metrics_primary"
export PRIMARY_DB_USER="creezylus"
export PRIMARY_DB_PASS="admin"
export PRIMARY_DB_PATH="public"

#FOR Sync worker
export LOCAL_DB_URL="postgres://creezylus:admin@localhost:5432/iot_metrics_station_1"
export PRIMARY_DATABASE_URL="postgres://creezylus:admin@localhost:5432/iot_metrics_primary"
export DATABASE_URL="postgres://creezylus:admin@localhost:5432/iot_metrics_station_1"


case "$1" in
    start)
        echo "Starting IoT services..."
        ../backend-server/target/debug/backend-server &
        sleep 2
        ../station-client/target/debug/station_client 6 50.5 10.1 &
        sleep 2
        ../edge-client/files/edge_client 1 &
        sleep 2
        ../sensor-client/files/sensor_client 1 &
	    sleep 1
        ../sensor-client/files/sensor_client 2 &
	    sleep 1
        ../sensor-client/files/sensor_client 3 &
        sleep 1
        ../sync_worker/target/debug/sync-worker &


        echo "All services started."
        ;;

    stop)
        echo "Stopping IoT services..."
        killall backend-server 2>/dev/null
        killall station_client 2>/dev/null
        killall edge_client 2>/dev/null
        killall sensor_client 2>/dev/null
        echo "All services stopped."
        ;;

    *)
        echo "Usage: $0 {start|stop}"
        exit 1
        ;;
esac
