//! Spawn primitive - Create child agents with 30% trust cap

use actoris_common::{TrustScore, UnifiedID};

/// Spawn a new child agent from a parent
pub struct SpawnPrimitive;

impl SpawnPrimitive {
    /// Maximum trust inheritance ratio
    pub const TRUST_CAP: f64 = 0.30;

    /// Spawn a new agent from parent
    pub fn spawn(parent: &UnifiedID, parent_trust: &TrustScore) -> (UnifiedID, TrustScore) {
        let (child_id, _signing_key) = UnifiedID::new_agent(Some(parent));
        let child_trust = TrustScore::new_spawned(parent_trust.score);
        (child_id, child_trust)
    }
}
