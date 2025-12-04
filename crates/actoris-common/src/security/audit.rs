//! Security Audit Logging
//!
//! Provides comprehensive audit logging for security-relevant events:
//! - Authentication attempts
//! - Authorization decisions
//! - Resource access
//! - Configuration changes
//! - Security policy violations

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Audit event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AuditSeverity {
    /// Informational - normal operation
    Info,
    /// Warning - potential issue
    Warning,
    /// Error - operation failed
    Error,
    /// Critical - security incident
    Critical,
}

impl std::fmt::Display for AuditSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditSeverity::Info => write!(f, "INFO"),
            AuditSeverity::Warning => write!(f, "WARN"),
            AuditSeverity::Error => write!(f, "ERROR"),
            AuditSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Audit event category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditCategory {
    /// Authentication events
    Authentication,
    /// Authorization decisions
    Authorization,
    /// Resource access
    ResourceAccess,
    /// Data modification
    DataModification,
    /// Configuration changes
    Configuration,
    /// Security policy
    SecurityPolicy,
    /// Key management
    KeyManagement,
    /// Network events
    Network,
    /// System events
    System,
}

impl std::fmt::Display for AuditCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditCategory::Authentication => write!(f, "AUTHN"),
            AuditCategory::Authorization => write!(f, "AUTHZ"),
            AuditCategory::ResourceAccess => write!(f, "ACCESS"),
            AuditCategory::DataModification => write!(f, "DATA"),
            AuditCategory::Configuration => write!(f, "CONFIG"),
            AuditCategory::SecurityPolicy => write!(f, "POLICY"),
            AuditCategory::KeyManagement => write!(f, "KEY"),
            AuditCategory::Network => write!(f, "NETWORK"),
            AuditCategory::System => write!(f, "SYSTEM"),
        }
    }
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub event_id: String,

    /// Timestamp (Unix millis)
    pub timestamp: i64,

    /// Event severity
    pub severity: AuditSeverity,

    /// Event category
    pub category: AuditCategory,

    /// Event action (e.g., "login", "access", "modify")
    pub action: String,

    /// Outcome (success/failure)
    pub outcome: AuditOutcome,

    /// Actor DID (who performed the action)
    pub actor_did: Option<String>,

    /// Target resource
    pub resource: Option<String>,

    /// Source IP address
    pub source_ip: Option<String>,

    /// Additional details
    pub details: HashMap<String, String>,

    /// Request ID for correlation
    pub request_id: Option<String>,

    /// Session ID
    pub session_id: Option<String>,

    /// User agent
    pub user_agent: Option<String>,
}

/// Audit outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditOutcome {
    Success,
    Failure,
    Unknown,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(
        category: AuditCategory,
        action: &str,
        outcome: AuditOutcome,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::now_v7().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            severity: match outcome {
                AuditOutcome::Success => AuditSeverity::Info,
                AuditOutcome::Failure => AuditSeverity::Warning,
                AuditOutcome::Unknown => AuditSeverity::Info,
            },
            category,
            action: action.to_string(),
            outcome,
            actor_did: None,
            resource: None,
            source_ip: None,
            details: HashMap::new(),
            request_id: None,
            session_id: None,
            user_agent: None,
        }
    }

    /// Set severity
    pub fn with_severity(mut self, severity: AuditSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set actor
    pub fn with_actor(mut self, did: &str) -> Self {
        self.actor_did = Some(did.to_string());
        self
    }

    /// Set resource
    pub fn with_resource(mut self, resource: &str) -> Self {
        self.resource = Some(resource.to_string());
        self
    }

    /// Set source IP
    pub fn with_source_ip(mut self, ip: &str) -> Self {
        self.source_ip = Some(ip.to_string());
        self
    }

    /// Add detail
    pub fn with_detail(mut self, key: &str, value: &str) -> Self {
        self.details.insert(key.to_string(), value.to_string());
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, id: &str) -> Self {
        self.request_id = Some(id.to_string());
        self
    }

    /// Set session ID
    pub fn with_session_id(mut self, id: &str) -> Self {
        self.session_id = Some(id.to_string());
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, ua: &str) -> Self {
        self.user_agent = Some(ua.to_string());
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Audit log sink
pub trait AuditSink: Send + Sync {
    /// Write an audit event
    fn write(&self, event: &AuditEvent);

    /// Flush pending events
    fn flush(&self);
}

/// Console audit sink (for development)
pub struct ConsoleAuditSink;

impl AuditSink for ConsoleAuditSink {
    fn write(&self, event: &AuditEvent) {
        let log_line = format!(
            "[{}] {} {} {} - actor={} resource={} outcome={:?}",
            event.severity,
            event.category,
            event.action,
            event.event_id,
            event.actor_did.as_deref().unwrap_or("-"),
            event.resource.as_deref().unwrap_or("-"),
            event.outcome,
        );

        match event.severity {
            AuditSeverity::Info => info!("{}", log_line),
            AuditSeverity::Warning => warn!("{}", log_line),
            AuditSeverity::Error => error!("{}", log_line),
            AuditSeverity::Critical => error!("CRITICAL: {}", log_line),
        }
    }

    fn flush(&self) {
        // Console logging is immediate
    }
}

/// File audit sink
pub struct FileAuditSink {
    path: String,
    buffer: Arc<RwLock<Vec<String>>>,
    max_buffer_size: usize,
}

impl FileAuditSink {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            buffer: Arc::new(RwLock::new(Vec::new())),
            max_buffer_size: 100,
        }
    }
}

