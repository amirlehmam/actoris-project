//! EventStoreDB client for immutable ledger
//!
//! Provides append-only storage for OutcomeRecords with:
//! - Stream-based organization by actor/action type
//! - Optimistic concurrency via expected revision
//! - Subscription support for real-time verification events

use actoris_common::{ActorisError, OutcomeRecord, Result};
use eventstore::{
    AppendToStreamOptions, Client, ClientSettings, EventData, ExpectedRevision, ReadStreamOptions,
    ResolvedEvent, StreamPosition, SubscribeToStreamOptions,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};

/// EventStoreDB client wrapper for TrustLedger
pub struct EventStoreClient {
    client: Client,
    /// Stream prefix for all Actoris events
    stream_prefix: String,
}

/// Event types stored in EventStoreDB
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum LedgerEvent {
    /// Action submitted for verification
    ActionSubmitted {
        request_id: String,
        actor_did: String,
        client_did: String,
        action_type: String,
        input_hash: [u8; 32],
        output_hash: [u8; 32],
        compute_hc: String,
        timestamp: i64,
    },
    /// Verification started
    VerificationStarted {
        request_id: String,
        oracle_count: u8,
        timestamp: i64,
    },
    /// Oracle vote received
    OracleVoteReceived {
        request_id: String,
        oracle_did: String,
        approved: bool,
        reason: Option<String>,
        timestamp: i64,
    },
    /// Verification completed
    VerificationCompleted {
        request_id: String,
        passed: bool,
        quorum_reached: bool,
        latency_ms: u32,
        signature: Vec<u8>,
        timestamp: i64,
    },
    /// Outcome record finalized (full record with Merkle proof)
    OutcomeRecordFinalized { record: OutcomeRecordData },
}

/// Serializable outcome record data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeRecordData {
    pub id: String,
    pub actor_did: String,
    pub client_did: String,
    pub action_type: String,
    pub input_hash: Vec<u8>,
    pub output_hash: Vec<u8>,
    pub compute_hc: String,
    pub verification_passed: bool,
    pub verification_latency_ms: u32,
    pub signature: Vec<u8>,
    pub signers: Vec<String>,
    pub merkle_proof: Vec<Vec<u8>>,
    pub merkle_root: Vec<u8>,
    pub merkle_index: u64,
    pub submitted_at: i64,
    pub verified_at: i64,
}

impl From<&OutcomeRecord> for OutcomeRecordData {
    fn from(record: &OutcomeRecord) -> Self {
        Self {
            id: record.id.to_string(),
            actor_did: record.actor_did.clone(),
            client_did: record.client_did.clone(),
            action_type: record.action_type.clone(),
            input_hash: record.input_hash.to_vec(),
            output_hash: record.output_hash.to_vec(),
            compute_hc: record.compute_hc.to_string(),
            verification_passed: record.verification.passed,
            verification_latency_ms: record.verification.latency_ms,
            signature: record.signature.signature.to_vec(),
            signers: record.signature.signers.clone(),
            merkle_proof: record.merkle_proof.iter().map(|p| p.to_vec()).collect(),
            merkle_root: record.merkle_root.to_vec(),
            merkle_index: record.merkle_index,
            submitted_at: record.submitted_at,
            verified_at: record.verified_at,
        }
    }
}

/// Stream metadata
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub stream_name: String,
    pub last_position: u64,
    pub event_count: u64,
}

impl EventStoreClient {
    /// Create a new EventStoreDB client
    #[instrument(skip(connection_string))]
    pub async fn new(connection_string: &str) -> Result<Self> {
        let settings = connection_string.parse::<ClientSettings>().map_err(|e| {
            ActorisError::Config(format!("Invalid EventStore connection string: {}", e))
        })?;

        let client = Client::new(settings)
            .map_err(|e| ActorisError::Storage(format!("Failed to create EventStore client: {}", e)))?;

        info!("Connected to EventStoreDB");

        Ok(Self {
            client,
            stream_prefix: "actoris".to_string(),
        })
    }

    /// Get stream name for a specific actor
    fn actor_stream(&self, actor_did: &str) -> String {
        // Sanitize DID for stream name (replace : with -)
        let sanitized = actor_did.replace(':', "-");
        format!("{}-actor-{}", self.stream_prefix, sanitized)
    }

    /// Get stream name for verification requests
    fn verification_stream(&self) -> String {
        format!("{}-verifications", self.stream_prefix)
    }

