//! Pricing module
//!
//! Provides Zen-Engine based pricing calculation with:
//! - Configurable business rules
//! - Redis caching for performance
//! - Trust-based discounts
//! - Risk-based premiums

pub mod cache;
pub mod engine;
pub mod formula;

pub use cache::{CacheStats, InMemoryPricingCache, PricingCache};
pub use engine::{PricingEngine, PricingInput, PricingOutput, PricingRulesBuilder};
