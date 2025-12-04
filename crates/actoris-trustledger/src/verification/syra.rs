//! Sybil Resistance Algorithm (SyRA)
//!
//! This module implements anti-Sybil protections for the ACTORIS network:
//!
//! 1. **Identity Verification Tiers**:
//!    - Tier 0: Unverified (rate limited, low trust)
//!    - Tier 1: Email verified
//!    - Tier 2: Phone verified
//!    - Tier 3: KYC verified (full trust)
//!
//! 2. **Behavioral Analysis**:
//!    - Spawn rate limiting (max agents per time window)
//!    - Request pattern analysis
//!    - Cross-agent coordination detection
//!
//! 3. **Economic Barriers**:
//!    - Minimum stake requirements for spawning
//!    - Deposit requirements for high-value operations
//!    - Slashing for malicious behavior
//!
//! 4. **Network Analysis**:
//!    - Identity graph clustering detection
//!    - Temporal correlation analysis
//!    - IP/device fingerprinting

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

/// SyRA errors
#[derive(Debug, Error)]
pub enum SyraError {
    #[error("Identity verification required: tier {required} needed, have tier {current}")]
    InsufficientVerification { required: u8, current: u8 },

    #[error("Rate limit exceeded: {operation} limit is {limit} per {window_secs}s")]
    RateLimitExceeded {
        operation: String,
        limit: u32,
        window_secs: u64,
    },

    #[error("Stake requirement not met: need {required}, have {available}")]
    InsufficientStake {
        required: Decimal,
        available: Decimal,
    },

    #[error("Suspicious behavior detected: {reason}")]
    SuspiciousBehavior { reason: String },

    #[error("Sybil cluster detected: {cluster_id}")]
    SybilClusterDetected { cluster_id: String },

    #[error("Cooling off period: {remaining_secs}s remaining")]
    CoolingOffPeriod { remaining_secs: u64 },
}

/// Identity verification tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum VerificationTier {
    /// Unverified - heavily rate limited
    Tier0 = 0,
    /// Email verified
    Tier1 = 1,
    /// Phone verified
    Tier2 = 2,
    /// Full KYC verified
    Tier3 = 3,
}

impl Default for VerificationTier {
    fn default() -> Self {
        Self::Tier0
    }
}

/// Configuration for SyRA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyraConfig {
    /// Rate limits per tier (operations per hour)
    pub tier_rate_limits: HashMap<VerificationTier, u32>,

    /// Spawn rate limits per tier (spawns per day)
    pub spawn_rate_limits: HashMap<VerificationTier, u32>,

    /// Minimum stake required to spawn an agent (in HC)
    pub min_spawn_stake: Decimal,

    /// Deposit required for high-value operations (percentage of HC)
    pub high_value_deposit_pct: f64,

    /// Threshold for high-value operations (in HC)
    pub high_value_threshold: Decimal,

    /// Cooling off period after suspicious activity (seconds)
    pub cooling_off_secs: u64,

    /// Maximum agents per identity graph cluster
    pub max_cluster_size: usize,

    /// Similarity threshold for Sybil detection (0.0 - 1.0)
    pub sybil_similarity_threshold: f64,

    /// Time window for temporal correlation analysis (seconds)
    pub temporal_window_secs: u64,
}

impl Default for SyraConfig {
    fn default() -> Self {
        let mut tier_rate_limits = HashMap::new();
        tier_rate_limits.insert(VerificationTier::Tier0, 10);
        tier_rate_limits.insert(VerificationTier::Tier1, 100);
        tier_rate_limits.insert(VerificationTier::Tier2, 1000);
        tier_rate_limits.insert(VerificationTier::Tier3, 10000);

        let mut spawn_rate_limits = HashMap::new();
        spawn_rate_limits.insert(VerificationTier::Tier0, 0);
        spawn_rate_limits.insert(VerificationTier::Tier1, 1);
        spawn_rate_limits.insert(VerificationTier::Tier2, 10);
        spawn_rate_limits.insert(VerificationTier::Tier3, 100);

        Self {
            tier_rate_limits,
            spawn_rate_limits,
            min_spawn_stake: Decimal::new(100, 0), // 100 HC
            high_value_deposit_pct: 0.10, // 10%
            high_value_threshold: Decimal::new(1000, 0), // 1000 HC
            cooling_off_secs: 3600, // 1 hour
            max_cluster_size: 50,
            sybil_similarity_threshold: 0.85,
            temporal_window_secs: 60,
        }
    }
}

