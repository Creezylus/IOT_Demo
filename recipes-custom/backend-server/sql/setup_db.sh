if [ -z "$1" ]; then
    echo "Usage: $0 <username>"
    exit 1
fi
USER="$1"
sudo -u postgres createdb iot_metrics
psql -U "$USER" -d iot_metrics -h localhost -f iot_metric_schema.sql
psql -U "$USER" -d iot_metrics -h localhost -f add_staion_locations.sql
psql -U "$USER" -d iot_metrics -h localhost -f add_metrics.sql

