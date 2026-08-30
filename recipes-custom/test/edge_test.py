import socket
import struct

# Configuration mapping the C Edge Client
HOST = '192.168.1.185'
PORT = 9090
MAX_CLIENTS = 5

# Constructing the format string for struct.unpack
# '@' means native byte order, size, and alignment
# 'i' = edge_id (int)
# '5i' = active_flags (5 ints)
# 'i Q 5f' * 5 = 5 sensors, each with 1 int (id), 1 unsigned long long (timestamp), and 5 floats
FORMAT_STRING = "< i 5i " + ("i Q 5f " * MAX_CLIENTS)
PACKET_SIZE = struct.calcsize(FORMAT_STRING)

def start_station():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server_socket:
        # Allow immediate reuse of the port
        server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server_socket.bind((HOST, PORT))
        server_socket.listen()

        print(f"Station Server listening on {HOST}:{PORT}...")
        print(f"Expecting packet size of {PACKET_SIZE} bytes.\n")

        while True:
            conn, addr = server_socket.accept()
            with conn:
                print(f"--- Edge Client Connected from {addr} ---")

                while True:
                    # Receive exactly the number of bytes required for one complete struct
                    data = b''
                    while len(data) < PACKET_SIZE:
                        packet = conn.recv(PACKET_SIZE - len(data))
                        if not packet:
                            break
                        data += packet

                    if not data:
                        print("Edge client disconnected.")
                        break

                    # Unpack the binary data
                    unpacked_data = struct.unpack(FORMAT_STRING, data)

                    # edge_id is at index 0
                    edge_id = unpacked_data[0]
                    
                    # active_flags are at indices 1 through 5
                    active_flags = unpacked_data[1:6]

                    print(f"\n[Edge ID: {edge_id}]")

                    # Read the sensor data chunks
                    # The first sensor's data starts at index 6 in the unpacked tuple
                    # Each sensor has 7 fields: id(int), timestamp(unsigned long long), + 5 floats
                    sensor_data_start_idx = 6
                    has_active_sensors = False

                    for i in range(MAX_CLIENTS):
                        if active_flags[i] == 1:
                            has_active_sensors = True

                            # Calculate the starting index for this sensor in the unpacked tuple
                            idx = sensor_data_start_idx + (i * 7)

                            s_id = unpacked_data[idx]
                            timestamp = unpacked_data[idx+1]
                            a_x = unpacked_data[idx+2]
                            a_y = unpacked_data[idx+3]
                            a_z = unpacked_data[idx+4]
                            hum = unpacked_data[idx+5]
                            seis = unpacked_data[idx+6]

                            print(f"  -> Slot {i} [Sensor ID: {s_id} | Time: {timestamp}]: "
                                  f"Accel=({a_x:.3f}, {a_y:.3f}, {a_z:.3f}) "
                                  f"Hum={hum:.3f}% Seismo={seis:.4f}")

                    if not has_active_sensors:
                        print("  -> No active sensor data in this window.")

if __name__ == "__main__":
    start_station()