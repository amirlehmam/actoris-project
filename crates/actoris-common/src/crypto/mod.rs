//! Cryptographic primitives for Actoris
//!
//! This module provides:
//! - FROST threshold signatures (3-of-N Schnorr)
//! - Merkle tree operations for audit proofs
//! - DID (Decentralized Identifier) operations

pub mod did;
pub mod frost;
pub mod merkle;

// Re-export commonly used items
pub use frost::{FrostKeyShare, FrostSigner, PartialSignature};
pub use merkle::{MerkleProof, MerkleTree};
