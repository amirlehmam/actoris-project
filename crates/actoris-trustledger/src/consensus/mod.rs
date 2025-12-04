//! Consensus module - BFT consensus with FROST threshold signatures
//!
//! This module provides:
//! - Malachite BFT consensus (HotStuff-2 based)
//! - Oracle node management
//! - Quorum management for 3-of-N verification

pub mod malachite;
pub mod oracle;
pub mod quorum;

pub use malachite::{
    Block, ConsensusConfig, ConsensusMessage, ConsensusMetrics, ConsensusNetwork,
    MalachiteConsensus, QuorumCertificate, VerificationRequest, VerificationResult, Vote,
    VoteType, ViewChange,
};
pub use oracle::OracleNode;
pub use quorum::QuorumManager;
