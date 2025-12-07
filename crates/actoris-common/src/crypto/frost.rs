//! FROST Threshold Signatures (RFC 9591)
//!
//! Production implementation of FROST (Flexible Round-Optimized Schnorr Threshold)
//! signatures using the frost-ed25519 crate. This enables:
//! - 3-of-N quorum verification (default 3-of-5)
//! - Distributed key generation (DKG)
//! - Two-round signing protocol
//! - 64-byte aggregated signatures (vs N*64 for multisig)
//!
//! Reference: RFC 9591 - https://www.rfc-editor.org/rfc/rfc9591.html

use crate::error::CryptoError;
use frost_ed25519 as frost;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Minimum threshold for production use (Byzantine fault tolerance)
pub const MIN_THRESHOLD: u16 = 3;

/// Default oracle count (allows 1 Byzantine failure with t=3)
pub const DEFAULT_ORACLE_COUNT: u16 = 5;

/// Maximum supported oracles
pub const MAX_ORACLE_COUNT: u16 = 100;

/// FROST participant identifier
pub type ParticipantId = frost::Identifier;

/// FROST key share held by a single oracle
#[derive(Clone)]
pub struct FrostKeyShare {
    /// Oracle's participant identifier
    pub identifier: ParticipantId,
    /// Secret key share (KEEP SECURE!)
    secret_share: frost::keys::KeyPackage,
    /// Public key package for verification
    pub public_key_package: frost::keys::PublicKeyPackage,
}

impl FrostKeyShare {
    /// Create from frost key package
    pub fn new(
        identifier: ParticipantId,
        secret_share: frost::keys::KeyPackage,
        public_key_package: frost::keys::PublicKeyPackage,
    ) -> Self {
        Self {
            identifier,
            secret_share,
            public_key_package,
        }
    }

    /// Get the group public key (verifying key)
    pub fn group_public_key(&self) -> [u8; 32] {
        self.public_key_package
            .verifying_key()
            .serialize()
            .as_ref()
            .try_into()
            .unwrap_or([0u8; 32])
    }

    /// Get this oracle's identifier as bytes
    pub fn identifier_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        let id_bytes = self.identifier.serialize();
        bytes[..id_bytes.len().min(32)].copy_from_slice(&id_bytes[..id_bytes.len().min(32)]);
        bytes
    }

    /// Get the key package for signing
    pub fn key_package(&self) -> &frost::keys::KeyPackage {
        &self.secret_share
    }
}

/// Signing commitment from round 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningCommitment {
    /// Participant identifier (serialized)
    pub identifier: Vec<u8>,
    /// Hiding commitment
    pub hiding: Vec<u8>,
    /// Binding commitment
    pub binding: Vec<u8>,
}

impl SigningCommitment {
    /// Convert to FROST SigningCommitments
    pub fn to_frost(&self) -> Result<(ParticipantId, frost::round1::SigningCommitments), CryptoError> {
        let id_bytes: [u8; 32] = self.identifier.clone().try_into()
            .map_err(|_| CryptoError::FrostError("Invalid identifier length".to_string()))?;
        let identifier = ParticipantId::deserialize(&id_bytes)
            .map_err(|e| CryptoError::FrostError(format!("Invalid identifier: {}", e)))?;

        // Reconstruct commitments from serialized form
        let hiding_bytes: [u8; 32] = self.hiding.clone().try_into()
            .map_err(|_| CryptoError::FrostError("Invalid hiding commitment length".to_string()))?;
        let hiding = frost::round1::NonceCommitment::deserialize(hiding_bytes)
            .map_err(|e| CryptoError::FrostError(format!("Invalid hiding commitment: {}", e)))?;

        let binding_bytes: [u8; 32] = self.binding.clone().try_into()
            .map_err(|_| CryptoError::FrostError("Invalid binding commitment length".to_string()))?;
        let binding = frost::round1::NonceCommitment::deserialize(binding_bytes)
            .map_err(|e| CryptoError::FrostError(format!("Invalid binding commitment: {}", e)))?;

        let commitments = frost::round1::SigningCommitments::new(hiding, binding);
        Ok((identifier, commitments))
    }
}

