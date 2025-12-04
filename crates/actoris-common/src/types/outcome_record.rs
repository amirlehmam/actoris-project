//! OutcomeRecord - Verified action record with oracle consensus
//!
//! Every action in Actoris that requires verification produces an OutcomeRecord.
//! This record contains:
//! - Action metadata (actor, client, type, hashes)
//! - Verification result from oracle consensus
//! - FROST threshold signature from oracles
//! - Merkle proof for audit trail

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// FROST threshold signature from oracle consensus
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrostSignature {
    /// Aggregated Schnorr signature (64 bytes, stored as Vec for serde)
    #[serde(with = "signature_bytes")]
    pub signature: [u8; 64],

    /// DIDs of participating oracles
    pub signers: Vec<String>,

    /// Group public key for verification
    pub group_key: [u8; 32],

    /// Threshold requirement (e.g., 3 for 3-of-5)
    pub threshold: u8,

    /// Total oracle count
    pub total_oracles: u8,
}

/// Serde helper for [u8; 64] arrays
mod signature_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Vec::deserialize(deserializer)?;
        if vec.len() != 64 {
            return Err(serde::de::Error::custom(format!(
                "expected 64 bytes, got {}",
                vec.len()
            )));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&vec);
        Ok(arr)
    }
}

impl FrostSignature {
    /// Create a new FROST signature
    pub fn new(
        signature: [u8; 64],
        signers: Vec<String>,
        group_key: [u8; 32],
        threshold: u8,
        total_oracles: u8,
    ) -> Self {
        Self {
            signature,
            signers,
            group_key,
            threshold,
            total_oracles,
        }
    }

    /// Check if quorum was reached
    pub fn quorum_reached(&self) -> bool {
        self.signers.len() >= self.threshold as usize
    }

    /// Get the number of signers
    pub fn signer_count(&self) -> usize {
        self.signers.len()
    }
}

/// Result of oracle verification process
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the action passed verification
    pub passed: bool,

    /// Number of oracles that participated
    pub oracle_count: u8,

    /// Whether quorum was reached
    pub quorum_reached: bool,

    /// Verification latency in milliseconds
    pub latency_ms: u32,

    /// Individual oracle votes (for dispute resolution)
    pub votes: Vec<OracleVote>,

    /// Reason if verification failed
    pub failure_reason: Option<String>,
}

/// Individual oracle vote
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleVote {
    /// Oracle's DID
    pub oracle_did: String,

    /// Vote (true = approve, false = reject)
    pub approved: bool,

    /// Optional rejection reason
    pub reason: Option<String>,

    /// Vote timestamp
    pub timestamp: i64,
}

impl VerificationResult {
    /// Create a successful verification result
    pub fn success(oracle_count: u8, latency_ms: u32, votes: Vec<OracleVote>) -> Self {
        Self {
            passed: true,
            oracle_count,
            quorum_reached: true,
            latency_ms,
            votes,
            failure_reason: None,
        }
    }

    /// Create a failed verification result
    pub fn failure(
        oracle_count: u8,
        quorum_reached: bool,
        latency_ms: u32,
        votes: Vec<OracleVote>,
        reason: String,
    ) -> Self {
        Self {
            passed: false,
            oracle_count,
            quorum_reached,
            latency_ms,
            votes,
            failure_reason: Some(reason),
        }
    }

    /// Check if verification met latency SLA
    pub fn met_latency_sla(&self) -> bool {
        self.latency_ms <= crate::TARGET_VERIFICATION_LATENCY_MS as u32
    }
}

/// Verified action record with full audit trail
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeRecord {
    /// Unique record ID (UUIDv7 for time-ordering)
    pub id: Uuid,

    /// Actor's DID (who performed the action)
    pub actor_did: String,

    /// Client's DID (who requested the action)
    pub client_did: String,

    /// Action type identifier (e.g., "llm.completion", "code.review")
    pub action_type: String,

    /// BLAKE3 hash of input (privacy: never store raw input)
    pub input_hash: [u8; 32],

    /// BLAKE3 hash of output
    pub output_hash: [u8; 32],

    /// Compute consumed (PFLOP-hours)
    pub compute_hc: Decimal,

    /// Verification result from oracle consensus
    pub verification: VerificationResult,

    /// FROST threshold signature from oracles
    pub signature: FrostSignature,

    /// Merkle proof path for audit trail
    pub merkle_proof: Vec<[u8; 32]>,

    /// Merkle root at time of inclusion
    pub merkle_root: [u8; 32],

    /// Leaf index in Merkle tree
    pub merkle_index: u64,

    /// Submission timestamp (Unix milliseconds)
    pub submitted_at: i64,

    /// Verification completion timestamp
    pub verified_at: i64,

    /// EventStoreDB stream position (for replay)
    pub stream_position: Option<u64>,
}

