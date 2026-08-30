use std::mem;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

const HOST: &str = "192.168.1.185";
const PORT: u16 = 9090;
const MAX_SENSORS: usize = 5;
const MAX_EDGE: usize = 3;

// #[repr(C, packed)] matches the C side's __attribute__((packed)):
// Veryyy Important depending on your C Compiler if this may cause mismatch in readings and print out grabage values.
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct Sensor {
    id: i32,
    timestamp: u64,
    a_x: f32,
    a_y: f32,
    a_z: f32,
    hum: f32,
    seis: f32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct EdgePacket {
    edge_id: i32,
    active_flags: [i32; MAX_SENSORS],
    sensors: [Sensor; MAX_SENSORS],
}

/// Entry point for the station client's own OS thread. Builds and owns
/// a dedicated tokio runtime so this can run independent of whatever
/// runtime (if any) the caller's thread is using.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(serve())
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(format!("{}:{}", HOST, PORT)).await?;
    let packet_size = mem::size_of::<EdgePacket>();

    println!("Station Server listening on {}:{}...", HOST, PORT);
    println!("Expecting packet size of {} bytes.\n", packet_size);

    // Sanity check: if this ever fails, the struct definitions have
    // drifted from the C side's on-wire layout.
    debug_assert_eq!(packet_size, 184, "EdgePacket size mismatch with C sender");

    // Semaphore strictly limits concurrent connections to 3
    let semaphore = Arc::new(Semaphore::new(MAX_EDGE));

    loop {
        let (mut socket, addr) = listener.accept().await?;
        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                println!("Max edge clients ({}) reached. Denying connection from {}", MAX_EDGE, addr);
                continue;
            }
        };

        tokio::spawn(async move {
            println!("--- Edge Client Connected from {} ---", addr);
            let mut buffer = vec![0u8; packet_size];

            loop {
                // Read exactly the required bytes for one struct
                match socket.read_exact(&mut buffer).await {
                    Ok(_) => {
                        // Safely cast the raw bytes into our C-compatible struct.
                        // read_unaligned is required since EdgePacket is packed
                        // and buffer.as_ptr() has no alignment guarantee anyway.
                        let packet: EdgePacket = unsafe {
                            std::ptr::read_unaligned(buffer.as_ptr() as *const EdgePacket)
                        };

                        // Packed structs force field alignment to 1, so taking
                        // a reference to any field (which println! does under
                        // the hood) is UB. Copy fields into locals first —
                        // that copy is a properly-aligned stack value.
                        let edge_id = packet.edge_id;
                        let active_flags = packet.active_flags; // whole array copy
                        let sensors = packet.sensors; // whole array copy

                        println!("\n[Edge ID: {}]", edge_id);

                        let mut has_active_sensors = false;

                        for i in 0..MAX_SENSORS {
                            if active_flags[i] == 1 {
                                has_active_sensors = true;
                                let s = sensors[i]; // aligned local copy of Sensor
                                let (id, a_x, a_y, a_z, hum, seis) =
                                    (s.id, s.a_x, s.a_y, s.a_z, s.hum, s.seis);

                                println!(
                                    "  -> Slot {} [Sensor ID: {}]: Accel=({:.3}, {:.3}, {:.3}) Hum={:.3}% Seismo={:.4}",
                                    i, id, a_x, a_y, a_z, hum, seis
                                );
                            }
                        }

                        if !has_active_sensors {
                            println!("  -> No active sensor data in this window.");
                        }
                    }
                    Err(e) => {
                        println!("Edge client {} disconnected or read error: {}", addr, e);
                        break;
                    }
                }
            }

            // Drop permit to free up a slot for a new client
            drop(permit);
        });
    }
}