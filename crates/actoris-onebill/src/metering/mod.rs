//! Metering module
//!
//! Provides usage event collection and aggregation:
//! - UsageCollector: Collects and batches usage events
//! - MeteringAggregator: Aggregates usage by actor/client/period

pub mod aggregator;
pub mod collector;

pub use aggregator::{AggregatedUsage, AggregationKey, ActorUsageSummary, MeteringAggregator};
pub use collector::{CollectorConfig, CollectorMetrics, UsageBatch, UsageCollector, UsageEvent};
