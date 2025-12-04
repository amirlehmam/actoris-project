//! TrustScore - Entity trustworthiness metric (0-1000)
//!
//! TrustScore is the core reputation primitive in Actoris. It affects:
//! - Pricing (up to 20% discount for high trust)
//! - Resource allocation (Darwinian selection)
//! - Protocol access (certain primitives require minimum trust)
//! - Credit limits (Lend primitive)

use serde::{Deserialize, Serialize};

/// Maximum possible trust score
pub const MAX_SCORE: u16 = 1000;

/// Minimum possible trust score
pub const MIN_SCORE: u16 = 0;

/// Default starting trust score for new entities
pub const DEFAULT_SCORE: u16 = 500;

/// Maximum trust discount rate (20%)
pub const MAX_DISCOUNT_RATE: f64 = 0.20;

/// Trust score breakdown by component
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustComponents {
    /// Outcome verification success rate (0-400 points)
    /// Based on percentage of actions that pass oracle verification
    pub verification_score: u16,

    /// Dispute history penalty (0-200 points deducted)
    /// Increases with each dispute, decays over time
    pub dispute_penalty: u16,

    /// SLA compliance score (0-200 points)
    /// Based on meeting promised latency/quality metrics
    pub sla_score: u16,

    /// Network reputation (EigenTrust-like) (0-200 points)
    /// Based on trust from other high-trust entities
    pub network_score: u16,
}

impl TrustComponents {
    /// Calculate total score from components
    pub fn total(&self) -> u16 {
        let base = self.verification_score + self.sla_score + self.network_score;
        base.saturating_sub(self.dispute_penalty).min(MAX_SCORE)
    }

    /// Create components for a new entity with default score
    pub fn default_new() -> Self {
        Self {
            verification_score: 200, // 50% of max
            dispute_penalty: 0,
            sla_score: 100, // 50% of max
            network_score: 100, // 50% of max
        }
    }
}

/// Entity trust score with full breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    /// Composite score from 0-1000
    pub score: u16,

    /// Component breakdown for transparency
    pub components: TrustComponents,

    /// Last update timestamp (Unix milliseconds)
    pub updated_at: i64,

    /// Total number of verified outcomes
    pub verified_outcomes: u64,

    /// Historical dispute rate (0.0 - 1.0)
    pub dispute_rate: f64,

    /// Score version for optimistic concurrency
    pub version: u64,
}

impl Default for TrustScore {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustScore {
    /// Create a new TrustScore with default values
    pub fn new() -> Self {
        let components = TrustComponents::default_new();
        Self {
            score: components.total(),
            components,
            updated_at: chrono::Utc::now().timestamp_millis(),
            verified_outcomes: 0,
            dispute_rate: 0.0,
            version: 0,
        }
    }

    /// Create a TrustScore capped at parent's 30% (for spawned agents)
    pub fn new_spawned(parent_score: u16) -> Self {
        let capped_score = ((parent_score as f64) * 0.30) as u16;
        let mut score = Self::new();
        score.score = capped_score.min(score.score);
        score
    }

    /// Get normalized tau value for fitness calculation (0.0 - 1.0)
    ///
    /// Used in Darwinian formula: η = τ × (revenue / cost)
    #[inline]
    pub fn tau(&self) -> f64 {
        self.score as f64 / MAX_SCORE as f64
    }

    /// Calculate trust discount rate for pricing (0.0 - 0.20)
    ///
    /// High-trust entities get up to 20% discount on pricing
    /// Formula: discount = (score / 1000) * 0.20
    #[inline]
    pub fn discount_rate(&self) -> f64 {
        (self.tau() * MAX_DISCOUNT_RATE).min(MAX_DISCOUNT_RATE)
    }

    /// Update verification component based on outcome
    pub fn record_verification(&mut self, success: bool) {
        self.verified_outcomes += 1;

        // Calculate new success rate
        let success_rate = if success {
            (self.components.verification_score as f64 / 400.0 * (self.verified_outcomes - 1) as f64
                + 1.0)
                / self.verified_outcomes as f64
        } else {
            (self.components.verification_score as f64 / 400.0 * (self.verified_outcomes - 1) as f64)
                / self.verified_outcomes as f64
        };

        // Update verification score (max 400)
        self.components.verification_score = (success_rate * 400.0) as u16;
        self.recalculate();
    }

    /// Record a dispute (reduces trust)
    pub fn record_dispute(&mut self) {
        // Each dispute adds penalty (decays over time in separate process)
        self.components.dispute_penalty = (self.components.dispute_penalty + 20).min(200);

        // Update dispute rate
        let total_interactions = self.verified_outcomes.max(1);
        self.dispute_rate = self.components.dispute_penalty as f64 / 200.0;

        self.recalculate();
    }

    /// Update SLA compliance score
    pub fn update_sla_score(&mut self, compliance_rate: f64) {
        self.components.sla_score = (compliance_rate * 200.0) as u16;
        self.recalculate();
    }

    /// Update network reputation score
    pub fn update_network_score(&mut self, reputation: f64) {
        self.components.network_score = (reputation * 200.0).min(200.0) as u16;
        self.recalculate();
    }

    /// Recalculate composite score from components
    fn recalculate(&mut self) {
        self.score = self.components.total();
        self.updated_at = chrono::Utc::now().timestamp_millis();
        self.version += 1;
    }

    /// Check if entity meets minimum trust threshold
    pub fn meets_threshold(&self, min_score: u16) -> bool {
        self.score >= min_score
    }

    /// Calculate credit limit multiplier for Lend primitive
    /// Higher trust = higher credit limits
    pub fn credit_multiplier(&self) -> f64 {
        // Exponential scaling: low trust = 0.1x, max trust = 3x
        0.1 + (self.tau().powf(2.0) * 2.9)
    }
}

impl std::fmt::Display for TrustScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TrustScore({}/1000, τ={:.3}, discount={:.1}%)",
            self.score,
            self.tau(),
            self.discount_rate() * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_trust_score() {
        let score = TrustScore::new();
        assert!(score.score > 0);
        assert!(score.score <= MAX_SCORE);
        assert_eq!(score.verified_outcomes, 0);
    }

    #[test]
    fn test_spawned_agent_cap() {
        let score = TrustScore::new_spawned(1000);
        assert!(score.score <= 300); // 30% cap
    }

    #[test]
    fn test_tau_calculation() {
        let mut score = TrustScore::new();
        score.score = 500;
        assert!((score.tau() - 0.5).abs() < 0.001);

        score.score = 1000;
        assert!((score.tau() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_discount_rate() {
        let mut score = TrustScore::new();
        score.score = 1000;
        assert!((score.discount_rate() - 0.20).abs() < 0.001);

        score.score = 500;
        assert!((score.discount_rate() - 0.10).abs() < 0.001);
    }

    #[test]
    fn test_credit_multiplier() {
        let mut score = TrustScore::new();
        score.score = 0;
        assert!((score.credit_multiplier() - 0.1).abs() < 0.01);

        score.score = 1000;
        assert!((score.credit_multiplier() - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_record_dispute() {
        let mut score = TrustScore::new();
        let initial = score.score;
        score.record_dispute();
        assert!(score.score < initial);
        assert!(score.components.dispute_penalty > 0);
    }
}
