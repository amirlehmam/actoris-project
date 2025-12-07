//! PRAXIS configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// PRAXIS service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PraxisConfig {
    /// Service host
    pub host: String,
    /// Service port
    pub port: u16,
    /// Retrieval configuration
    pub retrieval: RetrievalSettings,
    /// Storage configuration
    pub storage: StorageSettings,
    /// Embedding configuration
    pub embedding: EmbeddingSettings,
}

impl Default for PraxisConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8084,
            retrieval: RetrievalSettings::default(),
            storage: StorageSettings::default(),
            embedding: EmbeddingSettings::default(),
        }
    }
}

impl PraxisConfig {
    /// Load configuration from environment and files
    pub fn load() -> Result<Self> {
        // Try to load .env file
        let _ = dotenvy::dotenv();

        let mut cfg = Self::default();

        // Check for Railway's PORT env variable first (takes priority)
        if let Ok(port) = std::env::var("PORT") {
            if let Ok(p) = port.parse::<u16>() {
                cfg.port = p;
            }
        }

        // Then check for PRAXIS_ prefixed variables
        if let Ok(host) = std::env::var("PRAXIS_HOST") {
            cfg.host = host;
        }
        if let Ok(port) = std::env::var("PRAXIS_PORT") {
            if let Ok(p) = port.parse::<u16>() {
                cfg.port = p;
            }
        }

        // Retrieval settings
        if let Ok(val) = std::env::var("PRAXIS_RETRIEVAL_SEARCH_BREADTH") {
            if let Ok(v) = val.parse() {
                cfg.retrieval.search_breadth = v;
            }
        }
        if let Ok(val) = std::env::var("PRAXIS_RETRIEVAL_SIMILARITY_THRESHOLD") {
            if let Ok(v) = val.parse() {
                cfg.retrieval.similarity_threshold = v;
            }
        }

        // Storage settings
        if let Ok(val) = std::env::var("PRAXIS_STORAGE_MAX_MEMORIES_PER_AGENT") {
            if let Ok(v) = val.parse() {
                cfg.storage.max_memories_per_agent = v;
            }
        }

        Ok(cfg)
    }
}

/// Retrieval algorithm settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalSettings {
    /// Top-k candidates from environmental matching (k in Algorithm 1)
    pub search_breadth: usize,
    /// Minimum similarity threshold (τ in Algorithm 1)
    pub similarity_threshold: f32,
    /// Weight for IoU component in environmental matching
    pub iou_weight: f32,
    /// Weight for length overlap in environmental matching
    pub length_weight: f32,
    /// Whether to apply time decay to relevance scores
    pub enable_time_decay: bool,
    /// Half-life for time decay in days
    pub decay_half_life_days: f64,
}

impl Default for RetrievalSettings {
    fn default() -> Self {
        Self {
            search_breadth: crate::DEFAULT_RETRIEVAL_BREADTH,
            similarity_threshold: crate::DEFAULT_SIMILARITY_THRESHOLD,
            iou_weight: 1.0,
            length_weight: 1.0,
            enable_time_decay: true,
            decay_half_life_days: crate::MEMORY_DECAY_HALF_LIFE_DAYS,
        }
    }
}

/// Storage settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    /// Maximum memories per agent
    pub max_memories_per_agent: usize,
    /// EventStoreDB connection URL (optional)
    pub eventstore_url: Option<String>,
    /// Redis URL for caching (optional)
    pub redis_url: Option<String>,
    /// Enable persistence to EventStoreDB
    pub enable_persistence: bool,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            max_memories_per_agent: crate::MAX_MEMORIES_PER_AGENT,
            eventstore_url: None,
            redis_url: None,
            enable_persistence: false,
        }
    }
}

/// Embedding service settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingSettings {
    /// Embedding model endpoint (e.g., OpenAI, local model)
    pub endpoint: Option<String>,
    /// Embedding dimension
    pub dimension: usize,
    /// Whether to cache embeddings
    pub enable_cache: bool,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
}

impl Default for EmbeddingSettings {
    fn default() -> Self {
        Self {
            endpoint: None,
            dimension: 1536, // OpenAI ada-002 dimension
            enable_cache: true,
            cache_ttl_secs: 3600,
        }
    }
}
