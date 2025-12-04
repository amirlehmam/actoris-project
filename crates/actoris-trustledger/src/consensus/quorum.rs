//! Quorum management for 3-of-N verification

/// Manages quorum formation and voting
pub struct QuorumManager {
    threshold: u8,
    total: u8,
}

impl QuorumManager {
    pub fn new(threshold: u8, total: u8) -> Self {
        Self { threshold, total }
    }

    pub fn threshold(&self) -> u8 {
        self.threshold
    }

    pub fn total(&self) -> u8 {
        self.total
    }
}
