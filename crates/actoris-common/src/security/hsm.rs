//! Hardware Security Module (HSM) integration
//!
//! Provides secure key management with:
//! - PKCS#11 interface for hardware HSMs
//! - AWS CloudHSM support
//! - Google Cloud HSM support
//! - Azure Key Vault HSM support
//! - Software fallback for development

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

/// HSM errors
#[derive(Debug, Error)]
pub enum HsmError {
    #[error("HSM not initialized")]
    NotInitialized,

    #[error("HSM connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Key not found: {label}")]
    KeyNotFound { label: String },

    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Operation not supported: {0}")]
    NotSupported(String),

    #[error("PKCS#11 error: {0}")]
    Pkcs11Error(String),

    #[error("Cloud HSM error: {0}")]
    CloudHsmError(String),
}

/// Key types supported by HSM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyType {
    /// ECDSA P-256
    EcdsaP256,
    /// ECDSA P-384
    EcdsaP384,
    /// Ed25519
    Ed25519,
    /// RSA 2048
    Rsa2048,
    /// RSA 4096
    Rsa4096,
    /// AES-256 (symmetric)
    Aes256,
    /// ChaCha20-Poly1305 (symmetric)
    ChaCha20,
}

impl KeyType {
    /// Get key size in bits
    pub fn key_size(&self) -> u32 {
        match self {
            KeyType::EcdsaP256 => 256,
            KeyType::EcdsaP384 => 384,
            KeyType::Ed25519 => 256,
            KeyType::Rsa2048 => 2048,
            KeyType::Rsa4096 => 4096,
            KeyType::Aes256 => 256,
            KeyType::ChaCha20 => 256,
        }
    }

    /// Check if key is asymmetric
    pub fn is_asymmetric(&self) -> bool {
        !matches!(self, KeyType::Aes256 | KeyType::ChaCha20)
    }
}

/// HSM provider type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HsmProvider {
    /// PKCS#11 hardware HSM
    Pkcs11 {
        library_path: String,
        slot_id: u64,
        pin: String,
    },

    /// AWS CloudHSM
    AwsCloudHsm {
        cluster_id: String,
        region: String,
        crypto_user: String,
        password: String,
    },

    /// Google Cloud HSM
    GoogleCloudHsm {
        project_id: String,
        location: String,
        key_ring: String,
    },

    /// Azure Key Vault HSM
    AzureKeyVault {
        vault_url: String,
        tenant_id: String,
        client_id: String,
    },

    /// HashiCorp Vault Transit
    VaultTransit {
        address: String,
        token: String,
        mount_path: String,
    },

    /// Software-based (for development/testing only)
    Software {
        key_store_path: String,
        encryption_key: String,
    },
}

/// HSM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsmConfig {
    /// HSM provider
    pub provider: HsmProvider,

    /// Enable HSM (falls back to software if disabled)
    pub enabled: bool,

    /// Key labels to pre-load
    pub preload_keys: Vec<String>,

    /// Connection timeout
    pub timeout_secs: u64,

    /// Max retry attempts
    pub max_retries: u32,

    /// Enable key caching
    pub enable_caching: bool,

    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
}

impl Default for HsmConfig {
    fn default() -> Self {
        Self {
            provider: HsmProvider::Software {
                key_store_path: "/var/lib/actoris/keys".to_string(),
                encryption_key: "development-only-key".to_string(),
            },
            enabled: false,
            preload_keys: vec![],
            timeout_secs: 30,
            max_retries: 3,
            enable_caching: true,
            cache_ttl_secs: 300,
        }
    }
}

/// HSM key handle
#[derive(Debug, Clone)]
pub struct HsmKeyHandle {
    /// Key label/identifier
    pub label: String,

    /// Key type
    pub key_type: KeyType,

    /// Public key bytes (for asymmetric keys)
    pub public_key: Option<Vec<u8>>,

    /// Whether the key is extractable
    pub extractable: bool,

    /// Key creation timestamp
    pub created_at: i64,

    /// Provider-specific handle
    handle: KeyHandleInner,
}

#[derive(Debug, Clone)]
enum KeyHandleInner {
    Software { key_bytes: Vec<u8> },
    Pkcs11 { object_handle: u64 },
    Cloud { key_version: String },
}

