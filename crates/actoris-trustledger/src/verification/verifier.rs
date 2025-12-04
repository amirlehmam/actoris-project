//! Action Verification Engine
//!
//! Coordinates the verification process:
//! 1. Receives action submissions
//! 2. Dispatches to oracle quorum
//! 3. Collects votes and signatures
//! 4. Aggregates FROST signature
//! 5. Records to EventStoreDB

use crate::consensus::{OracleNode, QuorumManager};
use crate::ledger::eventstore::{EventStoreClient, OutcomeRecordData};
use actoris_common::{
    crypto::{frost, merkle::MerkleTree},
    error::VerificationError,
    types::outcome_record::{FrostSignature, OracleVote, OutcomeRecord, VerificationResult},
    ActorisError, Result,
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Verification request status
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationStatus {
    Pending,
    InProgress { votes_received: u8, votes_required: u8 },
    Completed(Box<OutcomeRecord>),
    Failed(String),
    Timeout,
}

/// Pending verification request
#[derive(Debug)]
struct PendingVerification {
    request_id: String,
    actor_did: String,
    client_did: String,
    action_type: String,
    input_hash: [u8; 32],
    output_hash: [u8; 32],
    compute_hc: Decimal,
    submitted_at: Instant,
    votes: Vec<OracleVote>,
    partial_signatures: Vec<frost::PartialSignature>,
    status: VerificationStatus,
}

/// Configuration for the verifier
#[derive(Debug, Clone)]
pub struct VerifierConfig {
    /// Verification timeout in milliseconds
    pub timeout_ms: u64,
    /// Quorum threshold (e.g., 3 for 3-of-5)
    pub quorum_threshold: u8,
    /// Total oracle count
    pub oracle_count: u8,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 2000,
            quorum_threshold: 3,
            oracle_count: 5,
        }
    }
}

/// Action verifier coordinating oracle consensus
pub struct ActionVerifier {
    config: VerifierConfig,
    /// Pending verifications
    pending: Arc<RwLock<HashMap<String, PendingVerification>>>,
    /// Quorum manager
    quorum: QuorumManager,
    /// EventStoreDB client
    eventstore: Option<Arc<EventStoreClient>>,
    /// Merkle tree for audit proofs
    merkle_tree: Arc<RwLock<MerkleTree>>,
    /// Group public key for FROST verification
    group_public_key: [u8; 32],
}

impl ActionVerifier {
    /// Create a new action verifier
    pub fn new(config: VerifierConfig) -> Self {
        let quorum = QuorumManager::new(config.quorum_threshold, config.oracle_count);

        // Generate placeholder group key (in production, from DKG)
        let mut group_key = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut group_key);

