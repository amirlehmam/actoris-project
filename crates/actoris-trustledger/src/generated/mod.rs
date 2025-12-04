//! Generated protobuf types for TrustLedger service
//!
//! These types are designed to match the proto definitions in proto/actoris/

pub mod common {
    pub mod v1 {
        // Common protobuf types matching proto/actoris/common.proto

        use prost::{Enumeration, Message};
        use serde::{Deserialize, Serialize};

        /// EntityType classification for UnifiedID
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration, Serialize, Deserialize)]
        #[repr(i32)]
        pub enum EntityType {
            Unspecified = 0,
            Human = 1,
            Agent = 2,
            Organization = 3,
        }

        /// W3C DID-based unified identity
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct UnifiedId {
            #[prost(string, tag = "1")]
            pub did: String,
            #[prost(enumeration = "EntityType", tag = "2")]
            pub entity_type: i32,
            #[prost(string, optional, tag = "3")]
            pub parent_did: Option<String>,
            #[prost(int64, tag = "4")]
            pub created_at: i64,
            #[prost(bytes = "vec", tag = "5")]
            pub public_key: Vec<u8>,
        }

        /// Trust score breakdown by component
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct TrustComponents {
            #[prost(uint32, tag = "1")]
            pub verification_score: u32,
            #[prost(uint32, tag = "2")]
            pub dispute_penalty: u32,
            #[prost(uint32, tag = "3")]
            pub sla_score: u32,
            #[prost(uint32, tag = "4")]
            pub network_score: u32,
        }

        /// Entity trust score with full breakdown
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct TrustScore {
            #[prost(uint32, tag = "1")]
            pub score: u32,
            #[prost(message, optional, tag = "2")]
            pub components: Option<TrustComponents>,
            #[prost(int64, tag = "3")]
            pub updated_at: i64,
            #[prost(uint64, tag = "4")]
            pub verified_outcomes: u64,
            #[prost(double, tag = "5")]
            pub dispute_rate: f64,
            #[prost(uint64, tag = "6")]
            pub version: u64,
        }

        /// HC Wallet for compute credit management
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct HcWallet {
            #[prost(string, tag = "1")]
            pub owner_did: String,
            #[prost(string, tag = "2")]
            pub available: String,
            #[prost(string, tag = "3")]
            pub locked: String,
            #[prost(int64, tag = "4")]
            pub expires_at: i64,
            #[prost(uint64, tag = "5")]
            pub version: u64,
            #[prost(int64, tag = "6")]
            pub updated_at: i64,
        }

        /// Task complexity levels
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration, Serialize, Deserialize)]
        #[repr(i32)]
        pub enum TaskComplexity {
            Unspecified = 0,
            Low = 1,
            Medium = 2,
            High = 3,
            Critical = 4,
        }

        /// Data sensitivity levels
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration, Serialize, Deserialize)]
        #[repr(i32)]
        pub enum DataSensitivity {
            Unspecified = 0,
            Public = 1,
            Internal = 2,
            Confidential = 3,
            Restricted = 4,
        }

        /// FROST threshold signature
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct FrostSignature {
            #[prost(bytes = "vec", tag = "1")]
            pub signature: Vec<u8>,
            #[prost(string, repeated, tag = "2")]
            pub signers: Vec<String>,
            #[prost(bytes = "vec", tag = "3")]
            pub group_key: Vec<u8>,
            #[prost(uint32, tag = "4")]
            pub threshold: u32,
            #[prost(uint32, tag = "5")]
            pub total_oracles: u32,
        }

        /// Individual oracle vote
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct OracleVote {
            #[prost(string, tag = "1")]
            pub oracle_did: String,
            #[prost(bool, tag = "2")]
            pub approved: bool,
            #[prost(string, optional, tag = "3")]
            pub reason: Option<String>,
            #[prost(int64, tag = "4")]
            pub timestamp: i64,
        }

        /// Verification result
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct VerificationResult {
            #[prost(bool, tag = "1")]
            pub passed: bool,
            #[prost(uint32, tag = "2")]
            pub oracle_count: u32,
            #[prost(bool, tag = "3")]
            pub quorum_reached: bool,
            #[prost(uint32, tag = "4")]
            pub latency_ms: u32,
            #[prost(message, repeated, tag = "5")]
            pub votes: Vec<OracleVote>,
            #[prost(string, optional, tag = "6")]
            pub failure_reason: Option<String>,
        }

        /// Verified action record
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct OutcomeRecord {
            #[prost(string, tag = "1")]
            pub id: String,
            #[prost(string, tag = "2")]
            pub actor_did: String,
            #[prost(string, tag = "3")]
            pub client_did: String,
            #[prost(string, tag = "4")]
            pub action_type: String,
            #[prost(bytes = "vec", tag = "5")]
            pub input_hash: Vec<u8>,
            #[prost(bytes = "vec", tag = "6")]
            pub output_hash: Vec<u8>,
            #[prost(string, tag = "7")]
            pub compute_hc: String,
            #[prost(message, optional, tag = "8")]
            pub verification: Option<VerificationResult>,
            #[prost(message, optional, tag = "9")]
            pub signature: Option<FrostSignature>,
            #[prost(bytes = "vec", repeated, tag = "10")]
            pub merkle_proof: Vec<Vec<u8>>,
            #[prost(bytes = "vec", tag = "11")]
            pub merkle_root: Vec<u8>,
            #[prost(uint64, tag = "12")]
            pub merkle_index: u64,
            #[prost(int64, tag = "13")]
            pub submitted_at: i64,
            #[prost(int64, tag = "14")]
            pub verified_at: i64,
            #[prost(uint64, optional, tag = "15")]
            pub stream_position: Option<u64>,
        }

        /// Error details
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct ErrorInfo {
            #[prost(string, tag = "1")]
            pub code: String,
            #[prost(string, tag = "2")]
            pub message: String,
            #[prost(map = "string, string", tag = "3")]
            pub details: std::collections::HashMap<String, String>,
        }

        /// Pagination request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct PageRequest {
            #[prost(uint32, tag = "1")]
            pub limit: u32,
            #[prost(string, optional, tag = "2")]
            pub cursor: Option<String>,
        }

        /// Pagination response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct PageResponse {
            #[prost(string, tag = "1")]
            pub next_cursor: String,
            #[prost(uint64, optional, tag = "2")]
            pub total_count: Option<u64>,
        }
    }
}

