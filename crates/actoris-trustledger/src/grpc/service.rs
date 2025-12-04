//! TrustLedger gRPC service implementation
//!
//! Implements the TrustLedgerService and OracleService from proto/actoris/trustledger.proto

use crate::generated::common::v1 as proto_common;
use crate::generated::trustledger::v1 as proto;
use crate::ledger::eventstore::EventStoreClient;
use crate::verification::verifier::{ActionVerifier, VerificationStatus, VerifierConfig};
use actoris_common::crypto::frost::PartialSignature;
use actoris_common::crypto::merkle::MerkleTree;
use actoris_common::types::outcome_record::OutcomeRecord;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, instrument, warn};

/// TrustLedger gRPC service handler
pub struct TrustLedgerGrpcService {
    verifier: Arc<ActionVerifier>,
    eventstore: Option<Arc<EventStoreClient>>,
    /// Cache of completed records for quick lookup
    records_cache: Arc<RwLock<HashMap<String, OutcomeRecord>>>,
    /// Statistics tracking
    stats: Arc<RwLock<LedgerStats>>,
}

/// Ledger statistics
#[derive(Default)]
struct LedgerStats {
    total_records: u64,
    verified_records: u64,
    failed_verifications: u64,
    latencies: Vec<u32>,
    stream_position: u64,
}

impl LedgerStats {
    fn avg_latency(&self) -> f64 {
        if self.latencies.is_empty() {
            0.0
        } else {
            self.latencies.iter().map(|&l| l as f64).sum::<f64>() / self.latencies.len() as f64
        }
    }

    fn p95_latency(&self) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        let mut sorted = self.latencies.clone();
        sorted.sort();
        let idx = (sorted.len() as f64 * 0.95) as usize;
        sorted.get(idx.min(sorted.len() - 1)).map(|&l| l as f64).unwrap_or(0.0)
    }
}

impl TrustLedgerGrpcService {
    /// Create a new TrustLedger gRPC service
    pub fn new(verifier: ActionVerifier) -> Self {
        Self {
            verifier: Arc::new(verifier),
            eventstore: None,
            records_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(LedgerStats::default())),
        }
    }

    /// Create with EventStore integration
    pub fn with_eventstore(mut self, client: Arc<EventStoreClient>) -> Self {
        self.eventstore = Some(client);
        self
    }

    /// Convert internal verification status to proto
    fn status_to_proto(status: &VerificationStatus) -> i32 {
        match status {
            VerificationStatus::Pending => proto::VerificationStatus::Pending as i32,
            VerificationStatus::InProgress { .. } => proto::VerificationStatus::InProgress as i32,
            VerificationStatus::Completed(_) => proto::VerificationStatus::Completed as i32,
            VerificationStatus::Failed(_) => proto::VerificationStatus::Failed as i32,
            VerificationStatus::Timeout => proto::VerificationStatus::Timeout as i32,
        }
    }

    /// Convert internal OutcomeRecord to proto
    fn record_to_proto(record: &OutcomeRecord) -> proto_common::OutcomeRecord {
        proto_common::OutcomeRecord {
            id: record.id.to_string(),
            actor_did: record.actor_did.clone(),
            client_did: record.client_did.clone(),
            action_type: record.action_type.clone(),
            input_hash: record.input_hash.to_vec(),
            output_hash: record.output_hash.to_vec(),
            compute_hc: record.compute_hc.to_string(),
            verification: Some(proto_common::VerificationResult {
                passed: record.verification.passed,
                oracle_count: record.verification.oracle_count as u32,
                quorum_reached: record.verification.quorum_reached,
                latency_ms: record.verification.latency_ms,
                votes: record
                    .verification
                    .votes
                    .iter()
                    .map(|v| proto_common::OracleVote {
                        oracle_did: v.oracle_did.clone(),
                        approved: v.approved,
                        reason: v.reason.clone(),
                        timestamp: v.timestamp,
                    })
                    .collect(),
                failure_reason: record.verification.failure_reason.clone(),
            }),
            signature: Some(proto_common::FrostSignature {
                signature: record.signature.signature.to_vec(),
                signers: record.signature.signers.clone(),
                group_key: record.signature.group_key.to_vec(),
                threshold: record.signature.threshold as u32,
                total_oracles: record.signature.total_oracles as u32,
            }),
            merkle_proof: record.merkle_proof.iter().map(|p| p.to_vec()).collect(),
            merkle_root: record.merkle_root.to_vec(),
            merkle_index: record.merkle_index,
            submitted_at: record.submitted_at,
            verified_at: record.verified_at,
            stream_position: None,
        }
    }
}

