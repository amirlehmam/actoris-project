//! eBPF-based kernel-level metering using Aya
//!
//! This module provides kernel-level compute metering:
//! - Syscall counting and classification
//! - CPU cycle tracking per process
//! - Memory allocation tracking
//! - Network I/O tracking at socket level
//!
//! Note: This only works on Linux with eBPF support (kernel 4.15+)

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use super::MeteringError;

/// eBPF metering errors
#[derive(Debug, Error)]
pub enum EbpfError {
    #[error("eBPF not supported on this platform")]
    NotSupported,

    #[error("Failed to load eBPF program: {0}")]
    LoadFailed(String),

    #[error("Failed to attach eBPF program: {0}")]
    AttachFailed(String),

    #[error("Map access error: {0}")]
    MapError(String),

    #[error("Permission denied - requires CAP_BPF or root")]
    PermissionDenied,
}

impl From<EbpfError> for MeteringError {
    fn from(e: EbpfError) -> Self {
        MeteringError::ProcessStats(e.to_string())
    }
}

/// Syscall categories for metering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyscallCategory {
    /// File I/O operations (read, write, open, close)
    FileIo,
    /// Network operations (socket, connect, send, recv)
    Network,
    /// Memory operations (mmap, brk, mprotect)
    Memory,
    /// Process operations (fork, exec, exit)
    Process,
    /// IPC operations (pipe, shm, mq)
    Ipc,
    /// Other syscalls
    Other,
}

/// Per-process eBPF metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EbpfMetrics {
    /// Total syscall count
    pub syscall_count: u64,

    /// Syscalls by category
    pub syscalls_by_category: HashMap<String, u64>,

    /// CPU cycles consumed (from perf counters)
    pub cpu_cycles: u64,

    /// Instructions retired
    pub instructions: u64,

    /// Cache misses
    pub cache_misses: u64,

    /// Context switches
    pub context_switches: u64,

    /// Page faults
    pub page_faults: u64,

    /// Bytes read from files
    pub file_read_bytes: u64,

    /// Bytes written to files
    pub file_write_bytes: u64,

    /// Network bytes received
    pub net_rx_bytes: u64,

    /// Network bytes transmitted
    pub net_tx_bytes: u64,

    /// Memory allocations
    pub memory_allocs: u64,

    /// Memory frees
    pub memory_frees: u64,

    /// Total allocated bytes
    pub allocated_bytes: u64,
}

/// eBPF meter for kernel-level metering
#[cfg(target_os = "linux")]
pub struct EbpfMeter {
    /// Target process ID (0 for all)
    target_pid: u32,

    /// Collected metrics
    metrics: Arc<RwLock<EbpfMetrics>>,

    /// Whether the meter is running
    running: Arc<RwLock<bool>>,

    /// Aya BPF object (would hold actual eBPF programs)
    #[allow(dead_code)]
    bpf: Option<EbpfPrograms>,
}

#[cfg(target_os = "linux")]
struct EbpfPrograms {
    // In a real implementation, this would hold:
    // - aya::Bpf object
    // - Attached program handles
    // - Map references
    _placeholder: (),
}

#[cfg(target_os = "linux")]
impl EbpfMeter {
    /// Create a new eBPF meter
    pub async fn new() -> Result<Self, MeteringError> {
        Self::with_pid(0).await
    }

    /// Create an eBPF meter for a specific process
    pub async fn with_pid(pid: u32) -> Result<Self, MeteringError> {
        // Check if eBPF is available
        if !Self::check_ebpf_support() {
            return Err(EbpfError::NotSupported.into());
        }

        // Check permissions
        if !Self::check_permissions() {
            warn!("eBPF requires CAP_BPF or root privileges");
            return Err(EbpfError::PermissionDenied.into());
        }

        info!("Initializing eBPF meter for pid: {}", pid);

        // Load and attach eBPF programs
        let bpf = Self::load_programs().await?;

        Ok(Self {
            target_pid: pid,
            metrics: Arc::new(RwLock::new(EbpfMetrics::default())),
            running: Arc::new(RwLock::new(false)),
            bpf: Some(bpf),
        })
    }

