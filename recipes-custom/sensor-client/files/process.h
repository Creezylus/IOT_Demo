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

typedef struct {
    float b0, b1, b2; // Feedforward (numerator)
    float a1, a2;     // Feedback (denominator)
    
    // State history
    float x1, x2;     // Previous two inputs: x[n-1], x[n-2]
    float y1, y2;     // Previous two outputs: y[n-1], y[n-2]
} BiquadFilter;


SensorData processSensorBucket(SensorData local_bucket[BUCKET_SIZE]);


#endif