/// TrustLedger service trait implementation
#[tonic::async_trait]
impl TrustLedgerService for TrustLedgerGrpcService {
    /// Submit an action for verification
    #[instrument(skip(self, request))]
    async fn submit_action(
        &self,
        request: Request<proto::SubmitActionRequest>,
    ) -> Result<Response<proto::SubmitActionResponse>, Status> {
        let req = request.into_inner();

        // Parse compute amount
        let compute_hc = Decimal::from_str(&req.compute_hc)
            .map_err(|e| Status::invalid_argument(format!("Invalid compute_hc: {}", e)))?;

        // Submit to verifier
        let request_id = self
            .verifier
            .submit_action(
                &req.actor_did,
                &req.client_did,
                &req.action_type,
                &req.input,
                &req.output,
                compute_hc,
            )
            .await
            .map_err(|e| Status::internal(format!("Submission failed: {}", e)))?;

        info!(request_id = %request_id, actor = %req.actor_did, "Action submitted");

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_records += 1;
        }

        // If synchronous, wait for completion
        if req.synchronous {
            let timeout_ms = if req.timeout_ms > 0 {
                req.timeout_ms as u64
            } else {
                2000
            };

            // Poll for completion
            let start = std::time::Instant::now();
            loop {
                if start.elapsed().as_millis() as u64 > timeout_ms {
                    return Ok(Response::new(proto::SubmitActionResponse {
                        request_id: request_id.clone(),
                        outcome_record: None,
                        status: proto::VerificationStatus::Timeout as i32,
                    }));
                }

                if let Some(status) = self.verifier.get_status(&request_id).await {
                    match status {
                        VerificationStatus::Completed(record) => {
                            // Cache the record
                            {
                                let mut cache = self.records_cache.write().await;
                                cache.insert(record.id.to_string(), (*record).clone());
                            }

                            // Update stats
                            {
                                let mut stats = self.stats.write().await;
                                stats.verified_records += 1;
                                stats.latencies.push(record.verification.latency_ms);
                            }

                            return Ok(Response::new(proto::SubmitActionResponse {
                                request_id,
                                outcome_record: Some(Self::record_to_proto(&record)),
                                status: proto::VerificationStatus::Completed as i32,
                            }));
                        }
                        VerificationStatus::Failed(reason) => {
                            // Update stats
                            {
                                let mut stats = self.stats.write().await;
                                stats.failed_verifications += 1;
                            }

                            return Ok(Response::new(proto::SubmitActionResponse {
                                request_id,
                                outcome_record: None,
                                status: proto::VerificationStatus::Failed as i32,
                            }));
                        }
                        VerificationStatus::Timeout => {
                            return Ok(Response::new(proto::SubmitActionResponse {
                                request_id,
                                outcome_record: None,
                                status: proto::VerificationStatus::Timeout as i32,
                            }));
                        }
                        _ => {
                            // Still in progress, wait
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        }
                    }
                }
            }
        }

