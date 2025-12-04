//! PRAXIS Retrieval Algorithm
//!
//! Implementation of Algorithm 1 from the PRAXIS paper:
//! Procedural Memory Retrieval using IoU + Embedding Similarity
//!
//! The algorithm:
//! 1. Calculate environmental similarity (IoU × length overlap) for all memories
//! 2. Get top-k by environmental similarity
//! 3. Re-rank by internal state embedding similarity
//! 4. Filter by similarity threshold τ

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use ordered_float::OrderedFloat;
use uuid::Uuid;

use super::memory::{EnvironmentState, InternalState, PraxisMemory};
use crate::infra::memory_store::MemoryStore;

/// Configuration for the retrieval algorithm
#[derive(Debug, Clone)]
pub struct RetrievalConfig {
    /// Top-k candidates from environmental matching (k in Algorithm 1)
    pub search_breadth: usize,
    /// Minimum similarity threshold (τ in Algorithm 1)
    pub similarity_threshold: f32,
    /// Weight for IoU component
    pub iou_weight: f32,
    /// Weight for length overlap component
    pub length_weight: f32,
    /// Whether to apply time decay
    pub enable_time_decay: bool,
    /// Half-life for time decay in days
    pub decay_half_life_days: f64,
    /// Whether to boost successful memories
    pub boost_successful: bool,
    /// Boost factor for successful memories
    pub success_boost: f32,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            search_breadth: crate::DEFAULT_RETRIEVAL_BREADTH,
            similarity_threshold: crate::DEFAULT_SIMILARITY_THRESHOLD,
            iou_weight: 1.0,
            length_weight: 1.0,
            enable_time_decay: true,
            decay_half_life_days: crate::MEMORY_DECAY_HALF_LIFE_DAYS,
            boost_successful: true,
            success_boost: 1.5,
        }
    }
}

/// A retrieved memory with relevance score
#[derive(Debug, Clone)]
pub struct RetrievedMemory {
    /// The retrieved memory
    pub memory: PraxisMemory,
    /// Environmental similarity score
    pub env_similarity: f32,
    /// Internal state similarity score
    pub internal_similarity: f32,
    /// Combined relevance score
    pub relevance_score: f32,
    /// Rank in the result set
    pub rank: usize,
}

impl RetrievedMemory {
    /// Check if this memory was successful
    pub fn is_successful(&self) -> bool {
        self.memory.is_successful()
    }
}

/// Memory retriever implementing Algorithm 1
pub struct MemoryRetriever {
    /// Memory store
    store: Arc<dyn MemoryStore>,
    /// Retrieval configuration
    config: RetrievalConfig,
}

impl MemoryRetriever {
    /// Create a new retriever
    pub fn new(store: Arc<dyn MemoryStore>, config: RetrievalConfig) -> Self {
        Self { store, config }
    }

