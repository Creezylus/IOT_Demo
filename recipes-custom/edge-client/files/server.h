#ifndef SERVER_H
#define SERVER_H 

#ifndef __PACKED__ 
    #define __PACKED__ __attribute__((__packed__))
#endif

#define BUCKET_SIZE 1000
#define PORT 8080
#define BUFFER_SIZE 1024
#define MAX_CLIENTS 5
#define STATION_PORT 9090
#define STATION_IP "192.168.1.185" //TODO Make this dynamic
// #define STATION_IP "127.0.0.1" //TODO Make this dynamic

typedef struct __PACKED__{
    int id;
    unsigned long long timestamp;
    float accel_x;
    float accel_y;
    float accel_z;
    float humidity;
    float seismo;
} SensorData;

// Payload to send to the station
typedef struct __PACKED__{
    int edge_id;
    int active_flags[MAX_CLIENTS];   // 1 if slot has fresh data, 0 if empty/stale
    SensorData sensors[MAX_CLIENTS]; // Fixed slots for each sensor
} EdgePacket;

#endif //SERVER_H
