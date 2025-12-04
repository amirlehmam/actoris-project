//! Security module - mTLS, HSM, and security hardening
//!
//! This module provides:
//! - Mutual TLS (mTLS) configuration for inter-service communication
//! - Hardware Security Module (HSM) integration for key management
//! - Security policy enforcement
//! - Audit logging

pub mod mtls;
pub mod hsm;
pub mod policy;
pub mod audit;

pub use mtls::{MtlsConfig, MtlsConnector, MtlsAcceptor, CertificateSource};
pub use hsm::{HsmConfig, HsmProvider, HsmKeyHandle, KeyType};
pub use policy::{SecurityPolicy, PolicyEnforcer, PolicyViolation};
pub use audit::{AuditLogger, AuditEvent, AuditSeverity};
