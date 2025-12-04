//! DID (Decentralized Identifier) Operations
//!
//! Implements W3C DID standard operations for:
//! - did:key - Self-sovereign agent identities (Ed25519)
//! - did:web - Organization identities (domain-backed)
//!
//! Reference: https://www.w3.org/TR/did-core/

use ed25519_dalek::{Signature, SigningKey, VerifyingKey, Signer, Verifier};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Ed25519 multicodec prefix for did:key
const ED25519_MULTICODEC: [u8; 2] = [0xed, 0x01];

/// DID method types supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DidMethod {
    /// Self-sovereign identity using Ed25519 key
    Key,
    /// Domain-backed identity
    Web,
}

impl std::fmt::Display for DidMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DidMethod::Key => write!(f, "key"),
            DidMethod::Web => write!(f, "web"),
        }
    }
}

/// Parsed DID components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDid {
    /// Full DID string
    pub did: String,
    /// DID method
    pub method: DidMethod,
    /// Method-specific identifier
    pub identifier: String,
}

impl ParsedDid {
    /// Parse a DID string into components
    pub fn parse(did: &str) -> Result<Self, DidError> {
        if !did.starts_with("did:") {
            return Err(DidError::InvalidFormat("DID must start with 'did:'".into()));
        }

        let parts: Vec<&str> = did.splitn(3, ':').collect();
        if parts.len() < 3 {
            return Err(DidError::InvalidFormat("DID must have method and identifier".into()));
        }

        let method = match parts[1] {
            "key" => DidMethod::Key,
            "web" => DidMethod::Web,
            other => return Err(DidError::UnsupportedMethod(other.to_string())),
        };

        Ok(Self {
            did: did.to_string(),
            method,
            identifier: parts[2].to_string(),
        })
    }

    /// Extract Ed25519 public key from did:key
    pub fn extract_public_key(&self) -> Result<[u8; 32], DidError> {
        if self.method != DidMethod::Key {
            return Err(DidError::WrongMethod {
                expected: DidMethod::Key,
                actual: self.method,
            });
        }

        decode_did_key(&self.identifier)
    }

    /// Get verifying key for signature verification
    pub fn verifying_key(&self) -> Result<VerifyingKey, DidError> {
        let key_bytes = self.extract_public_key()?;
        VerifyingKey::from_bytes(&key_bytes).map_err(|_| DidError::InvalidPublicKey)
    }
}

/// Encode an Ed25519 public key as did:key identifier
pub fn encode_did_key(public_key: &[u8; 32]) -> String {
    let mut prefixed = Vec::with_capacity(34);
    prefixed.extend_from_slice(&ED25519_MULTICODEC);
    prefixed.extend_from_slice(public_key);

    let encoded = bs58::encode(&prefixed).into_string();
    format!("did:key:z{}", encoded)
}

/// Decode a did:key identifier to Ed25519 public key
pub fn decode_did_key(identifier: &str) -> Result<[u8; 32], DidError> {
    // Handle z-prefixed multibase
    let encoded = if identifier.starts_with('z') {
        &identifier[1..]
    } else {
        identifier
    };

    let decoded = bs58::decode(encoded)
        .into_vec()
        .map_err(|_| DidError::InvalidEncoding)?;

    // Check multicodec prefix
    if decoded.len() < 34 || decoded[0] != ED25519_MULTICODEC[0] || decoded[1] != ED25519_MULTICODEC[1] {
        return Err(DidError::InvalidMulticodec);
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&decoded[2..34]);
    Ok(key)
}

/// Create a did:web identifier
pub fn create_did_web(domain: &str, path: Option<&str>) -> String {
    // Encode special characters
    let encoded_domain = domain.replace(':', "%3A");

    match path {
        Some(p) => format!("did:web:{}:{}", encoded_domain, p.replace('/', ":")),
        None => format!("did:web:{}", encoded_domain),
    }
}

/// DID Document (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    /// The DID subject
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: String,
    pub verification_method: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
}

