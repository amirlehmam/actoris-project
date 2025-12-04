//! # OneBill
//!
//! Pricing engine, metering, and billing for Actoris Economic OS.
//!
//! ## Pricing Formula
//!
//! ```text
//! Price = Compute + Risk - Trust
//! P = C + R - T
//! ```
//!
//! Where:
//! - C: Base compute cost (PFLOP-hours × rate)
//! - R: Risk premium (task complexity + data sensitivity)
//! - T: Trust discount (up to 20% for high-trust actors)

pub mod billing;
pub mod grpc;
pub mod metering;
pub mod pricing;

use actoris_common::{PricingRequest, PricingResponse, Result};

/// OneBill configuration
#[derive(Debug, Clone)]
pub struct OneBillConfig {
    /// Redis connection URL
    pub redis_url: String,
    /// Base rate per PFLOP-hour
    pub base_rate: rust_decimal::Decimal,
    /// Pricing rules directory
    pub rules_dir: String,
    /// gRPC listen address
    pub grpc_addr: String,
}

impl Default for OneBillConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            base_rate: rust_decimal::Decimal::ONE,
            rules_dir: "./rules".to_string(),
            grpc_addr: "[::1]:50052".to_string(),
        }
    }
}

/// OneBill service
pub struct OneBill {
    config: OneBillConfig,
}

impl OneBill {
    pub fn new(config: OneBillConfig) -> Self {
        Self { config }
    }

    /// Calculate price for a request
    pub async fn calculate_price(&self, request: &PricingRequest) -> Result<PricingResponse> {
        // Use the simple calculator from actoris-common for now
        let calculator =
            actoris_common::types::pricing::SimplePricingCalculator::new(self.config.base_rate);
        Ok(calculator.calculate(request))
    }
}
