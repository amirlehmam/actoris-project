//! ACTORIS eBPF Programs
//!
//! This crate contains eBPF programs for kernel-level compute metering:
//! - Syscall counting and classification
//! - CPU cycle tracking
//! - Memory allocation tracking
//! - Network I/O tracking
//!
//! Build with:
//! ```
//! cargo +nightly build -Z build-std=core --target bpfel-unknown-none --release
//! ```

#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    helpers::{bpf_get_current_pid_tgid, bpf_ktime_get_ns, bpf_probe_read_kernel},
    macros::{kprobe, kretprobe, map, tracepoint, xdp},
    maps::{HashMap, LruHashMap, PerfEventArray, PerCpuArray},
    programs::{ProbeContext, TracePointContext, XdpContext},
};
use aya_log_ebpf::info;

/// Maximum number of processes to track
const MAX_PROCESSES: u32 = 10240;

/// Maximum number of syscalls to track
const MAX_SYSCALLS: u32 = 512;

/// Per-process metrics
#[repr(C)]
pub struct ProcessMetrics {
    /// Process ID
    pub pid: u32,
    /// Thread group ID
    pub tgid: u32,
    /// Total syscalls made
    pub syscall_count: u64,
    /// CPU time in nanoseconds
    pub cpu_time_ns: u64,
    /// Memory allocations
    pub memory_allocs: u64,
    /// Memory frees
    pub memory_frees: u64,
    /// Total bytes allocated
    pub bytes_allocated: u64,
    /// Total bytes freed
    pub bytes_freed: u64,
    /// Network bytes received
    pub net_rx_bytes: u64,
    /// Network bytes transmitted
    pub net_tx_bytes: u64,
    /// Last update timestamp
    pub last_update_ns: u64,
}

/// Syscall event data
#[repr(C)]
pub struct SyscallEvent {
    /// Process ID
    pub pid: u32,
    /// Thread group ID
    pub tgid: u32,
    /// Syscall number
    pub syscall_nr: u64,
    /// Timestamp
    pub timestamp_ns: u64,
    /// Return value
    pub ret: i64,
    /// Latency in nanoseconds
    pub latency_ns: u64,
}

/// Memory event data
#[repr(C)]
pub struct MemoryEvent {
    /// Process ID
    pub pid: u32,
    /// Allocation size
    pub size: u64,
    /// Is allocation (true) or free (false)
    pub is_alloc: bool,
    /// Timestamp
    pub timestamp_ns: u64,
}

/// Network event data
#[repr(C)]
pub struct NetworkEvent {
    /// Process ID
    pub pid: u32,
    /// Bytes transferred
    pub bytes: u64,
    /// Direction: true = rx, false = tx
    pub is_rx: bool,
    /// Protocol (TCP = 6, UDP = 17)
    pub protocol: u8,
    /// Timestamp
    pub timestamp_ns: u64,
}

// ============ MAPS ============

/// Per-process metrics storage
#[map]
static PROCESS_METRICS: HashMap<u32, ProcessMetrics> = HashMap::with_max_entries(MAX_PROCESSES, 0);

/// Target PIDs to track (0 = track all)
#[map]
static TARGET_PIDS: HashMap<u32, u8> = HashMap::with_max_entries(1024, 0);

/// Syscall counts by number
#[map]
static SYSCALL_COUNTS: HashMap<u32, u64> = HashMap::with_max_entries(MAX_SYSCALLS, 0);

/// Syscall entry timestamps for latency calculation
#[map]
static SYSCALL_START: LruHashMap<u64, u64> = LruHashMap::with_max_entries(MAX_PROCESSES * 4, 0);

/// Perf event array for syscall events
#[map]
static SYSCALL_EVENTS: PerfEventArray<SyscallEvent> = PerfEventArray::new(0);

/// Perf event array for memory events
#[map]
static MEMORY_EVENTS: PerfEventArray<MemoryEvent> = PerfEventArray::new(0);

/// Perf event array for network events
#[map]
static NETWORK_EVENTS: PerfEventArray<NetworkEvent> = PerfEventArray::new(0);