impl OutcomeRecord {
    /// Create a new outcome record
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_did: String,
        client_did: String,
        action_type: String,
        input_hash: [u8; 32],
        output_hash: [u8; 32],
        compute_hc: Decimal,
        verification: VerificationResult,
        signature: FrostSignature,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: Uuid::now_v7(),
            actor_did,
            client_did,
            action_type,
            input_hash,
            output_hash,
            compute_hc,
            verification,
            signature,
            merkle_proof: Vec::new(),
            merkle_root: [0u8; 32],
            merkle_index: 0,
            submitted_at: now,
            verified_at: now,
            stream_position: None,
        }
    }

    /// Get the canonical hash of this record (for signing/verification)
    pub fn canonical_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.actor_did.as_bytes());
        hasher.update(self.client_did.as_bytes());
        hasher.update(self.action_type.as_bytes());
        hasher.update(&self.input_hash);
        hasher.update(&self.output_hash);
        hasher.update(&self.compute_hc.to_string().as_bytes());
        hasher.update(&self.submitted_at.to_le_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Set Merkle proof after tree inclusion
    pub fn set_merkle_proof(&mut self, proof: Vec<[u8; 32]>, root: [u8; 32], index: u64) {
        self.merkle_proof = proof;
        self.merkle_root = root;
        self.merkle_index = index;
    }

    /// Verify the Merkle proof is valid
    pub fn verify_merkle_proof(&self) -> bool {
        if self.merkle_proof.is_empty() {
            return false;
        }

        let leaf_hash = self.canonical_hash();
        let mut current = leaf_hash;
        let mut index = self.merkle_index;

        for sibling in &self.merkle_proof {
            current = if index % 2 == 0 {
                // Current is left child
                let mut hasher = blake3::Hasher::new();
                hasher.update(&current);
                hasher.update(sibling);
                *hasher.finalize().as_bytes()
            } else {
                // Current is right child
                let mut hasher = blake3::Hasher::new();
                hasher.update(sibling);
                hasher.update(&current);
                *hasher.finalize().as_bytes()
            };
            index /= 2;
        }

        current == self.merkle_root
    }

    /// Check if this record passed verification
    pub fn is_verified(&self) -> bool {
        self.verification.passed && self.signature.quorum_reached()
    }

    /// Get verification latency
    pub fn latency(&self) -> i64 {
        self.verified_at - self.submitted_at
    }
}

impl std::fmt::Display for OutcomeRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OutcomeRecord({}, {}, verified={}, latency={}ms)",
            self.id,
            self.action_type,
            self.is_verified(),
            self.verification.latency_ms
        )
    }
}

/// Builder for creating OutcomeRecord instances
pub struct OutcomeRecordBuilder {
    actor_did: Option<String>,
    client_did: Option<String>,
    action_type: Option<String>,
    input_hash: Option<[u8; 32]>,
    output_hash: Option<[u8; 32]>,
    compute_hc: Decimal,
}

impl Default for OutcomeRecordBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl OutcomeRecordBuilder {
    pub fn new() -> Self {
        Self {
            actor_did: None,
            client_did: None,
            action_type: None,
            input_hash: None,
            output_hash: None,
            compute_hc: Decimal::ZERO,
        }
    }

    pub fn actor_did(mut self, did: impl Into<String>) -> Self {
        self.actor_did = Some(did.into());
        self
    }

    pub fn client_did(mut self, did: impl Into<String>) -> Self {
        self.client_did = Some(did.into());
        self
    }

    pub fn action_type(mut self, action: impl Into<String>) -> Self {
        self.action_type = Some(action.into());
        self
    }

    pub fn input_hash(mut self, hash: [u8; 32]) -> Self {
        self.input_hash = Some(hash);
        self
    }

    pub fn output_hash(mut self, hash: [u8; 32]) -> Self {
        self.output_hash = Some(hash);
        self
    }

    pub fn compute_hc(mut self, hc: Decimal) -> Self {
        self.compute_hc = hc;
        self
    }

    /// Hash input bytes using BLAKE3
    pub fn hash_input(mut self, input: &[u8]) -> Self {
        self.input_hash = Some(*blake3::hash(input).as_bytes());
        self
    }

    /// Hash output bytes using BLAKE3
    pub fn hash_output(mut self, output: &[u8]) -> Self {
        self.output_hash = Some(*blake3::hash(output).as_bytes());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn create_test_record() -> OutcomeRecord {
        let verification = VerificationResult::success(
            3,
            150,
            vec![OracleVote {
                oracle_did: "did:key:oracle1".to_string(),
                approved: true,
                reason: None,
                timestamp: chrono::Utc::now().timestamp_millis(),
            }],
        );

        let signature = FrostSignature::new(
            [0u8; 64],
            vec!["did:key:oracle1".to_string()],
            [0u8; 32],
            3,
            5,
        );

        OutcomeRecord::new(
            "did:key:actor".to_string(),
            "did:key:client".to_string(),
            "test.action".to_string(),
            [1u8; 32],
            [2u8; 32],
            dec!(10),
            verification,
            signature,
        )
    }

    #[test]
    fn test_canonical_hash() {
        let record = create_test_record();
        let hash1 = record.canonical_hash();
        let hash2 = record.canonical_hash();
        assert_eq!(hash1, hash2); // Deterministic
    }

    #[test]
    fn test_frost_signature_quorum() {
        let sig = FrostSignature::new(
            [0u8; 64],
            vec![
                "did:key:o1".to_string(),
                "did:key:o2".to_string(),
                "did:key:o3".to_string(),
            ],
            [0u8; 32],
            3,
            5,
        );
        assert!(sig.quorum_reached());

        let sig_no_quorum = FrostSignature::new(
            [0u8; 64],
            vec!["did:key:o1".to_string(), "did:key:o2".to_string()],
            [0u8; 32],
            3,
            5,
        );
        assert!(!sig_no_quorum.quorum_reached());
    }

    #[test]
    fn test_verification_result_sla() {
        let fast = VerificationResult::success(3, 1500, vec![]);
        assert!(fast.met_latency_sla());

        let slow = VerificationResult::success(3, 3000, vec![]);
        assert!(!slow.met_latency_sla());
    }
}