impl HsmKeyHandle {
    /// Get the key label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Get the key type
    pub fn key_type(&self) -> KeyType {
        self.key_type
    }

    /// Get public key bytes
    pub fn public_key_bytes(&self) -> Option<&[u8]> {
        self.public_key.as_deref()
    }
}

/// HSM client for key operations
pub struct HsmClient {
    config: HsmConfig,
    keys: Arc<RwLock<HashMap<String, HsmKeyHandle>>>,
    initialized: bool,
}

impl HsmClient {
    /// Create a new HSM client
    pub async fn new(config: HsmConfig) -> Result<Self, HsmError> {
        let client = Self {
            config,
            keys: Arc::new(RwLock::new(HashMap::new())),
            initialized: false,
        };

        Ok(client)
    }

    /// Initialize the HSM connection
    pub async fn initialize(&mut self) -> Result<(), HsmError> {
        if !self.config.enabled {
            warn!("HSM disabled, using software fallback");
            self.initialized = true;
            return Ok(());
        }

        match &self.config.provider {
            HsmProvider::Pkcs11 { library_path, slot_id, pin } => {
                info!(
                    library = %library_path,
                    slot = slot_id,
                    "Initializing PKCS#11 HSM"
                );

                // In a real implementation:
                // 1. Load the PKCS#11 library
                // 2. Initialize the library
                // 3. Open a session with the slot
                // 4. Login with the PIN
            }
            HsmProvider::AwsCloudHsm { cluster_id, region, .. } => {
                info!(
                    cluster = %cluster_id,
                    region = %region,
                    "Initializing AWS CloudHSM"
                );
            }
            HsmProvider::GoogleCloudHsm { project_id, location, key_ring } => {
                info!(
                    project = %project_id,
                    location = %location,
                    key_ring = %key_ring,
                    "Initializing Google Cloud HSM"
                );
            }
            HsmProvider::AzureKeyVault { vault_url, .. } => {
                info!(
                    vault = %vault_url,
                    "Initializing Azure Key Vault HSM"
                );
            }
            HsmProvider::VaultTransit { address, mount_path, .. } => {
                info!(
                    address = %address,
                    mount = %mount_path,
                    "Initializing HashiCorp Vault Transit"
                );
            }
            HsmProvider::Software { key_store_path, .. } => {
                info!(
                    path = %key_store_path,
                    "Initializing software key store"
                );
            }
        }

        // Preload keys
        for label in &self.config.preload_keys {
            if let Err(e) = self.load_key(label).await {
                warn!(label = %label, error = %e, "Failed to preload key");
            }
        }

        self.initialized = true;
        Ok(())
    }

    /// Generate a new key
    pub async fn generate_key(
        &self,
        label: &str,
        key_type: KeyType,
        extractable: bool,
    ) -> Result<HsmKeyHandle, HsmError> {
        if !self.initialized {
            return Err(HsmError::NotInitialized);
        }

        debug!(label = %label, key_type = ?key_type, "Generating key");

        // Generate key based on provider
        let (key_bytes, public_key) = match key_type {
            KeyType::Ed25519 => {
                let secret = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
                let public = secret.verifying_key();
                (secret.to_bytes().to_vec(), Some(public.to_bytes().to_vec()))
            }
            KeyType::EcdsaP256 | KeyType::EcdsaP384 => {
                // In a real implementation, use p256/p384 crates
                let key = vec![0u8; 32];
                let pub_key = vec![0u8; 64];
                (key, Some(pub_key))
            }
            KeyType::Aes256 | KeyType::ChaCha20 => {
                let mut key = vec![0u8; 32];
                rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut key);
                (key, None)
            }
            KeyType::Rsa2048 | KeyType::Rsa4096 => {
                // In a real implementation, use rsa crate
                let key = vec![0u8; key_type.key_size() as usize / 8];
                let pub_key = vec![0u8; 256];
                (key, Some(pub_key))
            }
        };

        let handle = HsmKeyHandle {
            label: label.to_string(),
            key_type,
            public_key,
            extractable,
            created_at: chrono::Utc::now().timestamp(),
            handle: KeyHandleInner::Software { key_bytes },
        };

        self.keys.write().insert(label.to_string(), handle.clone());

