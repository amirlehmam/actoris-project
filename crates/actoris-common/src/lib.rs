//! # Actoris Common
//!
//! Shared types, errors, and cryptographic primitives for the Actoris Economic OS.
//!
//! ## Core Types
//!
//! - [`UnifiedID`]: DID-based identity for humans, agents, and organizations
//! - [`TrustScore`]: 0-1000 score representing entity trustworthiness
//! - [`HcWallet`]: HC (PFLOP-hours) credit balance management
//! - [`OutcomeRecord`]: Verified action record with oracle signatures
//! - [`PricingRequest`]/[`PricingResponse`]: Pricing calculation types
//!
//! ## Crypto
//!
//! - [`crypto::frost`]: FROST threshold signature (3-of-N Schnorr)
//! - [`crypto::merkle`]: Merkle tree for audit proofs
//! - [`crypto::did`]: W3C DID operations
//!
//! ## Security
//!
//! - [`security::mtls`]: Mutual TLS configuration
//! - [`security::hsm`]: Hardware Security Module integration
//! - [`security::policy`]: Security policy enforcement
//! - [`security::audit`]: Audit logging

pub mod crypto;
pub mod error;
pub mod security;
pub mod types;

// Re-export commonly used types at crate root
pub use error::{ActorisError, Result};
pub use types::{
    unified_id::{EntityType, UnifiedID},
    trust_score::{TrustComponents, TrustScore},
    hc_wallet::{HcWallet, WalletError},
    outcome_record::{FrostSignature, OutcomeRecord, VerificationResult},
    pricing::{
        DataSensitivity, PricingBreakdown, PricingRequest, PricingResponse, RiskFactor,
        TaskComplexity,
    },
};

/// Actoris version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum trust score value
pub const MAX_TRUST_SCORE: u16 = 1000;

/// Minimum trust score value
pub const MIN_TRUST_SCORE: u16 = 0;

/// HC wallet expiry in days
pub const HC_EXPIRY_DAYS: i64 = 30;

/// Target verification latency in milliseconds
pub const TARGET_VERIFICATION_LATENCY_MS: u64 = 2000;

/// Target pricing calculation latency in milliseconds
pub const TARGET_PRICING_LATENCY_MS: u64 = 10;

/// Maximum trust discount rate (20%)
pub const MAX_TRUST_DISCOUNT: f64 = 0.20;

/// Darwinian fitness target
pub const FITNESS_TARGET: f64 = 1.05;

/// Culling threshold
pub const CULLING_THRESHOLD: f64 = 0.7;

/// Grace epochs before culling
pub const GRACE_EPOCHS: u64 = 2;