/// Verification method in DID Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub method_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
}

impl DidDocument {
    /// Create a DID Document for a did:key
    pub fn for_did_key(public_key: &[u8; 32]) -> Self {
        let did = encode_did_key(public_key);
        let vm_id = format!("{}#{}", did, &did[8..]);

        // Encode public key as multibase (z = base58btc)
        let mut prefixed = Vec::with_capacity(34);
        prefixed.extend_from_slice(&ED25519_MULTICODEC);
        prefixed.extend_from_slice(public_key);
        let multibase = format!("z{}", bs58::encode(&prefixed).into_string());

        Self {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: did.clone(),
            verification_method: vec![VerificationMethod {
                id: vm_id.clone(),
                method_type: "Ed25519VerificationKey2020".to_string(),
                controller: did,
                public_key_multibase: multibase,
            }],
            authentication: vec![vm_id],
        }
    }
}

/// DID operation errors
#[derive(Debug, Error)]
pub enum DidError {
    #[error("Invalid DID format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported DID method: {0}")]
    UnsupportedMethod(String),

    #[error("Wrong DID method: expected {expected}, got {actual}")]
    WrongMethod { expected: DidMethod, actual: DidMethod },

    #[error("Invalid encoding")]
    InvalidEncoding,

    #[error("Invalid multicodec prefix")]
    InvalidMulticodec,

    #[error("Invalid public key")]
    InvalidPublicKey,

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Resolution failed: {0}")]
    ResolutionFailed(String),
}

/// Sign data using a signing key, returning signature bytes
pub fn sign_with_key(signing_key: &SigningKey, message: &[u8]) -> [u8; 64] {
    let signature = signing_key.sign(message);
    signature.to_bytes()
}

/// Verify signature using a DID
pub fn verify_with_did(did: &str, message: &[u8], signature: &[u8; 64]) -> Result<bool, DidError> {
    let parsed = ParsedDid::parse(did)?;
    let verifying_key = parsed.verifying_key()?;

    let sig = Signature::from_bytes(signature);

    match verifying_key.verify(message, &sig) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn test_did_key_roundtrip() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key = signing_key.verifying_key().to_bytes();

        let did = encode_did_key(&public_key);
        assert!(did.starts_with("did:key:z"));

        let decoded = decode_did_key(&did[8..]).unwrap(); // Skip "did:key:"
        assert_eq!(decoded, public_key);
    }

    #[test]
    fn test_parse_did_key() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key = signing_key.verifying_key().to_bytes();
        let did = encode_did_key(&public_key);

        let parsed = ParsedDid::parse(&did).unwrap();
        assert_eq!(parsed.method, DidMethod::Key);
        assert_eq!(parsed.extract_public_key().unwrap(), public_key);
    }

    #[test]
    fn test_did_web() {
        let did = create_did_web("example.com", None);
        assert_eq!(did, "did:web:example.com");

        let did_path = create_did_web("example.com", Some("users/alice"));
        assert_eq!(did_path, "did:web:example.com:users:alice");
    }

    #[test]
    fn test_sign_and_verify() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key = signing_key.verifying_key().to_bytes();
        let did = encode_did_key(&public_key);

        let message = b"test message";
        let signature = sign_with_key(&signing_key, message);

        assert!(verify_with_did(&did, message, &signature).unwrap());

        // Wrong message should fail
        assert!(!verify_with_did(&did, b"wrong message", &signature).unwrap());
    }

    #[test]
    fn test_did_document() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key = signing_key.verifying_key().to_bytes();

        let doc = DidDocument::for_did_key(&public_key);
        assert!(doc.id.starts_with("did:key:z"));
        assert!(!doc.verification_method.is_empty());

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&doc).unwrap();
        assert!(json.contains("Ed25519VerificationKey2020"));
    }

    #[test]
    fn test_invalid_did() {
        assert!(ParsedDid::parse("not-a-did").is_err());
        assert!(ParsedDid::parse("did:unknown:123").is_err());
    }
}
