//! Security Policy enforcement
//!
//! Defines and enforces security policies for the ACTORIS network:
//! - Rate limiting
//! - Access control
//! - Resource quotas
//! - Compliance requirements

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Policy violation
#[derive(Debug, Error)]
pub enum PolicyViolation {
    #[error("Rate limit exceeded: {resource}")]
    RateLimitExceeded { resource: String },

    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },

    #[error("Resource quota exceeded: {resource}, limit: {limit}")]
    QuotaExceeded { resource: String, limit: String },

    #[error("Compliance violation: {requirement}")]
    ComplianceViolation { requirement: String },

    #[error("IP blocked: {ip}")]
    IpBlocked { ip: String },

    #[error("Invalid operation: {operation}")]
    InvalidOperation { operation: String },
}

/// Security policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,

    /// Enable policy enforcement
    pub enabled: bool,

    /// Rate limiting rules
    pub rate_limits: Vec<RateLimitRule>,

    /// Access control rules
    pub access_control: Vec<AccessControlRule>,

    /// Resource quotas
    pub quotas: Vec<ResourceQuota>,

    /// IP allowlist (empty = allow all)
    pub ip_allowlist: Vec<String>,

    /// IP blocklist
    pub ip_blocklist: Vec<String>,

    /// Required TLS version
    pub min_tls_version: String,

    /// Required authentication
    pub require_auth: bool,

    /// Required mTLS
    pub require_mtls: bool,

    /// Audit all requests
    pub audit_all: bool,

    /// Max request size in bytes
    pub max_request_size: usize,

    /// Max response size in bytes
    pub max_response_size: usize,

    /// Request timeout
    pub request_timeout_secs: u64,

    /// Allowed HTTP methods
    pub allowed_methods: Vec<String>,

    /// Allowed content types
    pub allowed_content_types: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            enabled: true,
            rate_limits: vec![
                RateLimitRule {
                    name: "global".to_string(),
                    scope: RateLimitScope::Global,
                    requests: 1000,
                    window_secs: 60,
                    burst: 100,
                },
                RateLimitRule {
                    name: "per-ip".to_string(),
                    scope: RateLimitScope::PerIp,
                    requests: 100,
                    window_secs: 60,
                    burst: 20,
                },
            ],
            access_control: vec![],
            quotas: vec![],
            ip_allowlist: vec![],
            ip_blocklist: vec![],
            min_tls_version: "1.2".to_string(),
            require_auth: true,
            require_mtls: false,
            audit_all: true,
            max_request_size: 10 * 1024 * 1024, // 10MB
            max_response_size: 100 * 1024 * 1024, // 100MB
            request_timeout_secs: 30,
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
            ],
            allowed_content_types: vec![
                "application/json".to_string(),
                "application/grpc".to_string(),
                "application/protobuf".to_string(),
            ],
        }
    }
}

/// Rate limit rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitRule {
    /// Rule name
    pub name: String,
    /// Scope of rate limiting
    pub scope: RateLimitScope,
    /// Maximum requests
    pub requests: u64,
    /// Time window in seconds
    pub window_secs: u64,
    /// Burst allowance
    pub burst: u64,
}

/// Rate limit scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RateLimitScope {
    /// Global rate limit
    Global,
    /// Per IP address
    PerIp,
    /// Per DID
    PerDid,
    /// Per endpoint
    PerEndpoint,
}

/// Access control rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlRule {
    /// Rule name
    pub name: String,
    /// Resource pattern (glob)
    pub resource: String,
    /// Required roles
    pub required_roles: Vec<String>,
    /// Required trust score (0-1000)
    pub min_trust_score: Option<u16>,
    /// Required verification tier
    pub min_verification_tier: Option<u8>,
    /// Allowed DIDs (empty = all authenticated)
    pub allowed_dids: Vec<String>,
    /// Action: allow or deny
    pub action: AccessAction,
}

/// Access action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessAction {
    Allow,
    Deny,
}

/// Resource quota
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// Quota name
    pub name: String,
    /// Resource type
    pub resource: String,
    /// Maximum value
    pub limit: u64,
    /// Reset period in seconds (0 = never)
    pub reset_period_secs: u64,
    /// Scope
    pub scope: QuotaScope,
}