    /// Get stream name for finalized outcomes
    fn outcomes_stream(&self) -> String {
        format!("{}-outcomes", self.stream_prefix)
    }

    /// Append an event to a stream
    #[instrument(skip(self, event))]
    pub async fn append_event(
        &self,
        stream_name: &str,
        event: LedgerEvent,
        expected_revision: Option<u64>,
    ) -> Result<u64> {
        let event_type = match &event {
            LedgerEvent::ActionSubmitted { .. } => "ActionSubmitted",
            LedgerEvent::VerificationStarted { .. } => "VerificationStarted",
            LedgerEvent::OracleVoteReceived { .. } => "OracleVoteReceived",
            LedgerEvent::VerificationCompleted { .. } => "VerificationCompleted",
            LedgerEvent::OutcomeRecordFinalized { .. } => "OutcomeRecordFinalized",
        };

        let event_data = EventData::json(event_type, &event)
            .map_err(|e| ActorisError::Serialization(e.to_string()))?;

        let options = match expected_revision {
            Some(rev) => {
                AppendToStreamOptions::default().expected_revision(ExpectedRevision::Exact(rev))
            }
            None => AppendToStreamOptions::default().expected_revision(ExpectedRevision::Any),
        };

        let result = self
            .client
            .append_to_stream(stream_name.to_string(), &options, event_data)
            .await
            .map_err(|e| ActorisError::Storage(format!("Failed to append event: {}", e)))?;

        let position = result.next_expected_version;
        debug!(stream = stream_name, position = position, "Event appended");

        Ok(position)
    }