    /// Retrieve relevant memories for the current state
    ///
    /// Implements Algorithm 1 from the PRAXIS paper:
    /// 1. Calculate environmental similarity for all memories
    /// 2. Get top-k by environmental similarity
    /// 3. Re-rank by internal state similarity
    /// 4. Filter by threshold
    pub async fn retrieve(
        &self,
        agent_id: &Uuid,
        query_env: &EnvironmentState,
        query_internal: &InternalState,
    ) -> Vec<RetrievedMemory> {
        // Get all memories for this agent
        let memories = self.store.get_agent_memories(agent_id).await;

        if memories.is_empty() {
            return Vec::new();
        }

        // Step 1: Calculate environmental similarity scores for all memories
        let mut env_scores: Vec<(usize, f32)> = memories
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let s_env = self.calculate_env_similarity(&m.env_state_pre, query_env);
                (i, s_env)
            })
            .collect();

        // Step 2: Get top-k by environmental similarity
        env_scores.sort_by(|a, b| {
            OrderedFloat(b.1).cmp(&OrderedFloat(a.1))
        });

        let top_k: Vec<(usize, f32)> = env_scores
            .into_iter()
            .take(self.config.search_breadth)
            .collect();

        // Step 3: Calculate internal state similarity and combine scores
        let mut candidates: Vec<(usize, f32, f32, f32)> = top_k
            .into_iter()
            .map(|(i, s_env)| {
                let s_int = self.calculate_internal_similarity(
                    &memories[i].internal_state,
                    query_internal,
                );

                // Combine scores
                let mut combined = s_env * 0.6 + s_int * 0.4;

                // Apply time decay if enabled
                if self.config.enable_time_decay {
                    combined *= memories[i].time_decay_factor(self.config.decay_half_life_days);
                }

                // Boost successful memories
                if self.config.boost_successful && memories[i].is_successful() {
                    combined *= self.config.success_boost;
                }

                // Apply reinforcement score
                combined *= memories[i].reinforcement_score.sqrt();

                (i, s_env, s_int, combined)
            })
            .collect();

        // Re-sort by combined score
        candidates.sort_by(|a, b| {
            OrderedFloat(b.3).cmp(&OrderedFloat(a.3))
        });

        // Step 4: Filter by threshold and build results
        candidates
            .into_iter()
            .enumerate()
            .filter(|(_, (_, s_env, _, _))| *s_env >= self.config.similarity_threshold)
            .map(|(rank, (i, s_env, s_int, combined))| {
                RetrievedMemory {
                    memory: memories[i].clone(),
                    env_similarity: s_env,
                    internal_similarity: s_int,
                    relevance_score: combined,
                    rank,
                }
            })
            .collect()
    }

    /// Retrieve memories for any agent (cross-agent learning)
    pub async fn retrieve_global(
        &self,
        query_env: &EnvironmentState,
        query_internal: &InternalState,
        max_results: usize,
    ) -> Vec<RetrievedMemory> {
        let memories = self.store.get_all_memories().await;

        if memories.is_empty() {
            return Vec::new();
        }

        // Same algorithm but across all agents
        let mut scored: Vec<(usize, f32, f32, f32)> = memories
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let s_env = self.calculate_env_similarity(&m.env_state_pre, query_env);
                let s_int = self.calculate_internal_similarity(&m.internal_state, query_internal);
                let mut combined = s_env * 0.6 + s_int * 0.4;

                if self.config.enable_time_decay {
                    combined *= m.time_decay_factor(self.config.decay_half_life_days);
                }
                if self.config.boost_successful && m.is_successful() {
                    combined *= self.config.success_boost;
                }

                (i, s_env, s_int, combined)
            })
            .collect();

        scored.sort_by(|a, b| OrderedFloat(b.3).cmp(&OrderedFloat(a.3)));

        scored
            .into_iter()
            .take(max_results)
            .enumerate()
            .filter(|(_, (_, s_env, _, _))| *s_env >= self.config.similarity_threshold)
            .map(|(rank, (i, s_env, s_int, combined))| {
                RetrievedMemory {
                    memory: memories[i].clone(),
                    env_similarity: s_env,
                    internal_similarity: s_int,
                    relevance_score: combined,
                    rank,
                }
            })
            .collect()
    }

    /// Calculate environmental similarity (IoU × length overlap)
    ///
    /// From Algorithm 1:
    /// - v_i = IoU(M_env, Q_env)
    /// - l_i = LengthOverlap(len(M_env), len(Q_env))
    /// - s_env = v_i × l_i
    fn calculate_env_similarity(&self, memory_env: &EnvironmentState, query_env: &EnvironmentState) -> f32 {
        // IoU over state features
        let iou = self.calculate_iou(memory_env, query_env);

        // Length overlap
        let length_overlap = self.calculate_length_overlap(memory_env, query_env);

        // Element overlap (additional signal)
        let element_overlap = self.calculate_element_overlap(memory_env, query_env);

        // Combine with weights
        let base_score = iou * self.config.iou_weight + length_overlap * self.config.length_weight;

        // Add element overlap bonus
        let with_elements = base_score * 0.7 + element_overlap * 0.3;

        // Normalize
        with_elements / (self.config.iou_weight + self.config.length_weight + 0.3)
    }

    /// Calculate IoU (Intersection over Union) of state features
    fn calculate_iou(&self, memory_env: &EnvironmentState, query_env: &EnvironmentState) -> f32 {
        let m_keys: HashSet<&String> = memory_env.state_features.keys().collect();
        let q_keys: HashSet<&String> = query_env.state_features.keys().collect();

        if m_keys.is_empty() && q_keys.is_empty() {
            // If no features, fall back to text similarity
            return self.calculate_text_similarity(
                &memory_env.textual_repr,
                &query_env.textual_repr,
            );
        }

        let intersection = m_keys.intersection(&q_keys).count() as f32;
        let union = m_keys.union(&q_keys).count() as f32;

        if union == 0.0 {
            0.0
        } else {
            // Also consider value matches for intersecting keys
            let mut value_matches = 0.0;
            for key in m_keys.intersection(&q_keys) {
                if memory_env.state_features.get(*key) == query_env.state_features.get(*key) {
                    value_matches += 1.0;
                }
            }

            let key_iou = intersection / union;
            let value_match_ratio = if intersection > 0.0 {
                value_matches / intersection
            } else {
                0.0
            };

            // Weight key overlap more than value match
            key_iou * 0.7 + value_match_ratio * 0.3
        }
    }

    /// Calculate length overlap as defined in Algorithm 1
    /// LengthOverlap(l_m, l_q) = 1 - |l_m - l_q| / max(l_m, l_q)
    fn calculate_length_overlap(&self, memory_env: &EnvironmentState, query_env: &EnvironmentState) -> f32 {
        let l_m = memory_env.text_length() as f32;
        let l_q = query_env.text_length() as f32;

        if l_m == 0.0 && l_q == 0.0 {
            return 1.0;
        }

        let max_len = l_m.max(l_q);
        if max_len == 0.0 {
            return 1.0;
        }

        1.0 - (l_m - l_q).abs() / max_len
    }

    /// Calculate element ID overlap
    fn calculate_element_overlap(&self, memory_env: &EnvironmentState, query_env: &EnvironmentState) -> f32 {
        let m_elements: HashSet<&String> = memory_env.element_ids.iter().collect();
        let q_elements: HashSet<&String> = query_env.element_ids.iter().collect();

        if m_elements.is_empty() && q_elements.is_empty() {
            return 0.5; // Neutral when no elements
        }

        let intersection = m_elements.intersection(&q_elements).count() as f32;
        let union = m_elements.union(&q_elements).count() as f32;

        if union == 0.0 { 0.0 } else { intersection / union }
    }

    /// Simple text similarity using character n-grams
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f32 {
        if text1.is_empty() && text2.is_empty() {
            return 1.0;
        }
        if text1.is_empty() || text2.is_empty() {
            return 0.0;
        }

        // Use 3-grams for similarity
        let ngrams1: HashSet<&str> = text1
            .as_bytes()
            .windows(3)
            .map(|w| std::str::from_utf8(w).unwrap_or(""))
            .collect();
        let ngrams2: HashSet<&str> = text2
            .as_bytes()
            .windows(3)
            .map(|w| std::str::from_utf8(w).unwrap_or(""))
            .collect();

        let intersection = ngrams1.intersection(&ngrams2).count() as f32;
        let union = ngrams1.union(&ngrams2).count() as f32;

        if union == 0.0 { 0.0 } else { intersection / union }
    }

    /// Calculate internal state similarity
    ///
    /// Uses embedding cosine similarity if available, otherwise text similarity
    fn calculate_internal_similarity(
        &self,
        memory_internal: &InternalState,
        query_internal: &InternalState,
    ) -> f32 {
        // Try embedding similarity first
        if let (Some(m_emb), Some(q_emb)) = (&memory_internal.embedding, &query_internal.embedding) {
            return self.cosine_similarity(m_emb, q_emb);
        }

        // Fall back to text similarity
        let m_text = memory_internal.as_text();
        let q_text = query_internal.as_text();

        // Text similarity
        let text_sim = self.calculate_text_similarity(&m_text, &q_text);

        // Tag overlap
        let m_tags: HashSet<&String> = memory_internal.task_tags.iter().collect();
        let q_tags: HashSet<&String> = query_internal.task_tags.iter().collect();

        let tag_sim = if m_tags.is_empty() && q_tags.is_empty() {
            0.5
        } else {
            let intersection = m_tags.intersection(&q_tags).count() as f32;
            let union = m_tags.union(&q_tags).count() as f32;
            if union == 0.0 { 0.0 } else { intersection / union }
        };

        // Combine
        text_sim * 0.7 + tag_sim * 0.3
    }

    /// Calculate cosine similarity between two embedding vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
    }

    /// Update configuration
    pub fn set_config(&mut self, config: RetrievalConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &RetrievalConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::memory_store::InMemoryStore;

    fn create_test_retriever() -> MemoryRetriever {
        let store = Arc::new(InMemoryStore::new(1000));
        MemoryRetriever::new(store, RetrievalConfig::default())
    }

    #[test]
    fn test_iou_calculation() {
        let retriever = create_test_retriever();

        let mut env1 = EnvironmentState::new("test");
        env1.add_feature("a", "1");
        env1.add_feature("b", "2");
        env1.add_feature("c", "3");

        let mut env2 = EnvironmentState::new("test");
        env2.add_feature("b", "2");
        env2.add_feature("c", "3");
        env2.add_feature("d", "4");

        let iou = retriever.calculate_iou(&env1, &env2);
        // Intersection: {b, c} = 2, Union: {a, b, c, d} = 4
        // Key IoU = 2/4 = 0.5, Value matches = 2/2 = 1.0
        // Combined = 0.5 * 0.7 + 1.0 * 0.3 = 0.65
        assert!(iou > 0.5 && iou < 0.8);
    }

    #[test]
    fn test_length_overlap() {
        let retriever = create_test_retriever();

        let env1 = EnvironmentState::new("hello world");
        let env2 = EnvironmentState::new("hello");

        let overlap = retriever.calculate_length_overlap(&env1, &env2);
        // len1 = 11, len2 = 5
        // overlap = 1 - |11-5|/11 = 1 - 6/11 ≈ 0.45
        assert!(overlap > 0.4 && overlap < 0.6);
    }

    #[test]
    fn test_cosine_similarity() {
        let retriever = create_test_retriever();

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((retriever.cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(retriever.cosine_similarity(&a, &c).abs() < 0.001);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((retriever.cosine_similarity(&a, &d) + 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_retrieve_empty() {
        let retriever = create_test_retriever();
        let agent_id = Uuid::new_v4();

        let results = retriever.retrieve(
            &agent_id,
            &EnvironmentState::new("test"),
            &InternalState::new("test directive"),
        ).await;

        assert!(results.is_empty());
    }
}
