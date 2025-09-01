use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::os::unix::net::UnixStream;

#[derive(Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub values: Vec<f64>,
    pub max_size: usize,
}

impl GraphData {
    pub fn new_with_zeros(size: usize) -> Self {
        GraphData {
            values: vec![0.0; size],
            max_size: size,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AdvancedMemoryData {
    pub total: f64,
    pub apps: GraphData,
    pub cached: GraphData,
    pub buffers: GraphData,
    pub slab: GraphData,
    pub shmem: GraphData,
}

impl AdvancedMemoryData {
    pub fn new(size: usize) -> Self {
        AdvancedMemoryData {
            total: 0.0,
            apps: GraphData::new_with_zeros(size),
            cached: GraphData::new_with_zeros(size),
            buffers: GraphData::new_with_zeros(size),
            slab: GraphData::new_with_zeros(size),
            shmem: GraphData::new_with_zeros(size),
        }
    }
}

pub fn load_history() -> serde_json::Value {
    let history_json = if let Ok(mut stream) = UnixStream::connect("/tmp/ags-stats/stats.sock") {
        eprintln!("Connected to stats service socket");
        let mut buffer = String::new();
        match stream.read_to_string(&mut buffer) {
            Ok(size) => {
                eprintln!("Received {} bytes from socket", size);
                buffer
            }
            Err(e) => {
                eprintln!("Failed to read from socket: {}", e);
                fs::read_to_string("/tmp/ags-stats/history.json").unwrap_or_default()
            }
        }
    } else {
        eprintln!("Could not connect to socket, trying file");
        fs::read_to_string("/tmp/ags-stats/history.json").unwrap_or_default()
    };

    serde_json::from_str(&history_json).unwrap_or_else(|_| serde_json::json!({}))
}