/// Signature share from round 2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureShare {
    /// Participant identifier (serialized)
    pub identifier: Vec<u8>,
    /// Signature share bytes
    pub share: Vec<u8>,
}

impl SignatureShare {
    /// Convert to FROST SignatureShare
    pub fn to_frost(&self) -> Result<(ParticipantId, frost::round2::SignatureShare), CryptoError> {
        let id_bytes: [u8; 32] = self.identifier.clone().try_into()
            .map_err(|_| CryptoError::FrostError("Invalid identifier length".to_string()))?;
        let identifier = ParticipantId::deserialize(&id_bytes)
            .map_err(|e| CryptoError::FrostError(format!("Invalid identifier: {}", e)))?;

        let share_bytes: [u8; 32] = self.share.clone().try_into()
            .map_err(|_| CryptoError::FrostError("Invalid signature share length".to_string()))?;
        let share = frost::round2::SignatureShare::deserialize(share_bytes)
            .map_err(|e| CryptoError::FrostError(format!("Invalid signature share: {}", e)))?;

        Ok((identifier, share))
    }
}

/// Aggregated FROST signature (64 bytes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostSignature {
    /// R component (32 bytes)
    pub r: [u8; 32],
    /// s component (32 bytes)
    pub s: [u8; 32],
}

impl FrostSignature {
    /// Create from FROST Signature
    pub fn from_frost(sig: &frost::Signature) -> Self {
        let bytes = sig.serialize();
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&bytes[..32]);
        s.copy_from_slice(&bytes[32..64]);
        Self { r, s }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&self.r);
        bytes[32..].copy_from_slice(&self.s);
        bytes
    }

    /// Convert to FROST Signature
    pub fn to_frost(&self) -> Result<frost::Signature, CryptoError> {
        let bytes = self.to_bytes();
        frost::Signature::deserialize(bytes)
            .map_err(|e| CryptoError::FrostError(format!("Invalid signature: {}", e)))
    }
}

/// Signing session state for a single oracle
pub struct SigningSession {
    /// Session ID
    pub session_id: String,
    /// Message being signed
    pub message: Vec<u8>,
    /// Our signing nonces
    nonces: frost::round1::SigningNonces,
    /// Our commitment
    commitment: frost::round1::SigningCommitments,
    /// Collected commitments from all participants
    commitments: BTreeMap<ParticipantId, frost::round1::SigningCommitments>,
    /// Key package reference
    key_package: frost::keys::KeyPackage,
}

/// FROST signer for a single oracle node
pub struct FrostSigner {
    /// This oracle's key share
    key_share: FrostKeyShare,
    /// Active signing sessions
    sessions: Arc<RwLock<HashMap<String, SigningSession>>>,
}

impl FrostSigner {
    /// Create a new FROST signer with the given key share
    pub fn new(key_share: FrostKeyShare) -> Self {
        Self {
            key_share,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a new signing session (Round 1)
    ///
    /// Returns our commitment to share with other oracles
    pub async fn start_signing(
        &self,
        session_id: &str,
        message: &[u8],
    ) -> Result<SigningCommitment, CryptoError> {
        let mut rng = OsRng;

        // Generate nonces and commitment
        let (nonces, commitments) = frost::round1::commit(
            self.key_share.key_package().signing_share(),
            &mut rng,
        );

        let session = SigningSession {
            session_id: session_id.to_string(),
            message: message.to_vec(),
            nonces,
            commitment: commitments,
            commitments: BTreeMap::new(),
            key_package: self.key_share.secret_share.clone(),
        };

        // Store session
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), session);

        // Return serialized commitment
        Ok(SigningCommitment {
            identifier: self.key_share.identifier.serialize().to_vec(),
            hiding: commitments.hiding().serialize().to_vec(),
            binding: commitments.binding().serialize().to_vec(),
        })
    }