pub mod trustledger {
    pub mod v1 {
        use super::super::common::v1 as common;
        use prost::{Enumeration, Message};
        use serde::{Deserialize, Serialize};

        /// Verification status enum
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration, Serialize, Deserialize)]
        #[repr(i32)]
        pub enum VerificationStatus {
            Unspecified = 0,
            Pending = 1,
            InProgress = 2,
            Completed = 3,
            Failed = 4,
            Timeout = 5,
        }

        /// SubmitAction request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct SubmitActionRequest {
            #[prost(string, tag = "1")]
            pub actor_did: String,
            #[prost(string, tag = "2")]
            pub client_did: String,
            #[prost(string, tag = "3")]
            pub action_type: String,
            #[prost(bytes = "vec", tag = "4")]
            pub input: Vec<u8>,
            #[prost(bytes = "vec", tag = "5")]
            pub output: Vec<u8>,
            #[prost(string, tag = "6")]
            pub compute_hc: String,
            #[prost(bytes = "vec", tag = "7")]
            pub actor_signature: Vec<u8>,
            #[prost(int64, tag = "8")]
            pub timestamp: i64,
            #[prost(bool, tag = "9")]
            pub synchronous: bool,
            #[prost(uint32, tag = "10")]
            pub timeout_ms: u32,
        }

        /// SubmitAction response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct SubmitActionResponse {
            #[prost(string, tag = "1")]
            pub request_id: String,
            #[prost(message, optional, tag = "2")]
            pub outcome_record: Option<common::OutcomeRecord>,
            #[prost(enumeration = "VerificationStatus", tag = "3")]
            pub status: i32,
        }

        /// GetVerificationStatus request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetVerificationStatusRequest {
            #[prost(string, tag = "1")]
            pub request_id: String,
        }

        /// Verification progress info
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct VerificationProgress {
            #[prost(uint32, tag = "1")]
            pub oracle_votes_received: u32,
            #[prost(uint32, tag = "2")]
            pub oracle_votes_required: u32,
            #[prost(int64, tag = "3")]
            pub started_at: i64,
            #[prost(uint32, tag = "4")]
            pub elapsed_ms: u32,
        }

        /// GetVerificationStatus response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetVerificationStatusResponse {
            #[prost(string, tag = "1")]
            pub request_id: String,
            #[prost(enumeration = "VerificationStatus", tag = "2")]
            pub status: i32,
            #[prost(message, optional, tag = "3")]
            pub outcome_record: Option<common::OutcomeRecord>,
            #[prost(message, optional, tag = "4")]
            pub error: Option<common::ErrorInfo>,
            #[prost(message, optional, tag = "5")]
            pub progress: Option<VerificationProgress>,
        }

        /// GetOutcomeRecord request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetOutcomeRecordRequest {
            #[prost(string, tag = "1")]
            pub id: String,
        }

        /// GetOutcomeRecord response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetOutcomeRecordResponse {
            #[prost(message, optional, tag = "1")]
            pub outcome_record: Option<common::OutcomeRecord>,
        }

        /// QueryOutcomeRecords request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct QueryOutcomeRecordsRequest {
            #[prost(string, optional, tag = "1")]
            pub actor_did: Option<String>,
            #[prost(string, optional, tag = "2")]
            pub client_did: Option<String>,
            #[prost(string, optional, tag = "3")]
            pub action_type: Option<String>,
            #[prost(int64, optional, tag = "4")]
            pub from_timestamp: Option<i64>,
            #[prost(int64, optional, tag = "5")]
            pub to_timestamp: Option<i64>,
            #[prost(bool, optional, tag = "6")]
            pub verified_only: Option<bool>,
            #[prost(message, optional, tag = "7")]
            pub page: Option<common::PageRequest>,
        }

        /// QueryOutcomeRecords response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct QueryOutcomeRecordsResponse {
            #[prost(message, repeated, tag = "1")]
            pub records: Vec<common::OutcomeRecord>,
            #[prost(message, optional, tag = "2")]
            pub page: Option<common::PageResponse>,
        }

        /// GetMerkleProof request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetMerkleProofRequest {
            #[prost(string, tag = "1")]
            pub record_id: String,
        }

        /// GetMerkleProof response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetMerkleProofResponse {
            #[prost(bytes = "vec", repeated, tag = "1")]
            pub proof: Vec<Vec<u8>>,
            #[prost(bytes = "vec", tag = "2")]
            pub root: Vec<u8>,
            #[prost(uint64, tag = "3")]
            pub leaf_index: u64,
        }

        /// VerifyMerkleProof request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct VerifyMerkleProofRequest {
            #[prost(bytes = "vec", tag = "1")]
            pub leaf_hash: Vec<u8>,
            #[prost(bytes = "vec", repeated, tag = "2")]
            pub proof: Vec<Vec<u8>>,
            #[prost(bytes = "vec", tag = "3")]
            pub root: Vec<u8>,
            #[prost(uint64, tag = "4")]
            pub leaf_index: u64,
        }

        /// VerifyMerkleProof response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct VerifyMerkleProofResponse {
            #[prost(bool, tag = "1")]
            pub valid: bool,
        }

        /// StreamVerifications request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct StreamVerificationsRequest {
            #[prost(string, optional, tag = "1")]
            pub action_type: Option<String>,
            #[prost(string, optional, tag = "2")]
            pub actor_did: Option<String>,
            #[prost(uint64, optional, tag = "3")]
            pub from_position: Option<u64>,
        }

        /// VerificationEvent for streaming
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct VerificationEvent {
            #[prost(message, optional, tag = "1")]
            pub record: Option<common::OutcomeRecord>,
            #[prost(uint64, tag = "2")]
            pub stream_position: u64,
        }

        /// GetLedgerStats request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetLedgerStatsRequest {
            #[prost(int64, optional, tag = "1")]
            pub from_timestamp: Option<i64>,
            #[prost(int64, optional, tag = "2")]
            pub to_timestamp: Option<i64>,
        }

        /// GetLedgerStats response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetLedgerStatsResponse {
            #[prost(uint64, tag = "1")]
            pub total_records: u64,
            #[prost(uint64, tag = "2")]
            pub verified_records: u64,
            #[prost(uint64, tag = "3")]
            pub failed_verifications: u64,
            #[prost(double, tag = "4")]
            pub avg_latency_ms: f64,
            #[prost(double, tag = "5")]
            pub p95_latency_ms: f64,
            #[prost(uint32, tag = "6")]
            pub tree_height: u32,
            #[prost(uint64, tag = "7")]
            pub stream_position: u64,
        }

        // Oracle service messages

        /// JoinQuorum request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct JoinQuorumRequest {
            #[prost(string, tag = "1")]
            pub oracle_did: String,
            #[prost(bytes = "vec", tag = "2")]
            pub public_key: Vec<u8>,
        }

        /// JoinQuorum response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct JoinQuorumResponse {
            #[prost(bool, tag = "1")]
            pub accepted: bool,
            #[prost(uint32, tag = "2")]
            pub quorum_position: u32,
        }

        /// SubmitPartialSignature request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct SubmitPartialSignatureRequest {
            #[prost(string, tag = "1")]
            pub request_id: String,
            #[prost(string, tag = "2")]
            pub oracle_did: String,
            #[prost(bytes = "vec", tag = "3")]
            pub signature_share: Vec<u8>,
            #[prost(bytes = "vec", tag = "4")]
            pub commitment: Vec<u8>,
            #[prost(bool, tag = "5")]
            pub approved: bool,
            #[prost(string, optional, tag = "6")]
            pub reason: Option<String>,
        }

        /// SubmitPartialSignature response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct SubmitPartialSignatureResponse {
            #[prost(bool, tag = "1")]
            pub accepted: bool,
            #[prost(uint32, tag = "2")]
            pub signatures_collected: u32,
            #[prost(uint32, tag = "3")]
            pub signatures_required: u32,
        }

        /// GetCommitments request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetCommitmentsRequest {
            #[prost(string, tag = "1")]
            pub request_id: String,
        }

        /// Oracle commitment
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct OracleCommitment {
            #[prost(string, tag = "1")]
            pub oracle_did: String,
            #[prost(bytes = "vec", tag = "2")]
            pub commitment: Vec<u8>,
        }

        /// GetCommitments response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetCommitmentsResponse {
            #[prost(message, repeated, tag = "1")]
            pub commitments: Vec<OracleCommitment>,
        }

        /// ReportHealth request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct ReportHealthRequest {
            #[prost(string, tag = "1")]
            pub oracle_did: String,
            #[prost(double, tag = "2")]
            pub cpu_usage: f64,
            #[prost(double, tag = "3")]
            pub memory_usage: f64,
            #[prost(uint64, tag = "4")]
            pub verifications_processed: u64,
        }

        /// ReportHealth response
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct ReportHealthResponse {
            #[prost(bool, tag = "1")]
            pub acknowledged: bool,
        }
    }
}
