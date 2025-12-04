//! Generated protobuf types for PRAXIS service
//!
//! These types are designed to match the proto definitions in proto/actoris/praxis/v1/

pub mod praxis {
    pub mod v1 {
        use prost::{Enumeration, Message};
        use serde::{Deserialize, Serialize};
        use std::collections::HashMap;

        // ============================================================================
        // Core Data Types
        // ============================================================================

        /// Environment state before or after an action
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct EnvironmentState {
            /// Compressed textual representation (e.g., DOM structure)
            #[prost(string, tag = "1")]
            pub textual_repr: String,

            /// Visual feature hash for coarse matching (32 bytes)
            #[prost(bytes = "vec", optional, tag = "2")]
            pub visual_hash: Option<Vec<u8>>,

            /// Key-value state features for IoU matching
            #[prost(map = "string, string", tag = "3")]
            pub state_features: HashMap<String, String>,

            /// Element IDs present in the state
            #[prost(string, repeated, tag = "4")]
            pub element_ids: Vec<String>,

            /// When this state was captured (RFC3339)
            #[prost(int64, tag = "5")]
            pub captured_at: i64,

            /// Optional precomputed embedding
            #[prost(float, repeated, tag = "6")]
            pub embedding: Vec<f32>,
        }

        /// Agent's internal state (goal/directive)
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct InternalState {
            /// High-level goal/directive
            #[prost(string, tag = "1")]
            pub directive: String,

            /// Current sub-task
            #[prost(string, optional, tag = "2")]
            pub sub_task: Option<String>,

            /// Progress indicator (0.0-1.0)
            #[prost(float, tag = "3")]
            pub progress: f32,

            /// Task tags for categorization
            #[prost(string, repeated, tag = "4")]
            pub task_tags: Vec<String>,

            /// Optional precomputed embedding
            #[prost(float, repeated, tag = "5")]
            pub embedding: Vec<f32>,

            /// Additional context key-value pairs
            #[prost(map = "string, string", tag = "6")]
            pub context: HashMap<String, String>,
        }

        /// Action taken by the agent
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct AgentAction {
            /// Type of action (click, type, navigate, etc.)
            #[prost(string, tag = "1")]
            pub action_type: String,

            /// Target element or location
            #[prost(string, optional, tag = "2")]
            pub target: Option<String>,

            /// Action parameters (JSON-encoded values)
            #[prost(map = "string, string", tag = "3")]
            pub parameters: HashMap<String, String>,

            /// Raw action string for display
            #[prost(string, tag = "4")]
            pub raw_action: String,
        }

        /// Outcome of an action
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct ActionOutcome {
            /// Outcome type (oneof)
            #[prost(oneof = "action_outcome::Outcome", tags = "1, 2, 3")]
            pub outcome: Option<action_outcome::Outcome>,
        }

        pub mod action_outcome {
            use super::*;

            #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
            pub struct SuccessOutcome {
                #[prost(string, tag = "1")]
                pub description: String,
            }

            #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
            pub struct FailureOutcome {
                #[prost(string, tag = "1")]
                pub error_code: String,
                #[prost(string, tag = "2")]
                pub description: String,
                #[prost(bool, tag = "3")]
                pub recoverable: bool,
            }

            #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
            pub struct PartialOutcome {
                #[prost(float, tag = "1")]
                pub completion_pct: f32,
                #[prost(string, tag = "2")]
                pub description: String,
            }

            #[derive(Clone, PartialEq, Serialize, Deserialize)]
            pub enum Outcome {
                #[prost(message, tag = "1")]
                Success(SuccessOutcome),
                #[prost(message, tag = "2")]
                Failure(FailureOutcome),
                #[prost(message, tag = "3")]
                Partial(PartialOutcome),
            }
        }