/// Quota scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuotaScope {
    Global,
    PerDid,
    PerOrganization,
}

/// Request context for policy evaluation
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Source IP address
    pub source_ip: Option<IpAddr>,
    /// Authenticated DID
    pub did: Option<String>,
    /// Trust score
    pub trust_score: Option<u16>,
    /// Verification tier
    pub verification_tier: Option<u8>,
    /// Roles
    pub roles: Vec<String>,
    /// Request path/resource
    pub resource: String,
    /// HTTP method
    pub method: String,
    /// Content type
    pub content_type: Option<String>,
    /// Request size in bytes
    pub request_size: usize,
    /// TLS version
    pub tls_version: Option<String>,
    /// mTLS verified
    pub mtls_verified: bool,
}

/// Rate limiter state
struct RateLimiterState {
    count: u64,
    window_start: Instant,
    burst_remaining: u64,
}

/// Policy enforcer
pub struct PolicyEnforcer {
    policy: SecurityPolicy,
    /// Rate limiter states: key -> state
    rate_limiters: Arc<RwLock<HashMap<String, RateLimiterState>>>,
    /// Quota usage: key -> current value
    quota_usage: Arc<RwLock<HashMap<String, u64>>>,
}

impl PolicyEnforcer {
    /// Create a new policy enforcer
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            policy,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            quota_usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Evaluate a request against the policy
    pub fn evaluate(&self, ctx: &RequestContext) -> Result<(), PolicyViolation> {
        if !self.policy.enabled {
            return Ok(());
        }

        // Check IP blocklist
        if let Some(ip) = ctx.source_ip {
            let ip_str = ip.to_string();
            if self.policy.ip_blocklist.iter().any(|blocked| {
                ip_str.starts_with(blocked) || ip_str == *blocked
            }) {
                return Err(PolicyViolation::IpBlocked { ip: ip_str });
            }
        }

        // Check IP allowlist (if not empty)
        if !self.policy.ip_allowlist.is_empty() {
            if let Some(ip) = ctx.source_ip {
                let ip_str = ip.to_string();
                if !self.policy.ip_allowlist.iter().any(|allowed| {
                    ip_str.starts_with(allowed) || ip_str == *allowed
                }) {
                    return Err(PolicyViolation::IpBlocked { ip: ip_str });
                }
            }
        }

        // Check authentication requirement
        if self.policy.require_auth && ctx.did.is_none() {
            return Err(PolicyViolation::AccessDenied {
                reason: "Authentication required".to_string(),
            });
        }

        // Check mTLS requirement
        if self.policy.require_mtls && !ctx.mtls_verified {
            return Err(PolicyViolation::AccessDenied {
                reason: "mTLS required".to_string(),
            });
        }

        // Check TLS version
        if let Some(tls) = &ctx.tls_version {
            if !self.check_tls_version(tls) {
                return Err(PolicyViolation::ComplianceViolation {
                    requirement: format!(
                        "TLS {} required, got {}",
                        self.policy.min_tls_version, tls
                    ),
                });
            }
        }

        // Check HTTP method
        if !self.policy.allowed_methods.contains(&ctx.method) {
            return Err(PolicyViolation::InvalidOperation {
                operation: format!("Method {} not allowed", ctx.method),
            });
        }

        // Check content type
        if let Some(ct) = &ctx.content_type {
            if !self.policy.allowed_content_types.iter().any(|allowed| ct.starts_with(allowed)) {
                return Err(PolicyViolation::InvalidOperation {
                    operation: format!("Content type {} not allowed", ct),
                });
            }
        }

        // Check request size
        if ctx.request_size > self.policy.max_request_size {
            return Err(PolicyViolation::QuotaExceeded {
                resource: "request_size".to_string(),
                limit: format!("{} bytes", self.policy.max_request_size),
            });
        }

        // Check rate limits
        for rule in &self.policy.rate_limits {
            if !self.check_rate_limit(rule, ctx)? {
                return Err(PolicyViolation::RateLimitExceeded {
                    resource: rule.name.clone(),
                });
            }
        }

        // Check access control rules
        for rule in &self.policy.access_control {
            if self.matches_resource(&rule.resource, &ctx.resource) {
                self.check_access_rule(rule, ctx)?;
            }
        }

        Ok(())
    }