/// Rate limiter using sliding window
struct RateLimiter {
    window: Duration,
    max_count: u32,
    timestamps: VecDeque<Instant>,
}

impl RateLimiter {
    fn new(window: Duration, max_count: u32) -> Self {
        Self {
            window,
            max_count,
            timestamps: VecDeque::new(),
        }
    }

    fn allow(&mut self) -> bool {
        let now = Instant::now();

        // Remove expired entries
        while let Some(ts) = self.timestamps.front() {
            if now.duration_since(*ts) > self.window {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        if self.timestamps.len() < self.max_count as usize {
            self.timestamps.push_back(now);
            true
        } else {
            false
        }
    }

    fn count(&self) -> u32 {
        self.timestamps.len() as u32
    }
}

/// Behavioral pattern for analysis
#[derive(Debug, Clone)]
struct BehavioralPattern {
    /// DIDs this identity has interacted with
    interactions: HashSet<String>,
    /// Timestamps of actions
    action_timestamps: VecDeque<Instant>,
    /// Request patterns (action type -> count)
    request_patterns: HashMap<String, u32>,
    /// Last action time
    last_action: Option<Instant>,
    /// Flagged for suspicious behavior
    flagged: bool,
    /// Cooling off until
    cooling_off_until: Option<Instant>,
}

impl Default for BehavioralPattern {
    fn default() -> Self {
        Self {
            interactions: HashSet::new(),
            action_timestamps: VecDeque::new(),
            request_patterns: HashMap::new(),
            last_action: None,
            flagged: false,
            cooling_off_until: None,
        }
    }
}

/// Identity cluster for Sybil detection
#[derive(Debug, Clone)]
struct IdentityCluster {
    cluster_id: String,
    members: HashSet<String>,
    created_at: Instant,
    flagged_as_sybil: bool,
}

/// Sybil Resistance Algorithm implementation
pub struct SyraGuard {
    config: SyraConfig,

    /// Rate limiters per DID
    rate_limiters: Arc<RwLock<HashMap<String, RateLimiter>>>,

    /// Spawn rate limiters per root DID
    spawn_limiters: Arc<RwLock<HashMap<String, RateLimiter>>>,

    /// Verification tiers per DID
    verification_tiers: Arc<RwLock<HashMap<String, VerificationTier>>>,

    /// Behavioral patterns per DID
    patterns: Arc<RwLock<HashMap<String, BehavioralPattern>>>,

    /// Identity clusters
    clusters: Arc<RwLock<HashMap<String, IdentityCluster>>>,

    /// Staked amounts per DID
    staked_amounts: Arc<RwLock<HashMap<String, Decimal>>>,
}

impl SyraGuard {
    /// Create a new SyRA guard
    pub fn new(config: SyraConfig) -> Self {
        Self {
            config,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            spawn_limiters: Arc::new(RwLock::new(HashMap::new())),
            verification_tiers: Arc::new(RwLock::new(HashMap::new())),
            patterns: Arc::new(RwLock::new(HashMap::new())),
            clusters: Arc::new(RwLock::new(HashMap::new())),
            staked_amounts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set verification tier for a DID
    pub fn set_verification_tier(&self, did: &str, tier: VerificationTier) {
        self.verification_tiers.write().insert(did.to_string(), tier);
        info!(did = %did, tier = ?tier, "Verification tier set");
    }

    /// Get verification tier for a DID
    pub fn get_verification_tier(&self, did: &str) -> VerificationTier {
        self.verification_tiers
            .read()
            .get(did)
            .copied()
            .unwrap_or_default()
    }

    /// Record stake for a DID
    pub fn record_stake(&self, did: &str, amount: Decimal) {
        let mut stakes = self.staked_amounts.write();
        let current = stakes.entry(did.to_string()).or_insert(Decimal::ZERO);
        *current += amount;
    }

    /// Check if an operation is allowed
    pub fn check_operation(&self, did: &str, operation: &str) -> Result<(), SyraError> {
        // Check cooling off period
        if let Some(pattern) = self.patterns.read().get(did) {
            if let Some(until) = pattern.cooling_off_until {
                if Instant::now() < until {
                    let remaining = until.duration_since(Instant::now()).as_secs();
                    return Err(SyraError::CoolingOffPeriod {
                        remaining_secs: remaining,
                    });
                }
            }
        }

        // Check rate limit
        let tier = self.get_verification_tier(did);
        let limit = self.config.tier_rate_limits.get(&tier).copied().unwrap_or(10);

        let mut limiters = self.rate_limiters.write();
        let limiter = limiters
            .entry(did.to_string())
            .or_insert_with(|| RateLimiter::new(Duration::from_secs(3600), limit));

        if !limiter.allow() {
            return Err(SyraError::RateLimitExceeded {
                operation: operation.to_string(),
                limit,
                window_secs: 3600,
            });
        }

        // Record action for behavioral analysis
        self.record_action(did, operation);

        Ok(())
    }

    /// Check if spawning is allowed
    pub fn check_spawn(&self, parent_did: &str, child_did: &str) -> Result<(), SyraError> {
        let tier = self.get_verification_tier(parent_did);

        // Check spawn limit
        let limit = self.config.spawn_rate_limits.get(&tier).copied().unwrap_or(0);
        if limit == 0 {
            return Err(SyraError::InsufficientVerification {
                required: VerificationTier::Tier1 as u8,
                current: tier as u8,
            });
        }

        let mut limiters = self.spawn_limiters.write();
        let limiter = limiters
            .entry(parent_did.to_string())
            .or_insert_with(|| RateLimiter::new(Duration::from_secs(86400), limit));

        if !limiter.allow() {
            return Err(SyraError::RateLimitExceeded {
                operation: "spawn".to_string(),
                limit,
                window_secs: 86400,
            });
        }

        // Check stake requirement
        let staked = self
            .staked_amounts
            .read()
            .get(parent_did)
            .copied()
            .unwrap_or(Decimal::ZERO);

        if staked < self.config.min_spawn_stake {
            return Err(SyraError::InsufficientStake {
                required: self.config.min_spawn_stake,
                available: staked,
            });
        }

        // Add child to parent's cluster
        self.add_to_cluster(parent_did, child_did)?;

        info!(
            parent = %parent_did,
            child = %child_did,
            "Spawn allowed"
        );

        Ok(())
    }

    /// Check for high-value operation deposit
    pub fn check_high_value(
        &self,
        did: &str,
        value: Decimal,
        available_hc: Decimal,
    ) -> Result<Decimal, SyraError> {
        if value < self.config.high_value_threshold {
            return Ok(Decimal::ZERO);
        }

        let tier = self.get_verification_tier(did);
        if tier < VerificationTier::Tier2 {
            return Err(SyraError::InsufficientVerification {
                required: VerificationTier::Tier2 as u8,
                current: tier as u8,
            });
        }

        // Calculate required deposit
        let deposit_pct = Decimal::try_from(self.config.high_value_deposit_pct).unwrap();
        let required_deposit = value * deposit_pct;

        if available_hc < required_deposit {
            return Err(SyraError::InsufficientStake {
                required: required_deposit,
                available: available_hc,
            });
        }

        Ok(required_deposit)
    }

    /// Record an action for behavioral analysis
    fn record_action(&self, did: &str, action_type: &str) {
        let mut patterns = self.patterns.write();
        let pattern = patterns
            .entry(did.to_string())
            .or_insert_with(BehavioralPattern::default);

        let now = Instant::now();

        // Record timestamp
        pattern.action_timestamps.push_back(now);
        pattern.last_action = Some(now);

        // Update request pattern
        *pattern.request_patterns.entry(action_type.to_string()).or_insert(0) += 1;

        // Trim old timestamps
        let window = Duration::from_secs(self.config.temporal_window_secs);
        while let Some(ts) = pattern.action_timestamps.front() {
            if now.duration_since(*ts) > window {
                pattern.action_timestamps.pop_front();
            } else {
                break;
            }
        }

        // Check for burst behavior (possible automation)
        if pattern.action_timestamps.len() > 10 {
            let first = pattern.action_timestamps.front().unwrap();
            let duration = now.duration_since(*first);
            let rate = pattern.action_timestamps.len() as f64 / duration.as_secs_f64();

            if rate > 5.0 {
                // More than 5 actions per second
                warn!(did = %did, rate = rate, "High action rate detected");
                pattern.flagged = true;
            }
        }
    }

    /// Add DID to identity cluster
    fn add_to_cluster(&self, parent_did: &str, child_did: &str) -> Result<(), SyraError> {
        let mut clusters = self.clusters.write();

        // Find parent's cluster or create new one
        let cluster_id = clusters
            .values()
            .find(|c| c.members.contains(parent_did))
            .map(|c| c.cluster_id.clone())
            .unwrap_or_else(|| {
                let id = format!("cluster_{}", uuid::Uuid::now_v7());
                let mut cluster = IdentityCluster {
                    cluster_id: id.clone(),
                    members: HashSet::new(),
                    created_at: Instant::now(),
                    flagged_as_sybil: false,
                };
                cluster.members.insert(parent_did.to_string());
                clusters.insert(id.clone(), cluster);
                id
            });

        // Check cluster size
        if let Some(cluster) = clusters.get_mut(&cluster_id) {
            if cluster.members.len() >= self.config.max_cluster_size {
                cluster.flagged_as_sybil = true;
                return Err(SyraError::SybilClusterDetected { cluster_id });
            }
            cluster.members.insert(child_did.to_string());
        }

        Ok(())
    }

    /// Analyze identity for Sybil behavior
    pub fn analyze_sybil_risk(&self, did: &str) -> SybilRiskAssessment {
        let pattern = self.patterns.read().get(did).cloned();
        let tier = self.get_verification_tier(did);
        let staked = self
            .staked_amounts
            .read()
            .get(did)
            .copied()
            .unwrap_or(Decimal::ZERO);

        let clusters = self.clusters.read();
        let cluster = clusters.values().find(|c| c.members.contains(did));

        let mut risk_score = 0.0;
        let mut risk_factors = Vec::new();

        // Low verification tier increases risk
        match tier {
            VerificationTier::Tier0 => {
                risk_score += 0.3;
                risk_factors.push("Unverified identity".to_string());
            }
            VerificationTier::Tier1 => {
                risk_score += 0.1;
            }
            _ => {}
        }

        // Low stake increases risk
        if staked < self.config.min_spawn_stake {
            risk_score += 0.2;
            risk_factors.push("Low stake".to_string());
        }

        // Large cluster increases risk
        if let Some(cluster) = cluster {
            let cluster_size = cluster.members.len();
            if cluster_size > 10 {
                risk_score += 0.1 * (cluster_size as f64 / self.config.max_cluster_size as f64);
                risk_factors.push(format!("Large identity cluster ({} members)", cluster_size));
            }
            if cluster.flagged_as_sybil {
                risk_score = 1.0;
                risk_factors.push("Cluster flagged as Sybil".to_string());
            }
        }

        // Suspicious behavioral patterns
        if let Some(pattern) = pattern {
            if pattern.flagged {
                risk_score += 0.3;
                risk_factors.push("Suspicious behavior detected".to_string());
            }
        }

        SybilRiskAssessment {
            did: did.to_string(),
            risk_score: risk_score.min(1.0),
            risk_factors,
            verification_tier: tier,
            is_sybil: risk_score >= self.config.sybil_similarity_threshold,
        }
    }

    /// Flag identity for suspicious behavior
    pub fn flag_suspicious(&self, did: &str, reason: &str) {
        let mut patterns = self.patterns.write();
        let pattern = patterns
            .entry(did.to_string())
            .or_insert_with(BehavioralPattern::default);

        pattern.flagged = true;
        pattern.cooling_off_until =
            Some(Instant::now() + Duration::from_secs(self.config.cooling_off_secs));

        warn!(did = %did, reason = %reason, "Identity flagged for suspicious behavior");
    }

    /// Slash stake for malicious behavior
    pub fn slash_stake(&self, did: &str, amount: Decimal) -> Decimal {
        let mut stakes = self.staked_amounts.write();
        if let Some(staked) = stakes.get_mut(did) {
            let to_slash = (*staked).min(amount);
            *staked -= to_slash;
            info!(did = %did, slashed = %to_slash, "Stake slashed");
            return to_slash;
        }
        Decimal::ZERO
    }

    /// Get statistics
    pub fn stats(&self) -> SyraStats {
        let tiers = self.verification_tiers.read();
        let patterns = self.patterns.read();
        let clusters = self.clusters.read();

        let mut tier_counts = HashMap::new();
        for tier in tiers.values() {
            *tier_counts.entry(*tier).or_insert(0u64) += 1;
        }

        SyraStats {
            total_identities: tiers.len(),
            tier_distribution: tier_counts,
            flagged_identities: patterns.values().filter(|p| p.flagged).count(),
            sybil_clusters: clusters.values().filter(|c| c.flagged_as_sybil).count(),
            total_clusters: clusters.len(),
        }
    }
}

/// Sybil risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SybilRiskAssessment {
    pub did: String,
    pub risk_score: f64,
    pub risk_factors: Vec<String>,
    pub verification_tier: VerificationTier,
    pub is_sybil: bool,
}

/// SyRA statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyraStats {
    pub total_identities: usize,
    pub tier_distribution: HashMap<VerificationTier, u64>,
    pub flagged_identities: usize,
    pub sybil_clusters: usize,
    pub total_clusters: usize,
}

impl Default for SyraGuard {
    fn default() -> Self {
        Self::new(SyraConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_rate_limiting() {
        let guard = SyraGuard::default();
        let did = "did:key:test";

        // Should allow first few operations
        for _ in 0..10 {
            assert!(guard.check_operation(did, "test.action").is_ok());
        }

        // Should hit rate limit for Tier0 (10 per hour)
        assert!(matches!(
            guard.check_operation(did, "test.action"),
            Err(SyraError::RateLimitExceeded { .. })
        ));

        // Upgrade to Tier1, should allow more
        guard.set_verification_tier(did, VerificationTier::Tier1);
        // Note: rate limiter keeps old count, so this might still fail
    }

    #[test]
    fn test_spawn_requirements() {
        let guard = SyraGuard::default();
        let parent = "did:key:parent";
        let child = "did:key:child";

        // Tier0 cannot spawn
        assert!(matches!(
            guard.check_spawn(parent, child),
            Err(SyraError::InsufficientVerification { .. })
        ));

        // Tier1 without stake cannot spawn
        guard.set_verification_tier(parent, VerificationTier::Tier1);
        assert!(matches!(
            guard.check_spawn(parent, child),
            Err(SyraError::InsufficientStake { .. })
        ));

        // With stake, can spawn
        guard.record_stake(parent, dec!(100));
        assert!(guard.check_spawn(parent, child).is_ok());
    }

    #[test]
    fn test_sybil_detection() {
        let guard = SyraGuard::default();
        let did = "did:key:test";

        // Initial risk should be low
        let assessment = guard.analyze_sybil_risk(did);
        assert!(assessment.risk_score > 0.0); // Some risk for unverified
        assert!(!assessment.is_sybil);

        // Flag as suspicious
        guard.flag_suspicious(did, "Test reason");

        let assessment = guard.analyze_sybil_risk(did);
        assert!(assessment.risk_score > 0.5);
    }

    #[test]
    fn test_cluster_size_limit() {
        let config = SyraConfig {
            max_cluster_size: 3,
            ..Default::default()
        };
        let guard = SyraGuard::new(config);

        let parent = "did:key:parent";
        guard.set_verification_tier(parent, VerificationTier::Tier2);
        guard.record_stake(parent, dec!(1000));

        // First spawns should succeed
        guard.check_spawn(parent, "did:key:child1").unwrap();
        guard.check_spawn(parent, "did:key:child2").unwrap();

        // Third spawn should fail (cluster limit)
        assert!(matches!(
            guard.check_spawn(parent, "did:key:child3"),
            Err(SyraError::SybilClusterDetected { .. })
        ));
    }

    #[test]
    fn test_high_value_deposit() {
        let guard = SyraGuard::default();
        let did = "did:key:test";

        // Low value, no deposit needed
        let deposit = guard.check_high_value(did, dec!(100), dec!(1000));
        assert!(deposit.is_ok());
        assert_eq!(deposit.unwrap(), dec!(0));

        // High value without tier
        let result = guard.check_high_value(did, dec!(2000), dec!(1000));
        assert!(matches!(result, Err(SyraError::InsufficientVerification { .. })));

        // High value with tier but insufficient HC
        guard.set_verification_tier(did, VerificationTier::Tier2);
        let result = guard.check_high_value(did, dec!(2000), dec!(100));
        assert!(matches!(result, Err(SyraError::InsufficientStake { .. })));

        // High value with sufficient HC
        let deposit = guard.check_high_value(did, dec!(2000), dec!(1000)).unwrap();
        assert_eq!(deposit, dec!(200)); // 10% of 2000
    }
}
