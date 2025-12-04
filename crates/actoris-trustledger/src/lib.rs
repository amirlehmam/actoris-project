//! # TrustLedger
//!
//! Consensus engine and immutable ledger for Actoris Economic OS.
//!
//! ## Components
//!
//! - **Consensus**: BFT consensus with FROST threshold signatures
//! - **Ledger**: EventStoreDB-backed immutable record storage
//! - **Verification**: Oracle-based action verification
//! - **gRPC**: Service API for clients
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      TrustLedger                            │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
//! │  │  Consensus  │  │   Ledger    │  │    Verification     │ │
//! │  │   (FROST)   │──│ (EventStore │──│   (Oracle Nodes)    │ │
//! │  │             │  │    DB)      │  │                     │ │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod consensus;
pub mod generated;
pub mod grpc;
pub mod ledger;
pub mod verification;

pub use consensus::{OracleNode, QuorumManager};
pub use grpc::{OracleGrpcService, TrustLedgerGrpcService};
pub use ledger::eventstore::{EventStoreClient, LedgerEvent, OutcomeRecordData, StreamInfo};
pub use verification::verifier::{ActionVerifier, VerificationStatus, VerifierConfig};

use actoris_common::{OutcomeRecord, Result};
use std::sync::Arc;

/// TrustLedger configuration
#[derive(Debug, Clone)]
pub struct TrustLedgerConfig {
    /// EventStoreDB connection string
    pub eventstore_url: String,
    /// NATS server URL
    pub nats_url: String,
    /// Quorum threshold (e.g., 3 for 3-of-5)
    pub quorum_threshold: u8,
    /// Total oracle count
    pub oracle_count: u8,
    /// Verification timeout in milliseconds
    pub verification_timeout_ms: u64,
    /// gRPC listen address
    pub grpc_addr: String,
}

impl Default for TrustLedgerConfig {
    fn default() -> Self {
        Self {
            eventstore_url: "esdb://localhost:2113?tls=false".to_string(),
            nats_url: "nats://localhost:4222".to_string(),
            quorum_threshold: 3,
            oracle_count: 5,
            verification_timeout_ms: 2000,
            grpc_addr: "[::1]:50051".to_string(),
        }
    }
}

/// TrustLedger service
pub struct TrustLedger {
    config: TrustLedgerConfig,
    eventstore: Option<Arc<EventStoreClient>>,
    verifier: ActionVerifier,
}

impl TrustLedger {
    /// Create a new TrustLedger instance
    pub async fn new(config: TrustLedgerConfig) -> Result<Self> {
        let verifier_config = VerifierConfig {
            timeout_ms: config.verification_timeout_ms,
            quorum_threshold: config.quorum_threshold,
            oracle_count: config.oracle_count,
        };

        let eventstore = EventStoreClient::new(&config.eventstore_url).await?;
        let eventstore = Arc::new(eventstore);

        let verifier = ActionVerifier::new(verifier_config)
            .with_eventstore(eventstore.clone());

        Ok(Self {
            config,
            eventstore: Some(eventstore),
            verifier,
        })
    }

    /// Create TrustLedger without EventStoreDB (for testing)
    pub fn new_standalone(config: TrustLedgerConfig) -> Self {
        let verifier_config = VerifierConfig {
            timeout_ms: config.verification_timeout_ms,
            quorum_threshold: config.quorum_threshold,
            oracle_count: config.oracle_count,
        };

        Self {
            config,
            eventstore: None,
            verifier: ActionVerifier::new(verifier_config),
        }
    }

    /// Get the verifier
    pub fn verifier(&self) -> &ActionVerifier {
        &self.verifier
    }

    /// Get the EventStore client
    pub fn eventstore(&self) -> Option<&Arc<EventStoreClient>> {
        self.eventstore.as_ref()
    }

    /// Get configuration
    pub fn config(&self) -> &TrustLedgerConfig {
        &self.config
    }

    /// Create gRPC service from this TrustLedger instance
    pub fn into_grpc_service(self) -> TrustLedgerGrpcService {
        let mut service = TrustLedgerGrpcService::new(self.verifier);
        if let Some(es) = self.eventstore {
            service = service.with_eventstore(es);
        }
        service
    }
}

/// Action candidate for verification
#[derive(Debug, Clone)]
pub struct ActionCandidate {
    pub actor_did: String,
    pub client_did: String,
    pub action_type: String,
    pub input: Vec<u8>,
    pub output: Vec<u8>,
    pub compute_hc: rust_decimal::Decimal,
    pub actor_signature: [u8; 64],
    pub timestamp: i64,
}
