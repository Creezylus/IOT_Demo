#ifndef SERVER_H
#define SERVER_H 


#define BUCKET_SIZE 1000
#define PORT 8080
#define BUFFER_SIZE 1024
#define MAX_CLIENTS 30

typedef struct {
    int id;
    float accel_x;
    float accel_y;
    float accel_z;
    float humidity;
    float seismo;
} SensorData;

#endif //SERVER_H