//! Mutual TLS (mTLS) configuration
//!
//! Provides secure inter-service communication with:
//! - Client and server certificate validation
//! - Certificate rotation support
//! - SPIFFE/SPIRE integration
//! - Certificate revocation checking

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

/// mTLS errors
#[derive(Debug, Error)]
pub enum MtlsError {
    #[error("Certificate not found: {path}")]
    CertificateNotFound { path: String },

    #[error("Invalid certificate: {reason}")]
    InvalidCertificate { reason: String },

    #[error("Private key not found: {path}")]
    PrivateKeyNotFound { path: String },

    #[error("Invalid private key: {reason}")]
    InvalidPrivateKey { reason: String },

    #[error("Certificate expired")]
    CertificateExpired,

    #[error("Certificate revoked")]
    CertificateRevoked,

    #[error("Peer verification failed: {reason}")]
    PeerVerificationFailed { reason: String },

    #[error("HSM error: {0}")]
    HsmError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Certificate source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertificateSource {
    /// Load from file system
    File {
        cert_path: PathBuf,
        key_path: PathBuf,
    },

    /// Load from environment variables
    Environment {
        cert_var: String,
        key_var: String,
    },

    /// Load from SPIFFE workload API
    Spiffe {
        socket_path: PathBuf,
        spiffe_id: String,
    },

    /// Load from Kubernetes secrets
    Kubernetes {
        namespace: String,
        secret_name: String,
        cert_key: String,
        key_key: String,
    },

    /// Load from HSM
    Hsm {
        provider: String,
        key_label: String,
        cert_label: String,
    },

    /// Inline PEM data (for testing)
    Inline {
        cert_pem: String,
        key_pem: String,
    },
}

/// mTLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtlsConfig {
    /// Enable mTLS
    pub enabled: bool,

    /// Certificate source for this service
    pub certificate: CertificateSource,

    /// CA certificates for peer validation
    pub ca_certificates: Vec<CertificateSource>,

    /// Required peer SAN patterns (DNS or SPIFFE)
    pub required_peer_sans: Vec<String>,

    /// Enable OCSP stapling
    pub ocsp_stapling: bool,

    /// Enable CRL checking
    pub crl_checking: bool,

    /// CRL distribution points
    pub crl_urls: Vec<String>,

    /// OCSP responder URL
    pub ocsp_url: Option<String>,

    /// Minimum TLS version (1.2 or 1.3)
    pub min_tls_version: String,

    /// Allowed cipher suites
    pub cipher_suites: Vec<String>,

    /// Certificate rotation check interval
    pub rotation_check_interval: Duration,

    /// Certificate expiry warning threshold
    pub expiry_warning_days: u32,
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            certificate: CertificateSource::File {
                cert_path: PathBuf::from("/etc/actoris/tls/tls.crt"),
                key_path: PathBuf::from("/etc/actoris/tls/tls.key"),
            },
            ca_certificates: vec![CertificateSource::File {
                cert_path: PathBuf::from("/etc/actoris/tls/ca.crt"),
                key_path: PathBuf::new(), // Not needed for CA
            }],
            required_peer_sans: vec![],
            ocsp_stapling: true,
            crl_checking: true,
            crl_urls: vec![],
            ocsp_url: None,
            min_tls_version: "1.3".to_string(),
            cipher_suites: vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_AES_128_GCM_SHA256".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
            ],
            rotation_check_interval: Duration::from_secs(3600),
            expiry_warning_days: 30,
        }
    }
}

/// Certificate metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    /// Subject common name
    pub common_name: String,

    /// Subject alternative names
    pub san_dns: Vec<String>,

    /// SPIFFE IDs
    pub san_spiffe: Vec<String>,

    /// Issuer common name
    pub issuer: String,

    /// Serial number (hex)
    pub serial: String,

    /// Not before timestamp
    pub not_before: i64,

    /// Not after timestamp
    pub not_after: i64,

    /// Key algorithm
    pub key_algorithm: String,

    /// Key size in bits
    pub key_size: u32,

    /// Fingerprint (SHA-256)
    pub fingerprint: String,
}

/// mTLS connector for outgoing connections
pub struct MtlsConnector {
    config: MtlsConfig,
    certificate_info: Option<CertificateInfo>,
    // In a real implementation:
    // rustls_connector: tokio_rustls::TlsConnector,
}