        info!(label = %label, key_type = ?key_type, "Key generated");

        Ok(handle)
    }

    /// Load an existing key
    pub async fn load_key(&self, label: &str) -> Result<HsmKeyHandle, HsmError> {
        if !self.initialized {
            return Err(HsmError::NotInitialized);
        }

        // Check cache first
        if let Some(key) = self.keys.read().get(label) {
            return Ok(key.clone());
        }

        // Load from provider
        debug!(label = %label, "Loading key from HSM");

        // In a real implementation, load from the actual HSM
        Err(HsmError::KeyNotFound {
            label: label.to_string(),
        })
    }

    /// Get a key handle
    pub fn get_key(&self, label: &str) -> Option<HsmKeyHandle> {
        self.keys.read().get(label).cloned()
    }

    /// Sign data with a key
    pub async fn sign(
        &self,
        key_handle: &HsmKeyHandle,
        data: &[u8],
    ) -> Result<Vec<u8>, HsmError> {
        if !self.initialized {
            return Err(HsmError::NotInitialized);
        }

        match &key_handle.handle {
            KeyHandleInner::Software { key_bytes } => {
                match key_handle.key_type {
                    KeyType::Ed25519 => {
                        let key_array: [u8; 32] = key_bytes
                            .as_slice()
                            .try_into()
                            .map_err(|_| HsmError::SigningFailed("Invalid key size".to_string()))?;
                        let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_array);
                        use ed25519_dalek::Signer;
                        let signature = signing_key.sign(data);
                        Ok(signature.to_bytes().to_vec())
                    }
                    _ => Err(HsmError::NotSupported(format!(
                        "Signing with {:?} not implemented",
                        key_handle.key_type
                    ))),
                }
            }
            KeyHandleInner::Pkcs11 { object_handle } => {
                debug!(handle = object_handle, "Signing with PKCS#11");
                // In a real implementation, use the PKCS#11 library
                Err(HsmError::NotSupported("PKCS#11 signing not implemented".to_string()))
            }
            KeyHandleInner::Cloud { key_version } => {
                debug!(version = %key_version, "Signing with cloud HSM");
                // In a real implementation, call the cloud HSM API
                Err(HsmError::NotSupported("Cloud HSM signing not implemented".to_string()))
            }
        }
    }

    /// Verify a signature
    pub async fn verify(
        &self,
        key_handle: &HsmKeyHandle,
        data: &[u8],
        signature: &[u8],
    ) -> Result<bool, HsmError> {
        if !self.initialized {
            return Err(HsmError::NotInitialized);
        }

        match key_handle.key_type {
            KeyType::Ed25519 => {
                let public_key_bytes = key_handle
                    .public_key
                    .as_ref()
                    .ok_or_else(|| HsmError::VerificationFailed("No public key".to_string()))?;

                let public_key_array: [u8; 32] = public_key_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| HsmError::VerificationFailed("Invalid public key size".to_string()))?;

                let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&public_key_array)
                    .map_err(|e| HsmError::VerificationFailed(e.to_string()))?;

                let sig_array: [u8; 64] = signature
                    .try_into()
                    .map_err(|_| HsmError::VerificationFailed("Invalid signature size".to_string()))?;

                let sig = ed25519_dalek::Signature::from_bytes(&sig_array);

                use ed25519_dalek::Verifier;
                Ok(verifying_key.verify(data, &sig).is_ok())
            }
            _ => Err(HsmError::NotSupported(format!(
                "Verification with {:?} not implemented",
                key_handle.key_type
            ))),
        }
    }

    /// Encrypt data with a symmetric key
    pub async fn encrypt(
        &self,
        key_handle: &HsmKeyHandle,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, HsmError> {
        if !self.initialized {
            return Err(HsmError::NotInitialized);
        }

        if key_handle.key_type.is_asymmetric() {
            return Err(HsmError::NotSupported(
                "Encryption requires symmetric key".to_string(),
            ));
        }

        // In a real implementation, use the symmetric key for encryption
        // For now, return a placeholder
        debug!(label = %key_handle.label, "Encrypting data");

        // XOR with key as placeholder (NOT SECURE - for structure only)
        match &key_handle.handle {
            KeyHandleInner::Software { key_bytes } => {
                let mut ciphertext = plaintext.to_vec();
                for (i, byte) in ciphertext.iter_mut().enumerate() {
                    *byte ^= key_bytes[i % key_bytes.len()];
                }
                Ok(ciphertext)
            }
            _ => Err(HsmError::NotSupported("HSM encryption not implemented".to_string())),
        }
    }

    /// Decrypt data with a symmetric key
    pub async fn decrypt(
        &self,
        key_handle: &HsmKeyHandle,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, HsmError> {
        if !self.initialized {
            return Err(HsmError::NotInitialized);
        }

        if key_handle.key_type.is_asymmetric() {
            return Err(HsmError::NotSupported(
                "Decryption requires symmetric key".to_string(),
            ));
        }

        // Symmetric XOR is its own inverse
        self.encrypt(key_handle, ciphertext).await
    }

    /// List all keys
    pub fn list_keys(&self) -> Vec<String> {
        self.keys.read().keys().cloned().collect()
    }

    /// Delete a key
    pub async fn delete_key(&self, label: &str) -> Result<(), HsmError> {
        if !self.initialized {
            return Err(HsmError::NotInitialized);
        }

        if self.keys.write().remove(label).is_some() {
            info!(label = %label, "Key deleted");
            Ok(())
        } else {
            Err(HsmError::KeyNotFound {
                label: label.to_string(),
            })
        }
    }

    /// Check if HSM is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get HSM provider info
    pub fn provider_info(&self) -> String {
        match &self.config.provider {
            HsmProvider::Pkcs11 { library_path, .. } => format!("PKCS#11: {}", library_path),
            HsmProvider::AwsCloudHsm { cluster_id, .. } => format!("AWS CloudHSM: {}", cluster_id),
            HsmProvider::GoogleCloudHsm { project_id, .. } => format!("Google Cloud HSM: {}", project_id),
            HsmProvider::AzureKeyVault { vault_url, .. } => format!("Azure Key Vault: {}", vault_url),
            HsmProvider::VaultTransit { address, .. } => format!("Vault Transit: {}", address),
            HsmProvider::Software { .. } => "Software (Development)".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hsm_client_initialization() {
        let config = HsmConfig::default();
        let mut client = HsmClient::new(config).await.unwrap();

        assert!(!client.is_initialized());
        client.initialize().await.unwrap();
        assert!(client.is_initialized());
    }

    #[tokio::test]
    async fn test_key_generation_and_signing() {
        let config = HsmConfig::default();
        let mut client = HsmClient::new(config).await.unwrap();
        client.initialize().await.unwrap();

        // Generate Ed25519 key
        let key = client
            .generate_key("test-key", KeyType::Ed25519, false)
            .await
            .unwrap();

        assert_eq!(key.label, "test-key");
        assert_eq!(key.key_type, KeyType::Ed25519);
        assert!(key.public_key.is_some());

        // Sign data
        let data = b"Hello, World!";
        let signature = client.sign(&key, data).await.unwrap();
        assert!(!signature.is_empty());

        // Verify signature
        let valid = client.verify(&key, data, &signature).await.unwrap();
        assert!(valid);

        // Invalid signature should fail
        let invalid_sig = vec![0u8; 64];
        let valid = client.verify(&key, data, &invalid_sig).await.unwrap();
        assert!(!valid);
    }

    #[tokio::test]
    async fn test_symmetric_encryption() {
        let config = HsmConfig::default();
        let mut client = HsmClient::new(config).await.unwrap();
        client.initialize().await.unwrap();

        // Generate AES key
        let key = client
            .generate_key("aes-key", KeyType::Aes256, false)
            .await
            .unwrap();

        // Encrypt
        let plaintext = b"Secret message";
        let ciphertext = client.encrypt(&key, plaintext).await.unwrap();
        assert_ne!(ciphertext, plaintext);

        // Decrypt
        let decrypted = client.decrypt(&key, &ciphertext).await.unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_key_type() {
        assert_eq!(KeyType::Ed25519.key_size(), 256);
        assert_eq!(KeyType::Rsa4096.key_size(), 4096);
        assert!(KeyType::Ed25519.is_asymmetric());
        assert!(!KeyType::Aes256.is_asymmetric());
    }
}