    /// Check rate limit
    fn check_rate_limit(
        &self,
        rule: &RateLimitRule,
        ctx: &RequestContext,
    ) -> Result<bool, PolicyViolation> {
        let key = match rule.scope {
            RateLimitScope::Global => rule.name.clone(),
            RateLimitScope::PerIp => {
                if let Some(ip) = ctx.source_ip {
                    format!("{}:{}", rule.name, ip)
                } else {
                    return Ok(true); // No IP, skip
                }
            }
            RateLimitScope::PerDid => {
                if let Some(did) = &ctx.did {
                    format!("{}:{}", rule.name, did)
                } else {
                    return Ok(true); // No DID, skip
                }
            }
            RateLimitScope::PerEndpoint => {
                format!("{}:{}", rule.name, ctx.resource)
            }
        };

        let now = Instant::now();
        let window = Duration::from_secs(rule.window_secs);

        let mut limiters = self.rate_limiters.write();
        let state = limiters.entry(key).or_insert_with(|| RateLimiterState {
            count: 0,
            window_start: now,
            burst_remaining: rule.burst,
        });

        // Reset window if expired
        if now.duration_since(state.window_start) > window {
            state.count = 0;
            state.window_start = now;
            state.burst_remaining = rule.burst;
        }

        // Check limit
        if state.count >= rule.requests {
            // Try using burst
            if state.burst_remaining > 0 {
                state.burst_remaining -= 1;
                return Ok(true);
            }
            return Ok(false);
        }

        state.count += 1;
        Ok(true)
    }

    /// Check access control rule
    fn check_access_rule(
        &self,
        rule: &AccessControlRule,
        ctx: &RequestContext,
    ) -> Result<(), PolicyViolation> {
        // If rule is deny, check if it matches
        if rule.action == AccessAction::Deny {
            let matches = self.check_rule_match(rule, ctx);
            if matches {
                return Err(PolicyViolation::AccessDenied {
                    reason: format!("Denied by rule: {}", rule.name),
                });
            }
            return Ok(());
        }

        // For allow rules, check if requirements are met
        if !self.check_rule_match(rule, ctx) {
            return Err(PolicyViolation::AccessDenied {
                reason: format!("Access control rule {} not satisfied", rule.name),
            });
        }

        Ok(())
    }