    /// Add a commitment from another participant
    pub async fn add_commitment(
        &self,
        session_id: &str,
        commitment: SigningCommitment,
    ) -> Result<(), CryptoError> {
        let (identifier, frost_commitment) = commitment.to_frost()?;

        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            CryptoError::FrostError(format!("Session not found: {}", session_id))
        })?;

        session.commitments.insert(identifier, frost_commitment);
        Ok(())
    }

    /// Generate signature share (Round 2)
    ///
    /// Call this after collecting commitments from threshold participants
    pub async fn sign(&self, session_id: &str) -> Result<SignatureShare, CryptoError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.remove(session_id).ok_or_else(|| {
            CryptoError::FrostError(format!("Session not found: {}", session_id))
        })?;

        // Add our own commitment
        let mut commitments = session.commitments;
        commitments.insert(self.key_share.identifier, session.commitment);

        // Create signing package
        let signing_package = frost::SigningPackage::new(commitments, &session.message);

        // Generate signature share
        let signature_share = frost::round2::sign(&signing_package, &session.nonces, &session.key_package)
            .map_err(|e| CryptoError::FrostError(format!("Signing failed: {}", e)))?;

        Ok(SignatureShare {
            identifier: self.key_share.identifier.serialize().to_vec(),
            share: signature_share.serialize().to_vec(),
        })
    }

    /// Get the group public key
    pub fn group_public_key(&self) -> [u8; 32] {
        self.key_share.group_public_key()
    }

    /// Get our identifier
    pub fn identifier(&self) -> ParticipantId {
        self.key_share.identifier
    }
}

/// Coordinator for aggregating FROST signatures
pub struct FrostCoordinator {
    /// Public key package
    public_key_package: frost::keys::PublicKeyPackage,
    /// Minimum threshold
    threshold: u16,
}

impl FrostCoordinator {
    /// Create a new coordinator
    pub fn new(public_key_package: frost::keys::PublicKeyPackage, threshold: u16) -> Self {
        Self {
            public_key_package,
            threshold,
        }
    }

    /// Aggregate signature shares into final signature
    pub fn aggregate(
        &self,
        message: &[u8],
        commitments: &[SigningCommitment],
        shares: &[SignatureShare],
    ) -> Result<FrostSignature, CryptoError> {
        if shares.len() < self.threshold as usize {
            return Err(CryptoError::ThresholdNotMet {
                signers: shares.len() as u8,
                threshold: self.threshold as u8,
            });
        }

        // Convert commitments
        let frost_commitments: BTreeMap<ParticipantId, frost::round1::SigningCommitments> =
            commitments
                .iter()
                .map(|c| c.to_frost())
                .collect::<Result<_, _>>()?;

        // Create signing package
        let signing_package = frost::SigningPackage::new(frost_commitments, message);

        // Convert signature shares
        let frost_shares: BTreeMap<ParticipantId, frost::round2::SignatureShare> = shares
            .iter()
            .map(|s| s.to_frost())
            .collect::<Result<_, _>>()?;

        // Aggregate signatures
        let signature = frost::aggregate(&signing_package, &frost_shares, &self.public_key_package)
            .map_err(|e| CryptoError::FrostError(format!("Aggregation failed: {}", e)))?;

        Ok(FrostSignature::from_frost(&signature))
    }

