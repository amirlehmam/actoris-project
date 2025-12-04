//! Metering module - Compute resource metering for HC (Harness Credits) calculation
//!
//! This module provides:
//! - Process-level CPU and memory metering
//! - Network I/O tracking
//! - PFLOP-hour estimation for HC calculation
//! - eBPF-based kernel-level metering (Linux only)

pub mod ebpf;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Metering errors
#[derive(Debug, Error)]
pub enum MeteringError {
    #[error("Failed to read process stats: {0}")]
    ProcessStats(String),

    #[error("eBPF not available on this platform")]
    EbpfNotAvailable,

    #[error("Metering collector not started")]
    NotStarted,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Compute metrics captured for a request/response cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeMetrics {
    /// CPU time in microseconds
    pub cpu_time_us: u64,

    /// Peak memory usage in bytes
    pub memory_bytes: u64,

    /// Wall clock time in microseconds
    pub wall_time_us: u64,

    /// Estimated PFLOP-hours (for HC calculation)
    pub pflop_hours: f64,

    /// Network bytes received
    pub network_in_bytes: u64,

    /// Network bytes sent
    pub network_out_bytes: u64,

    /// Number of syscalls (if eBPF available)
    pub syscall_count: Option<u64>,

    /// Disk I/O bytes (if available)
    pub disk_io_bytes: Option<u64>,

    /// Timestamp when metrics were collected
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for ComputeMetrics {
    fn default() -> Self {
        Self {
            cpu_time_us: 0,
            memory_bytes: 0,
            wall_time_us: 0,
            pflop_hours: 0.0,
            network_in_bytes: 0,
            network_out_bytes: 0,
            syscall_count: None,
            disk_io_bytes: None,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl ComputeMetrics {
    /// Calculate HC (Harness Credits) from metrics
    /// HC = PFLOP-hours * base_rate + network_bytes * network_rate + memory_gb_hours * memory_rate
    pub fn calculate_hc(&self, rates: &HcRates) -> f64 {
        let compute_hc = self.pflop_hours * rates.pflop_hour_rate;
        let network_hc =
            (self.network_in_bytes + self.network_out_bytes) as f64 * rates.network_byte_rate;
        let memory_gb_hours =
            (self.memory_bytes as f64 / 1_073_741_824.0) * (self.wall_time_us as f64 / 3_600_000_000.0);
        let memory_hc = memory_gb_hours * rates.memory_gb_hour_rate;

        compute_hc + network_hc + memory_hc
    }

    /// Merge two metrics (for aggregation)
    pub fn merge(&mut self, other: &ComputeMetrics) {
        self.cpu_time_us += other.cpu_time_us;
        self.memory_bytes = self.memory_bytes.max(other.memory_bytes);
        self.wall_time_us += other.wall_time_us;
        self.pflop_hours += other.pflop_hours;
        self.network_in_bytes += other.network_in_bytes;
        self.network_out_bytes += other.network_out_bytes;

        if let (Some(a), Some(b)) = (self.syscall_count, other.syscall_count) {
            self.syscall_count = Some(a + b);
        }
        if let (Some(a), Some(b)) = (self.disk_io_bytes, other.disk_io_bytes) {
            self.disk_io_bytes = Some(a + b);
        }
    }
}

/// HC (Harness Credits) rate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HcRates {
    /// Rate per PFLOP-hour
    pub pflop_hour_rate: f64,

    /// Rate per network byte
    pub network_byte_rate: f64,

    /// Rate per GB-hour of memory
    pub memory_gb_hour_rate: f64,
}

impl Default for HcRates {
    fn default() -> Self {
        Self {
            pflop_hour_rate: 1.0,
            network_byte_rate: 0.000000001, // 1 HC per GB
            memory_gb_hour_rate: 0.1,
        }
    }
}

/// Metering session for tracking a single request
#[derive(Debug)]
pub struct MeteringSession {
    start_time: Instant,
    start_cpu_time: u64,
    start_memory: u64,
    network_in: AtomicU64,
    network_out: AtomicU64,
    cpu_frequency_ghz: f64,
}

impl MeteringSession {
    /// Create a new metering session
    pub fn new(cpu_frequency_ghz: f64) -> Self {
        let (cpu_time, memory) = get_process_stats().unwrap_or((0, 0));

        Self {
            start_time: Instant::now(),
            start_cpu_time: cpu_time,
            start_memory: memory,
            network_in: AtomicU64::new(0),
            network_out: AtomicU64::new(0),
            cpu_frequency_ghz,
        }
    }

    /// Record network bytes received
    pub fn record_network_in(&self, bytes: u64) {
        self.network_in.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record network bytes sent
    pub fn record_network_out(&self, bytes: u64) {
        self.network_out.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Complete the session and return metrics
    pub fn complete(self) -> ComputeMetrics {
        let wall_time = self.start_time.elapsed();
        let (end_cpu_time, end_memory) = get_process_stats().unwrap_or((self.start_cpu_time, self.start_memory));

        let cpu_time_us = end_cpu_time.saturating_sub(self.start_cpu_time);
        let memory_bytes = end_memory.max(self.start_memory);
        let wall_time_us = wall_time.as_micros() as u64;

        // Estimate PFLOP-hours from CPU time and frequency
        // FLOP = CPU_cycles * FLOP_per_cycle (assume 16 for modern AVX-512)
        // PFLOP = FLOP / 10^15
        let cpu_cycles = (cpu_time_us as f64 / 1_000_000.0) * self.cpu_frequency_ghz * 1e9;
        let flop_per_cycle = 16.0; // AVX-512 assumption
        let pflops = (cpu_cycles * flop_per_cycle) / 1e15;
        let hours = wall_time_us as f64 / 3_600_000_000.0;
        let pflop_hours = pflops * hours;

        ComputeMetrics {
            cpu_time_us,
            memory_bytes,
            wall_time_us,
            pflop_hours,
            network_in_bytes: self.network_in.load(Ordering::Relaxed),
            network_out_bytes: self.network_out.load(Ordering::Relaxed),
            syscall_count: None,
            disk_io_bytes: None,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Metering collector for continuous process monitoring
pub struct MeteringCollector {
    /// Whether the collector is running
    running: Arc<RwLock<bool>>,

    /// Aggregated metrics
    aggregated: Arc<RwLock<ComputeMetrics>>,

    /// CPU frequency in GHz (for PFLOP estimation)
    cpu_frequency_ghz: f64,

    /// Collection interval
    interval: Duration,

    /// HC rate configuration
    rates: HcRates,

    /// eBPF meter (if available)
    #[cfg(target_os = "linux")]
    ebpf_meter: Option<ebpf::EbpfMeter>,
}

impl MeteringCollector {
    /// Create a new metering collector
    pub fn new(interval: Duration) -> Self {
        let cpu_frequency_ghz = detect_cpu_frequency();
        info!(
            "Metering collector initialized with CPU frequency: {:.2} GHz",
            cpu_frequency_ghz
        );

        Self {
            running: Arc::new(RwLock::new(false)),
            aggregated: Arc::new(RwLock::new(ComputeMetrics::default())),
            cpu_frequency_ghz,
            interval,
            rates: HcRates::default(),
            #[cfg(target_os = "linux")]
            ebpf_meter: None,
        }
    }

    /// Set HC rates
    pub fn with_rates(mut self, rates: HcRates) -> Self {
        self.rates = rates;
        self
    }

    /// Start a new metering session
    pub fn start_session(&self) -> MeteringSession {
        MeteringSession::new(self.cpu_frequency_ghz)
    }

    /// Start the background collection task
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let running = self.running.clone();
        let aggregated = self.aggregated.clone();
        let interval = self.interval;

        *running.write() = true;

        tokio::spawn(async move {
            let mut last_cpu_time = 0u64;
            let mut last_check = Instant::now();

            while *running.read() {
                tokio::time::sleep(interval).await;

                if let Ok((cpu_time, memory)) = get_process_stats() {
                    let elapsed = last_check.elapsed();
                    let cpu_delta = cpu_time.saturating_sub(last_cpu_time);

                    let mut metrics = aggregated.write();
                    metrics.cpu_time_us += cpu_delta;
                    metrics.memory_bytes = metrics.memory_bytes.max(memory);
                    metrics.wall_time_us += elapsed.as_micros() as u64;

                    last_cpu_time = cpu_time;
                    last_check = Instant::now();

                    debug!(
                        "Collected metrics: cpu={}us, mem={}MB",
                        metrics.cpu_time_us,
                        metrics.memory_bytes / 1_048_576
                    );
                }
            }
        })
    }

    /// Stop the collector
    pub fn stop(&self) {
        *self.running.write() = false;
    }

    /// Get current aggregated metrics
    pub fn get_metrics(&self) -> ComputeMetrics {
        self.aggregated.read().clone()
    }

    /// Reset aggregated metrics
    pub fn reset(&self) {
        *self.aggregated.write() = ComputeMetrics::default();
    }

    /// Calculate current HC value
    pub fn calculate_hc(&self) -> f64 {
        self.aggregated.read().calculate_hc(&self.rates)
    }

    /// Get CPU frequency
    pub fn cpu_frequency_ghz(&self) -> f64 {
        self.cpu_frequency_ghz
    }

    /// Initialize eBPF metering (Linux only)
    #[cfg(target_os = "linux")]
    pub async fn init_ebpf(&mut self) -> Result<(), MeteringError> {
        match ebpf::EbpfMeter::new().await {
            Ok(meter) => {
                self.ebpf_meter = Some(meter);
                info!("eBPF metering initialized successfully");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to initialize eBPF metering: {}", e);
                Err(e)
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub async fn init_ebpf(&mut self) -> Result<(), MeteringError> {
        warn!("eBPF metering not available on this platform");
        Err(MeteringError::EbpfNotAvailable)
    }
}

/// Get process CPU time and memory usage
#[cfg(target_os = "linux")]
fn get_process_stats() -> Result<(u64, u64), MeteringError> {
    use std::fs;

    // Read /proc/self/stat for CPU time
    let stat = fs::read_to_string("/proc/self/stat")
        .map_err(|e| MeteringError::ProcessStats(e.to_string()))?;

    let parts: Vec<&str> = stat.split_whitespace().collect();
    if parts.len() < 23 {
        return Err(MeteringError::ProcessStats("Invalid stat format".into()));
    }

    // utime (14) + stime (15) in clock ticks
    let utime: u64 = parts[13].parse().unwrap_or(0);
    let stime: u64 = parts[14].parse().unwrap_or(0);
    let clock_ticks_per_sec = 100u64; // Usually 100 on Linux
    let cpu_time_us = ((utime + stime) * 1_000_000) / clock_ticks_per_sec;

    // Read /proc/self/statm for memory
    let statm = fs::read_to_string("/proc/self/statm")
        .map_err(|e| MeteringError::ProcessStats(e.to_string()))?;

    let mem_parts: Vec<&str> = statm.split_whitespace().collect();
    let rss_pages: u64 = mem_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let page_size = 4096u64; // Standard page size
    let memory_bytes = rss_pages * page_size;

    Ok((cpu_time_us, memory_bytes))
}

#[cfg(target_os = "windows")]
fn get_process_stats() -> Result<(u64, u64), MeteringError> {
    use std::mem;

    // Use Windows API to get process times and memory info
    // This is a simplified implementation
    unsafe {
        let handle = windows_sys::Win32::System::Threading::GetCurrentProcess();

        // Get process times
        let mut creation_time = mem::zeroed();
        let mut exit_time = mem::zeroed();
        let mut kernel_time = mem::zeroed();
        let mut user_time = mem::zeroed();

        let success = windows_sys::Win32::System::Threading::GetProcessTimes(
            handle,
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        );

        if success == 0 {
            return Err(MeteringError::ProcessStats("GetProcessTimes failed".into()));
        }

        // FILETIME is in 100-nanosecond intervals
        let kernel_us = filetime_to_us(&kernel_time);
        let user_us = filetime_to_us(&user_time);
        let cpu_time_us = kernel_us + user_us;

        // Get memory info
        let mut mem_info: windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS =
            mem::zeroed();
        mem_info.cb = mem::size_of::<windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS>() as u32;

        let mem_success = windows_sys::Win32::System::ProcessStatus::GetProcessMemoryInfo(
            handle,
            &mut mem_info,
            mem_info.cb,
        );

        let memory_bytes = if mem_success != 0 {
            mem_info.WorkingSetSize
        } else {
            0
        };

        Ok((cpu_time_us, memory_bytes))
    }
}

#[cfg(target_os = "windows")]
fn filetime_to_us(ft: &windows_sys::Win32::Foundation::FILETIME) -> u64 {
    let time = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    time / 10 // 100-nanosecond to microseconds
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn get_process_stats() -> Result<(u64, u64), MeteringError> {
    // Fallback for other platforms
    Ok((0, 0))
}

/// Detect CPU frequency in GHz
fn detect_cpu_frequency() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in content.lines() {
                if line.starts_with("cpu MHz") {
                    if let Some(mhz_str) = line.split(':').nth(1) {
                        if let Ok(mhz) = mhz_str.trim().parse::<f64>() {
                            return mhz / 1000.0;
                        }
                    }
                }
            }
        }
        // Default assumption
        3.0
    }

    #[cfg(target_os = "windows")]
    {
        // Try to read from registry or use WMI
        // For now, use a reasonable default
        3.5
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_metrics_hc_calculation() {
        let metrics = ComputeMetrics {
            cpu_time_us: 1_000_000, // 1 second
            memory_bytes: 1_073_741_824, // 1 GB
            wall_time_us: 1_000_000, // 1 second
            pflop_hours: 0.001,
            network_in_bytes: 1_000_000,
            network_out_bytes: 500_000,
            syscall_count: None,
            disk_io_bytes: None,
            timestamp: chrono::Utc::now(),
        };

        let rates = HcRates::default();
        let hc = metrics.calculate_hc(&rates);
        assert!(hc > 0.0);
    }

    #[test]
    fn test_metering_session() {
        let session = MeteringSession::new(3.5);
        session.record_network_in(1000);
        session.record_network_out(500);

        let metrics = session.complete();
        assert_eq!(metrics.network_in_bytes, 1000);
        assert_eq!(metrics.network_out_bytes, 500);
    }

    #[test]
    fn test_metrics_merge() {
        let mut m1 = ComputeMetrics {
            cpu_time_us: 100,
            memory_bytes: 1000,
            wall_time_us: 100,
            pflop_hours: 0.001,
            network_in_bytes: 500,
            network_out_bytes: 250,
            syscall_count: Some(10),
            disk_io_bytes: Some(1000),
            timestamp: chrono::Utc::now(),
        };

        let m2 = ComputeMetrics {
            cpu_time_us: 200,
            memory_bytes: 2000,
            wall_time_us: 200,
            pflop_hours: 0.002,
            network_in_bytes: 1000,
            network_out_bytes: 500,
            syscall_count: Some(20),
            disk_io_bytes: Some(2000),
            timestamp: chrono::Utc::now(),
        };

        m1.merge(&m2);

        assert_eq!(m1.cpu_time_us, 300);
        assert_eq!(m1.memory_bytes, 2000); // max
        assert_eq!(m1.wall_time_us, 300);
        assert_eq!(m1.network_in_bytes, 1500);
        assert_eq!(m1.syscall_count, Some(30));
    }

    #[tokio::test]
    async fn test_metering_collector() {
        let collector = MeteringCollector::new(Duration::from_millis(100));
        let handle = collector.start();

        tokio::time::sleep(Duration::from_millis(250)).await;

        let metrics = collector.get_metrics();
        // Should have collected some metrics
        assert!(metrics.wall_time_us > 0);

        collector.stop();
        handle.abort();
    }
}