/// Per-CPU temporary storage
#[map]
static PERCPU_SCRATCH: PerCpuArray<[u8; 256]> = PerCpuArray::with_max_entries(1, 0);

// ============ HELPER FUNCTIONS ============

/// Check if we should track this PID
#[inline(always)]
fn should_track_pid(pid: u32) -> bool {
    // Check if specific PIDs are set
    unsafe {
        if let Some(&tracking) = TARGET_PIDS.get(&pid) {
            return tracking != 0;
        }
        // Check if tracking all (pid 0 entry)
        if let Some(&track_all) = TARGET_PIDS.get(&0) {
            return track_all != 0;
        }
    }
    false
}

/// Get or create process metrics
#[inline(always)]
fn get_or_create_metrics(pid: u32, tgid: u32) -> Option<*mut ProcessMetrics> {
    unsafe {
        if let Some(metrics) = PROCESS_METRICS.get_ptr_mut(&pid) {
            return Some(metrics);
        }

        // Create new entry
        let now = bpf_ktime_get_ns();
        let new_metrics = ProcessMetrics {
            pid,
            tgid,
            syscall_count: 0,
            cpu_time_ns: 0,
            memory_allocs: 0,
            memory_frees: 0,
            bytes_allocated: 0,
            bytes_freed: 0,
            net_rx_bytes: 0,
            net_tx_bytes: 0,
            last_update_ns: now,
        };

        if PROCESS_METRICS.insert(&pid, &new_metrics, 0).is_ok() {
            return PROCESS_METRICS.get_ptr_mut(&pid);
        }
    }
    None
}

// ============ TRACEPOINTS ============

