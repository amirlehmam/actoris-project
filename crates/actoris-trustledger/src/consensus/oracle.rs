//! Oracle node implementation for action verification

use actoris_common::crypto::frost::FrostKeyShare;
use std::sync::Arc;

/// Oracle node for participating in verification consensus
pub struct OracleNode {
    /// This node's DID
    pub did: String,
    /// FROST key share
    _frost_share: Arc<FrostKeyShare>,
}

impl OracleNode {
    /// Create a new oracle node
    pub fn new(did: String, frost_share: FrostKeyShare) -> Self {
        Self {
            did,
            _frost_share: Arc::new(frost_share),
        }
    }
}