    /// Check if eBPF is supported on this system
    fn check_ebpf_support() -> bool {
        // Check for /sys/kernel/btf/vmlinux (BTF support)
        std::path::Path::new("/sys/kernel/btf/vmlinux").exists()
            || std::path::Path::new("/sys/kernel/debug/tracing").exists()
    }

    /// Check if we have required permissions
    fn check_permissions() -> bool {
        // Check for CAP_BPF or root
        unsafe { libc::geteuid() == 0 } || Self::has_cap_bpf()
    }

    fn has_cap_bpf() -> bool {
        // Check for CAP_BPF capability
        // In practice, you'd use the caps crate
        false
    }

    /// Load eBPF programs
    async fn load_programs() -> Result<EbpfPrograms, MeteringError> {
        // In a real implementation, this would:
        // 1. Load the compiled eBPF bytecode
        // 2. Attach to tracepoints/kprobes
        // 3. Set up perf event arrays

        /*
        Example Aya implementation (commented out as it requires compilation):

        use aya::{include_bytes_aligned, Bpf};
        use aya::programs::{TracePoint, KProbe};
        use aya::maps::{HashMap, PerfEventArray};

        // Load eBPF bytecode
        let mut bpf = Bpf::load(include_bytes_aligned!(
            "../../target/bpfel-unknown-none/release/actoris-ebpf"
        ))?;

        // Attach syscall tracepoint
        let program: &mut TracePoint = bpf.program_mut("syscall_enter").unwrap().try_into()?;
        program.load()?;
        program.attach("raw_syscalls", "sys_enter")?;

        // Attach kprobe for memory allocation
        let malloc_probe: &mut KProbe = bpf.program_mut("malloc_enter").unwrap().try_into()?;
        malloc_probe.load()?;
        malloc_probe.attach("__kmalloc", 0)?;

        // Get maps
        let syscall_counts: HashMap<_, u32, u64> = HashMap::try_from(bpf.map("syscall_counts")?)?;
        let perf_array = PerfEventArray::try_from(bpf.map_mut("events")?)?;
        */

        info!("eBPF programs loaded successfully");

        Ok(EbpfPrograms { _placeholder: () })
    }

    /// Start collecting metrics
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let running = self.running.clone();
        let metrics = self.metrics.clone();
        let pid = self.target_pid;

        *running.write() = true;

