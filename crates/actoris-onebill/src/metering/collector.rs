//! Usage event collector
//!
//! Collects and validates usage events from actors, providing:
//! - Event validation and deduplication
//! - Batch processing for efficiency
//! - Real-time streaming to aggregator

use actoris_common::{ActorisError, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Usage event representing compute consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    /// Unique event ID
    pub event_id: Uuid,
    /// Actor's DID
    pub actor_did: String,
    /// Client's DID (who requested the action)
    pub client_did: String,
    /// Action type identifier
    pub action_type: String,
    /// Compute consumed (PFLOP-hours)
    pub compute_hc: Decimal,
    /// Input data size (bytes)
    pub input_bytes: u64,
    /// Output data size (bytes)
    pub output_bytes: u64,
    /// Latency in milliseconds
    pub latency_ms: u32,
    /// Event timestamp (Unix millis)
    pub timestamp: i64,
    /// Associated outcome record ID (if verified)
    pub outcome_id: Option<Uuid>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

impl UsageEvent {
    /// Create a new usage event
    pub fn new(
        actor_did: String,
        client_did: String,
        action_type: String,
        compute_hc: Decimal,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            actor_did,
            client_did,
            action_type,
            compute_hc,
            input_bytes: 0,
            output_bytes: 0,
            latency_ms: 0,
            timestamp: chrono::Utc::now().timestamp_millis(),
            outcome_id: None,
            metadata: None,
        }
    }

    /// Set input/output sizes
    pub fn with_io_sizes(mut self, input_bytes: u64, output_bytes: u64) -> Self {
        self.input_bytes = input_bytes;
        self.output_bytes = output_bytes;
        self
    }

    /// Set latency
    pub fn with_latency(mut self, latency_ms: u32) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// Set associated outcome record
    pub fn with_outcome(mut self, outcome_id: Uuid) -> Self {
        self.outcome_id = Some(outcome_id);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Validate the event
    pub fn validate(&self) -> Result<()> {
        if self.actor_did.is_empty() {
            return Err(ActorisError::Validation("actor_did is required".into()));
        }
        if self.client_did.is_empty() {
            return Err(ActorisError::Validation("client_did is required".into()));
        }
        if self.action_type.is_empty() {
            return Err(ActorisError::Validation("action_type is required".into()));
        }
        if self.compute_hc < Decimal::ZERO {
            return Err(ActorisError::Validation("compute_hc cannot be negative".into()));
        }
        Ok(())
    }
}

/// Batch of usage events
#[derive(Debug, Clone)]
pub struct UsageBatch {
    /// Batch ID
    pub batch_id: Uuid,
    /// Events in this batch
    pub events: Vec<UsageEvent>,
    /// Batch created timestamp
    pub created_at: i64,
}

impl UsageBatch {
    pub fn new(events: Vec<UsageEvent>) -> Self {
        Self {
            batch_id: Uuid::new_v4(),
            events,
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Total compute in this batch
    pub fn total_compute(&self) -> Decimal {
        self.events.iter().map(|e| e.compute_hc).sum()
    }

    /// Number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Configuration for the usage collector
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    /// Maximum events per batch
    pub batch_size: usize,
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
    /// Channel buffer size
    pub channel_buffer: usize,
    /// Enable deduplication
    pub deduplicate: bool,
    /// Deduplication window (milliseconds)
    pub dedup_window_ms: u64,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval_ms: 1000,
            channel_buffer: 10000,
            deduplicate: true,
            dedup_window_ms: 60000, // 1 minute
        }
    }
}

/// Usage event collector
pub struct UsageCollector {
    config: CollectorConfig,
    /// Channel sender for incoming events
    event_tx: mpsc::Sender<UsageEvent>,
    /// Recent event IDs for deduplication
    seen_events: Arc<dashmap::DashMap<Uuid, i64>>,
    /// Metrics
    metrics: CollectorMetrics,
}

/// Collector metrics
#[derive(Debug, Default)]
pub struct CollectorMetrics {
    pub events_received: std::sync::atomic::AtomicU64,
    pub events_dropped: std::sync::atomic::AtomicU64,
    pub events_deduplicated: std::sync::atomic::AtomicU64,
    pub batches_sent: std::sync::atomic::AtomicU64,
}

impl UsageCollector {
    /// Create a new usage collector
    pub fn new(config: CollectorConfig) -> (Self, mpsc::Receiver<UsageBatch>) {
        let (event_tx, event_rx) = mpsc::channel(config.channel_buffer);
        let (batch_tx, batch_rx) = mpsc::channel(100);

        let collector = Self {
            config: config.clone(),
            event_tx,
            seen_events: Arc::new(dashmap::DashMap::new()),
            metrics: CollectorMetrics::default(),
        };

        // Spawn background batch processor
        let seen = collector.seen_events.clone();
        tokio::spawn(Self::batch_processor(config, event_rx, batch_tx, seen));

        (collector, batch_rx)
    }

    /// Submit a usage event
    #[instrument(skip(self, event))]
    pub async fn submit(&self, event: UsageEvent) -> Result<()> {
        // Validate event
        event.validate()?;

        // Check for duplicates
        if self.config.deduplicate {
            let now = chrono::Utc::now().timestamp_millis();
            if let Some(seen_at) = self.seen_events.get(&event.event_id) {
                if now - *seen_at < self.config.dedup_window_ms as i64 {
                    self.metrics.events_deduplicated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    debug!(event_id = %event.event_id, "Deduplicated event");
                    return Ok(());
                }
            }
            self.seen_events.insert(event.event_id, now);
        }

        // Send to channel
        self.event_tx
            .send(event)
            .await
            .map_err(|_| ActorisError::Internal("Event channel closed".into()))?;

        self.metrics.events_received.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Get collector metrics
    pub fn metrics(&self) -> &CollectorMetrics {
        &self.metrics
    }

    /// Background batch processor
    async fn batch_processor(
        config: CollectorConfig,
        mut event_rx: mpsc::Receiver<UsageEvent>,
        batch_tx: mpsc::Sender<UsageBatch>,
        seen_events: Arc<dashmap::DashMap<Uuid, i64>>,
    ) {
        let mut pending_events: Vec<UsageEvent> = Vec::with_capacity(config.batch_size);
        let flush_interval = tokio::time::Duration::from_millis(config.flush_interval_ms);
        let mut flush_timer = tokio::time::interval(flush_interval);

        // Periodic cleanup of seen events
        let cleanup_interval = tokio::time::Duration::from_secs(60);
        let mut cleanup_timer = tokio::time::interval(cleanup_interval);

        loop {
            tokio::select! {
                // Receive events
                event = event_rx.recv() => {
                    match event {
                        Some(e) => {
                            pending_events.push(e);

                            // Flush if batch is full
                            if pending_events.len() >= config.batch_size {
                                let batch = UsageBatch::new(std::mem::take(&mut pending_events));
                                if let Err(e) = batch_tx.send(batch).await {
                                    error!("Failed to send batch: {}", e);
                                }
                            }
                        }
                        None => {
                            // Channel closed, flush remaining and exit
                            if !pending_events.is_empty() {
                                let batch = UsageBatch::new(std::mem::take(&mut pending_events));
                                let _ = batch_tx.send(batch).await;
                            }
                            info!("Event channel closed, batch processor exiting");
                            break;
                        }
                    }
                }

                // Periodic flush
                _ = flush_timer.tick() => {
                    if !pending_events.is_empty() {
                        let batch = UsageBatch::new(std::mem::take(&mut pending_events));
                        if let Err(e) = batch_tx.send(batch).await {
                            error!("Failed to send batch: {}", e);
                        }
                    }
                }

                // Cleanup old seen events
                _ = cleanup_timer.tick() => {
                    let now = chrono::Utc::now().timestamp_millis();
                    let window = config.dedup_window_ms as i64;
                    seen_events.retain(|_, seen_at| now - *seen_at < window);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_usage_event_creation() {
        let event = UsageEvent::new(
            "did:key:actor".to_string(),
            "did:key:client".to_string(),
            "test.action".to_string(),
            dec!(10.5),
        );

        assert!(!event.actor_did.is_empty());
        assert_eq!(event.compute_hc, dec!(10.5));
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_usage_event_validation() {
        let event = UsageEvent::new(
            "".to_string(),
            "did:key:client".to_string(),
            "test.action".to_string(),
            dec!(10),
        );

        assert!(event.validate().is_err());
    }

    #[test]
    fn test_batch_total_compute() {
        let events = vec![
            UsageEvent::new("did:key:a".into(), "did:key:c".into(), "t".into(), dec!(10)),
            UsageEvent::new("did:key:a".into(), "did:key:c".into(), "t".into(), dec!(20)),
            UsageEvent::new("did:key:a".into(), "did:key:c".into(), "t".into(), dec!(30)),
        ];

        let batch = UsageBatch::new(events);
        assert_eq!(batch.total_compute(), dec!(60));
    }

    #[tokio::test]
    async fn test_collector_submit() {
        let config = CollectorConfig::default();
        let (collector, _rx) = UsageCollector::new(config);

        let event = UsageEvent::new(
            "did:key:actor".to_string(),
            "did:key:client".to_string(),
            "test.action".to_string(),
            dec!(10),
        );

        let result = collector.submit(event).await;
        assert!(result.is_ok());
    }
}