        Ok(Response::new(proto::SubmitActionResponse {
            request_id,
            outcome_record: None,
            status: proto::VerificationStatus::Pending as i32,
        }))
    }

    /// Get verification status
    #[instrument(skip(self, request))]
    async fn get_verification_status(
        &self,
        request: Request<proto::GetVerificationStatusRequest>,
    ) -> Result<Response<proto::GetVerificationStatusResponse>, Status> {
        let req = request.into_inner();

        let status = self
            .verifier
            .get_status(&req.request_id)
            .await
            .ok_or_else(|| Status::not_found("Request not found"))?;

        let (outcome_record, error, progress) = match &status {
            VerificationStatus::Completed(record) => (Some(Self::record_to_proto(record)), None, None),
            VerificationStatus::Failed(reason) => (
                None,
                Some(proto_common::ErrorInfo {
                    code: "VERIFICATION_FAILED".to_string(),
                    message: reason.clone(),
                    details: HashMap::new(),
                }),
                None,
            ),
            VerificationStatus::InProgress {
                votes_received,
                votes_required,
            } => (
                None,
                None,
                Some(proto::VerificationProgress {
                    oracle_votes_received: *votes_received as u32,
                    oracle_votes_required: *votes_required as u32,
                    started_at: chrono::Utc::now().timestamp_millis(),
                    elapsed_ms: 0,
                }),
            ),
            _ => (None, None, None),
        };

        Ok(Response::new(proto::GetVerificationStatusResponse {
            request_id: req.request_id,
            status: Self::status_to_proto(&status),
            outcome_record,
            error,
            progress,
        }))
    }

    /// Get outcome record by ID
    #[instrument(skip(self, request))]
    async fn get_outcome_record(
        &self,
        request: Request<proto::GetOutcomeRecordRequest>,
    ) -> Result<Response<proto::GetOutcomeRecordResponse>, Status> {
        let req = request.into_inner();

        // Check cache first
        {
            let cache = self.records_cache.read().await;
            if let Some(record) = cache.get(&req.id) {
                return Ok(Response::new(proto::GetOutcomeRecordResponse {
                    outcome_record: Some(Self::record_to_proto(record)),
                }));
            }
        }

        // If not in cache, could query EventStore here
        Err(Status::not_found("Record not found"))
    }

    /// Query outcome records
    #[instrument(skip(self, request))]
    async fn query_outcome_records(
        &self,
        request: Request<proto::QueryOutcomeRecordsRequest>,
    ) -> Result<Response<proto::QueryOutcomeRecordsResponse>, Status> {
        let req = request.into_inner();

        let cache = self.records_cache.read().await;
        let mut records: Vec<proto_common::OutcomeRecord> = cache
            .values()
            .filter(|r| {
                // Apply filters
                if let Some(ref actor) = req.actor_did {
                    if &r.actor_did != actor {
                        return false;
                    }
                }
                if let Some(ref client) = req.client_did {
                    if &r.client_did != client {
                        return false;
                    }
                }
                if let Some(ref action_type) = req.action_type {
                    if &r.action_type != action_type {
                        return false;
                    }
                }
                if let Some(from_ts) = req.from_timestamp {
                    if r.submitted_at < from_ts {
                        return false;
                    }
                }
                if let Some(to_ts) = req.to_timestamp {
                    if r.submitted_at > to_ts {
                        return false;
                    }
                }
                if let Some(verified_only) = req.verified_only {
                    if verified_only && !r.verification.passed {
                        return false;
                    }
                }
                true
            })
            .map(Self::record_to_proto)
            .collect();

        // Apply pagination
        let limit = req
            .page
            .as_ref()
            .map(|p| p.limit as usize)
            .unwrap_or(100);
        records.truncate(limit);

        Ok(Response::new(proto::QueryOutcomeRecordsResponse {
            records,
            page: Some(proto_common::PageResponse {
                next_cursor: String::new(),
                total_count: Some(cache.len() as u64),
            }),
        }))
    }

    /// Get Merkle proof for a record
    #[instrument(skip(self, request))]
    async fn get_merkle_proof(
        &self,
        request: Request<proto::GetMerkleProofRequest>,
    ) -> Result<Response<proto::GetMerkleProofResponse>, Status> {
        let req = request.into_inner();

        let cache = self.records_cache.read().await;
        let record = cache
            .get(&req.record_id)
            .ok_or_else(|| Status::not_found("Record not found"))?;

        Ok(Response::new(proto::GetMerkleProofResponse {
            proof: record.merkle_proof.iter().map(|p| p.to_vec()).collect(),
            root: record.merkle_root.to_vec(),
            leaf_index: record.merkle_index,
        }))
    }

    /// Verify a Merkle proof
    #[instrument(skip(self, request))]
    async fn verify_merkle_proof(
        &self,
        request: Request<proto::VerifyMerkleProofRequest>,
    ) -> Result<Response<proto::VerifyMerkleProofResponse>, Status> {
        let req = request.into_inner();

        // Convert proof to fixed arrays
        let leaf_hash: [u8; 32] = req
            .leaf_hash
            .try_into()
            .map_err(|_| Status::invalid_argument("Invalid leaf hash length"))?;

        let root: [u8; 32] = req
            .root
            .try_into()
            .map_err(|_| Status::invalid_argument("Invalid root length"))?;

        let proof: Vec<[u8; 32]> = req
            .proof
            .iter()
            .map(|p| {
                p.clone()
                    .try_into()
                    .map_err(|_| Status::invalid_argument("Invalid proof element length"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Verify using MerkleTree
        let valid = MerkleTree::verify_proof(&leaf_hash, &proof, req.leaf_index as usize, &root);

        Ok(Response::new(proto::VerifyMerkleProofResponse { valid }))
    }

    type StreamVerificationsStream =
        Pin<Box<dyn Stream<Item = Result<proto::VerificationEvent, Status>> + Send>>;

    /// Stream verification events
    #[instrument(skip(self, request))]
    async fn stream_verifications(
        &self,
        request: Request<proto::StreamVerificationsRequest>,
    ) -> Result<Response<Self::StreamVerificationsStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        // Clone what we need for the spawned task
        let eventstore = self.eventstore.clone();
        let action_type_filter = req.action_type;
        let actor_filter = req.actor_did;

        tokio::spawn(async move {
            if let Some(es) = eventstore {
                let (event_tx, mut event_rx) = mpsc::channel(100);

                if let Err(e) = es
                    .subscribe_outcomes(event_tx, req.from_position)
                    .await
                {
                    error!("Failed to subscribe: {}", e);
                    return;
                }

                let mut position = req.from_position.unwrap_or(0);
                while let Some(record_data) = event_rx.recv().await {
                    // Apply filters
                    if let Some(ref action_type) = action_type_filter {
                        if &record_data.action_type != action_type {
                            continue;
                        }
                    }
                    if let Some(ref actor) = actor_filter {
                        if &record_data.actor_did != actor {
                            continue;
                        }
                    }

                    position += 1;

                    let event = proto::VerificationEvent {
                        record: Some(proto_common::OutcomeRecord {
                            id: record_data.id,
                            actor_did: record_data.actor_did,
                            client_did: record_data.client_did,
                            action_type: record_data.action_type,
                            input_hash: record_data.input_hash,
                            output_hash: record_data.output_hash,
                            compute_hc: record_data.compute_hc,
                            verification: Some(proto_common::VerificationResult {
                                passed: record_data.verification_passed,
                                oracle_count: 0,
                                quorum_reached: true,
                                latency_ms: record_data.verification_latency_ms,
                                votes: vec![],
                                failure_reason: None,
                            }),
                            signature: Some(proto_common::FrostSignature {
                                signature: record_data.signature,
                                signers: record_data.signers,
                                group_key: vec![],
                                threshold: 3,
                                total_oracles: 5,
                            }),
                            merkle_proof: record_data.merkle_proof,
                            merkle_root: record_data.merkle_root,
                            merkle_index: record_data.merkle_index,
                            submitted_at: record_data.submitted_at,
                            verified_at: record_data.verified_at,
                            stream_position: Some(position),
                        }),
                        stream_position: position,
                    };

                    if tx.send(Ok(event)).await.is_err() {
                        break;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    /// Get ledger statistics
    #[instrument(skip(self, request))]
    async fn get_ledger_stats(
        &self,
        request: Request<proto::GetLedgerStatsRequest>,
    ) -> Result<Response<proto::GetLedgerStatsResponse>, Status> {
        let stats = self.stats.read().await;

        Ok(Response::new(proto::GetLedgerStatsResponse {
            total_records: stats.total_records,
            verified_records: stats.verified_records,
            failed_verifications: stats.failed_verifications,
            avg_latency_ms: stats.avg_latency(),
            p95_latency_ms: stats.p95_latency(),
            tree_height: 0, // Would need to query Merkle tree
            stream_position: stats.stream_position,
        }))
    }
}

/// Oracle service for internal oracle nodes
pub struct OracleGrpcService {
    verifier: Arc<ActionVerifier>,
    /// Connected oracles
    oracles: Arc<RwLock<HashMap<String, OracleInfo>>>,
}

/// Oracle node information
struct OracleInfo {
    did: String,
    public_key: Vec<u8>,
    position: u32,
    verifications_processed: u64,
    last_health_report: i64,
}

impl OracleGrpcService {
    pub fn new(verifier: Arc<ActionVerifier>) -> Self {
        Self {
            verifier,
            oracles: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tonic::async_trait]
impl OracleService for OracleGrpcService {
    /// Oracle joins the quorum
    #[instrument(skip(self, request))]
    async fn join_quorum(
        &self,
        request: Request<proto::JoinQuorumRequest>,
    ) -> Result<Response<proto::JoinQuorumResponse>, Status> {
        let req = request.into_inner();

        let mut oracles = self.oracles.write().await;
        let position = oracles.len() as u32;

        // Check if we have room for more oracles
        if position >= 10 {
            return Ok(Response::new(proto::JoinQuorumResponse {
                accepted: false,
                quorum_position: 0,
            }));
        }

        oracles.insert(
            req.oracle_did.clone(),
            OracleInfo {
                did: req.oracle_did.clone(),
                public_key: req.public_key,
                position,
                verifications_processed: 0,
                last_health_report: chrono::Utc::now().timestamp_millis(),
            },
        );

        info!(oracle = %req.oracle_did, position = position, "Oracle joined quorum");

        Ok(Response::new(proto::JoinQuorumResponse {
            accepted: true,
            quorum_position: position,
        }))
    }

    /// Submit partial signature for verification
    #[instrument(skip(self, request))]
    async fn submit_partial_signature(
        &self,
        request: Request<proto::SubmitPartialSignatureRequest>,
    ) -> Result<Response<proto::SubmitPartialSignatureResponse>, Status> {
        let req = request.into_inner();

        // Create partial signature
        let mut identifier = [0u8; 32];
        let did_bytes = req.oracle_did.as_bytes();
        let len = did_bytes.len().min(32);
        identifier[..len].copy_from_slice(&did_bytes[..len]);

        let partial_sig = PartialSignature {
            identifier,
            share: req.signature_share,
            commitment: req.commitment,
        };

        // Record the vote
        let status = self
            .verifier
            .record_vote(
                &req.request_id,
                &req.oracle_did,
                req.approved,
                req.reason,
                partial_sig,
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to record vote: {}", e)))?;

        // Update oracle stats
        {
            let mut oracles = self.oracles.write().await;
            if let Some(oracle) = oracles.get_mut(&req.oracle_did) {
                oracle.verifications_processed += 1;
            }
        }

        let (collected, required) = match status {
            VerificationStatus::InProgress {
                votes_received,
                votes_required,
            } => (votes_received as u32, votes_required as u32),
            VerificationStatus::Completed(_) => (3, 3), // Quorum reached
            _ => (0, 3),
        };

        Ok(Response::new(proto::SubmitPartialSignatureResponse {
            accepted: true,
            signatures_collected: collected,
            signatures_required: required,
        }))
    }

    /// Get commitments from other oracles
    #[instrument(skip(self, request))]
    async fn get_commitments(
        &self,
        request: Request<proto::GetCommitmentsRequest>,
    ) -> Result<Response<proto::GetCommitmentsResponse>, Status> {
        // This would typically be stored per-verification session
        // For now, return empty - in production would track commitments
        Ok(Response::new(proto::GetCommitmentsResponse {
            commitments: vec![],
        }))
    }

    /// Report oracle health
    #[instrument(skip(self, request))]
    async fn report_health(
        &self,
        request: Request<proto::ReportHealthRequest>,
    ) -> Result<Response<proto::ReportHealthResponse>, Status> {
        let req = request.into_inner();

        let mut oracles = self.oracles.write().await;
        if let Some(oracle) = oracles.get_mut(&req.oracle_did) {
            oracle.last_health_report = chrono::Utc::now().timestamp_millis();
            oracle.verifications_processed = req.verifications_processed;
        }

        debug!(
            oracle = %req.oracle_did,
            cpu = req.cpu_usage,
            memory = req.memory_usage,
            "Oracle health report"
        );

        Ok(Response::new(proto::ReportHealthResponse {
            acknowledged: true,
        }))
    }
}

/// TrustLedger service trait (would be auto-generated by tonic-build)
#[tonic::async_trait]
pub trait TrustLedgerService: Send + Sync + 'static {
    async fn submit_action(
        &self,
        request: Request<proto::SubmitActionRequest>,
    ) -> Result<Response<proto::SubmitActionResponse>, Status>;

    async fn get_verification_status(
        &self,
        request: Request<proto::GetVerificationStatusRequest>,
    ) -> Result<Response<proto::GetVerificationStatusResponse>, Status>;

    async fn get_outcome_record(
        &self,
        request: Request<proto::GetOutcomeRecordRequest>,
    ) -> Result<Response<proto::GetOutcomeRecordResponse>, Status>;

    async fn query_outcome_records(
        &self,
        request: Request<proto::QueryOutcomeRecordsRequest>,
    ) -> Result<Response<proto::QueryOutcomeRecordsResponse>, Status>;

    async fn get_merkle_proof(
        &self,
        request: Request<proto::GetMerkleProofRequest>,
    ) -> Result<Response<proto::GetMerkleProofResponse>, Status>;

    async fn verify_merkle_proof(
        &self,
        request: Request<proto::VerifyMerkleProofRequest>,
    ) -> Result<Response<proto::VerifyMerkleProofResponse>, Status>;

    type StreamVerificationsStream: Stream<Item = Result<proto::VerificationEvent, Status>>
        + Send
        + 'static;

    async fn stream_verifications(
        &self,
        request: Request<proto::StreamVerificationsRequest>,
    ) -> Result<Response<Self::StreamVerificationsStream>, Status>;

    async fn get_ledger_stats(
        &self,
        request: Request<proto::GetLedgerStatsRequest>,
    ) -> Result<Response<proto::GetLedgerStatsResponse>, Status>;
}

/// Oracle service trait (would be auto-generated by tonic-build)
#[tonic::async_trait]
pub trait OracleService: Send + Sync + 'static {
    async fn join_quorum(
        &self,
        request: Request<proto::JoinQuorumRequest>,
    ) -> Result<Response<proto::JoinQuorumResponse>, Status>;

    async fn submit_partial_signature(
        &self,
        request: Request<proto::SubmitPartialSignatureRequest>,
    ) -> Result<Response<proto::SubmitPartialSignatureResponse>, Status>;

    async fn get_commitments(
        &self,
        request: Request<proto::GetCommitmentsRequest>,
    ) -> Result<Response<proto::GetCommitmentsResponse>, Status>;

    async fn report_health(
        &self,
        request: Request<proto::ReportHealthRequest>,
    ) -> Result<Response<proto::ReportHealthResponse>, Status>;
}

impl Default for TrustLedgerGrpcService {
    fn default() -> Self {
        Self::new(ActionVerifier::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::SubmitActionRequest;

    #[tokio::test]
    async fn test_submit_action() {
        let service = TrustLedgerGrpcService::default();

        let request = Request::new(SubmitActionRequest {
            actor_did: "did:key:actor123".to_string(),
            client_did: "did:key:client456".to_string(),
            action_type: "test.action".to_string(),
            input: b"test input".to_vec(),
            output: b"test output".to_vec(),
            compute_hc: "10.5".to_string(),
            actor_signature: vec![0u8; 64],
            timestamp: chrono::Utc::now().timestamp_millis(),
            synchronous: false,
            timeout_ms: 2000,
        });

        let response = service.submit_action(request).await.unwrap();
        let resp = response.into_inner();

        assert!(!resp.request_id.is_empty());
        assert_eq!(resp.status, proto::VerificationStatus::Pending as i32);
    }

    #[tokio::test]
    async fn test_get_ledger_stats() {
        let service = TrustLedgerGrpcService::default();

        let request = Request::new(proto::GetLedgerStatsRequest {
            from_timestamp: None,
            to_timestamp: None,
        });

        let response = service.get_ledger_stats(request).await.unwrap();
        let stats = response.into_inner();

        assert_eq!(stats.total_records, 0);
        assert_eq!(stats.verified_records, 0);
    }
}
