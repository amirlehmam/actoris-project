//! UnifiedID - DID-based identity for all Actoris entities
//!
//! Supports three entity types:
//! - Human: Real users, identified via did:web or did:key
//! - Agent: AI agents, always use did:key with Ed25519
//! - Organization: Companies/DAOs, typically use did:web

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// Entity type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// Human user
    Human,
    /// AI Agent
    Agent,
    /// Organization (company, DAO, etc.)
    Organization,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Human => write!(f, "human"),
            EntityType::Agent => write!(f, "agent"),
            EntityType::Organization => write!(f, "organization"),
        }
    }
}

/// W3C DID-based unified identity
///
/// Every entity in Actoris (humans, agents, organizations) has a UnifiedID.
/// This enables:
/// - Cryptographic authentication via DID verification
/// - Trust score tracking
/// - Lineage tracking for spawned agents
/// - HC wallet association
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedID {
    /// W3C DID string (did:key:z6Mk... for agents, did:web:... for orgs)
    pub did: String,

    /// Entity classification
    pub entity_type: EntityType,

    /// Parent DID for spawned agents (enables lineage tracking)
    /// Spawned agents inherit 30% of parent's trust score initially
    pub parent_did: Option<String>,

    /// Creation timestamp (Unix milliseconds)
    pub created_at: i64,

    /// Ed25519 public key bytes (32 bytes)
    pub public_key: [u8; 32],
}

impl UnifiedID {
    /// Generate a new UnifiedID for an agent using did:key method
    ///
    /// # Arguments
    /// * `parent` - Optional parent UnifiedID for spawned agents
    ///
    /// # Returns
    /// Tuple of (UnifiedID, SigningKey) - keep the signing key secure!
    ///
    /// # Example
    /// ```
    /// use actoris_common::types::unified_id::UnifiedID;
    ///
    /// let (agent_id, signing_key) = UnifiedID::new_agent(None);
    /// assert!(agent_id.did.starts_with("did:key:z6Mk"));
    /// ```
    pub fn new_agent(parent: Option<&UnifiedID>) -> (Self, SigningKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let public_key_bytes: [u8; 32] = verifying_key.to_bytes();

        // Create did:key using Ed25519 multicodec prefix (0xed01)
        let did = Self::encode_did_key(&public_key_bytes);

        let id = Self {
            did,
            entity_type: EntityType::Agent,
            parent_did: parent.map(|p| p.did.clone()),
            created_at: chrono::Utc::now().timestamp_millis(),
            public_key: public_key_bytes,
        };

        (id, signing_key)
    }

    /// Create a UnifiedID from an existing Ed25519 public key
    ///
    /// # Arguments
    /// * `public_key` - 32-byte Ed25519 public key
    /// * `entity_type` - Type of entity
    /// * `parent` - Optional parent DID
    pub fn from_public_key(
        public_key: [u8; 32],
        entity_type: EntityType,
        parent_did: Option<String>,
    ) -> Self {
        let did = Self::encode_did_key(&public_key);

        Self {
            did,
            entity_type,
            parent_did,
            created_at: chrono::Utc::now().timestamp_millis(),
            public_key,
        }
    }

    /// Create a human UnifiedID from a did:web identifier
    ///
    /// # Arguments
    /// * `domain` - The web domain (e.g., "example.com")
    /// * `public_key` - Ed25519 public key for authentication
    pub fn new_human_web(domain: &str, public_key: [u8; 32]) -> Self {
        Self {
            did: format!("did:web:{}", domain),
            entity_type: EntityType::Human,
            parent_did: None,
            created_at: chrono::Utc::now().timestamp_millis(),
            public_key,
        }
    }

    /// Create an organization UnifiedID
    ///
    /// # Arguments
    /// * `domain` - The organization's domain
    /// * `public_key` - Ed25519 public key
    pub fn new_organization(domain: &str, public_key: [u8; 32]) -> Self {
        Self {
            did: format!("did:web:{}", domain),
            entity_type: EntityType::Organization,
            parent_did: None,
            created_at: chrono::Utc::now().timestamp_millis(),
            public_key,
        }
    }