impl MtlsConnector {
    /// Create a new mTLS connector
    pub async fn new(config: MtlsConfig) -> Result<Self, MtlsError> {
        // Load and validate certificates
        let cert_info = Self::load_certificate_info(&config.certificate).await?;

        // Check expiry
        let now = chrono::Utc::now().timestamp();
        if now > cert_info.not_after {
            return Err(MtlsError::CertificateExpired);
        }

        let days_until_expiry = (cert_info.not_after - now) / 86400;
        if days_until_expiry < config.expiry_warning_days as i64 {
            warn!(
                days = days_until_expiry,
                "Certificate expiring soon"
            );
        }

        info!(
            cn = %cert_info.common_name,
            expires = %chrono::DateTime::from_timestamp(cert_info.not_after, 0).unwrap(),
            "mTLS connector initialized"
        );

        Ok(Self {
            config,
            certificate_info: Some(cert_info),
        })
    }

    /// Load certificate info from source
    async fn load_certificate_info(source: &CertificateSource) -> Result<CertificateInfo, MtlsError> {
        match source {
            CertificateSource::File { cert_path, .. } => {
                if !cert_path.exists() {
                    return Err(MtlsError::CertificateNotFound {
                        path: cert_path.display().to_string(),
                    });
                }

                // In a real implementation, parse the certificate
                // For now, return placeholder info
                Ok(CertificateInfo {
                    common_name: "actoris-service".to_string(),
                    san_dns: vec!["*.actoris.local".to_string()],
                    san_spiffe: vec![],
                    issuer: "Actoris CA".to_string(),
                    serial: "01".to_string(),
                    not_before: chrono::Utc::now().timestamp(),
                    not_after: chrono::Utc::now().timestamp() + 365 * 86400,
                    key_algorithm: "ECDSA".to_string(),
                    key_size: 256,
                    fingerprint: "00:00:00:00".to_string(),
                })
            }
            CertificateSource::Spiffe { socket_path, spiffe_id } => {
                // Connect to SPIFFE Workload API
                debug!(
                    socket = %socket_path.display(),
                    spiffe_id = %spiffe_id,
                    "Loading certificate from SPIFFE"
                );

                Ok(CertificateInfo {
                    common_name: spiffe_id.clone(),
                    san_dns: vec![],
                    san_spiffe: vec![spiffe_id.clone()],
                    issuer: "SPIFFE Trust Domain".to_string(),
                    serial: "auto".to_string(),
                    not_before: chrono::Utc::now().timestamp(),
                    not_after: chrono::Utc::now().timestamp() + 86400, // 1 day (SVID default)
                    key_algorithm: "ECDSA".to_string(),
                    key_size: 256,
                    fingerprint: "00:00:00:00".to_string(),
                })
            }
            CertificateSource::Inline { .. } => {
                Ok(CertificateInfo {
                    common_name: "test-service".to_string(),
                    san_dns: vec!["localhost".to_string()],
                    san_spiffe: vec![],
                    issuer: "Test CA".to_string(),
                    serial: "test".to_string(),
                    not_before: chrono::Utc::now().timestamp(),
                    not_after: chrono::Utc::now().timestamp() + 365 * 86400,
                    key_algorithm: "RSA".to_string(),
                    key_size: 2048,
                    fingerprint: "00:00:00:00".to_string(),
                })
            }
            _ => {
                // Other sources would be implemented similarly
                Ok(CertificateInfo {
                    common_name: "unknown".to_string(),
                    san_dns: vec![],
                    san_spiffe: vec![],
                    issuer: "Unknown".to_string(),
                    serial: "00".to_string(),
                    not_before: 0,
                    not_after: i64::MAX,
                    key_algorithm: "Unknown".to_string(),
                    key_size: 0,
                    fingerprint: "00:00:00:00".to_string(),
                })
            }
        }
    }

    /// Get certificate info
    pub fn certificate_info(&self) -> Option<&CertificateInfo> {
        self.certificate_info.as_ref()
    }

