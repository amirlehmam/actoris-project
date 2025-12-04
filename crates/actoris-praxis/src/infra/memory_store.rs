//! Memory Storage Implementations
//!
//! Storage backends for procedural memories.

use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::indexing::MultiIndex;
use crate::domain::memory::PraxisMemory;

/// Trait for memory storage backends
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store a new memory
    async fn store(&self, memory: PraxisMemory) -> Result<Uuid, StoreError>;

    /// Get a memory by ID
    async fn get(&self, id: &Uuid) -> Option<PraxisMemory>;

    /// Get all memories for an agent
    async fn get_agent_memories(&self, agent_id: &Uuid) -> Vec<PraxisMemory>;

    /// Get all memories (for cross-agent retrieval)
    async fn get_all_memories(&self) -> Vec<PraxisMemory>;

    /// Update a memory (e.g., after retrieval)
    async fn update(&self, memory: PraxisMemory) -> Result<(), StoreError>;

    /// Delete a memory
    async fn delete(&self, id: &Uuid) -> Result<(), StoreError>;

    /// Get memory count for an agent
    async fn count_agent_memories(&self, agent_id: &Uuid) -> usize;

    /// Get total memory count
    async fn total_count(&self) -> usize;

    /// Clear all memories for an agent
    async fn clear_agent_memories(&self, agent_id: &Uuid) -> Result<usize, StoreError>;
}

/// Errors from memory store operations
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Memory not found: {0}")]
    NotFound(Uuid),

    #[error("Memory limit exceeded for agent {0}: max {1}")]
    LimitExceeded(Uuid, usize),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// In-memory storage implementation
///
/// Uses DashMap for concurrent access with per-agent limits.
pub struct InMemoryStore {
    /// All memories by ID
    memories: DashMap<Uuid, PraxisMemory>,

    /// Index of memory IDs by agent
    by_agent: DashMap<Uuid, Vec<Uuid>>,

    /// Multi-index for efficient retrieval
    index: RwLock<MultiIndex>,

    /// Maximum memories per agent
    max_per_agent: usize,
}

impl InMemoryStore {
    /// Create a new in-memory store
    pub fn new(max_per_agent: usize) -> Self {
        Self {
            memories: DashMap::new(),
            by_agent: DashMap::new(),
            index: RwLock::new(MultiIndex::new()),
            max_per_agent,
        }
    }

    /// Evict oldest memories if limit exceeded
    fn evict_if_needed(&self, agent_id: &Uuid) {
        let mut agent_memories = self.by_agent.entry(*agent_id).or_default();

        while agent_memories.len() > self.max_per_agent {
            // Find oldest memory
            if let Some((oldest_idx, oldest_id)) = agent_memories
                .iter()
                .enumerate()
                .min_by_key(|(_, id)| {
                    self.memories
                        .get(id)
                        .map(|m| m.created_at)
                        .unwrap_or_else(|| chrono::Utc::now())
                })
            {
                let id_to_remove = *oldest_id;
                agent_memories.remove(oldest_idx);
                self.memories.remove(&id_to_remove);

                // Remove from index
                let mut index = self.index.write();
                index.remove(&id_to_remove);
            } else {
                break;
            }
        }
    }
}

#[async_trait]
impl MemoryStore for InMemoryStore {
    async fn store(&self, memory: PraxisMemory) -> Result<Uuid, StoreError> {
        let id = memory.id;
        let agent_id = memory.agent_id;

        // Check limit
        let current_count = self.count_agent_memories(&agent_id).await;
        if current_count >= self.max_per_agent {
            self.evict_if_needed(&agent_id);
        }

        // Add to index
        {
            let mut index = self.index.write();
            index.add(&memory);
        }

        // Store memory
        self.memories.insert(id, memory);

        // Update agent index
        self.by_agent.entry(agent_id).or_default().push(id);

        Ok(id)
    }

    async fn get(&self, id: &Uuid) -> Option<PraxisMemory> {
        self.memories.get(id).map(|m| m.clone())
    }

