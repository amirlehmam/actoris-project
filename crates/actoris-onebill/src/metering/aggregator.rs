//! Usage aggregation with DashMap
//!
//! Aggregates usage events by actor, client, and time period for billing.

use super::collector::{UsageBatch, UsageEvent};
use actoris_common::Result;
use dashmap::DashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

/// Aggregation key for grouping usage
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregationKey {
    /// Actor DID
    pub actor_did: String,
    /// Client DID
    pub client_did: String,
    /// Action type
    pub action_type: String,
    /// Aggregation period (hour boundary, Unix millis)
    pub period_start: i64,
}

impl AggregationKey {
    /// Create a key from a usage event
    pub fn from_event(event: &UsageEvent, period_ms: u64) -> Self {
        let period_start = (event.timestamp / period_ms as i64) * period_ms as i64;
        Self {
            actor_did: event.actor_did.clone(),
            client_did: event.client_did.clone(),
            action_type: event.action_type.clone(),
            period_start,
        }
    }
}

/// Aggregated usage for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedUsage {
    /// Aggregation key
    pub key: AggregationKey,
    /// Total compute consumed (PFLOP-hours)
    pub total_compute: Decimal,
    /// Total input bytes
    pub total_input_bytes: u64,
    /// Total output bytes
    pub total_output_bytes: u64,
    /// Event count
    pub event_count: u64,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// First event timestamp
    pub first_event_at: i64,
    /// Last event timestamp
    pub last_event_at: i64,
    /// Associated outcome IDs
    pub outcome_ids: Vec<Uuid>,
}

impl AggregatedUsage {
    fn new(key: AggregationKey, event: &UsageEvent) -> Self {
        Self {
            key,
            total_compute: event.compute_hc,
            total_input_bytes: event.input_bytes,
            total_output_bytes: event.output_bytes,
            event_count: 1,
            avg_latency_ms: event.latency_ms as f64,
            first_event_at: event.timestamp,
            last_event_at: event.timestamp,
            outcome_ids: event.outcome_id.into_iter().collect(),
        }
    }

    fn add_event(&mut self, event: &UsageEvent) {
        self.total_compute += event.compute_hc;
        self.total_input_bytes += event.input_bytes;
        self.total_output_bytes += event.output_bytes;

        // Update average latency
        let old_count = self.event_count as f64;
        let new_count = old_count + 1.0;
        self.avg_latency_ms = (self.avg_latency_ms * old_count + event.latency_ms as f64) / new_count;

        self.event_count += 1;
        self.last_event_at = event.timestamp.max(self.last_event_at);

        if let Some(outcome_id) = event.outcome_id {
            self.outcome_ids.push(outcome_id);
        }
    }
}

/// Actor usage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorUsageSummary {
    /// Actor DID
    pub actor_did: String,
    /// Period start (Unix millis)
    pub period_start: i64,
    /// Period end (Unix millis)
    pub period_end: i64,
    /// Total compute consumed
    pub total_compute: Decimal,
    /// Total events
    pub total_events: u64,
    /// Unique clients served
    pub unique_clients: u64,
    /// Action types used
    pub action_types: Vec<String>,
}

/// Metering aggregator
pub struct MeteringAggregator {
    /// Aggregation period in milliseconds (default: 1 hour)
    period_ms: u64,
    /// Current aggregations by key
    aggregations: Arc<DashMap<AggregationKey, AggregatedUsage>>,
    /// Completed period summaries
    completed: Arc<DashMap<String, Vec<AggregatedUsage>>>,
}

impl MeteringAggregator {
    /// Create a new metering aggregator
    pub fn new(period_ms: u64) -> Self {
        Self {
            period_ms,
            aggregations: Arc::new(DashMap::new()),
            completed: Arc::new(DashMap::new()),
        }
    }

    /// Create with 1-hour aggregation period
    pub fn hourly() -> Self {
        Self::new(3600000) // 1 hour in ms
    }

    /// Process a batch of usage events
    #[instrument(skip(self, batch))]
    pub fn process_batch(&self, batch: &UsageBatch) {
        for event in &batch.events {
            self.process_event(event);
        }
        debug!(batch_id = %batch.batch_id, events = batch.events.len(), "Processed batch");
    }

    /// Process a single usage event
    pub fn process_event(&self, event: &UsageEvent) {
        let key = AggregationKey::from_event(event, self.period_ms);

        self.aggregations
            .entry(key.clone())
            .and_modify(|agg| agg.add_event(event))
            .or_insert_with(|| AggregatedUsage::new(key, event));
    }