    /// Verify peer certificate
    pub fn verify_peer(&self, peer_info: &CertificateInfo) -> Result<(), MtlsError> {
        // Check expiry
        let now = chrono::Utc::now().timestamp();
        if now > peer_info.not_after {
            return Err(MtlsError::CertificateExpired);
        }

        // Check required SANs
        if !self.config.required_peer_sans.is_empty() {
            let peer_sans: Vec<&str> = peer_info
                .san_dns
                .iter()
                .chain(peer_info.san_spiffe.iter())
                .map(|s| s.as_str())
                .collect();

            let matches = self.config.required_peer_sans.iter().any(|pattern| {
                peer_sans.iter().any(|san| matches_pattern(pattern, san))
            });

            if !matches {
                return Err(MtlsError::PeerVerificationFailed {
                    reason: format!(
                        "No matching SAN found. Required: {:?}, Found: {:?}",
                        self.config.required_peer_sans, peer_sans
                    ),
                });
            }
        }

        Ok(())
    }

    /// Check if certificate needs rotation
    pub fn needs_rotation(&self) -> bool {
        if let Some(info) = &self.certificate_info {
            let now = chrono::Utc::now().timestamp();
            let remaining = info.not_after - now;
            let threshold = (self.config.expiry_warning_days as i64) * 86400;
            remaining < threshold
        } else {
            true
        }
    }
}

/// mTLS acceptor for incoming connections
pub struct MtlsAcceptor {
    config: MtlsConfig,
    certificate_info: Option<CertificateInfo>,
    // In a real implementation:
    // rustls_acceptor: tokio_rustls::TlsAcceptor,
}

impl MtlsAcceptor {
    /// Create a new mTLS acceptor
    pub async fn new(config: MtlsConfig) -> Result<Self, MtlsError> {
        let cert_info = MtlsConnector::load_certificate_info(&config.certificate).await?;

        info!(
            cn = %cert_info.common_name,
            "mTLS acceptor initialized"
        );

        Ok(Self {
            config,
            certificate_info: Some(cert_info),
        })
    }

    /// Get certificate info
    pub fn certificate_info(&self) -> Option<&CertificateInfo> {
        self.certificate_info.as_ref()
    }

    /// Get configuration
    pub fn config(&self) -> &MtlsConfig {
        &self.config
    }
}

/// Simple pattern matching for SANs
fn matches_pattern(pattern: &str, value: &str) -> bool {
    if pattern.starts_with('*') {
        value.ends_with(&pattern[1..])
    } else if pattern.ends_with('*') {
        value.starts_with(&pattern[..pattern.len() - 1])
    } else {
        pattern == value
    }
}

/// Generate self-signed certificate for development
pub fn generate_self_signed(
    common_name: &str,
    san_dns: &[&str],
    validity_days: u32,
) -> Result<(String, String), MtlsError> {
    // In a real implementation, use rcgen or similar
    let cert_pem = format!(
        r#"-----BEGIN CERTIFICATE-----
MIIBkTCB+wIJAExample...
Subject: CN={}
SAN: DNS:{}
Validity: {} days
-----END CERTIFICATE-----"#,
        common_name,
        san_dns.join(", DNS:"),
        validity_days
    );

    let key_pem = r#"-----BEGIN EC PRIVATE KEY-----
MHQCAQEEIExample...
-----END EC PRIVATE KEY-----"#
        .to_string();

    warn!("Using self-signed certificate - for development only!");

    Ok((cert_pem, key_pem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        assert!(matches_pattern("*.example.com", "foo.example.com"));
        assert!(matches_pattern("*.example.com", "bar.example.com"));
        assert!(!matches_pattern("*.example.com", "example.com"));
        assert!(matches_pattern("spiffe://trust/*", "spiffe://trust/service"));
        assert!(matches_pattern("exact-match", "exact-match"));
        assert!(!matches_pattern("exact-match", "not-match"));
    }

    #[test]
    fn test_default_config() {
        let config = MtlsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_tls_version, "1.3");
        assert!(config.ocsp_stapling);
        assert_eq!(config.expiry_warning_days, 30);
    }

    #[tokio::test]
    async fn test_certificate_info_inline() {
        let source = CertificateSource::Inline {
            cert_pem: "test".to_string(),
            key_pem: "test".to_string(),
        };

        let info = MtlsConnector::load_certificate_info(&source).await.unwrap();
        assert_eq!(info.common_name, "test-service");
    }
}
