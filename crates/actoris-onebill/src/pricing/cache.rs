//! Redis pricing cache
//!
//! Caches pricing quotes to avoid redundant calculations and provide
//! consistent pricing for the same request within validity window.

use actoris_common::{ActorisError, PricingRequest, PricingResponse, Result};
use redis::{aio::MultiplexedConnection, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

/// Redis-based pricing cache
pub struct PricingCache {
    /// Redis client
    client: Client,
    /// Connection pool
    connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    /// Key prefix for cache entries
    prefix: String,
    /// Default TTL for cache entries
    default_ttl: Duration,
}

/// Cache key components
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct CacheKey {
    actor_did: String,
    action_type: String,
    compute_hc: String,
    trust_score: u32,
    task_complexity: u8,
    data_sensitivity: u8,
}

impl From<&PricingRequest> for CacheKey {
    fn from(req: &PricingRequest) -> Self {
        Self {
            actor_did: req.actor_did.clone(),
            action_type: req.action_type.clone(),
            compute_hc: req.compute_hc.to_string(),
            trust_score: req.trust_score,
            task_complexity: req.task_complexity.clone() as u8,
            data_sensitivity: req.data_sensitivity.clone() as u8,
        }
    }
}

impl CacheKey {
    fn to_redis_key(&self, prefix: &str) -> String {
        // Create a deterministic key from components
        let hash = blake3::hash(
            format!(
                "{}:{}:{}:{}:{}:{}",
                self.actor_did,
                self.action_type,
                self.compute_hc,
                self.trust_score,
                self.task_complexity,
                self.data_sensitivity
            )
            .as_bytes(),
        );
        format!("{}:quote:{}", prefix, hash.to_hex())
    }
}

/// Cached pricing response with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedResponse {
    response: PricingResponse,
    cached_at: i64,
}

impl PricingCache {
    /// Create a new pricing cache
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| ActorisError::Config(format!("Failed to create Redis client: {}", e)))?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ActorisError::Storage(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            client,
            connection: Arc::new(RwLock::new(Some(connection))),
            prefix: "actoris:pricing".to_string(),
            default_ttl: Duration::from_secs(300), // 5 minutes
        })
    }

    /// Create cache with custom prefix
    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Create cache with custom TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Get cached response for a request
    #[instrument(skip(self))]
    pub async fn get(&self, request: &PricingRequest) -> Result<Option<PricingResponse>> {
        let key = CacheKey::from(request);
        let redis_key = key.to_redis_key(&self.prefix);

        let mut conn = self.get_connection().await?;

        let cached: Option<String> = conn.get(&redis_key).await.map_err(|e| {
            warn!("Cache get error: {}", e);
            ActorisError::Storage(format!("Redis get failed: {}", e))
        })?;

        match cached {
            Some(json) => {
                let cached_response: CachedResponse = serde_json::from_str(&json).map_err(|e| {
                    ActorisError::Serialization(format!("Failed to deserialize cached response: {}", e))
                })?;

                // Check if still valid
                let now = chrono::Utc::now().timestamp_millis();
                if now < cached_response.response.expires_at {
                    debug!(key = %redis_key, "Cache hit");
                    Ok(Some(cached_response.response))
                } else {
                    debug!(key = %redis_key, "Cache expired");
                    Ok(None)
                }
            }
            None => {
                debug!(key = %redis_key, "Cache miss");
                Ok(None)
            }
        }
    }

    /// Cache a pricing response
    #[instrument(skip(self, response))]
    pub async fn set(&self, request: &PricingRequest, response: &PricingResponse) -> Result<()> {
        let key = CacheKey::from(request);
        let redis_key = key.to_redis_key(&self.prefix);

        let cached = CachedResponse {
            response: response.clone(),
            cached_at: chrono::Utc::now().timestamp_millis(),
        };

        let json = serde_json::to_string(&cached)
            .map_err(|e| ActorisError::Serialization(format!("Failed to serialize response: {}", e)))?;

        let mut conn = self.get_connection().await?;

        // Calculate TTL from response validity
        let ttl_secs = (response.valid_for_ms / 1000).max(1);

        conn.set_ex::<_, _, ()>(&redis_key, json, ttl_secs as u64)
            .await
            .map_err(|e| {
                warn!("Cache set error: {}", e);
                ActorisError::Storage(format!("Redis set failed: {}", e))
            })?;

        debug!(key = %redis_key, ttl_secs, "Cached pricing response");
        Ok(())
    }

    /// Invalidate cache for a specific actor
    #[instrument(skip(self))]
    pub async fn invalidate_actor(&self, actor_did: &str) -> Result<u64> {
        let pattern = format!("{}:quote:*", self.prefix);
        let mut conn = self.get_connection().await?;

        // Scan for matching keys (in production, use a secondary index)
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| ActorisError::Storage(format!("Redis KEYS failed: {}", e)))?;

        let mut deleted = 0u64;
        for key in keys {
            // In production, you'd want to check if the key belongs to the actor
            // For now, we delete all matching keys
            let result: u64 = conn.del(&key).await.unwrap_or(0);
            deleted += result;
        }

        debug!(actor = %actor_did, deleted, "Invalidated cache entries");
        Ok(deleted)
    }

    /// Clear all cached pricing data
    #[instrument(skip(self))]
    pub async fn clear_all(&self) -> Result<u64> {
        let pattern = format!("{}:*", self.prefix);
        let mut conn = self.get_connection().await?;

        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| ActorisError::Storage(format!("Redis KEYS failed: {}", e)))?;

        if keys.is_empty() {
            return Ok(0);
        }

        let deleted: u64 = conn.del(&keys).await.map_err(|e| {
            ActorisError::Storage(format!("Redis DEL failed: {}", e))
        })?;

        debug!(deleted, "Cleared all cache entries");
        Ok(deleted)
    }

    /// Get cache statistics
    #[instrument(skip(self))]
    pub async fn stats(&self) -> Result<CacheStats> {
        let pattern = format!("{}:quote:*", self.prefix);
        let mut conn = self.get_connection().await?;

        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| ActorisError::Storage(format!("Redis KEYS failed: {}", e)))?;

        let mut total_size = 0usize;
        for key in &keys {
            let size: usize = conn
                .strlen(key)
                .await
                .unwrap_or(0);
            total_size += size;
        }

        Ok(CacheStats {
            entry_count: keys.len() as u64,
            total_size_bytes: total_size as u64,
        })
    }

    /// Get a connection from the pool
    async fn get_connection(&self) -> Result<MultiplexedConnection> {
        let guard = self.connection.read().await;
        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
        }
        drop(guard);

        // Need to reconnect
        let mut guard = self.connection.write().await;
        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
        }

        let connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ActorisError::Storage(format!("Failed to reconnect to Redis: {}", e)))?;

        *guard = Some(connection.clone());
        Ok(connection)
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cached entries
    pub entry_count: u64,
    /// Total size in bytes
    pub total_size_bytes: u64,
}

