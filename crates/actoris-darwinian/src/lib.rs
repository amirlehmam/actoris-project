//! # Darwinian
//!
//! PID-controlled resource allocation engine for Actoris Economic OS.
//!
//! ## Fitness Formula
//!
//! ```text
//! η = τ × (revenue / cost)
//! ```
//!
//! Where:
//! - η: Fitness score
//! - τ: Trust score (0-1)
//! - revenue: HC earned
//! - cost: HC consumed
//!
//! ## Culling Policy
//!
//! Agents with fitness < 0.7 for 2 consecutive epochs are culled.

pub mod controller;
pub mod culling;
pub mod fitness;
pub mod telemetry;

/// Darwinian configuration
#[derive(Debug, Clone)]
pub struct DarwinianConfig {
    /// Target efficiency ratio
    pub target_efficiency: f64,
    /// Culling threshold
    pub cull_threshold: f64,
    /// Grace epochs before culling
    pub grace_epochs: u64,
    /// PID Kp coefficient
    pub pid_kp: f64,
    /// PID Ki coefficient
    pub pid_ki: f64,
    /// PID Kd coefficient
    pub pid_kd: f64,
}

impl Default for DarwinianConfig {
    fn default() -> Self {
        Self {
            target_efficiency: 1.05,
            cull_threshold: 0.7,
            grace_epochs: 2,
            pid_kp: 0.5,
            pid_ki: 0.1,
            pid_kd: 0.05,
        }
    }
}
