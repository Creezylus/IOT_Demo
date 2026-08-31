use std::mem;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

const HOST: &str = "192.168.1.185";
const PORT: u16 = 9090;
const MAX_SENSORS: usize = 5;
const MAX_EDGE: usize = 3;
use serde::Serialize;

// Add 'pub' to structures so backend_client.rs can utilize them
#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Serialize)]
pub struct Sensor {
    pub id: i32,
    pub timestamp: u64,
    pub a_x: f32,
    pub a_y: f32,
    pub a_z: f32,
    pub hum: f32,
    pub seis: f32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Serialize)]
pub struct EdgePacket {
    pub edge_id: i32,
    pub active_flags: [i32; MAX_SENSORS],
    pub sensors: [Sensor; MAX_SENSORS],
}

pub fn run(shared_data: Arc<Mutex<Vec<EdgePacket>>>) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(serve(shared_data))
}

async fn serve(shared_data: Arc<Mutex<Vec<EdgePacket>>>) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(format!("{}:{}", HOST, PORT)).await?;
    let packet_size = mem::size_of::<EdgePacket>();

    println!("Station Server listening on {}:{}...", HOST, PORT);
    debug_assert_eq!(packet_size, 184, "EdgePacket size mismatch with C sender");

    let semaphore = Arc::new(Semaphore::new(MAX_EDGE));

    loop {
        let (mut socket, addr) = listener.accept().await?;
        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                println!("Max edge clients reached. Denying {}", addr);
                continue;
            }
        };

        // Clone the Arc pointer for the specific connection task
        let task_data = Arc::clone(&shared_data);

        tokio::spawn(async move {
            println!("--- Edge Client Connected from {} ---", addr);
            let mut read_buf = vec![0u8; packet_size];

            loop {
                match socket.read_exact(&mut read_buf).await {
                    Ok(_) => {
                        let mut packet: EdgePacket = unsafe {
                            std::ptr::read_unaligned(read_buf.as_ptr() as *const EdgePacket)
                        };

                        // OVERWRITE TIMESTAMP WITH CURRENT LAPTOP TIME
                        let current_ts = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;

                        for i in 0..MAX_SENSORS {
                            if packet.active_flags[i] == 1 {
                                // Extract the sensor, modify its ts, and safely put it back
                                // to avoid unaligned reference warnings on packed structs
                                let mut s = packet.sensors[i];
                                s.timestamp = current_ts;
                                packet.sensors[i] = s;
                            } else {
                                let s = Sensor {
                                    id: -1,
                                    timestamp: 0,
                                    a_x: 0.0,
                                    a_y: 0.0,
                                    a_z: 0.0,
                                    hum: 0.0,
                                    seis: 0.0,
                                };
                                packet.sensors[i] = s;
                            }
                        }

                        // SAFELY APPEND TO BUFFER
                        {
                            let mut lock = task_data.lock().unwrap();
                            lock.push(packet);
                        }

                        // ... Original printing logic ...
                        let edge_id = packet.edge_id;
                        let active_flags = packet.active_flags;
                        let sensors = packet.sensors;

                        println!("\n[Edge ID: {}]", edge_id);
                        // Rest of the display logic...
                        let mut has_active_sensors = false;

                        for i in 0..MAX_SENSORS {
                            if active_flags[i] == 1 {
                                has_active_sensors = true;
                                let s = sensors[i];
                                let id = { s.id };
                                let ts = { s.timestamp };
                                let a_x = { s.a_x };
                                let a_y = { s.a_y };
                                let a_z = { s.a_z };
                                let hum = { s.hum };
                                let seis = { s.seis };

                                println!(
                                    "  -> Slot {} [Sensor ID: {}]: Accel=({:.3}, {:.3}, {:.3}) Hum={:.3}% Seismo={:.4} (TS: {})",
                                    i, id, a_x, a_y, a_z, hum, seis, ts
                                );
                            }
                        }

                        if !has_active_sensors {
                            println!("  -> No active sensor data in this window.");
                        }
                    }
                    Err(e) => {
                        println!("Edge client {} disconnected: {}", addr, e);
                        break;
                    }
                }
            }
            drop(permit);
        });
    }
}