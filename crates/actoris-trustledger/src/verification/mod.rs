//! Verification module
//!
//! This module provides:
//! - Action verification with oracle quorum
//! - SyRA (Sybil Resistance Algorithm) protection
//! - Protocol DNA primitives (SPAWN, LEND, INSURE, DELEGATE)

pub mod syra;
pub mod verifier;
pub mod dna;

pub use verifier::ActionVerifier;
pub use syra::{SyraGuard, SyraConfig, SyraError, SybilRiskAssessment, VerificationTier};
pub use dna::{ProtocolDna, DnaPrimitive, SpawnRequest, LendRequest, InsureRequest, DelegateRequest};
