//! Embedding Service
//!
//! Generate embeddings for state representations.

use async_trait::async_trait;
use std::collections::HashMap;
use parking_lot::RwLock;

/// Trait for embedding services
#[async_trait]
pub trait EmbeddingService: Send + Sync {
    /// Generate embedding for text
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError>;

    /// Get embedding dimension
    fn dimension(&self) -> usize;
}

/// Errors from embedding operations
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Service unavailable")]
    Unavailable,
}

/// Simple local embedding using character n-grams
///
/// This is a lightweight fallback when no external embedding service is available.
/// It creates sparse embeddings based on character trigrams.
pub struct LocalEmbedding {
    dimension: usize,
    cache: RwLock<HashMap<String, Vec<f32>>>,
    cache_size_limit: usize,
}

impl LocalEmbedding {
    /// Create a new local embedding service
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            cache: RwLock::new(HashMap::new()),
            cache_size_limit: 10000,
        }
    }

    /// Hash a trigram to an index
    fn hash_trigram(trigram: &[u8]) -> usize {
        let mut hash: usize = 0;
        for (i, &b) in trigram.iter().enumerate() {
            hash = hash.wrapping_add((b as usize).wrapping_mul(31_usize.pow(i as u32)));
        }
        hash
    }

    /// Generate a sparse embedding from text
    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0f32; self.dimension];

        if text.is_empty() {
            return embedding;
        }

        let bytes = text.as_bytes();
        let trigrams: Vec<_> = bytes.windows(3).collect();

        if trigrams.is_empty() {
            // For very short text, use character-level features
            for (i, &b) in bytes.iter().enumerate() {
                let idx = (b as usize + i * 256) % self.dimension;
                embedding[idx] += 1.0;
            }
        } else {
            // Use trigram features
            for trigram in &trigrams {
                let idx = Self::hash_trigram(trigram) % self.dimension;
                embedding[idx] += 1.0;
            }
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        embedding
    }
}

impl Default for LocalEmbedding {
    fn default() -> Self {
        Self::new(256)
    }
}

#[async_trait]
impl EmbeddingService for LocalEmbedding {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        // Check cache
        {
            let cache = self.cache.read();
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }

        // Generate embedding
        let embedding = self.generate_embedding(text);

        // Cache it
        {
            let mut cache = self.cache.write();
            if cache.len() < self.cache_size_limit {
                cache.insert(text.to_string(), embedding.clone());
            }
        }

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());

        for text in texts {
            results.push(self.embed_text(text).await?);
        }

        Ok(results)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Cached wrapper for any embedding service
pub struct CachedEmbedding<E: EmbeddingService> {
    inner: E,
    cache: RwLock<HashMap<String, Vec<f32>>>,
    cache_size_limit: usize,
}

impl<E: EmbeddingService> CachedEmbedding<E> {
    /// Create a new cached embedding service
    pub fn new(inner: E, cache_size_limit: usize) -> Self {
        Self {
            inner,
            cache: RwLock::new(HashMap::new()),
            cache_size_limit,
        }
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read();
        (cache.len(), self.cache_size_limit)
    }
}

#[async_trait]
impl<E: EmbeddingService> EmbeddingService for CachedEmbedding<E> {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        // Check cache
        {
            let cache = self.cache.read();
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }

        // Generate embedding
        let embedding = self.inner.embed_text(text).await?;

        // Cache it
        {
            let mut cache = self.cache.write();
            if cache.len() < self.cache_size_limit {
                cache.insert(text.to_string(), embedding.clone());
            }
        }

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        let mut to_embed = Vec::new();
        let mut indices = Vec::new();

        // Check cache for each text
        {
            let cache = self.cache.read();
            for (i, text) in texts.iter().enumerate() {
                if let Some(embedding) = cache.get(text) {
                    results.push(Some(embedding.clone()));
                } else {
                    results.push(None);
                    to_embed.push(text.clone());
                    indices.push(i);
                }
            }
        }

        // Embed missing ones
        if !to_embed.is_empty() {
            let embeddings = self.inner.embed_batch(&to_embed).await?;

            // Update results and cache
            let mut cache = self.cache.write();
            for (embed_idx, orig_idx) in indices.iter().enumerate() {
                let embedding = embeddings[embed_idx].clone();
                results[*orig_idx] = Some(embedding.clone());

                if cache.len() < self.cache_size_limit {
                    cache.insert(to_embed[embed_idx].clone(), embedding);
                }
            }
        }

        Ok(results.into_iter().map(|o| o.unwrap()).collect())
    }

    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_embedding() {
        let service = LocalEmbedding::new(256);

        let embedding = service.embed_text("hello world").await.unwrap();

        assert_eq!(embedding.len(), 256);

        // Check normalization
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_similar_texts_similar_embeddings() {
        let service = LocalEmbedding::new(256);

        let emb1 = service.embed_text("hello world").await.unwrap();
        let emb2 = service.embed_text("hello world!").await.unwrap();
        let emb3 = service.embed_text("goodbye universe").await.unwrap();

        // Cosine similarity
        let sim_12: f32 = emb1.iter().zip(&emb2).map(|(a, b)| a * b).sum();
        let sim_13: f32 = emb1.iter().zip(&emb3).map(|(a, b)| a * b).sum();

        // Similar texts should have higher similarity
        assert!(sim_12 > sim_13);
    }

    #[tokio::test]
    async fn test_caching() {
        let service = LocalEmbedding::new(256);

        // First call
        let emb1 = service.embed_text("test").await.unwrap();

        // Second call (should hit cache)
        let emb2 = service.embed_text("test").await.unwrap();

        assert_eq!(emb1, emb2);
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let service = LocalEmbedding::new(256);

        let texts = vec![
            "hello".to_string(),
            "world".to_string(),
            "test".to_string(),
        ];

        let embeddings = service.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.len(), 256);
        }
    }
}
