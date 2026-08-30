#ifndef CLIENT_H
#define CLIENT_H

#include <pthread.h>
#include "process.h"

#define SERVER_ADDRESS "127.0.0.1"
#define PORT 8080


// Struct to hold a single simulated reading
extern SensorData bucket[BUCKET_SIZE];
extern int bucket_count;
extern int client_id;
extern pthread_mutex_t lock;
extern pthread_cond_t cvar;


float random_float(float min, float max);
void* net_thread_func(void* arg);
void* sim_thread_func(void* arg);

#endif // CLIENT_H