/// In-memory fallback cache using DashMap
pub struct InMemoryPricingCache {
    cache: dashmap::DashMap<String, CachedResponse>,
    prefix: String,
    max_entries: usize,
}

impl InMemoryPricingCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: dashmap::DashMap::new(),
            prefix: "pricing".to_string(),
            max_entries,
        }
    }

    pub fn get(&self, request: &PricingRequest) -> Option<PricingResponse> {
        let key = CacheKey::from(request);
        let redis_key = key.to_redis_key(&self.prefix);

        self.cache.get(&redis_key).and_then(|entry| {
            let now = chrono::Utc::now().timestamp_millis();
            if now < entry.response.expires_at {
                Some(entry.response.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&self, request: &PricingRequest, response: &PricingResponse) {
        // Evict old entries if at capacity
        if self.cache.len() >= self.max_entries {
            let now = chrono::Utc::now().timestamp_millis();
            self.cache.retain(|_, v| now < v.response.expires_at);

            // If still at capacity, remove oldest
            if self.cache.len() >= self.max_entries {
                if let Some(oldest_key) = self.cache.iter().next().map(|e| e.key().clone()) {
                    self.cache.remove(&oldest_key);
                }
            }
        }

        let key = CacheKey::from(request);
        let redis_key = key.to_redis_key(&self.prefix);

        self.cache.insert(
            redis_key,
            CachedResponse {
                response: response.clone(),
                cached_at: chrono::Utc::now().timestamp_millis(),
            },
        );
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_cache_key_generation() {
        let request = PricingRequest::new("did:key:test", "test.action", dec!(100), 500);
        let key = CacheKey::from(&request);
        let redis_key = key.to_redis_key("test");

        assert!(redis_key.starts_with("test:quote:"));
        assert!(redis_key.len() > 20);
    }

    #[test]
    fn test_in_memory_cache() {
        let cache = InMemoryPricingCache::new(100);
        let request = PricingRequest::new("did:key:test", "test.action", dec!(100), 500);

        // Should be empty initially
        assert!(cache.get(&request).is_none());

        // Create a response
        let response = PricingResponse {
            quote_id: uuid::Uuid::new_v4(),
            final_price: dec!(100),
            breakdown: actoris_common::PricingBreakdown {
                compute_cost: dec!(100),
                risk_premium: dec!(10),
                trust_discount: dec!(10),
                risk_factor: actoris_common::RiskFactor::Low,
            },
            currency: "HC".to_string(),
            valid_for_ms: 300000,
            expires_at: chrono::Utc::now().timestamp_millis() + 300000,
            computed_at: chrono::Utc::now().timestamp_millis(),
        };

        cache.set(&request, &response);

        // Should be cached now
        let cached = cache.get(&request);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().final_price, dec!(100));
    }

    #[test]
    fn test_in_memory_cache_eviction() {
        let cache = InMemoryPricingCache::new(2);

        for i in 0..5 {
            let request = PricingRequest::new(
                &format!("did:key:test{}", i),
                "test.action",
                dec!(100),
                500,
            );
            let response = PricingResponse {
                quote_id: uuid::Uuid::new_v4(),
                final_price: dec!(100),
                breakdown: actoris_common::PricingBreakdown {
                    compute_cost: dec!(100),
                    risk_premium: dec!(10),
                    trust_discount: dec!(10),
                    risk_factor: actoris_common::RiskFactor::Low,
                },
                currency: "HC".to_string(),
                valid_for_ms: 300000,
                expires_at: chrono::Utc::now().timestamp_millis() + 300000,
                computed_at: chrono::Utc::now().timestamp_millis(),
            };
            cache.set(&request, &response);
        }

        // Should have evicted old entries
        assert!(cache.len() <= 2);
    }
}