    /// Verify a FROST signature
    pub fn verify(&self, message: &[u8], signature: &FrostSignature) -> Result<bool, CryptoError> {
        let frost_sig = signature.to_frost()?;
        let verifying_key = self.public_key_package.verifying_key();

        match verifying_key.verify(message, &frost_sig) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get the group public key
    pub fn group_public_key(&self) -> [u8; 32] {
        self.public_key_package
            .verifying_key()
            .serialize()
            .as_ref()
            .try_into()
            .unwrap_or([0u8; 32])
    }
}

/// Distributed Key Generation (DKG) result
pub struct DkgResult {
    /// Key shares for each participant
    pub key_shares: Vec<FrostKeyShare>,
    /// Public key package (same for all)
    pub public_key_package: frost::keys::PublicKeyPackage,
}

/// Generate key shares using trusted dealer (for development/testing)
///
/// In production, use full DKG protocol with round1/round2/round3
pub fn generate_key_shares_trusted(
    threshold: u16,
    num_shares: u16,
) -> Result<DkgResult, CryptoError> {
    if threshold < MIN_THRESHOLD {
        return Err(CryptoError::FrostError(format!(
            "Threshold must be at least {}",
            MIN_THRESHOLD
        )));
    }
    if num_shares > MAX_ORACLE_COUNT {
        return Err(CryptoError::FrostError(format!(
            "Max {} oracles supported",
            MAX_ORACLE_COUNT
        )));
    }
    if threshold > num_shares {
        return Err(CryptoError::FrostError(
            "Threshold cannot exceed number of shares".to_string(),
        ));
    }

    let mut rng = OsRng;

    // Generate identifiers
    let max_signers = num_shares;
    let min_signers = threshold;

    // Use trusted dealer to generate shares
    let (shares, public_key_package) = frost::keys::generate_with_dealer(
        max_signers,
        min_signers,
        frost::keys::IdentifierList::Default,
        &mut rng,
    )
    .map_err(|e| CryptoError::FrostError(format!("Key generation failed: {}", e)))?;

    // Convert to our format
    let key_shares: Result<Vec<FrostKeyShare>, CryptoError> = shares
        .into_iter()
        .map(|(id, secret_share)| {
            let key_package = frost::keys::KeyPackage::try_from(secret_share)
                .map_err(|e| CryptoError::FrostError(format!("Failed to convert secret share: {}", e)))?;
            Ok(FrostKeyShare::new(id, key_package, public_key_package.clone()))
        })
        .collect();

    Ok(DkgResult {
        key_shares: key_shares?,
        public_key_package,
    })
}

/// DKG Round 1 - Generate secret polynomial and commitment
pub struct DkgRound1 {
    pub identifier: ParticipantId,
    pub secret_package: frost::keys::dkg::round1::SecretPackage,
    pub package: frost::keys::dkg::round1::Package,
}

/// Run DKG Round 1 for a participant
pub fn dkg_round1(
    identifier: ParticipantId,
    max_signers: u16,
    min_signers: u16,
) -> Result<DkgRound1, CryptoError> {
    let mut rng = OsRng;

    let (secret_package, package) = frost::keys::dkg::part1(
        identifier,
        max_signers,
        min_signers,
        &mut rng,
    )
    .map_err(|e| CryptoError::FrostError(format!("DKG round 1 failed: {}", e)))?;

    Ok(DkgRound1 {
        identifier,
        secret_package,
        package,
    })
}

/// DKG Round 2 output
pub struct DkgRound2 {
    pub identifier: ParticipantId,
    pub secret_package: frost::keys::dkg::round2::SecretPackage,
    pub packages: BTreeMap<ParticipantId, frost::keys::dkg::round2::Package>,
}

/// Run DKG Round 2 for a participant
pub fn dkg_round2(
    round1: DkgRound1,
    round1_packages: &BTreeMap<ParticipantId, frost::keys::dkg::round1::Package>,
) -> Result<DkgRound2, CryptoError> {
    let (secret_package, packages) = frost::keys::dkg::part2(
        round1.secret_package,
        round1_packages,
    )
    .map_err(|e| CryptoError::FrostError(format!("DKG round 2 failed: {}", e)))?;

    Ok(DkgRound2 {
        identifier: round1.identifier,
        secret_package,
        packages,
    })
}

/// Run DKG Round 3 (finalize) for a participant
pub fn dkg_round3(
    round2: DkgRound2,
    round1_packages: &BTreeMap<ParticipantId, frost::keys::dkg::round1::Package>,
    round2_packages: &BTreeMap<ParticipantId, frost::keys::dkg::round2::Package>,
) -> Result<FrostKeyShare, CryptoError> {
    let (key_package, public_key_package) = frost::keys::dkg::part3(
        &round2.secret_package,
        round1_packages,
        round2_packages,
    )
    .map_err(|e| CryptoError::FrostError(format!("DKG round 3 failed: {}", e)))?;

    Ok(FrostKeyShare::new(
        round2.identifier,
        key_package,
        public_key_package,
    ))
}

/// Verify a FROST signature with a public key
pub fn verify_signature(
    message: &[u8],
    signature: &FrostSignature,
    group_public_key: &[u8; 32],
) -> Result<bool, CryptoError> {
    let frost_sig = signature.to_frost()?;

    let verifying_key = frost::VerifyingKey::deserialize(*group_public_key)
        .map_err(|e| CryptoError::FrostError(format!("Invalid public key: {}", e)))?;

    match verifying_key.verify(message, &frost_sig) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Verify a FROST signature (raw bytes)
pub fn verify_signature_bytes(
    message: &[u8],
    signature: &[u8; 64],
    group_public_key: &[u8; 32],
) -> Result<bool, CryptoError> {
    let frost_sig = frost::Signature::deserialize(*signature)
        .map_err(|e| CryptoError::FrostError(format!("Invalid signature: {}", e)))?;

    let verifying_key = frost::VerifyingKey::deserialize(*group_public_key)
        .map_err(|e| CryptoError::FrostError(format!("Invalid public key: {}", e)))?;

    match verifying_key.verify(message, &frost_sig) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trusted_dealer_key_generation() {
        let result = generate_key_shares_trusted(3, 5).unwrap();
        assert_eq!(result.key_shares.len(), 5);

        // All shares should have same group public key
        let gpk = result.key_shares[0].group_public_key();
        for share in &result.key_shares {
            assert_eq!(share.group_public_key(), gpk);
        }
    }

    #[tokio::test]
    async fn test_threshold_signing() {
        // Generate 3-of-5 key shares
        let dkg_result = generate_key_shares_trusted(3, 5).unwrap();
        let shares = dkg_result.key_shares;

        // Create signers for threshold (3 of 5)
        let signers: Vec<FrostSigner> = shares[..3]
            .iter()
            .cloned()
            .map(FrostSigner::new)
            .collect();

        let message = b"test message for FROST signing";
        let session_id = "test-session-1";

        // Round 1: Generate commitments
        let mut commitments = Vec::new();
        for signer in &signers {
            let commitment = signer.start_signing(session_id, message).await.unwrap();
            commitments.push(commitment);
        }

        // Exchange commitments
        for signer in &signers {
            for commitment in &commitments {
                signer.add_commitment(session_id, commitment.clone()).await.unwrap();
            }
        }

        // Round 2: Generate signature shares
        let mut sig_shares = Vec::new();
        for signer in &signers {
            let share = signer.sign(session_id).await.unwrap();
            sig_shares.push(share);
        }

        // Aggregate
        let coordinator = FrostCoordinator::new(dkg_result.public_key_package.clone(), 3);
        let signature = coordinator.aggregate(message, &commitments, &sig_shares).unwrap();

        // Verify
        let valid = coordinator.verify(message, &signature).unwrap();
        assert!(valid);

        // Verify with standalone function
        let gpk = shares[0].group_public_key();
        let valid2 = verify_signature(message, &signature, &gpk).unwrap();
        assert!(valid2);

        // Verify wrong message fails
        let wrong_msg = b"wrong message";
        let invalid = coordinator.verify(wrong_msg, &signature).unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_threshold_not_met() {
        let dkg_result = generate_key_shares_trusted(3, 5).unwrap();
        let coordinator = FrostCoordinator::new(dkg_result.public_key_package, 3);

        let result = coordinator.aggregate(b"test", &[], &[]);
        assert!(matches!(result, Err(CryptoError::ThresholdNotMet { .. })));
    }

    #[test]
    fn test_min_threshold_check() {
        let result = generate_key_shares_trusted(2, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_serialization() {
        let sig = FrostSignature {
            r: [1u8; 32],
            s: [2u8; 32],
        };

        let bytes = sig.to_bytes();
        assert_eq!(&bytes[..32], &[1u8; 32]);
        assert_eq!(&bytes[32..], &[2u8; 32]);
    }
}