    /// Encode a public key as a did:key string
    ///
    /// Uses the Ed25519 multicodec prefix (0xed01) and base58btc encoding
    fn encode_did_key(public_key: &[u8; 32]) -> String {
        // Ed25519 multicodec prefix: 0xed 0x01
        let mut prefixed = vec![0xed, 0x01];
        prefixed.extend_from_slice(public_key);

        // Base58btc encode with 'z' prefix
        let encoded = bs58::encode(&prefixed).into_string();
        format!("did:key:z{}", encoded)
    }

    /// Decode a did:key string to extract the public key
    ///
    /// # Returns
    /// The 32-byte Ed25519 public key, or error if invalid
    pub fn decode_did_key(did: &str) -> Result<[u8; 32], UnifiedIdError> {
        if !did.starts_with("did:key:z") {
            return Err(UnifiedIdError::InvalidDidFormat);
        }

        let encoded = &did[9..]; // Skip "did:key:z"
        let decoded = bs58::decode(encoded)
            .into_vec()
            .map_err(|_| UnifiedIdError::InvalidBase58)?;

        // Check Ed25519 multicodec prefix
        if decoded.len() < 34 || decoded[0] != 0xed || decoded[1] != 0x01 {
            return Err(UnifiedIdError::InvalidMulticodec);
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded[2..34]);
        Ok(key)
    }

    /// Get the verifying key for signature verification
    pub fn verifying_key(&self) -> Result<VerifyingKey, UnifiedIdError> {
        VerifyingKey::from_bytes(&self.public_key).map_err(|_| UnifiedIdError::InvalidPublicKey)
    }

    /// Check if this entity is a spawned agent (has a parent)
    pub fn is_spawned(&self) -> bool {
        self.parent_did.is_some()
    }

    /// Calculate initial trust score for spawned agent (30% of parent)
    pub fn initial_trust_cap(&self, parent_trust: u16) -> u16 {
        ((parent_trust as f64) * 0.30) as u16
    }
}

impl std::fmt::Display for UnifiedID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.entity_type, &self.did[..20.min(self.did.len())])
    }
}

impl PartialEq for UnifiedID {
    fn eq(&self, other: &Self) -> bool {
        self.did == other.did
    }
}

impl Eq for UnifiedID {}

impl std::hash::Hash for UnifiedID {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.did.hash(state);
    }
}

/// Errors related to UnifiedID operations
#[derive(Debug, thiserror::Error)]
pub enum UnifiedIdError {
    #[error("Invalid DID format")]
    InvalidDidFormat,

    #[error("Invalid base58 encoding")]
    InvalidBase58,

    #[error("Invalid multicodec prefix")]
    InvalidMulticodec,

    #[error("Invalid public key")]
    InvalidPublicKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_agent() {
        let (agent, _signing_key) = UnifiedID::new_agent(None);

        assert!(agent.did.starts_with("did:key:z"));
        assert_eq!(agent.entity_type, EntityType::Agent);
        assert!(agent.parent_did.is_none());
        assert!(!agent.is_spawned());
    }

    #[test]
    fn test_spawned_agent() {
        let (parent, _) = UnifiedID::new_agent(None);
        let (child, _) = UnifiedID::new_agent(Some(&parent));

        assert!(child.is_spawned());
        assert_eq!(child.parent_did, Some(parent.did.clone()));
    }

    #[test]
    fn test_did_key_roundtrip() {
        let (agent, _) = UnifiedID::new_agent(None);
        let decoded = UnifiedID::decode_did_key(&agent.did).unwrap();
        assert_eq!(decoded, agent.public_key);
    }

    #[test]
    fn test_initial_trust_cap() {
        let (agent, _) = UnifiedID::new_agent(None);
        assert_eq!(agent.initial_trust_cap(1000), 300); // 30% of max
        assert_eq!(agent.initial_trust_cap(500), 150);
    }
}