    /// Check if a rule matches the context
    fn check_rule_match(&self, rule: &AccessControlRule, ctx: &RequestContext) -> bool {
        // Check required roles
        if !rule.required_roles.is_empty() {
            if !rule.required_roles.iter().any(|r| ctx.roles.contains(r)) {
                return false;
            }
        }

        // Check trust score
        if let Some(min_trust) = rule.min_trust_score {
            if ctx.trust_score.unwrap_or(0) < min_trust {
                return false;
            }
        }

        // Check verification tier
        if let Some(min_tier) = rule.min_verification_tier {
            if ctx.verification_tier.unwrap_or(0) < min_tier {
                return false;
            }
        }

        // Check allowed DIDs
        if !rule.allowed_dids.is_empty() {
            if let Some(did) = &ctx.did {
                if !rule.allowed_dids.contains(did) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Check if resource matches pattern
    fn matches_resource(&self, pattern: &str, resource: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if pattern.ends_with('*') {
            return resource.starts_with(&pattern[..pattern.len() - 1]);
        }
        if pattern.starts_with('*') {
            return resource.ends_with(&pattern[1..]);
        }
        pattern == resource
    }

    /// Check TLS version
    fn check_tls_version(&self, version: &str) -> bool {
        let min_version: f32 = self.policy.min_tls_version.parse().unwrap_or(1.2);
        let actual: f32 = version.parse().unwrap_or(0.0);
        actual >= min_version
    }

    /// Record quota usage
    pub fn record_quota_usage(&self, resource: &str, amount: u64, scope_key: &str) {
        let key = format!("{}:{}", resource, scope_key);
        let mut usage = self.quota_usage.write();
        let current = usage.entry(key).or_insert(0);
        *current += amount;
    }

    /// Check quota
    pub fn check_quota(&self, resource: &str, scope_key: &str) -> Result<u64, PolicyViolation> {
        for quota in &self.policy.quotas {
            if quota.resource == resource {
                let key = format!("{}:{}", resource, scope_key);
                let usage = self.quota_usage.read().get(&key).copied().unwrap_or(0);

                if usage >= quota.limit {
                    return Err(PolicyViolation::QuotaExceeded {
                        resource: resource.to_string(),
                        limit: quota.limit.to_string(),
                    });
                }

                return Ok(quota.limit - usage);
            }
        }

        Ok(u64::MAX) // No quota defined
    }

    /// Get current policy
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    /// Update policy
    pub fn update_policy(&mut self, policy: SecurityPolicy) {
        info!(name = %policy.name, "Policy updated");
        self.policy = policy;
    }

    /// Reset rate limiters
    pub fn reset_rate_limiters(&self) {
        self.rate_limiters.write().clear();
    }

    /// Reset quota usage
    pub fn reset_quotas(&self) {
        self.quota_usage.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_default_policy() {
        let policy = SecurityPolicy::default();
        assert!(policy.enabled);
        assert!(policy.require_auth);
        assert_eq!(policy.rate_limits.len(), 2);
    }

    #[test]
    fn test_policy_evaluation() {
        let policy = SecurityPolicy::default();
        let enforcer = PolicyEnforcer::new(policy);

        let ctx = RequestContext {
            source_ip: Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            did: Some("did:key:test".to_string()),
            trust_score: Some(800),
            verification_tier: Some(2),
            roles: vec!["user".to_string()],
            resource: "/api/v1/test".to_string(),
            method: "GET".to_string(),
            content_type: Some("application/json".to_string()),
            request_size: 1024,
            tls_version: Some("1.3".to_string()),
            mtls_verified: false,
        };

        assert!(enforcer.evaluate(&ctx).is_ok());
    }

    #[test]
    fn test_ip_blocklist() {
        let mut policy = SecurityPolicy::default();
        policy.ip_blocklist.push("10.0.0.".to_string());

        let enforcer = PolicyEnforcer::new(policy);

        let ctx = RequestContext {
            source_ip: Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
            did: Some("did:key:test".to_string()),
            trust_score: None,
            verification_tier: None,
            roles: vec![],
            resource: "/test".to_string(),
            method: "GET".to_string(),
            content_type: None,
            request_size: 0,
            tls_version: None,
            mtls_verified: false,
        };

        assert!(matches!(
            enforcer.evaluate(&ctx),
            Err(PolicyViolation::IpBlocked { .. })
        ));
    }

    #[test]
    fn test_rate_limiting() {
        let mut policy = SecurityPolicy::default();
        policy.rate_limits = vec![RateLimitRule {
            name: "test".to_string(),
            scope: RateLimitScope::Global,
            requests: 3,
            window_secs: 60,
            burst: 0,
        }];

        let enforcer = PolicyEnforcer::new(policy);

        let ctx = RequestContext {
            source_ip: None,
            did: Some("did:key:test".to_string()),
            trust_score: None,
            verification_tier: None,
            roles: vec![],
            resource: "/test".to_string(),
            method: "GET".to_string(),
            content_type: None,
            request_size: 0,
            tls_version: None,
            mtls_verified: false,
        };

        // First 3 should pass
        assert!(enforcer.evaluate(&ctx).is_ok());
        assert!(enforcer.evaluate(&ctx).is_ok());
        assert!(enforcer.evaluate(&ctx).is_ok());

        // Fourth should fail
        assert!(matches!(
            enforcer.evaluate(&ctx),
            Err(PolicyViolation::RateLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_resource_matching() {
        let policy = SecurityPolicy::default();
        let enforcer = PolicyEnforcer::new(policy);

        assert!(enforcer.matches_resource("*", "/any/path"));
        assert!(enforcer.matches_resource("/api/*", "/api/v1/users"));
        assert!(!enforcer.matches_resource("/api/*", "/other/path"));
        assert!(enforcer.matches_resource("*.json", "data.json"));
        assert!(enforcer.matches_resource("/exact", "/exact"));
        assert!(!enforcer.matches_resource("/exact", "/other"));
    }
}
