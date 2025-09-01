use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::time;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::AsyncWriteExt;
use chrono::Local;

const HISTORY_SIZE: usize = 60;
const DATA_DIR: &str = "/tmp/ags-stats";
const SOCKET_PATH: &str = "/tmp/ags-stats/stats.sock";
const UPDATE_INTERVAL_MS: u64 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemStats {
    timestamp: i64,
    cpu_usage: f64,
    memory_usage: f64,
    network_download: f64,
    network_upload: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatsHistory {
    cpu: VecDeque<f64>,
    memory: VecDeque<f64>,
    network_download: VecDeque<f64>,
    network_upload: VecDeque<f64>,
    last_update: i64,
}

impl StatsHistory {
    fn new() -> Self {
        let mut history = StatsHistory {
            cpu: VecDeque::with_capacity(HISTORY_SIZE),
            memory: VecDeque::with_capacity(HISTORY_SIZE),
            network_download: VecDeque::with_capacity(HISTORY_SIZE),
            network_upload: VecDeque::with_capacity(HISTORY_SIZE),
            last_update: 0,
        };
        
        // Pre-fill with zeros
        for _ in 0..HISTORY_SIZE {
            history.cpu.push_back(0.0);
            history.memory.push_back(0.0);
            history.network_download.push_back(0.0);
            history.network_upload.push_back(0.0);
        }
        
        history
    }
    
    fn add_stats(&mut self, stats: &SystemStats) {
        Self::add_value(&mut self.cpu, stats.cpu_usage);
        Self::add_value(&mut self.memory, stats.memory_usage);
        Self::add_value(&mut self.network_download, stats.network_download);
        Self::add_value(&mut self.network_upload, stats.network_upload);
        self.last_update = stats.timestamp;
    }
    
    fn add_value(queue: &mut VecDeque<f64>, value: f64) {
        queue.push_back(value);
        if queue.len() > HISTORY_SIZE {
            queue.pop_front();
        }
    }
}

// CPU tracking
static mut PREV_CPU_VALUES: Option<(f64, f64)> = None;

fn read_cpu_usage() -> f64 {
    if let Ok(content) = fs::read_to_string("/proc/stat") {
        if let Some(line) = content.lines().next() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 8 {
                let user = parts[1].parse::<f64>().unwrap_or(0.0);
                let nice = parts[2].parse::<f64>().unwrap_or(0.0);
                let system = parts[3].parse::<f64>().unwrap_or(0.0);
                let idle = parts[4].parse::<f64>().unwrap_or(0.0);
                let iowait = parts[5].parse::<f64>().unwrap_or(0.0);
                let irq = parts[6].parse::<f64>().unwrap_or(0.0);
                let softirq = parts[7].parse::<f64>().unwrap_or(0.0);
                
                let idle_time = idle + iowait;
                let non_idle = user + nice + system + irq + softirq;
                let total = idle_time + non_idle;
                
                unsafe {
                    if let Some((prev_total, prev_idle)) = PREV_CPU_VALUES {
                        let total_delta = total - prev_total;
                        let idle_delta = idle_time - prev_idle;
                        
                        PREV_CPU_VALUES = Some((total, idle_time));
                        
                        if total_delta > 0.0 {
                            return ((total_delta - idle_delta) / total_delta) * 100.0;
                        }
                    } else {
                        PREV_CPU_VALUES = Some((total, idle_time));
                    }
                }
            }
        }
    }
    0.0
}

fn read_memory_usage() -> f64 {
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        let mut total = 0.0;
        let mut available = 0.0;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                total = line.split_whitespace().nth(1).unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            } else if line.starts_with("MemAvailable:") {
                available = line.split_whitespace().nth(1).unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            }
        }
        if total > 0.0 {
            return ((total - available) / total) * 100.0;
        }
    }
    0.0
}

// Network tracking
static mut PREV_NET_VALUES: Option<(f64, f64, Instant)> = None;