/// Syscall entry tracepoint
#[tracepoint]
pub fn syscall_enter(ctx: TracePointContext) -> u32 {
    match try_syscall_enter(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_syscall_enter(ctx: TracePointContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    if !should_track_pid(pid) && !should_track_pid(tgid) {
        return Ok(0);
    }

    // Read syscall number from context
    // Offset 8 in sys_enter tracepoint format
    let syscall_nr: u64 = unsafe { ctx.read_at(8)? };

    // Store entry timestamp for latency calculation
    let now = unsafe { bpf_ktime_get_ns() };
    let key = pid_tgid;
    unsafe {
        let _ = SYSCALL_START.insert(&key, &now, 0);
    }

    // Increment syscall count
    unsafe {
        let syscall_key = syscall_nr as u32;
        if let Some(count) = SYSCALL_COUNTS.get_ptr_mut(&syscall_key) {
            *count += 1;
        } else {
            let _ = SYSCALL_COUNTS.insert(&syscall_key, &1u64, 0);
        }

        // Update process metrics
        if let Some(metrics) = get_or_create_metrics(pid, tgid) {
            (*metrics).syscall_count += 1;
            (*metrics).last_update_ns = now;
        }
    }

    Ok(0)
}

/// Syscall exit tracepoint
#[tracepoint]
pub fn syscall_exit(ctx: TracePointContext) -> u32 {
    match try_syscall_exit(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_syscall_exit(ctx: TracePointContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    if !should_track_pid(pid) {
        return Ok(0);
    }

    // Calculate latency
    let now = unsafe { bpf_ktime_get_ns() };
    let key = pid_tgid;

    unsafe {
        if let Some(&start) = SYSCALL_START.get(&key) {
            let latency = now.saturating_sub(start);

            // Read syscall number and return value
            let syscall_nr: u64 = ctx.read_at(8)?;
            let ret: i64 = ctx.read_at(16)?;

            // Send event to userspace
            let event = SyscallEvent {
                pid,
                tgid: pid_tgid as u32,
                syscall_nr,
                timestamp_ns: now,
                ret,
                latency_ns: latency,
            };

            SYSCALL_EVENTS.output(&ctx, &event, 0);

            // Clean up
            let _ = SYSCALL_START.remove(&key);
        }
    }

    Ok(0)
}

// ============ KPROBES ============

/// Track memory allocations (kmalloc)
#[kprobe]
pub fn kmalloc_enter(ctx: ProbeContext) -> u32 {
    match try_kmalloc_enter(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_kmalloc_enter(ctx: ProbeContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    if !should_track_pid(pid) {
        return Ok(0);
    }

    // Size is first argument
    let size: u64 = ctx.arg(0).ok_or(1i64)?;
    let now = unsafe { bpf_ktime_get_ns() };

    unsafe {
        if let Some(metrics) = get_or_create_metrics(pid, tgid) {
            (*metrics).memory_allocs += 1;
            (*metrics).bytes_allocated += size;
            (*metrics).last_update_ns = now;
        }
    }

    // Send event
    let event = MemoryEvent {
        pid,
        size,
        is_alloc: true,
        timestamp_ns: now,
    };

    MEMORY_EVENTS.output(&ctx, &event, 0);

    Ok(0)
}

/// Track memory frees (kfree)
#[kprobe]
pub fn kfree_enter(ctx: ProbeContext) -> u32 {
    match try_kfree_enter(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_kfree_enter(ctx: ProbeContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    if !should_track_pid(pid) {
        return Ok(0);
    }

    let now = unsafe { bpf_ktime_get_ns() };

    unsafe {
        if let Some(metrics) = get_or_create_metrics(pid, tgid) {
            (*metrics).memory_frees += 1;
            (*metrics).last_update_ns = now;
        }
    }

    // Send event (size unknown for kfree)
    let event = MemoryEvent {
        pid,
        size: 0,
        is_alloc: false,
        timestamp_ns: now,
    };

    MEMORY_EVENTS.output(&ctx, &event, 0);

    Ok(0)
}

/// Track TCP receive
#[kprobe]
pub fn tcp_recvmsg_enter(ctx: ProbeContext) -> u32 {
    match try_tcp_recv(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_tcp_recv(ctx: ProbeContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    if !should_track_pid(pid) {
        return Ok(0);
    }

    // Size is third argument in tcp_recvmsg
    let size: u64 = ctx.arg(2).ok_or(1i64)?;
    let now = unsafe { bpf_ktime_get_ns() };

    unsafe {
        if let Some(metrics) = get_or_create_metrics(pid, tgid) {
            (*metrics).net_rx_bytes += size;
            (*metrics).last_update_ns = now;
        }
    }

    // Send event
    let event = NetworkEvent {
        pid,
        bytes: size,
        is_rx: true,
        protocol: 6, // TCP
        timestamp_ns: now,
    };

    NETWORK_EVENTS.output(&ctx, &event, 0);

    Ok(0)
}

/// Track TCP send
#[kprobe]
pub fn tcp_sendmsg_enter(ctx: ProbeContext) -> u32 {
    match try_tcp_send(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_tcp_send(ctx: ProbeContext) -> Result<u32, i64> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    if !should_track_pid(pid) {
        return Ok(0);
    }

    // Size is third argument in tcp_sendmsg
    let size: u64 = ctx.arg(2).ok_or(1i64)?;
    let now = unsafe { bpf_ktime_get_ns() };

    unsafe {
        if let Some(metrics) = get_or_create_metrics(pid, tgid) {
            (*metrics).net_tx_bytes += size;
            (*metrics).last_update_ns = now;
        }
    }

    // Send event
    let event = NetworkEvent {
        pid,
        bytes: size,
        is_rx: false,
        protocol: 6, // TCP
        timestamp_ns: now,
    };

    NETWORK_EVENTS.output(&ctx, &event, 0);

    Ok(0)
}

// ============ XDP (Optional) ============

/// XDP program for early packet processing
#[xdp]
pub fn actoris_xdp(ctx: XdpContext) -> u32 {
    match try_xdp(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_PASS,
    }
}

fn try_xdp(_ctx: XdpContext) -> Result<u32, i64> {
    // Pass all packets - this is just a placeholder for future packet analysis
    Ok(xdp_action::XDP_PASS)
}

// ============ PANIC HANDLER ============

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