impl AuditSink for FileAuditSink {
    fn write(&self, event: &AuditEvent) {
        let json = event.to_json();
        let mut buffer = self.buffer.write();
        buffer.push(json);

        if buffer.len() >= self.max_buffer_size {
            // In a real implementation, write to file
            debug!(path = %self.path, count = buffer.len(), "Flushing audit buffer");
            buffer.clear();
        }
    }

    fn flush(&self) {
        let mut buffer = self.buffer.write();
        if !buffer.is_empty() {
            debug!(path = %self.path, count = buffer.len(), "Flushing audit buffer");
            // In a real implementation, write to file
            buffer.clear();
        }
    }
}

/// Audit logger
pub struct AuditLogger {
    sinks: Vec<Box<dyn AuditSink>>,
    /// Minimum severity to log
    min_severity: AuditSeverity,
    /// Categories to log (empty = all)
    enabled_categories: Vec<AuditCategory>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new() -> Self {
        Self {
            sinks: vec![Box::new(ConsoleAuditSink)],
            min_severity: AuditSeverity::Info,
            enabled_categories: vec![],
        }
    }

    /// Add a sink
    pub fn add_sink(&mut self, sink: Box<dyn AuditSink>) {
        self.sinks.push(sink);
    }

    /// Set minimum severity
    pub fn set_min_severity(&mut self, severity: AuditSeverity) {
        self.min_severity = severity;
    }

    /// Set enabled categories
    pub fn set_enabled_categories(&mut self, categories: Vec<AuditCategory>) {
        self.enabled_categories = categories;
    }

    /// Log an audit event
    pub fn log(&self, event: AuditEvent) {
        // Check severity
        if event.severity < self.min_severity {
            return;
        }

        // Check category
        if !self.enabled_categories.is_empty()
            && !self.enabled_categories.contains(&event.category)
        {
            return;
        }

        // Write to all sinks
        for sink in &self.sinks {
            sink.write(&event);
        }
    }

    /// Log authentication event
    pub fn log_authentication(
        &self,
        actor: &str,
        method: &str,
        success: bool,
        source_ip: Option<&str>,
    ) {
        let outcome = if success {
            AuditOutcome::Success
        } else {
            AuditOutcome::Failure
        };

        let mut event = AuditEvent::new(AuditCategory::Authentication, "authenticate", outcome)
            .with_actor(actor)
            .with_detail("method", method);

        if !success {
            event = event.with_severity(AuditSeverity::Warning);
        }

        if let Some(ip) = source_ip {
            event = event.with_source_ip(ip);
        }

        self.log(event);
    }

    /// Log authorization event
    pub fn log_authorization(
        &self,
        actor: &str,
        resource: &str,
        action: &str,
        allowed: bool,
    ) {
        let outcome = if allowed {
            AuditOutcome::Success
        } else {
            AuditOutcome::Failure
        };

        let mut event = AuditEvent::new(AuditCategory::Authorization, action, outcome)
            .with_actor(actor)
            .with_resource(resource);

        if !allowed {
            event = event.with_severity(AuditSeverity::Warning);
        }

        self.log(event);
    }

    /// Log resource access
    pub fn log_access(
        &self,
        actor: &str,
        resource: &str,
        operation: &str,
        success: bool,
    ) {
        let outcome = if success {
            AuditOutcome::Success
        } else {
            AuditOutcome::Failure
        };

        let event = AuditEvent::new(AuditCategory::ResourceAccess, operation, outcome)
            .with_actor(actor)
            .with_resource(resource);

        self.log(event);
    }

