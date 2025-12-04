//! Error types for Actoris system
//!
//! Provides a unified error type and domain-specific error variants

use thiserror::Error;

/// Result type alias using ActorisError
pub type Result<T> = std::result::Result<T, ActorisError>;

/// Unified error type for Actoris operations
#[derive(Debug, Error)]
pub enum ActorisError {
    // Identity errors
    #[error("Identity error: {0}")]
    Identity(#[from] IdentityError),

    // Wallet errors
    #[error("Wallet error: {0}")]
    Wallet(#[from] crate::types::hc_wallet::WalletError),

    // Crypto errors
    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),

    // Verification errors
    #[error("Verification error: {0}")]
    Verification(#[from] VerificationError),

    // Pricing errors
    #[error("Pricing error: {0}")]
    Pricing(#[from] PricingError),

    // Storage errors
    #[error("Storage error: {0}")]
    Storage(String),

    // Network errors
    #[error("Network error: {0}")]
    Network(String),

    // Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    // Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),

    // Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),
}

/// Identity-related errors
#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("Invalid DID format: {0}")]
    InvalidDid(String),

    #[error("DID not found: {0}")]
    NotFound(String),

    #[error("Entity type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Lineage verification failed: {0}")]
    LineageError(String),

    #[error("Trust score below minimum threshold: {score} < {minimum}")]
    InsufficientTrust { score: u16, minimum: u16 },

    #[error("Entity is not authorized for this operation")]
    Unauthorized,

    #[error("Parent entity not found for spawning")]
    ParentNotFound,
}

/// Cryptographic operation errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid public key")]
    InvalidPublicKey,

    #[error("FROST protocol error: {0}")]
    FrostError(String),

    #[error("Key generation failed: {0}")]
    KeyGeneration(String),

    #[error("Merkle proof verification failed")]
    MerkleProofInvalid,

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Threshold not met: {signers} of {threshold} required")]
    ThresholdNotMet { signers: u8, threshold: u8 },
}

/// Verification process errors
#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("Quorum not reached: {votes} of {required} oracles voted")]
    QuorumNotReached { votes: u8, required: u8 },

    #[error("Verification timeout after {elapsed_ms}ms (limit: {limit_ms}ms)")]
    Timeout { elapsed_ms: u64, limit_ms: u64 },

    #[error("Oracle unavailable: {oracle_did}")]
    OracleUnavailable { oracle_did: String },

    #[error("Action semantics invalid: {reason}")]
    SemanticFailure { reason: String },

    #[error("Duplicate action detected: {action_id}")]
    DuplicateAction { action_id: String },

    #[error("Action type not supported: {action_type}")]
    UnsupportedAction { action_type: String },

    #[error("Input validation failed: {0}")]
    InputValidation(String),
}

/// Pricing calculation errors
#[derive(Debug, Error)]
pub enum PricingError {
    #[error("Invalid compute amount: must be positive")]
    InvalidComputeAmount,

    #[error("Rules engine error: {0}")]
    RulesEngine(String),

    #[error("Price quote expired")]
    QuoteExpired,

    #[error("Budget exceeded: price {price} > budget {budget}")]
    BudgetExceeded { price: String, budget: String },

    #[error("Rate not configured for action type: {action_type}")]
    RateNotConfigured { action_type: String },

    #[error("Pricing calculation overflow")]
    Overflow,
}

// Implement From for common external error types
impl From<serde_json::Error> for ActorisError {
    fn from(err: serde_json::Error) -> Self {
        ActorisError::Serialization(err.to_string())
    }
}

impl From<std::io::Error> for ActorisError {
    fn from(err: std::io::Error) -> Self {
        ActorisError::Storage(err.to_string())
    }
}

impl From<anyhow::Error> for ActorisError {
    fn from(err: anyhow::Error) -> Self {
        ActorisError::Internal(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ActorisError::Identity(IdentityError::NotFound("did:key:test".to_string()));
        assert!(err.to_string().contains("did:key:test"));
    }

    #[test]
    fn test_verification_error() {
        let err = VerificationError::QuorumNotReached {
            votes: 2,
            required: 3,
        };
        assert!(err.to_string().contains("2 of 3"));
    }
}
