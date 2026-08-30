#include "process.h"
static BiquadFilter seismo_filter;
static int is_filter_initialized = 0;

void init_bandpass_filter(BiquadFilter *f, float b0, float b1, float b2, float a1, float a2) {
    f->b0 = b0; f->b1 = b1; f->b2 = b2;
    f->a1 = a1; f->a2 = a2;
    f->x1 = 0.0f; f->x2 = 0.0f;
    f->y1 = 0.0f; f->y2 = 0.0f;
}

float apply_filter(BiquadFilter *f, float input) {
    // Calculate the filtered output
    float output = (f->b0 * input) + (f->b1 * f->x1) + (f->b2 * f->x2) 
                 - (f->a1 * f->y1) - (f->a2 * f->y2);

    // Shift state arrays forward for the next sample
    f->x2 = f->x1;
    f->x1 = input;
    f->y2 = f->y1;
    f->y1 = output;

    return output;
}


SensorData processSensorBucket(SensorData local_bucket[BUCKET_SIZE]) {
    if (!is_filter_initialized) {
        init_bandpass_filter(&seismo_filter, 0.02f, 0.0f, -0.02f, -1.95f, 0.96f);
        is_filter_initialized = 1;
    }

    float sum_accel_x = 0.0, sum_accel_y = 0.0, sum_accel_z = 0.0;
    float sum_humidity = 0.0, sum_seismo = 0.0, sum_seismo_raw = 0.0;
    int current_id = local_bucket[0].id; 
    
    // Accumulate all values
    for (int i = 0; i < BUCKET_SIZE; i++) {
        sum_accel_x += local_bucket[i].accel_x;
        sum_accel_y += local_bucket[i].accel_y;
        sum_accel_z += local_bucket[i].accel_z;
        sum_humidity += local_bucket[i].humidity;
        sum_seismo_raw += local_bucket[i].seismo;

        float filtered_seismo = apply_filter(&seismo_filter, local_bucket[i].seismo);
        sum_seismo += fabs(filtered_seismo);
    }

    float count = (float)BUCKET_SIZE;
    SensorData avg_data;
    avg_data.id = current_id;
    avg_data.accel_x = sum_accel_x / count;
    avg_data.accel_y = sum_accel_y / count;
    avg_data.accel_z = sum_accel_z / count;
    avg_data.humidity = sum_humidity / count;
    avg_data.seismo = sum_seismo / count;
    sum_seismo_raw /= count;
    printf("Raw Avg data: %.3f\n", sum_seismo_raw);
    return avg_data;
}