    /// Log data modification
    pub fn log_modification(
        &self,
        actor: &str,
        resource: &str,
        operation: &str,
        old_hash: Option<&str>,
        new_hash: Option<&str>,
    ) {
        let mut event = AuditEvent::new(AuditCategory::DataModification, operation, AuditOutcome::Success)
            .with_actor(actor)
            .with_resource(resource);

        if let Some(old) = old_hash {
            event = event.with_detail("old_hash", old);
        }
        if let Some(new) = new_hash {
            event = event.with_detail("new_hash", new);
        }

        self.log(event);
    }

    /// Log configuration change
    pub fn log_config_change(
        &self,
        actor: &str,
        component: &str,
        setting: &str,
        old_value: &str,
        new_value: &str,
    ) {
        let event = AuditEvent::new(AuditCategory::Configuration, "change", AuditOutcome::Success)
            .with_actor(actor)
            .with_resource(component)
            .with_detail("setting", setting)
            .with_detail("old_value", old_value)
            .with_detail("new_value", new_value);

        self.log(event);
    }

    /// Log security policy violation
    pub fn log_policy_violation(
        &self,
        actor: Option<&str>,
        resource: &str,
        violation: &str,
        source_ip: Option<&str>,
    ) {
        let mut event = AuditEvent::new(
            AuditCategory::SecurityPolicy,
            "violation",
            AuditOutcome::Failure,
        )
        .with_severity(AuditSeverity::Warning)
        .with_resource(resource)
        .with_detail("violation", violation);

        if let Some(a) = actor {
            event = event.with_actor(a);
        }
        if let Some(ip) = source_ip {
            event = event.with_source_ip(ip);
        }

        self.log(event);
    }

    /// Log key management event
    pub fn log_key_event(
        &self,
        actor: &str,
        key_id: &str,
        operation: &str,
        success: bool,
    ) {
        let outcome = if success {
            AuditOutcome::Success
        } else {
            AuditOutcome::Failure
        };

        let event = AuditEvent::new(AuditCategory::KeyManagement, operation, outcome)
            .with_actor(actor)
            .with_resource(key_id);

        self.log(event);
    }

    /// Log critical security incident
    pub fn log_security_incident(
        &self,
        description: &str,
        actor: Option<&str>,
        source_ip: Option<&str>,
        details: HashMap<String, String>,
    ) {
        let mut event = AuditEvent::new(
            AuditCategory::SecurityPolicy,
            "incident",
            AuditOutcome::Failure,
        )
        .with_severity(AuditSeverity::Critical);

        event.details = details;
        event = event.with_detail("description", description);

        if let Some(a) = actor {
            event = event.with_actor(a);
        }
        if let Some(ip) = source_ip {
            event = event.with_source_ip(ip);
        }

        self.log(event);
    }

    /// Flush all sinks
    pub fn flush(&self) {
        for sink in &self.sinks {
            sink.flush();
        }
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditCategory::Authentication, "login", AuditOutcome::Success)
            .with_actor("did:key:test")
            .with_source_ip("127.0.0.1")
            .with_detail("method", "mTLS");

        assert_eq!(event.category, AuditCategory::Authentication);
        assert_eq!(event.action, "login");
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.actor_did, Some("did:key:test".to_string()));
        assert_eq!(event.details.get("method"), Some(&"mTLS".to_string()));
    }

    #[test]
    fn test_audit_event_json() {
        let event = AuditEvent::new(AuditCategory::Authorization, "access", AuditOutcome::Failure)
            .with_resource("/api/secret");

        let json = event.to_json();
        assert!(json.contains("AUTHZ") || json.contains("Authorization"));
        assert!(json.contains("access"));
    }

    #[test]
    fn test_audit_logger() {
        let logger = AuditLogger::new();

        // Should not panic
        logger.log_authentication("did:key:test", "password", true, Some("127.0.0.1"));
        logger.log_authorization("did:key:test", "/resource", "read", true);
        logger.log_access("did:key:test", "/data", "GET", true);
        logger.log_policy_violation(Some("did:key:bad"), "/admin", "rate_limit", None);

        logger.flush();
    }

    #[test]
    fn test_severity_ordering() {
        assert!(AuditSeverity::Info < AuditSeverity::Warning);
        assert!(AuditSeverity::Warning < AuditSeverity::Error);
        assert!(AuditSeverity::Error < AuditSeverity::Critical);
    }
}
