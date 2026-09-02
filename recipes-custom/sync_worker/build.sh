#SETUP THESE Before calling build
# Primary database connection variables
export PRIMARY_DB_HOST="localhost"
export PRIMARY_DB_PORT="5432"
export PRIMARY_DB_NAME="iot_metrics_primary"
export PRIMARY_DB_USER="creezylus"
export PRIMARY_DB_PASS="admin"
export PRIMARY_DB_PATH="public"

export LOCAL_DB_URL="postgres://creezylus:admin@localhost:5432/iot_metrics_station_1"
export PRIMARY_DATABASE_URL="postgres://creezylus:admin@localhost:5432/iot_metrics_primary"
# Compile-time database connection for sqlx macros
export DATABASE_URL="postgres://creezylus:admin@localhost:5432/iot_metrics_station_1"

cargo clean
cargo build
