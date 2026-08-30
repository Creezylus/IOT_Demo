#ifndef PROCESS_H
#define PROCESS_H

#define BUCKET_SIZE 1000

typedef struct {
    int id;
    float accel_x;
    float accel_y;
    float accel_z;
    float humidity;
    float seismo;
} SensorData;


SensorData processSensorBucket(SensorData local_bucket[BUCKET_SIZE]);

#endif