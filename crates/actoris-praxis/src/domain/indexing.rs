//! Memory Indexing
//!
//! Strategies for indexing memories for efficient retrieval.

use std::collections::HashMap;
use uuid::Uuid;

use super::memory::PraxisMemory;

/// Index entry for a memory
#[derive(Debug, Clone)]
pub struct MemoryIndex {
    /// Memory ID
    pub memory_id: Uuid,
    /// Agent ID
    pub agent_id: Uuid,
    /// Action type (for action-based retrieval)
    pub action_type: String,
    /// Directive hash (for goal-based retrieval)
    pub directive_hash: u64,
    /// Feature keys present in the memory
    pub feature_keys: Vec<String>,
    /// Element IDs present
    pub element_ids: Vec<String>,
    /// Task tags
    pub task_tags: Vec<String>,
    /// Is successful
    pub is_successful: bool,
}

impl MemoryIndex {
    /// Create index from a memory
    pub fn from_memory(memory: &PraxisMemory) -> Self {
        Self {
            memory_id: memory.id,
            agent_id: memory.agent_id,
            action_type: memory.action.action_type.clone(),
            directive_hash: Self::hash_directive(&memory.internal_state.directive),
            feature_keys: memory.env_state_pre.state_features.keys().cloned().collect(),
            element_ids: memory.env_state_pre.element_ids.clone(),
            task_tags: memory.internal_state.task_tags.clone(),
            is_successful: memory.is_successful(),
        }
    }

    /// Simple hash function for directives
    fn hash_directive(directive: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        directive.to_lowercase().hash(&mut hasher);
        hasher.finish()
    }
}

/// Multi-index for efficient memory retrieval
pub struct MultiIndex {
    /// All indices
    indices: Vec<MemoryIndex>,
    /// Index by agent
    by_agent: HashMap<Uuid, Vec<usize>>,
    /// Index by action type
    by_action_type: HashMap<String, Vec<usize>>,
    /// Index by directive hash
    by_directive: HashMap<u64, Vec<usize>>,
    /// Index by feature key
    by_feature_key: HashMap<String, Vec<usize>>,
    /// Index by element ID
    by_element_id: HashMap<String, Vec<usize>>,
    /// Index by task tag
    by_task_tag: HashMap<String, Vec<usize>>,
    /// Index of successful memories only
    successful_indices: Vec<usize>,
}

impl MultiIndex {
    /// Create a new multi-index
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
            by_agent: HashMap::new(),
            by_action_type: HashMap::new(),
            by_directive: HashMap::new(),
            by_feature_key: HashMap::new(),
            by_element_id: HashMap::new(),
            by_task_tag: HashMap::new(),
            successful_indices: Vec::new(),
        }
    }

    /// Add a memory to the index
    pub fn add(&mut self, memory: &PraxisMemory) {
        let index = MemoryIndex::from_memory(memory);
        let idx = self.indices.len();

        // Add to agent index
        self.by_agent
            .entry(index.agent_id)
            .or_default()
            .push(idx);

        // Add to action type index
        self.by_action_type
            .entry(index.action_type.clone())
            .or_default()
            .push(idx);

        // Add to directive index
        self.by_directive
            .entry(index.directive_hash)
            .or_default()
            .push(idx);

        // Add to feature key indices
        for key in &index.feature_keys {
            self.by_feature_key
                .entry(key.clone())
                .or_default()
                .push(idx);
        }

        // Add to element ID indices
        for element in &index.element_ids {
            self.by_element_id
                .entry(element.clone())
                .or_default()
                .push(idx);
        }

        // Add to task tag indices
        for tag in &index.task_tags {
            self.by_task_tag
                .entry(tag.clone())
                .or_default()
                .push(idx);
        }

        // Add to successful index if applicable
        if index.is_successful {
            self.successful_indices.push(idx);
        }

        self.indices.push(index);
    }

    /// Remove a memory from the index
    pub fn remove(&mut self, memory_id: &Uuid) {
        // Find the index
        let idx = self.indices.iter().position(|i| i.memory_id == *memory_id);

        if let Some(idx) = idx {
            let index = &self.indices[idx];

            // Remove from all secondary indices
            Self::remove_from_vec(&mut self.by_agent.get_mut(&index.agent_id), idx);
            Self::remove_from_vec(&mut self.by_action_type.get_mut(&index.action_type), idx);
            Self::remove_from_vec(&mut self.by_directive.get_mut(&index.directive_hash), idx);

            for key in &index.feature_keys {
                Self::remove_from_vec(&mut self.by_feature_key.get_mut(key), idx);
            }

            for element in &index.element_ids {
                Self::remove_from_vec(&mut self.by_element_id.get_mut(element), idx);
            }

            for tag in &index.task_tags {
                Self::remove_from_vec(&mut self.by_task_tag.get_mut(tag), idx);
            }

            if index.is_successful {
                self.successful_indices.retain(|&i| i != idx);
            }

            // Note: We don't actually remove from self.indices to preserve index validity
            // In a production system, you'd use a more sophisticated approach
        }
    }

    fn remove_from_vec(vec: &mut Option<&mut Vec<usize>>, idx: usize) {
        if let Some(v) = vec {
            v.retain(|&i| i != idx);
        }
    }

    /// Get candidate memory indices for a query
    pub fn get_candidates(
        &self,
        agent_id: Option<&Uuid>,
        action_type: Option<&str>,
        feature_keys: &[String],
        element_ids: &[String],
        task_tags: &[String],
        successful_only: bool,
    ) -> Vec<usize> {
        let mut candidates = std::collections::HashSet::new();

        // Start with agent-specific memories if specified
        if let Some(agent_id) = agent_id {
            if let Some(indices) = self.by_agent.get(agent_id) {
                candidates.extend(indices.iter().copied());
            }
        } else {
            // Include all memories
            candidates.extend(0..self.indices.len());
        }

        // Filter by action type if specified
        if let Some(action_type) = action_type {
            if let Some(indices) = self.by_action_type.get(action_type) {
                let action_set: std::collections::HashSet<_> = indices.iter().copied().collect();
                candidates = candidates.intersection(&action_set).copied().collect();
            }
        }

        // Boost candidates with matching feature keys
        let mut feature_matches: HashMap<usize, usize> = HashMap::new();
        for key in feature_keys {
            if let Some(indices) = self.by_feature_key.get(key) {
                for &idx in indices {
                    if candidates.contains(&idx) {
                        *feature_matches.entry(idx).or_default() += 1;
                    }
                }
            }
        }

        // Boost candidates with matching elements
        for element in element_ids {
            if let Some(indices) = self.by_element_id.get(element) {
                for &idx in indices {
                    if candidates.contains(&idx) {
                        *feature_matches.entry(idx).or_default() += 1;
                    }
                }
            }
        }

        // Boost candidates with matching tags
        for tag in task_tags {
            if let Some(indices) = self.by_task_tag.get(tag) {
                for &idx in indices {
                    if candidates.contains(&idx) {
                        *feature_matches.entry(idx).or_default() += 1;
                    }
                }
            }
        }

        // Filter to successful only if requested
        if successful_only {
            let successful_set: std::collections::HashSet<_> =
                self.successful_indices.iter().copied().collect();
            candidates = candidates.intersection(&successful_set).copied().collect();
        }

        // Sort by match count (descending)
        let mut result: Vec<usize> = candidates.into_iter().collect();
        result.sort_by(|a, b| {
            let count_a = feature_matches.get(a).unwrap_or(&0);
            let count_b = feature_matches.get(b).unwrap_or(&0);
            count_b.cmp(count_a)
        });

        result
    }

    /// Get statistics about the index
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            total_memories: self.indices.len(),
            unique_agents: self.by_agent.len(),
            unique_action_types: self.by_action_type.len(),
            unique_directives: self.by_directive.len(),
            unique_feature_keys: self.by_feature_key.len(),
            unique_elements: self.by_element_id.len(),
            unique_tags: self.by_task_tag.len(),
            successful_memories: self.successful_indices.len(),
        }
    }
}