    /// Record action submission
    #[instrument(skip(self))]
    pub async fn record_action_submitted(
        &self,
        request_id: &str,
        actor_did: &str,
        client_did: &str,
        action_type: &str,
        input_hash: [u8; 32],
        output_hash: [u8; 32],
        compute_hc: &str,
    ) -> Result<u64> {
        let event = LedgerEvent::ActionSubmitted {
            request_id: request_id.to_string(),
            actor_did: actor_did.to_string(),
            client_did: client_did.to_string(),
            action_type: action_type.to_string(),
            input_hash,
            output_hash,
            compute_hc: compute_hc.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        // Append to both verification stream and actor stream
        let verification_stream = self.verification_stream();
        let verification_pos = self
            .append_event(&verification_stream, event.clone(), None)
            .await?;

        let actor_stream = self.actor_stream(actor_did);
        self.append_event(&actor_stream, event, None).await?;

        Ok(verification_pos)
    }

    /// Record verification completion
    #[instrument(skip(self, signature))]
    pub async fn record_verification_completed(
        &self,
        request_id: &str,
        passed: bool,
        quorum_reached: bool,
        latency_ms: u32,
        signature: &[u8],
    ) -> Result<u64> {
        let event = LedgerEvent::VerificationCompleted {
            request_id: request_id.to_string(),
            passed,
            quorum_reached,
            latency_ms,
            signature: signature.to_vec(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        let stream = self.verification_stream();
        self.append_event(&stream, event, None).await
    }

    /// Record finalized outcome
    #[instrument(skip(self, record))]
    pub async fn record_outcome_finalized(&self, record: &OutcomeRecord) -> Result<u64> {
        let event = LedgerEvent::OutcomeRecordFinalized {
            record: OutcomeRecordData::from(record),
        };

        // Append to outcomes stream
        let outcomes_stream = self.outcomes_stream();
        let position = self
            .append_event(&outcomes_stream, event.clone(), None)
            .await?;

        // Also append to actor stream
        let actor_stream = self.actor_stream(&record.actor_did);
        self.append_event(&actor_stream, event, None).await?;

        Ok(position)
    }

    /// Read events from a stream
    #[instrument(skip(self))]
    pub async fn read_stream(
        &self,
        stream_name: &str,
        from_position: Option<u64>,
        limit: usize,
    ) -> Result<Vec<(u64, LedgerEvent)>> {
        let options = ReadStreamOptions::default()
            .position(match from_position {
                Some(pos) => StreamPosition::Position(pos),
                None => StreamPosition::Start,
            })
            .max_count(limit);

        let result = self
            .client
            .read_stream(stream_name.to_string(), &options)
            .await;

        match result {
            Ok(mut stream) => {
                let mut events = Vec::new();
                loop {
                    match stream.next().await {
                        Ok(Some(resolved)) => {
                            if let Some(event) = Self::parse_event(&resolved) {
                                let position =
                                    resolved.event.as_ref().map(|e| e.revision).unwrap_or(0);
                                events.push((position, event));
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            error!("Error reading event: {}", e);
                            break;
                        }
                    }
                }
                Ok(events)
            }
            Err(e) => Err(ActorisError::Storage(format!("Failed to read stream: {}", e))),
        }
    }

    /// Read outcome records for an actor
    #[instrument(skip(self))]
    pub async fn read_actor_outcomes(
        &self,
        actor_did: &str,
        from_position: Option<u64>,
        limit: usize,
    ) -> Result<Vec<OutcomeRecordData>> {
        let stream_name = self.actor_stream(actor_did);
        let events = self.read_stream(&stream_name, from_position, limit).await?;

        let outcomes: Vec<OutcomeRecordData> = events
            .into_iter()
            .filter_map(|(_, event)| {
                if let LedgerEvent::OutcomeRecordFinalized { record } = event {
                    Some(record)
                } else {
                    None
                }
            })
            .collect();

        Ok(outcomes)
    }

    /// Subscribe to verification events
    #[instrument(skip(self, sender))]
    pub async fn subscribe_verifications(
        &self,
        sender: mpsc::Sender<LedgerEvent>,
        from_position: Option<u64>,
    ) -> Result<()> {
        let options = SubscribeToStreamOptions::default().start_from(match from_position {
            Some(pos) => StreamPosition::Position(pos),
            None => StreamPosition::Start,
        });

        let stream_name = self.verification_stream();
        let mut subscription = self
            .client
            .subscribe_to_stream(stream_name, &options)
            .await;

        tokio::spawn(async move {
            loop {
                match subscription.next().await {
                    Ok(Some(resolved)) => {
                        if let Some(event) = Self::parse_event(&resolved) {
                            if sender.send(event).await.is_err() {
                                break;
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        error!("Subscription error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Subscribe to finalized outcomes
    #[instrument(skip(self, sender))]
    pub async fn subscribe_outcomes(
        &self,
        sender: mpsc::Sender<OutcomeRecordData>,
        from_position: Option<u64>,
    ) -> Result<()> {
        let options = SubscribeToStreamOptions::default().start_from(match from_position {
            Some(pos) => StreamPosition::Position(pos),
            None => StreamPosition::Start,
        });

        let stream_name = self.outcomes_stream();
        let mut subscription = self.client.subscribe_to_stream(stream_name, &options).await;

        tokio::spawn(async move {
            loop {
                match subscription.next().await {
                    Ok(Some(resolved)) => {
                        if let Some(LedgerEvent::OutcomeRecordFinalized { record }) =
                            Self::parse_event(&resolved)
                        {
                            if sender.send(record).await.is_err() {
                                break;
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        error!("Subscription error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Parse event from resolved event
    fn parse_event(resolved: &ResolvedEvent) -> Option<LedgerEvent> {
        resolved
            .event
            .as_ref()
            .and_then(|event| event.as_json::<LedgerEvent>().ok())
    }

    /// Get stream statistics
    #[instrument(skip(self))]
    pub async fn get_stream_info(&self, stream_name: &str) -> Result<StreamInfo> {
        // Read from end to get last position
        let options = ReadStreamOptions::default()
            .position(StreamPosition::End)
            .backwards()
            .max_count(1);

        let result = self
            .client
            .read_stream(stream_name.to_string(), &options)
            .await;

        let mut last_position = 0u64;
        if let Ok(mut stream) = result {
            if let Ok(Some(resolved)) = stream.next().await {
                if let Some(event) = resolved.event {
                    last_position = event.revision;
                }
            }
        }

        Ok(StreamInfo {
            stream_name: stream_name.to_string(),
            last_position,
            event_count: last_position + 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running EventStoreDB instance
    // Run with: cargo test --features integration-tests

    #[tokio::test]
    #[ignore = "requires EventStoreDB"]
    async fn test_append_and_read() {
        let client = EventStoreClient::new("esdb://localhost:2113?tls=false")
            .await
            .unwrap();

        let event = LedgerEvent::ActionSubmitted {
            request_id: "test-123".to_string(),
            actor_did: "did:key:test".to_string(),
            client_did: "did:key:client".to_string(),
            action_type: "test.action".to_string(),
            input_hash: [1u8; 32],
            output_hash: [2u8; 32],
            compute_hc: "10.0".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        let _pos = client
            .append_event("test-stream", event, None)
            .await
            .unwrap();

        let events = client.read_stream("test-stream", None, 10).await.unwrap();
        assert!(!events.is_empty());
    }
}
