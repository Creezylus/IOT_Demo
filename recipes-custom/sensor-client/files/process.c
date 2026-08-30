#include "process.h"

SensorData processSensorBucket(SensorData local_bucket[BUCKET_SIZE]) {
    float sum_accel_x = 0.0, sum_accel_y = 0.0, sum_accel_z = 0.0;
    float sum_humidity = 0.0, sum_seismo = 0.0;
    int current_id = local_bucket[0].id; 
    // Accumulate all values
    for (int i = 0; i < BUCKET_SIZE; i++) {
        sum_accel_x += local_bucket[i].accel_x;
        sum_accel_y += local_bucket[i].accel_y;
        sum_accel_z += local_bucket[i].accel_z;
        sum_humidity += local_bucket[i].humidity;
        sum_seismo += local_bucket[i].seismo;
    }

    
    float count = (float)BUCKET_SIZE;
    SensorData avg_data;
    avg_data.id = current_id;
    avg_data.accel_x = sum_accel_x / count;
    avg_data.accel_y = sum_accel_y / count;
    avg_data.accel_z = sum_accel_z / count;
    avg_data.humidity = sum_humidity / count;
    avg_data.seismo = sum_seismo / count;

    return avg_data;
}