impl Default for MultiIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the memory index
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub total_memories: usize,
    pub unique_agents: usize,
    pub unique_action_types: usize,
    pub unique_directives: usize,
    pub unique_feature_keys: usize,
    pub unique_elements: usize,
    pub unique_tags: usize,
    pub successful_memories: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::memory::*;

    fn create_test_memory(agent_id: Uuid, action_type: &str, successful: bool) -> PraxisMemory {
        let mut env = EnvironmentState::new("test");
        env.add_feature("url", "https://example.com");
        env.add_element("button#submit");

        let mut internal = InternalState::new("Test directive");
        internal.add_tag("test");

        PraxisMemory::new(
            agent_id,
            env,
            internal,
            AgentAction::new(action_type, format!("{}()", action_type)),
            EnvironmentState::new("result"),
            if successful {
                ActionOutcome::success("ok")
            } else {
                ActionOutcome::failure("ERR", "failed")
            },
        )
    }

    #[test]
    fn test_multi_index_add_and_stats() {
        let mut index = MultiIndex::new();
        let agent1 = Uuid::new_v4();
        let agent2 = Uuid::new_v4();

        index.add(&create_test_memory(agent1, "click", true));
        index.add(&create_test_memory(agent1, "type", true));
        index.add(&create_test_memory(agent2, "click", false));

        let stats = index.stats();
        assert_eq!(stats.total_memories, 3);
        assert_eq!(stats.unique_agents, 2);
        assert_eq!(stats.unique_action_types, 2);
        assert_eq!(stats.successful_memories, 2);
    }

    #[test]
    fn test_get_candidates() {
        let mut index = MultiIndex::new();
        let agent = Uuid::new_v4();

        index.add(&create_test_memory(agent, "click", true));
        index.add(&create_test_memory(agent, "type", true));
        index.add(&create_test_memory(agent, "click", false));

        // Get all for agent
        let candidates = index.get_candidates(
            Some(&agent),
            None,
            &[],
            &[],
            &[],
            false,
        );
        assert_eq!(candidates.len(), 3);

        // Get only clicks
        let candidates = index.get_candidates(
            Some(&agent),
            Some("click"),
            &[],
            &[],
            &[],
            false,
        );
        assert_eq!(candidates.len(), 2);

        // Get only successful
        let candidates = index.get_candidates(
            Some(&agent),
            None,
            &[],
            &[],
            &[],
            true,
        );
        assert_eq!(candidates.len(), 2);
    }
}