fn read_network_stats() -> (f64, f64) {
    if let Ok(content) = fs::read_to_string("/proc/net/dev") {
        let mut rx_bytes = 0u64;
        let mut tx_bytes = 0u64;
        
        for line in content.lines() {
            // Skip loopback and header lines
            if line.contains(':') && !line.contains("lo:") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() == 2 {
                    let values: Vec<&str> = parts[1].split_whitespace().collect();
                    if values.len() >= 9 {
                        rx_bytes += values[0].parse::<u64>().unwrap_or(0);
                        tx_bytes += values[8].parse::<u64>().unwrap_or(0);
                    }
                }
            }
        }
        
        let now = Instant::now();
        
        unsafe {
            if let Some((prev_rx, prev_tx, prev_time)) = PREV_NET_VALUES {
                let time_diff = now.duration_since(prev_time).as_secs_f64();
                
                if time_diff > 0.0 {
                    let download = ((rx_bytes as f64 - prev_rx) / 1024.0) / time_diff; // KB/s
                    let upload = ((tx_bytes as f64 - prev_tx) / 1024.0) / time_diff;
                    
                    PREV_NET_VALUES = Some((rx_bytes as f64, tx_bytes as f64, now));
                    
                    return (download, upload);
                }
            }
            
            PREV_NET_VALUES = Some((rx_bytes as f64, tx_bytes as f64, now));
        }
    }
    
    (0.0, 0.0)
}

fn write_history(history: &StatsHistory) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(history)?;
    let mut file = File::create(format!("{}/history.json", DATA_DIR))?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

fn write_latest(stats: &SystemStats) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(stats)?;
    let mut file = File::create(format!("{}/latest.json", DATA_DIR))?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

async fn handle_client(mut stream: UnixStream, history: Arc<Mutex<StatsHistory>>) {
    // Send the full history immediately when client connects
    let hist = history.lock().await;
    let json = serde_json::to_string(&*hist).unwrap_or_default();
    drop(hist); // Release lock before async operation
    
    if let Err(e) = stream.write_all(json.as_bytes()).await {
        eprintln!("Failed to send history to client: {}", e);
    }
    
    // Close connection after sending
    let _ = stream.shutdown().await;
}

async fn run_socket_server(history: Arc<Mutex<StatsHistory>>) {
    // Remove old socket if it exists
    let _ = fs::remove_file(SOCKET_PATH);
    
    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind socket: {}", e);
            return;
        }
    };
    
    println!("Socket server listening on {}", SOCKET_PATH);
    
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let hist_clone = history.clone();
                tokio::spawn(async move {
                    handle_client(stream, hist_clone).await;
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Starting AGS Stats Service...");
    
    // Create data directory
    fs::create_dir_all(DATA_DIR).expect("Failed to create data directory");
    
    // Check if service is already running
    let pid_file = format!("{}/service.pid", DATA_DIR);
    if Path::new(&pid_file).exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Check if process is still running
                if Path::new(&format!("/proc/{}", pid)).exists() {
                    eprintln!("Service is already running with PID {}", pid);
                    std::process::exit(1);
                }
            }
        }
    }
    
    // Write PID file
    let mut pid_file = File::create(&pid_file).expect("Failed to create PID file");
    writeln!(pid_file, "{}", std::process::id()).expect("Failed to write PID");
    
    let history = Arc::new(Mutex::new(StatsHistory::new()));
    
    // Start socket server in background
    let history_socket = history.clone();
    tokio::spawn(async move {
        run_socket_server(history_socket).await;
    });
    
    // Main collection loop
    let mut interval = time::interval(Duration::from_millis(UPDATE_INTERVAL_MS));
    
    loop {
        interval.tick().await;
        
        let cpu = read_cpu_usage();
        let memory = read_memory_usage();
        let (download, upload) = read_network_stats();
        
        let stats = SystemStats {
            timestamp: Local::now().timestamp_millis(),
            cpu_usage: cpu,
            memory_usage: memory,
            network_download: download,
            network_upload: upload,
        };
        
        // Update history
        {
            let mut hist = history.lock().await;
            hist.add_stats(&stats);
            
            // Write to files
            if let Err(e) = write_history(&hist) {
                eprintln!("Failed to write history: {}", e);
            }
        }
        
        if let Err(e) = write_latest(&stats) {
            eprintln!("Failed to write latest stats: {}", e);
        }
        
        // Print current stats for debugging
        println!("CPU: {:.1}% | MEM: {:.1}% | NET: ↓{:.1} ↑{:.1} KB/s", 
                 cpu, memory, download, upload);
    }
}
