#!/bin/bash

#Storing here for future ref.
export DATABASE_URL="postgres://creezylus:admin@localhost/iot_metrics"
export SERVER_ADDRESS="0.0.0.0:5000"

case "$1" in
    run)
        echo "Starting IoT services..."

        ../backend-server/target/debug/backend-server &
        sleep 2

        ../station-client/target/debug/station_client 2 50.5 10.1 &
        sleep 2

        ../edge-client/files/edge_client 1 &
        sleep 2

        ../sensor-client/files/sensor_client 1 &

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
        echo "Usage: $0 {run|stop}"
        exit 1
        ;;
esac