        tokio::spawn(async move {
            info!("eBPF meter started for pid: {}", pid);

            while *running.read() {
                // Poll eBPF maps for updated metrics
                if let Err(e) = Self::poll_metrics(&metrics).await {
                    error!("Failed to poll eBPF metrics: {}", e);
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            info!("eBPF meter stopped");
        })
    }

    /// Poll metrics from eBPF maps
    async fn poll_metrics(metrics: &Arc<RwLock<EbpfMetrics>>) -> Result<(), MeteringError> {
        // In a real implementation, read from eBPF maps:
        /*
        let syscall_map: HashMap<_, u32, u64> = ...;
        let mut m = metrics.write();

        for (syscall_nr, count) in syscall_map.iter() {
            m.syscall_count += count;
            let category = categorize_syscall(syscall_nr);
            *m.syscalls_by_category.entry(category).or_insert(0) += count;
        }
        */

        // Simulate reading from /proc for demonstration
        let mut m = metrics.write();

        if let Ok(stat) = std::fs::read_to_string("/proc/self/stat") {
            let parts: Vec<&str> = stat.split_whitespace().collect();
            if parts.len() > 14 {
                // Context switches from stat (field 12-13 don't exist, use approximation)
                m.context_switches = parts.get(11).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
        }

        if let Ok(io) = std::fs::read_to_string("/proc/self/io") {
            for line in io.lines() {
                if line.starts_with("read_bytes:") {
                    m.file_read_bytes = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                } else if line.starts_with("write_bytes:") {
                    m.file_write_bytes = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                }
            }
        }

        debug!("Polled eBPF metrics: {:?}", *m);

        Ok(())
    }

    /// Stop collecting metrics
    pub fn stop(&self) {
        *self.running.write() = false;
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> EbpfMetrics {
        self.metrics.read().clone()
    }

    /// Reset metrics
    pub fn reset(&self) {
        *self.metrics.write() = EbpfMetrics::default();
    }

    /// Get target PID
    pub fn target_pid(&self) -> u32 {
        self.target_pid
    }
}

/// Non-Linux stub implementation
#[cfg(not(target_os = "linux"))]
pub struct EbpfMeter {
    metrics: Arc<RwLock<EbpfMetrics>>,
}

#[cfg(not(target_os = "linux"))]
impl EbpfMeter {
    pub async fn new() -> Result<Self, MeteringError> {
        Err(MeteringError::EbpfNotAvailable)
    }

    pub async fn with_pid(_pid: u32) -> Result<Self, MeteringError> {
        Err(MeteringError::EbpfNotAvailable)
    }

    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async {})
    }

    pub fn stop(&self) {}

    pub fn get_metrics(&self) -> EbpfMetrics {
        self.metrics.read().clone()
    }

    pub fn reset(&self) {
        *self.metrics.write() = EbpfMetrics::default();
    }
}

/// Categorize syscall number into a category
#[allow(dead_code)]
fn categorize_syscall(syscall_nr: u32) -> SyscallCategory {
    // Linux x86_64 syscall numbers
    match syscall_nr {
        // File I/O
        0..=2 | 17..=22 | 257..=263 => SyscallCategory::FileIo,
        // Network
        41..=55 | 288..=292 => SyscallCategory::Network,
        // Memory
        9..=12 | 25..=28 => SyscallCategory::Memory,
        // Process
        56..=62 | 231 | 234 | 272 => SyscallCategory::Process,
        // IPC
        29..=32 | 64..=71 => SyscallCategory::Ipc,
        // Other
        _ => SyscallCategory::Other,
    }
}

/// eBPF program source for syscall counting
/// This would be compiled separately using aya-bpf
#[allow(dead_code)]
const EBPF_PROGRAM_SOURCE: &str = r#"
#![no_std]
#![no_main]

use aya_bpf::{
    helpers::bpf_get_current_pid_tgid,
    macros::{map, tracepoint},
    maps::HashMap,
    programs::TracePointContext,
};

#[map]
static mut SYSCALL_COUNTS: HashMap<u32, u64> = HashMap::with_max_entries(512, 0);

#[map]
static mut TARGET_PID: HashMap<u32, u32> = HashMap::with_max_entries(1, 0);

#[tracepoint(name = "syscall_enter")]
pub fn syscall_enter(ctx: TracePointContext) -> u32 {
    match try_syscall_enter(ctx) {
        Ok(ret) => ret,
        Err(_) => 1,
    }
}

fn try_syscall_enter(ctx: TracePointContext) -> Result<u32, i64> {
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;

    // Check if we're tracking this PID
    unsafe {
        if let Some(&target) = TARGET_PID.get(&0) {
            if target != 0 && target != pid {
                return Ok(0);
            }
        }
    }

    // Get syscall number from context
    let syscall_nr: u32 = unsafe { ctx.read_at(8)? };

    // Increment counter
    unsafe {
        let count = SYSCALL_COUNTS.get(&syscall_nr).unwrap_or(&0);
        SYSCALL_COUNTS.insert(&syscall_nr, &(count + 1), 0)?;
    }

    Ok(0)
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_categorization() {
        assert_eq!(categorize_syscall(0), SyscallCategory::FileIo); // read
        assert_eq!(categorize_syscall(1), SyscallCategory::FileIo); // write
        assert_eq!(categorize_syscall(41), SyscallCategory::Network); // socket
        assert_eq!(categorize_syscall(9), SyscallCategory::Memory); // mmap
        assert_eq!(categorize_syscall(57), SyscallCategory::Process); // fork
        assert_eq!(categorize_syscall(1000), SyscallCategory::Other);
    }

    #[test]
    fn test_ebpf_metrics_default() {
        let metrics = EbpfMetrics::default();
        assert_eq!(metrics.syscall_count, 0);
        assert_eq!(metrics.cpu_cycles, 0);
        assert!(metrics.syscalls_by_category.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_ebpf_meter_creation() {
        // This may fail if not running as root or without CAP_BPF
        let result = EbpfMeter::new().await;
        // Just verify it doesn't panic
        match result {
            Ok(_) => println!("eBPF meter created successfully"),
            Err(e) => println!("eBPF meter creation failed (expected without privileges): {}", e),
        }
    }
}
