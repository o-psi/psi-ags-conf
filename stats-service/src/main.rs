use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::time;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::AsyncWriteExt;
use chrono::Local;
use num_cpus;

const HISTORY_SIZE: usize = 60;
const DATA_DIR: &str = "/tmp/ags-stats";
const SOCKET_PATH: &str = "/tmp/ags-stats/stats.sock";
const UPDATE_INTERVAL_MS: u64 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MemoryStats {
    total: f64,
    available: f64,
    used_percentage: f64,
    // Detailed breakdown in KB
    apps: f64,
    cached: f64,
    buffers: f64,
    slab: f64,
    shmem: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemStats {
    timestamp: i64,
    cpu_usage: f64,
    cpu_cores: Vec<f64>,
    cpu_iowait: f64,
    memory: MemoryStats,
    network_download: f64,
    network_upload: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatsHistory {
    cpu: VecDeque<f64>,
    cpu_cores: Vec<VecDeque<f64>>,
    cpu_iowait: VecDeque<f64>,
    memory: VecDeque<f64>,
    memory_total: f64,
    memory_apps: VecDeque<f64>,
    memory_cached: VecDeque<f64>,
    memory_buffers: VecDeque<f64>,
    memory_slab: VecDeque<f64>,
    memory_shmem: VecDeque<f64>,
    network_download: VecDeque<f64>,
    network_upload: VecDeque<f64>,
    last_update: i64,
}

impl StatsHistory {
    fn new() -> Self {
        let num_cores = num_cpus::get();
        let mut cpu_cores = Vec::new();
        
        for _ in 0..num_cores {
            let mut core_history = VecDeque::with_capacity(HISTORY_SIZE);
            for _ in 0..HISTORY_SIZE {
                core_history.push_back(0.0);
            }
            cpu_cores.push(core_history);
        }
        
        let mut history = StatsHistory {
            cpu: VecDeque::with_capacity(HISTORY_SIZE),
            cpu_cores,
            cpu_iowait: VecDeque::with_capacity(HISTORY_SIZE),
            memory: VecDeque::with_capacity(HISTORY_SIZE),
            memory_total: 0.0,
            memory_apps: VecDeque::with_capacity(HISTORY_SIZE),
            memory_cached: VecDeque::with_capacity(HISTORY_SIZE),
            memory_buffers: VecDeque::with_capacity(HISTORY_SIZE),
            memory_slab: VecDeque::with_capacity(HISTORY_SIZE),
            memory_shmem: VecDeque::with_capacity(HISTORY_SIZE),
            network_download: VecDeque::with_capacity(HISTORY_SIZE),
            network_upload: VecDeque::with_capacity(HISTORY_SIZE),
            last_update: 0,
        };
        
        for _ in 0..HISTORY_SIZE {
            history.cpu.push_back(0.0);
            history.cpu_iowait.push_back(0.0);
            history.memory.push_back(0.0);
            history.memory_apps.push_back(0.0);
            history.memory_cached.push_back(0.0);
            history.memory_buffers.push_back(0.0);
            history.memory_slab.push_back(0.0);
            history.memory_shmem.push_back(0.0);
            history.network_download.push_back(0.0);
            history.network_upload.push_back(0.0);
        }
        
        history
    }
    
    fn add_stats(&mut self, stats: &SystemStats) {
        Self::add_value(&mut self.cpu, stats.cpu_usage);
        Self::add_value(&mut self.cpu_iowait, stats.cpu_iowait);
        
        for (i, core_usage) in stats.cpu_cores.iter().enumerate() {
            if i < self.cpu_cores.len() {
                Self::add_value(&mut self.cpu_cores[i], *core_usage);
            }
        }
        
        Self::add_value(&mut self.memory, stats.memory.used_percentage);
        self.memory_total = stats.memory.total;
        Self::add_value(&mut self.memory_apps, stats.memory.apps);
        Self::add_value(&mut self.memory_cached, stats.memory.cached);
        Self::add_value(&mut self.memory_buffers, stats.memory.buffers);
        Self::add_value(&mut self.memory_slab, stats.memory.slab);
        Self::add_value(&mut self.memory_shmem, stats.memory.shmem);
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

// CPU tracking - overall and per-core
static mut PREV_CPU_VALUES: Option<(f64, f64, f64)> = None; // (total, idle, iowait)
static mut PREV_CORE_VALUES: Option<Vec<(f64, f64)>> = None; // per-core (total, idle)

#[derive(Debug)]
struct CpuStats {
    overall_usage: f64,
    core_usage: Vec<f64>,
    iowait_percentage: f64,
}

fn read_cpu_stats() -> CpuStats {
    let mut result = CpuStats {
        overall_usage: 0.0,
        core_usage: Vec::new(),
        iowait_percentage: 0.0,
    };
    
    if let Ok(content) = fs::read_to_string("/proc/stat") {
        let lines: Vec<&str> = content.lines().collect();
        
        // Parse overall CPU (first line)
        if let Some(line) = lines.first() {
            if line.starts_with("cpu ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 8 {
                    let user = parts[1].parse::<f64>().unwrap_or(0.0);
                    let nice = parts[2].parse::<f64>().unwrap_or(0.0);
                    let system = parts[3].parse::<f64>().unwrap_or(0.0);
                    let idle = parts[4].parse::<f64>().unwrap_or(0.0);
                    let iowait = parts[5].parse::<f64>().unwrap_or(0.0);
                    let irq = parts[6].parse::<f64>().unwrap_or(0.0);
                    let softirq = parts[7].parse::<f64>().unwrap_or(0.0);
                    
                    let idle_time = idle;
                    let non_idle = user + nice + system + irq + softirq;
                    let total = idle_time + non_idle + iowait;
                    
                    unsafe {
                        if let Some((prev_total, prev_idle, prev_iowait)) = PREV_CPU_VALUES {
                            let total_delta = total - prev_total;
                            let idle_delta = idle_time - prev_idle;
                            let iowait_delta = iowait - prev_iowait;
                            
                            PREV_CPU_VALUES = Some((total, idle_time, iowait));
                            
                            if total_delta > 0.0 {
                                result.overall_usage = ((total_delta - idle_delta - iowait_delta) / total_delta) * 100.0;
                                result.iowait_percentage = (iowait_delta / total_delta) * 100.0;
                            }
                        } else {
                            PREV_CPU_VALUES = Some((total, idle_time, iowait));
                        }
                    }
                }
            }
        }
        
        // Parse individual cores
        let mut core_stats = Vec::new();
        for line in lines.iter().skip(1) {
            if line.starts_with("cpu") && line.contains("cpu") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 8 {
                    let user = parts[1].parse::<f64>().unwrap_or(0.0);
                    let nice = parts[2].parse::<f64>().unwrap_or(0.0);
                    let system = parts[3].parse::<f64>().unwrap_or(0.0);
                    let idle = parts[4].parse::<f64>().unwrap_or(0.0);
                    let iowait = parts[5].parse::<f64>().unwrap_or(0.0);
                    let irq = parts[6].parse::<f64>().unwrap_or(0.0);
                    let softirq = parts[7].parse::<f64>().unwrap_or(0.0);
                    
                    let idle_time = idle;
                    let non_idle = user + nice + system + irq + softirq;
                    let total = idle_time + non_idle + iowait;
                    
                    core_stats.push((total, idle_time));
                }
            } else {
                break; // End of CPU lines
            }
        }
        
        unsafe {
            if let Some(prev_cores) = &PREV_CORE_VALUES {
                if prev_cores.len() == core_stats.len() {
                    for ((total, idle), (prev_total, prev_idle)) in 
                        core_stats.iter().zip(prev_cores.iter()) {
                        
                        let total_delta = total - prev_total;
                        let idle_delta = idle - prev_idle;
                        
                        if total_delta > 0.0 {
                            let usage = ((total_delta - idle_delta) / total_delta) * 100.0;
                            result.core_usage.push(usage);
                        } else {
                            result.core_usage.push(0.0);
                        }
                    }
                } else {
                    // Core count mismatch, fill with zeros
                    result.core_usage = vec![0.0; core_stats.len()];
                }
            } else {
                // No previous data, fill with zeros
                result.core_usage = vec![0.0; core_stats.len()];
            }
            
            PREV_CORE_VALUES = Some(core_stats);
        }
    }
    
    result
}

fn read_memory_stats() -> MemoryStats {
    let mut stats = MemoryStats::default();
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        let mut mem_info = HashMap::new();
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 2 {
                if let Ok(value) = parts[1].trim().split_whitespace().next().unwrap_or("0").parse::<f64>() {
                    mem_info.insert(parts[0], value);
                }
            }
        }

        let total = mem_info.get("MemTotal").copied().unwrap_or(0.0);
        let available = mem_info.get("MemAvailable").copied().unwrap_or(0.0);
        let active_anon = mem_info.get("Active(anon)").copied().unwrap_or(0.0);
        let inactive_anon = mem_info.get("Inactive(anon)").copied().unwrap_or(0.0);
        let shmem = mem_info.get("Shmem").copied().unwrap_or(0.0);
        let slab = mem_info.get("Slab").copied().unwrap_or(0.0);
        let buffers = mem_info.get("Buffers").copied().unwrap_or(0.0);
        let cached = mem_info.get("Cached").copied().unwrap_or(0.0);

        stats.total = total;
        stats.available = available;
        if total > 0.0 {
            stats.used_percentage = ((total - available) / total) * 100.0;
        }

        stats.apps = active_anon + inactive_anon;
        stats.cached = cached;
        stats.buffers = buffers;
        stats.slab = slab;
        stats.shmem = shmem;
    }
    stats
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
    println!("Starting Enhanced AGS Stats Service...");
    
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
        
        let cpu_stats = read_cpu_stats();
        let memory_stats = read_memory_stats();
        let (download, upload) = read_network_stats();
        
        let stats = SystemStats {
            timestamp: Local::now().timestamp_millis(),
            cpu_usage: cpu_stats.overall_usage,
            cpu_cores: cpu_stats.core_usage,
            cpu_iowait: cpu_stats.iowait_percentage,
            memory: memory_stats,
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
        let core_summary = if stats.cpu_cores.len() <= 4 {
            format!("[{}]", stats.cpu_cores.iter().map(|c| format!("{:.1}", c)).collect::<Vec<_>>().join(","))
        } else {
            format!("[{:.1},{:.1}...{:.1},{:.1}]", 
                   stats.cpu_cores[0], stats.cpu_cores[1], 
                   stats.cpu_cores[stats.cpu_cores.len()-2], stats.cpu_cores[stats.cpu_cores.len()-1])
        };
        println!("CPU: {:.1}% {} | IO: {:.1}% | MEM: {:.1}% (A:{:.1} C:{:.1} B:{:.1} L:{:.1} S:{:.1}) | NET: ↓{:.1} ↑{:.1} KB/s", 
                 stats.cpu_usage, core_summary, stats.cpu_iowait, 
                 stats.memory.used_percentage,
                 stats.memory.apps / 1024.0, // to MB
                 stats.memory.cached / 1024.0,
                 stats.memory.buffers / 1024.0,
                 stats.memory.slab / 1024.0,
                 stats.memory.shmem / 1024.0,
                 download, upload);
    }
}