    async fn get_agent_memories(&self, agent_id: &Uuid) -> Vec<PraxisMemory> {
        self.by_agent
            .get(agent_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.memories.get(id).map(|m| m.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn get_all_memories(&self) -> Vec<PraxisMemory> {
        self.memories.iter().map(|m| m.clone()).collect()
    }

    async fn update(&self, memory: PraxisMemory) -> Result<(), StoreError> {
        let id = memory.id;

        if !self.memories.contains_key(&id) {
            return Err(StoreError::NotFound(id));
        }

        // Update index
        {
            let mut index = self.index.write();
            index.remove(&id);
            index.add(&memory);
        }

        self.memories.insert(id, memory);
        Ok(())
    }

    async fn delete(&self, id: &Uuid) -> Result<(), StoreError> {
        let memory = self.memories.remove(id);

        if let Some((_, memory)) = memory {
            // Remove from agent index
            if let Some(mut agent_memories) = self.by_agent.get_mut(&memory.agent_id) {
                agent_memories.retain(|mid| mid != id);
            }

            // Remove from multi-index
            let mut index = self.index.write();
            index.remove(id);

            Ok(())
        } else {
            Err(StoreError::NotFound(*id))
        }
    }

    async fn count_agent_memories(&self, agent_id: &Uuid) -> usize {
        self.by_agent
            .get(agent_id)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    async fn total_count(&self) -> usize {
        self.memories.len()
    }

    async fn clear_agent_memories(&self, agent_id: &Uuid) -> Result<usize, StoreError> {
        let ids: Vec<Uuid> = self
            .by_agent
            .get(agent_id)
            .map(|ids| ids.clone())
            .unwrap_or_default();

        let count = ids.len();

        for id in &ids {
            self.memories.remove(id);

            // Remove from index
            let mut index = self.index.write();
            index.remove(id);
        }

        self.by_agent.remove(agent_id);

        Ok(count)
    }
}

/// Statistics about the memory store
#[derive(Debug, Clone)]
pub struct StoreStats {
    pub total_memories: usize,
    pub unique_agents: usize,
    pub avg_memories_per_agent: f32,
    pub max_memories_per_agent: usize,
    pub successful_memories: usize,
}

impl InMemoryStore {
    /// Get statistics about the store
    pub fn stats(&self) -> StoreStats {
        let total = self.memories.len();
        let agents = self.by_agent.len();
        let max_per = self
            .by_agent
            .iter()
            .map(|e| e.len())
            .max()
            .unwrap_or(0);
        let successful = self
            .memories
            .iter()
            .filter(|m| m.is_successful())
            .count();

        StoreStats {
            total_memories: total,
            unique_agents: agents,
            avg_memories_per_agent: if agents > 0 {
                total as f32 / agents as f32
            } else {
                0.0
            },
            max_memories_per_agent: max_per,
            successful_memories: successful,
        }
    }

    /// Get the multi-index for direct queries
    pub fn index(&self) -> &RwLock<MultiIndex> {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::memory::*;

    fn create_test_memory(agent_id: Uuid) -> PraxisMemory {
        PraxisMemory::new(
            agent_id,
            EnvironmentState::new("test"),
            InternalState::new("test"),
            AgentAction::new("test", "test"),
            EnvironmentState::new("result"),
            ActionOutcome::success("ok"),
        )
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = InMemoryStore::new(100);
        let agent_id = Uuid::new_v4();
        let memory = create_test_memory(agent_id);
        let id = memory.id;

        store.store(memory).await.unwrap();

        let retrieved = store.get(&id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_agent_memories() {
        let store = InMemoryStore::new(100);
        let agent_id = Uuid::new_v4();

        for _ in 0..5 {
            store.store(create_test_memory(agent_id)).await.unwrap();
        }

        let memories = store.get_agent_memories(&agent_id).await;
        assert_eq!(memories.len(), 5);
    }

    #[tokio::test]
    async fn test_eviction() {
        let store = InMemoryStore::new(3);
        let agent_id = Uuid::new_v4();

        // Store 5 memories with limit of 3
        for _ in 0..5 {
            store.store(create_test_memory(agent_id)).await.unwrap();
        }

        // Should have at most 3
        let count = store.count_agent_memories(&agent_id).await;
        assert!(count <= 3);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = InMemoryStore::new(100);
        let agent_id = Uuid::new_v4();
        let memory = create_test_memory(agent_id);
        let id = memory.id;

        store.store(memory).await.unwrap();
        store.delete(&id).await.unwrap();

        assert!(store.get(&id).await.is_none());
    }

    #[tokio::test]
    async fn test_clear_agent() {
        let store = InMemoryStore::new(100);
        let agent_id = Uuid::new_v4();

        for _ in 0..5 {
            store.store(create_test_memory(agent_id)).await.unwrap();
        }

        let cleared = store.clear_agent_memories(&agent_id).await.unwrap();
        assert_eq!(cleared, 5);

        let count = store.count_agent_memories(&agent_id).await;
        assert_eq!(count, 0);
    }
}