    /// Close a period and move aggregations to completed
    #[instrument(skip(self))]
    pub fn close_period(&self, period_end: i64) -> Vec<AggregatedUsage> {
        let mut closed = Vec::new();

        self.aggregations.retain(|key, agg| {
            if key.period_start + self.period_ms as i64 <= period_end {
                // This period is complete
                let actor_key = agg.key.actor_did.clone();
                self.completed
                    .entry(actor_key)
                    .or_default()
                    .push(agg.clone());
                closed.push(agg.clone());
                false // Remove from active aggregations
            } else {
                true // Keep in active aggregations
            }
        });

        info!(closed_count = closed.len(), "Closed period aggregations");
        closed
    }

    /// Get current aggregation for a key
    pub fn get_aggregation(&self, key: &AggregationKey) -> Option<AggregatedUsage> {
        self.aggregations.get(key).map(|r| r.clone())
    }

    /// Get actor usage summary for a time range
    pub fn get_actor_summary(&self, actor_did: &str, start: i64, end: i64) -> ActorUsageSummary {
        let mut total_compute = Decimal::ZERO;
        let mut total_events = 0u64;
        let mut clients = std::collections::HashSet::new();
        let mut actions = std::collections::HashSet::new();

        // Check active aggregations
        for entry in self.aggregations.iter() {
            let (key, agg) = entry.pair();
            if key.actor_did == actor_did
                && key.period_start >= start
                && key.period_start < end
            {
                total_compute += agg.total_compute;
                total_events += agg.event_count;
                clients.insert(key.client_did.clone());
                actions.insert(key.action_type.clone());
            }
        }

        // Check completed aggregations
        if let Some(completed) = self.completed.get(actor_did) {
            for agg in completed.iter() {
                if agg.key.period_start >= start && agg.key.period_start < end {
                    total_compute += agg.total_compute;
                    total_events += agg.event_count;
                    clients.insert(agg.key.client_did.clone());
                    actions.insert(agg.key.action_type.clone());
                }
            }
        }

        ActorUsageSummary {
            actor_did: actor_did.to_string(),
            period_start: start,
            period_end: end,
            total_compute,
            total_events,
            unique_clients: clients.len() as u64,
            action_types: actions.into_iter().collect(),
        }
    }

    /// Get all current aggregations for an actor
    pub fn get_actor_aggregations(&self, actor_did: &str) -> Vec<AggregatedUsage> {
        self.aggregations
            .iter()
            .filter(|entry| entry.key().actor_did == actor_did)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Clear all aggregations (for testing)
    pub fn clear(&self) {
        self.aggregations.clear();
        self.completed.clear();
    }

    /// Start background period closer
    pub fn start_period_closer(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_millis(self.period_ms)
            );

            loop {
                interval.tick().await;
                let now = chrono::Utc::now().timestamp_millis();
                let closed = self.close_period(now);
                if !closed.is_empty() {
                    info!(count = closed.len(), "Auto-closed period aggregations");
                }
            }
        })
    }
}

impl Default for MeteringAggregator {
    fn default() -> Self {
        Self::hourly()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_aggregation_key_from_event() {
        let event = UsageEvent::new(
            "did:key:actor".into(),
            "did:key:client".into(),
            "test.action".into(),
            dec!(10),
        );

        let key = AggregationKey::from_event(&event, 3600000);
        assert_eq!(key.actor_did, "did:key:actor");
        assert_eq!(key.action_type, "test.action");
    }

    #[test]
    fn test_process_event() {
        let aggregator = MeteringAggregator::hourly();

        let event1 = UsageEvent::new(
            "did:key:actor".into(),
            "did:key:client".into(),
            "test.action".into(),
            dec!(10),
        );

        let event2 = UsageEvent::new(
            "did:key:actor".into(),
            "did:key:client".into(),
            "test.action".into(),
            dec!(20),
        );

        aggregator.process_event(&event1);
        aggregator.process_event(&event2);

        let key = AggregationKey::from_event(&event1, 3600000);
        let agg = aggregator.get_aggregation(&key).unwrap();

        assert_eq!(agg.total_compute, dec!(30));
        assert_eq!(agg.event_count, 2);
    }

    #[test]
    fn test_actor_summary() {
        let aggregator = MeteringAggregator::hourly();

        for i in 0..5 {
            let event = UsageEvent::new(
                "did:key:actor".into(),
                format!("did:key:client{}", i),
                "test.action".into(),
                dec!(10),
            );
            aggregator.process_event(&event);
        }

        let now = chrono::Utc::now().timestamp_millis();
        let summary = aggregator.get_actor_summary(
            "did:key:actor",
            now - 3600000,
            now + 3600000,
        );

        assert_eq!(summary.total_compute, dec!(50));
        assert_eq!(summary.total_events, 5);
        assert_eq!(summary.unique_clients, 5);
    }
}
