#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pthread.h>
#include <sys/socket.h>
#include <arpa/inet.h>
#include <math.h>
#include <time.h>
#include "client.h"



// Shared Vars
SensorData bucket[BUCKET_SIZE];
int bucket_count = 0;
int client_id = 0; 
pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
pthread_cond_t cvar = PTHREAD_COND_INITIALIZER;


float random_float(float min, float max) {
    float scale = rand() / (float) RAND_MAX;
    return min + scale * (max - min);
}

// THREAD 1: Network & Processing Thread
void* net_thread_func(void* arg) {
    printf("Server Thread: Attempting to connect to %s:%d...\n", SERVER_ADDRESS, PORT);

    int sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock < 0) {
        printf("Server Thread: Socket creation error\n");
        return NULL;
    }

    struct sockaddr_in serv_addr;
    serv_addr.sin_family = AF_INET;
    serv_addr.sin_port = htons(PORT);
    
    if (inet_pton(AF_INET, SERVER_ADDRESS, &serv_addr.sin_addr) <= 0) {
        printf("Server Thread: Invalid address\n");
        return NULL;
    }

    if (connect(sock, (struct sockaddr *)&serv_addr, sizeof(serv_addr)) < 0) {
        printf("Server Thread: Failed to connect\n");
        return NULL;
    }
    
    printf("Server Thread: Successfully connected to server!\n");

    unsigned long long packet_num = 1;
    SensorData local_bucket[BUCKET_SIZE];

    while (1) {
        
        pthread_mutex_lock(&lock);
        while (bucket_count < BUCKET_SIZE) {
            pthread_cond_wait(&cvar, &lock);
        }
        
        memcpy(local_bucket, bucket, sizeof(SensorData) * BUCKET_SIZE);
        bucket_count = 0;
        pthread_mutex_unlock(&lock);
        SensorData avg_data = processSensorBucket(local_bucket);
        char message[256];
        snprintf(message, sizeof(message), 
            "#%llu [ID:%d] Accel=[%.3f, %.3f, %.3f] Hum=%.2f%% Seismo=%.4f mm/s\n",
            packet_num, avg_data.id, avg_data.accel_x, avg_data.accel_y, avg_data.accel_z, avg_data.humidity, avg_data.seismo);

        if (send(sock, &avg_data, sizeof(SensorData), 0) < 0) {
            printf("Server Thread: Failed to send data\n");
            break;
        } else {
            char print_msg[256];
            strcpy(print_msg, message);
            print_msg[strcspn(print_msg, "\n")] = 0;
            printf("Server Thread: Sent -> %s\n", print_msg);
        }

        packet_num++;
    }
    
    close(sock);
    return NULL;
}

// THREAD 2: Simulation Thread --- This Thread an also be used to gather data from connected sensors
void* sim_thread_func(void* arg) {
    printf("Sim Thread: Starting data generation...\n");
    srand((unsigned int)time(NULL));
    float time_step = 0.0f;
    while (1) {
        SensorData data;
        // 1. Accel: Continuous sinusoidal vibration + gravity baseline + sensor noise
        data.id = client_id; 
        data.accel_x = 0.5f * sin(time_step * 2.5f) + random_float(-0.5f, 0.5f); 
        data.accel_y = 0.3f * cos(time_step * 1.8f) + random_float(-0.5f, 0.5f);
        data.accel_z = 9.81f + 0.1f * sin(time_step * 5.0f) + random_float(-0.02f, 0.02f);
        
        // 2. Humidity: Extremely slow drift throughout the day
        data.humidity = 45.0f + 15.0f * sin(time_step * 0.001f) + random_float(-1.0f, 1.0f);
        
        // 3. Seismo: Low-level background rumble + 5% chance of a high-magnitude spike
        float background_rumble = fabs(0.1f * sin(time_step * 0.5f));
        float earthquake_spike = (rand() % 1000 < 50) ? random_float(3.0f, 8.0f) : 0.0f; 
        data.seismo = background_rumble + earthquake_spike + random_float(0.0f, 0.02f);

        
        pthread_mutex_lock(&lock);
        if (bucket_count < BUCKET_SIZE) {
            bucket[bucket_count++] = data;
        }

        // Notify the OTher Thread
        if (bucket_count >= BUCKET_SIZE) {
            pthread_cond_signal(&cvar);
        }
        pthread_mutex_unlock(&lock);

        //1 KHz
        usleep(1000); 
    }
    return NULL;
}

int main(int argc, char *argv[]) {
    
    if (argc < 2) {
        printf("Usage: %s <client_id>\n", argv[0]);
        return 1;
    }

    
    client_id = atoi(argv[1]);
    printf("Starting client with ID: %d\n", client_id);

    pthread_t net_thread, sim_thread;

    // Spawn the threads
    pthread_create(&net_thread, NULL, net_thread_func, NULL);
    pthread_create(&sim_thread, NULL, sim_thread_func, NULL);
    pthread_join(net_thread, NULL);
    pthread_join(sim_thread, NULL);

    return 0;
}