        /// Source of a memory
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration, Serialize, Deserialize)]
        #[repr(i32)]
        pub enum MemorySource {
            Unspecified = 0,
            AgentExperience = 1,
            HumanDemonstration = 2,
            AgentTransfer = 3,
            Synthetic = 4,
        }

        /// A complete procedural memory entry
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct PraxisMemory {
            /// Unique memory ID
            #[prost(string, tag = "1")]
            pub id: String,

            /// Agent that created this memory
            #[prost(string, tag = "2")]
            pub agent_id: String,

            /// Environmental state BEFORE the action
            #[prost(message, optional, tag = "3")]
            pub env_state_pre: Option<EnvironmentState>,

            /// Agent's internal state
            #[prost(message, optional, tag = "4")]
            pub internal_state: Option<InternalState>,

            /// Action taken
            #[prost(message, optional, tag = "5")]
            pub action: Option<AgentAction>,

            /// Environmental state AFTER the action
            #[prost(message, optional, tag = "6")]
            pub env_state_post: Option<EnvironmentState>,

            /// Outcome of the action
            #[prost(message, optional, tag = "7")]
            pub outcome: Option<ActionOutcome>,

            /// When this memory was created
            #[prost(int64, tag = "8")]
            pub created_at: i64,

            /// Number of times retrieved
            #[prost(uint64, tag = "9")]
            pub retrieval_count: u64,

            /// Last retrieval time
            #[prost(int64, optional, tag = "10")]
            pub last_retrieved: Option<i64>,

            /// Reinforcement score
            #[prost(float, tag = "11")]
            pub reinforcement_score: f32,

            /// Source of the memory
            #[prost(enumeration = "MemorySource", tag = "12")]
            pub source: i32,

            /// Link to OutcomeRecord in TrustLedger
            #[prost(string, optional, tag = "13")]
            pub outcome_record_id: Option<String>,
        }

        /// Retrieved memory with relevance scores
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct RetrievedMemory {
            /// The memory
            #[prost(message, optional, tag = "1")]
            pub memory: Option<PraxisMemory>,

            /// Environmental similarity score
            #[prost(float, tag = "2")]
            pub env_similarity: f32,

            /// Internal state similarity score
            #[prost(float, tag = "3")]
            pub internal_similarity: f32,

            /// Combined relevance score
            #[prost(float, tag = "4")]
            pub relevance_score: f32,

            /// Rank in results (1 = most relevant)
            #[prost(uint32, tag = "5")]
            pub rank: u32,
        }

        // ============================================================================
        // Request/Response Types
        // ============================================================================

        /// Store memory request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct StoreMemoryRequest {
            #[prost(string, tag = "1")]
            pub agent_id: String,
            #[prost(message, optional, tag = "2")]
            pub env_state_pre: Option<EnvironmentState>,
            #[prost(message, optional, tag = "3")]
            pub internal_state: Option<InternalState>,
            #[prost(message, optional, tag = "4")]
            pub action: Option<AgentAction>,
            #[prost(message, optional, tag = "5")]
            pub env_state_post: Option<EnvironmentState>,
            #[prost(message, optional, tag = "6")]
            pub outcome: Option<ActionOutcome>,
            #[prost(enumeration = "MemorySource", tag = "7")]
            pub source: i32,
            #[prost(string, optional, tag = "8")]
            pub outcome_record_id: Option<String>,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct StoreMemoryResponse {
            #[prost(string, tag = "1")]
            pub memory_id: String,
            #[prost(int64, tag = "2")]
            pub created_at: i64,
        }

        /// Retrieve memories request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct RetrieveMemoriesRequest {
            #[prost(string, tag = "1")]
            pub agent_id: String,
            #[prost(message, optional, tag = "2")]
            pub current_env: Option<EnvironmentState>,
            #[prost(message, optional, tag = "3")]
            pub current_internal: Option<InternalState>,
            #[prost(uint32, optional, tag = "4")]
            pub max_results: Option<u32>,
            #[prost(float, optional, tag = "5")]
            pub similarity_threshold: Option<f32>,
            #[prost(bool, optional, tag = "6")]
            pub include_global: Option<bool>,
            #[prost(bool, optional, tag = "7")]
            pub successful_only: Option<bool>,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct RetrieveMemoriesResponse {
            #[prost(message, repeated, tag = "1")]
            pub memories: Vec<RetrievedMemory>,
            #[prost(uint32, tag = "2")]
            pub total_searched: u32,
            #[prost(float, tag = "3")]
            pub search_time_ms: f32,
        }

        /// Get memory request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetMemoryRequest {
            #[prost(string, tag = "1")]
            pub memory_id: String,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetMemoryResponse {
            #[prost(message, optional, tag = "1")]
            pub memory: Option<PraxisMemory>,
        }

        /// Update memory request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct UpdateMemoryRequest {
            #[prost(string, tag = "1")]
            pub memory_id: String,
            #[prost(bool, optional, tag = "2")]
            pub record_retrieval: Option<bool>,
            #[prost(float, optional, tag = "3")]
            pub reinforcement_delta: Option<f32>,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct UpdateMemoryResponse {
            #[prost(message, optional, tag = "1")]
            pub memory: Option<PraxisMemory>,
        }

        /// Delete memory request
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct DeleteMemoryRequest {
            #[prost(string, tag = "1")]
            pub memory_id: String,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct DeleteMemoryResponse {
            #[prost(bool, tag = "1")]
            pub deleted: bool,
        }

        // ============================================================================
        // Competence & Metrics Types
        // ============================================================================

        /// Procedural competence metrics
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct ProceduralCompetence {
            #[prost(uint64, tag = "1")]
            pub total_memories: u64,
            #[prost(uint64, tag = "2")]
            pub successful_memories: u64,
            #[prost(float, tag = "3")]
            pub success_rate: f32,
            #[prost(float, tag = "4")]
            pub diversity_score: f32,
            #[prost(float, tag = "5")]
            pub generalization_score: f32,
            #[prost(float, tag = "6")]
            pub learning_velocity: f32,
            #[prost(float, tag = "7")]
            pub retrieval_accuracy: f32,
            #[prost(float, tag = "8")]
            pub memory_utilization: f32,
            #[prost(float, tag = "9")]
            pub fitness_multiplier: f32,
            #[prost(int64, tag = "10")]
            pub calculated_at: i64,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetProceduralCompetenceRequest {
            #[prost(string, tag = "1")]
            pub agent_id: String,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetProceduralCompetenceResponse {
            #[prost(message, optional, tag = "1")]
            pub competence: Option<ProceduralCompetence>,
        }

        /// Competence snapshot for history
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct CompetenceSnapshot {
            #[prost(int64, tag = "1")]
            pub timestamp: i64,
            #[prost(float, tag = "2")]
            pub success_rate: f32,
            #[prost(uint64, tag = "3")]
            pub total_memories: u64,
            #[prost(float, tag = "4")]
            pub fitness_multiplier: f32,
        }

        /// Learning metrics
        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct LearningMetrics {
            #[prost(string, tag = "1")]
            pub agent_id: String,
            #[prost(message, optional, tag = "2")]
            pub current: Option<ProceduralCompetence>,
            #[prost(message, repeated, tag = "3")]
            pub history: Vec<CompetenceSnapshot>,
            #[prost(int32, tag = "4")]
            pub trend: i32,
            #[prost(uint32, tag = "5")]
            pub days_since_improvement: u32,
            #[prost(bool, tag = "6")]
            pub is_actively_learning: bool,
            #[prost(bool, tag = "7")]
            pub should_protect_for_learning: bool,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetLearningMetricsRequest {
            #[prost(string, tag = "1")]
            pub agent_id: String,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetLearningMetricsResponse {
            #[prost(message, optional, tag = "1")]
            pub metrics: Option<LearningMetrics>,
        }

        // ============================================================================
        // Streaming Types
        // ============================================================================

        /// Memory update type enum
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration, Serialize, Deserialize)]
        #[repr(i32)]
        pub enum MemoryUpdateType {
            Unspecified = 0,
            Created = 1,
            Retrieved = 2,
            Deleted = 3,
            Reinforced = 4,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct StreamMemoryUpdatesRequest {
            #[prost(string, tag = "1")]
            pub agent_id: String,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct MemoryUpdate {
            #[prost(string, tag = "1")]
            pub memory_id: String,
            #[prost(string, tag = "2")]
            pub agent_id: String,
            #[prost(enumeration = "MemoryUpdateType", tag = "3")]
            pub update_type: i32,
            #[prost(message, optional, tag = "4")]
            pub memory: Option<PraxisMemory>,
            #[prost(int64, tag = "5")]
            pub timestamp: i64,
        }

        // ============================================================================
        // Import/Export Types
        // ============================================================================

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct DemonstrationEntry {
            #[prost(message, optional, tag = "1")]
            pub env_state_pre: Option<EnvironmentState>,
            #[prost(message, optional, tag = "2")]
            pub internal_state: Option<InternalState>,
            #[prost(message, optional, tag = "3")]
            pub action: Option<AgentAction>,
            #[prost(message, optional, tag = "4")]
            pub env_state_post: Option<EnvironmentState>,
            #[prost(message, optional, tag = "5")]
            pub outcome: Option<ActionOutcome>,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct ImportDemonstrationsRequest {
            #[prost(string, tag = "1")]
            pub agent_id: String,
            #[prost(message, repeated, tag = "2")]
            pub demonstrations: Vec<DemonstrationEntry>,
            #[prost(string, tag = "3")]
            pub demonstrator_id: String,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct ImportDemonstrationsResponse {
            #[prost(uint32, tag = "1")]
            pub imported_count: u32,
            #[prost(string, repeated, tag = "2")]
            pub memory_ids: Vec<String>,
            #[prost(string, repeated, tag = "3")]
            pub failed_reasons: Vec<String>,
        }

        // ============================================================================
        // Action Augmentation Types
        // ============================================================================

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct AugmentedMemory {
            #[prost(uint32, tag = "1")]
            pub rank: u32,
            #[prost(float, tag = "2")]
            pub relevance: f32,
            #[prost(string, tag = "3")]
            pub action: String,
            #[prost(bool, tag = "4")]
            pub was_successful: bool,
            #[prost(string, tag = "5")]
            pub outcome_summary: String,
            #[prost(string, tag = "6")]
            pub directive: String,
            #[prost(uint64, tag = "7")]
            pub usage_count: u64,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct SuggestedAction {
            #[prost(string, tag = "1")]
            pub action: String,
            #[prost(string, tag = "2")]
            pub reasoning: String,
            #[prost(uint32, repeated, tag = "3")]
            pub supporting_memories: Vec<u32>,
            #[prost(float, tag = "4")]
            pub confidence: f32,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct ActionAugmentation {
            #[prost(message, repeated, tag = "1")]
            pub memories: Vec<AugmentedMemory>,
            #[prost(message, optional, tag = "2")]
            pub suggested_action: Option<SuggestedAction>,
            #[prost(float, tag = "3")]
            pub confidence: f32,
            #[prost(string, optional, tag = "4")]
            pub warning: Option<String>,
            #[prost(string, tag = "5")]
            pub context_string: String,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetActionAugmentationRequest {
            #[prost(string, tag = "1")]
            pub agent_id: String,
            #[prost(message, optional, tag = "2")]
            pub current_env: Option<EnvironmentState>,
            #[prost(message, optional, tag = "3")]
            pub current_internal: Option<InternalState>,
            #[prost(uint32, optional, tag = "4")]
            pub max_memories: Option<u32>,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetActionAugmentationResponse {
            #[prost(message, optional, tag = "1")]
            pub augmentation: Option<ActionAugmentation>,
        }

        // ============================================================================
        // Store Statistics
        // ============================================================================

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct StoreStats {
            #[prost(uint64, tag = "1")]
            pub total_memories: u64,
            #[prost(uint32, tag = "2")]
            pub unique_agents: u32,
            #[prost(float, tag = "3")]
            pub avg_memories_per_agent: f32,
            #[prost(uint64, tag = "4")]
            pub max_memories_per_agent: u64,
            #[prost(uint64, tag = "5")]
            pub successful_memories: u64,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetStoreStatsRequest {}

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetStoreStatsResponse {
            #[prost(message, optional, tag = "1")]
            pub stats: Option<StoreStats>,
        }

        // ============================================================================
        // Get Agent Memories
        // ============================================================================

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetAgentMemoriesRequest {
            #[prost(string, tag = "1")]
            pub agent_id: String,
            #[prost(uint32, optional, tag = "2")]
            pub limit: Option<u32>,
            #[prost(uint32, optional, tag = "3")]
            pub offset: Option<u32>,
        }

        #[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
        pub struct GetAgentMemoriesResponse {
            #[prost(message, repeated, tag = "1")]
            pub memories: Vec<PraxisMemory>,
            #[prost(uint32, tag = "2")]
            pub total: u32,
        }
    }
}