        Self {
            config,
            pending: Arc::new(RwLock::new(HashMap::new())),
            quorum,
            eventstore: None,
            merkle_tree: Arc::new(RwLock::new(MerkleTree::new())),
            group_public_key: group_key,
        }
    }

    /// Set EventStoreDB client
    pub fn with_eventstore(mut self, client: Arc<EventStoreClient>) -> Self {
        self.eventstore = Some(client);
        self
    }

    /// Submit an action for verification
    #[instrument(skip(self, input, output))]
    pub async fn submit_action(
        &self,
        actor_did: &str,
        client_did: &str,
        action_type: &str,
        input: &[u8],
        output: &[u8],
        compute_hc: Decimal,
    ) -> Result<String> {
        let request_id = Uuid::now_v7().to_string();

        // Hash input and output
        let input_hash = *blake3::hash(input).as_bytes();
        let output_hash = *blake3::hash(output).as_bytes();

        let pending = PendingVerification {
            request_id: request_id.clone(),
            actor_did: actor_did.to_string(),
            client_did: client_did.to_string(),
            action_type: action_type.to_string(),
            input_hash,
            output_hash,
            compute_hc,
            submitted_at: Instant::now(),
            votes: Vec::new(),
            partial_signatures: Vec::new(),
            status: VerificationStatus::Pending,
        };

        // Store pending verification
        {
            let mut pending_map = self.pending.write().await;
            pending_map.insert(request_id.clone(), pending);
        }

        // Record to EventStore if available
        if let Some(es) = &self.eventstore {
            es.record_action_submitted(
                &request_id,
                actor_did,
                client_did,
                action_type,
                input_hash,
                output_hash,
                &compute_hc.to_string(),
            )
            .await?;
        }

        info!(request_id = %request_id, actor = %actor_did, "Action submitted for verification");

        Ok(request_id)
    }

    /// Record an oracle vote
    #[instrument(skip(self))]
    pub async fn record_vote(
        &self,
        request_id: &str,
        oracle_did: &str,
        approved: bool,
        reason: Option<String>,
        partial_sig: frost::PartialSignature,
    ) -> Result<VerificationStatus> {
        let mut pending_map = self.pending.write().await;

        let verification = pending_map
            .get_mut(request_id)
            .ok_or_else(|| ActorisError::Verification(VerificationError::DuplicateAction {
                action_id: request_id.to_string(),
            }))?;

        // Check for timeout
        if verification.submitted_at.elapsed() > Duration::from_millis(self.config.timeout_ms) {
            verification.status = VerificationStatus::Timeout;
            return Ok(VerificationStatus::Timeout);
        }

        // Record vote
        verification.votes.push(OracleVote {
            oracle_did: oracle_did.to_string(),
            approved,
            reason,
            timestamp: chrono::Utc::now().timestamp_millis(),
        });

        // Record partial signature
        verification.partial_signatures.push(partial_sig);

        let votes_received = verification.votes.len() as u8;
        let votes_required = self.config.quorum_threshold;

        debug!(
            request_id = %request_id,
            oracle = %oracle_did,
            approved = approved,
            votes = votes_received,
            required = votes_required,
            "Oracle vote recorded"
        );

        // Check if quorum reached
        if votes_received >= votes_required {
            // Count approvals
            let approvals = verification.votes.iter().filter(|v| v.approved).count() as u8;
            let passed = approvals >= votes_required;

            // Aggregate signature
            let signature = frost::aggregate_signatures(
                &verification.input_hash,
                &verification.partial_signatures,
                &self.group_public_key,
            )?;

            // Calculate latency
            let latency_ms = verification.submitted_at.elapsed().as_millis() as u32;

            // Build verification result
            let result = VerificationResult {
                passed,
                oracle_count: votes_received,
                quorum_reached: true,
                latency_ms,
                votes: verification.votes.clone(),
                failure_reason: if passed {
                    None
                } else {
                    Some("Quorum rejected action".to_string())
                },
            };

            // Build FROST signature
            let frost_sig = FrostSignature::new(
                signature,
                verification
                    .votes
                    .iter()
                    .map(|v| v.oracle_did.clone())
                    .collect(),
                self.group_public_key,
                self.config.quorum_threshold,
                self.config.oracle_count,
            );

            // Create outcome record
            let mut record = OutcomeRecord::new(
                verification.actor_did.clone(),
                verification.client_did.clone(),
                verification.action_type.clone(),
                verification.input_hash,
                verification.output_hash,
                verification.compute_hc,
                result,
                frost_sig,
            );

            // Add to Merkle tree
            {
                let mut tree = self.merkle_tree.write().await;
                let canonical_hash = record.canonical_hash();
                let index = tree.append(canonical_hash);
                tree.commit();

                if let Some(proof) = tree.generate_proof(index) {
                    record.set_merkle_proof(proof.siblings, proof.root, index);
                }
            }

            // Record to EventStore
            if let Some(es) = &self.eventstore {
                es.record_verification_completed(
                    request_id,
                    passed,
                    true,
                    latency_ms,
                    &signature,
                )
                .await?;

                es.record_outcome_finalized(&record).await?;
            }

            verification.status = VerificationStatus::Completed(Box::new(record.clone()));

            info!(
                request_id = %request_id,
                passed = passed,
                latency_ms = latency_ms,
                "Verification completed"
            );

            Ok(VerificationStatus::Completed(Box::new(record)))
        } else {
            verification.status = VerificationStatus::InProgress {
                votes_received,
                votes_required,
            };
            Ok(verification.status.clone())
        }
    }

    /// Get verification status
    pub async fn get_status(&self, request_id: &str) -> Option<VerificationStatus> {
        let pending_map = self.pending.read().await;
        pending_map.get(request_id).map(|v| v.status.clone())
    }

    /// Clean up timed out verifications
    #[instrument(skip(self))]
    pub async fn cleanup_timeouts(&self) -> usize {
        let mut pending_map = self.pending.write().await;
        let timeout = Duration::from_millis(self.config.timeout_ms);

        let timed_out: Vec<String> = pending_map
            .iter()
            .filter(|(_, v)| {
                v.submitted_at.elapsed() > timeout
                    && !matches!(v.status, VerificationStatus::Completed(_))
            })
            .map(|(k, _)| k.clone())
            .collect();

        for request_id in &timed_out {
            if let Some(v) = pending_map.get_mut(request_id) {
                v.status = VerificationStatus::Timeout;
                warn!(request_id = %request_id, "Verification timed out");
            }
        }

        timed_out.len()
    }

    /// Get group public key
    pub fn group_public_key(&self) -> [u8; 32] {
        self.group_public_key
    }

    /// Get current Merkle root
    pub async fn merkle_root(&self) -> Option<[u8; 32]> {
        let tree = self.merkle_tree.read().await;
        tree.root()
    }
}

impl Default for ActionVerifier {
    fn default() -> Self {
        Self::new(VerifierConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_submit_action() {
        let verifier = ActionVerifier::default();

        let request_id = verifier
            .submit_action(
                "did:key:actor",
                "did:key:client",
                "test.action",
                b"input data",
                b"output data",
                dec!(10),
            )
            .await
            .unwrap();

        assert!(!request_id.is_empty());

        let status = verifier.get_status(&request_id).await;
        assert!(matches!(status, Some(VerificationStatus::Pending)));
    }

    #[tokio::test]
    async fn test_quorum_completion() {
        let config = VerifierConfig {
            timeout_ms: 5000,
            quorum_threshold: 2,
            oracle_count: 3,
        };
        let verifier = ActionVerifier::new(config);

        let request_id = verifier
            .submit_action(
                "did:key:actor",
                "did:key:client",
                "test.action",
                b"input",
                b"output",
                dec!(5),
            )
            .await
            .unwrap();

        // Simulate oracle votes
        let key_shares = frost::generate_test_key_shares(2, 3).unwrap();

        for (i, share) in key_shares.iter().take(2).enumerate() {
            let mut signer = frost::FrostSigner::new(share.clone());
            let partial = signer.start_signing(b"input").unwrap();

            let status = verifier
                .record_vote(
                    &request_id,
                    &format!("did:key:oracle{}", i),
                    true,
                    None,
                    partial,
                )
                .await
                .unwrap();

            if i == 1 {
                assert!(matches!(status, VerificationStatus::Completed(_)));
            }
        }
    }
}
