#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <time.h>
#include <errno.h>
#include "server.h"
#include "log.h"

// --- Helper Functions ---
long long current_timestamp_ms() {
    struct timeval te; 
    gettimeofday(&te, NULL);
    return te.tv_sec * 1000LL + te.tv_usec / 1000;
}

#ifdef DO_DEBUG
void print_edge_packet(const EdgePacket *packet) {
    iotlogger("\n=== Sending Edge Packet to Station (Edge ID: %d), Total size of Edge Packet: %d ===\n", packet->edge_id, sizeof(EdgePacket));
    
    for (int i = 0; i < MAX_CLIENTS; i++) {        
        const char* status = packet->active_flags[i] ? "FRESH" : "STALE/EMPTY";
        
        iotlogger("  [Slot %d - %s] Sensor ID: %d | Time: %llu \n", 
               i, status, packet->sensors[i].id, packet->sensors[i].timestamp);
        iotlogger("             Accel: (%.3f, %.3f, %.3f) | Hum: %.3f | Seis: %.3f\n",
               packet->sensors[i].accel_x,
               packet->sensors[i].accel_y,
               packet->sensors[i].accel_z,
               packet->sensors[i].humidity,
               packet->sensors[i].seismo);
    }
    iotlogger("========================================================\n\n");
}
#endif

// --- Main ---
int main(int argc, char *argv[]) {
    // 1. Parse Edge ID
    if (argc != 2) {
        iotlogger("Usage: %s <edge_id>\n", argv[0]);
        exit(EXIT_FAILURE);
    }

    int edge_id = atoi(argv[1]);
    iotlogger("Startingz Edge Client [ID: %d]\n", edge_id);

    // 2. Setup Station Client Socket (Connecting to Upstream Server)
    int station_client = socket(AF_INET, SOCK_STREAM, 0);
    if (station_client < 0) {
        perror("Station socket creation failed");
        exit(EXIT_FAILURE);
    }
    
    struct sockaddr_in station_addr;
    station_addr.sin_family = AF_INET;
    station_addr.sin_port = htons(STATION_PORT);
    if (inet_pton(AF_INET, STATION_IP, &station_addr.sin_addr) <= 0) {
        perror("Invalid station address");
        exit(EXIT_FAILURE);
    }

    iotlogger("Connecting to Station at %s:%d...\n", STATION_IP, STATION_PORT);
    if (connect(station_client, (struct sockaddr *)&station_addr, sizeof(station_addr)) < 0) {
        perror("Connection to station failed");
        exit(EXIT_FAILURE); 
    }
    iotlogger("Connected to Station successfully.\n");

    // 3. Setup Edge Server (Listening for Sensors)
    int server_fd, new_socket, client_sockets[MAX_CLIENTS];
    int activity, i, valread, sd, max_sd;
    struct sockaddr_in address;
    int opt = 1;
    int addrlen = sizeof(address);
    char buffer[BUFFER_SIZE];
    fd_set readfds;

    for (i = 0; i < MAX_CLIENTS; i++) {
        client_sockets[i] = 0;
    }

    if ((server_fd = socket(AF_INET, SOCK_STREAM, 0)) == 0) {
        perror("Socket creation failed");
        exit(EXIT_FAILURE);
    }

    if (setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR | SO_REUSEPORT, &opt, sizeof(opt))) {
        perror("Setsockopt failed");
        exit(EXIT_FAILURE);
    }

    address.sin_family = AF_INET;
    address.sin_addr.s_addr = INADDR_ANY;
    address.sin_port = htons(PORT);

    if (bind(server_fd, (struct sockaddr *)&address, sizeof(address)) < 0) {
        perror("Bind failed");
        exit(EXIT_FAILURE);
    }

    if (listen(server_fd, MAX_CLIENTS) < 0) {
        perror("Listen failed");
        exit(EXIT_FAILURE);
    }

    iotlogger("Edge Server listening for sensors on port %d...\n", PORT);

    // 4. Timer and Payload setup
    EdgePacket current_payload;
    memset(&current_payload, 0, sizeof(EdgePacket)); // Zero out everything initially
    current_payload.edge_id = edge_id;
    long long last_send_time = current_timestamp_ms();

    // 5. Main Event Loop
    while (1) {
        FD_ZERO(&readfds);
        FD_SET(server_fd, &readfds);
        max_sd = server_fd;

        for (i = 0; i < MAX_CLIENTS; i++) {
            sd = client_sockets[i];
            if (sd > 0) FD_SET(sd, &readfds);
            if (sd > max_sd) max_sd = sd;
        }

        // Timeout of 100ms for responsiveness to the 1-second timer
        struct timeval timeout;
        timeout.tv_sec = 0;
        timeout.tv_usec = 100000; 

        activity = select(max_sd + 1, &readfds, NULL, NULL, &timeout);

        if ((activity < 0) && (errno != EINTR)) {
            iotlogger("Select error\n");
        }

        // Check if 1 second has passed
        long long now = current_timestamp_ms();
        if (now - last_send_time >= 1500) {
            
            // Verify if we actually have any fresh data to send
            int has_data = 0;
            for (int j = 0; j < MAX_CLIENTS; j++) {
                if (current_payload.active_flags[j] == 1) {
                    has_data = 1;
                    break;
                }
            }

            if (has_data) {
#ifdef DO_DEBUG
                print_edge_packet(&current_payload);
#endif
                if (send(station_client, &current_payload, sizeof(EdgePacket), 0) < 0) {
                    perror("Failed to send to station");
                }
                
                // Clear the active flags for the next 1-second window
                // (We leave the old data in the struct, but flag it as not fresh)
                memset(current_payload.active_flags, 0, sizeof(current_payload.active_flags));
            }
            last_send_time = now;
        }

        // Handle new incoming sensor connections
        if (FD_ISSET(server_fd, &readfds)) {
            if ((new_socket = accept(server_fd, (struct sockaddr *)&address, (socklen_t*)&addrlen)) < 0) {
                perror("Accept failed");
                exit(EXIT_FAILURE);
            }

            // Assign the new sensor to a fixed slot index 'i'
            for (i = 0; i < MAX_CLIENTS; i++) {
                if (client_sockets[i] == 0) {
                    client_sockets[i] = new_socket;
                    iotlogger("Sensor connected: assigned to slot %d (FD %d)\n", i, new_socket);
                    break;
                }
            }
        }

        // Handle I/O from existing sensor connections
        for (i = 0; i < MAX_CLIENTS; i++) {
            sd = client_sockets[i];

            if (FD_ISSET(sd, &readfds)) {
                if ((valread = recv(sd, buffer, BUFFER_SIZE - 1, 0)) == 0) {
                    // Client disconnected
                    iotlogger("Sensor in slot %d disconnected.\n", i);
                    close(sd);
                    client_sockets[i] = 0;
                    current_payload.active_flags[i] = 0; // Clear its flag
                } else if (valread >= sizeof(SensorData)) {
                    SensorData *received_data = (SensorData *)buffer;
                    
                    // Assign data directly to this connection's dedicated slot
                    // Overwrites previous data if they send too fast
                    current_payload.sensors[i] = *received_data;
                    current_payload.active_flags[i] = 1; // Mark this slot as containing fresh data
                }
            }
        }
    }
    
    close(station_client);
    return 0;
}
