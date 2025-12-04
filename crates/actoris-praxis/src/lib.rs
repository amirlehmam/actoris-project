//! # Actoris PRAXIS
//!
//! Procedural Recall for Agents with eXperiences Indexed by State.
//!
//! PRAXIS is a lightweight post-training learning mechanism that stores the consequences
//! of actions and retrieves them by jointly matching environmental and internal states
//! of past episodes to the current state.
//!
//! ## Key Concepts
//!
//! - **Environmental State**: The state of the environment (e.g., DOM, visual features)
//! - **Internal State**: The agent's goal/directive and progress
//! - **Procedural Memory**: State-action-result exemplars stored for retrieval
//! - **Retrieval Algorithm**: IoU + embedding similarity for state matching
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    PraxisService                        │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
//! │  │   Store     │  │  Retrieve   │  │  Metrics    │     │
//! │  │   Memory    │  │  Memories   │  │  & Stats    │     │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘     │
//! │         │                │                │            │
//! │  ┌──────┴────────────────┴────────────────┴──────┐     │
//! │  │              MemoryRetriever                   │     │
//! │  │  (Algorithm 1: IoU + Embedding Similarity)    │     │
//! │  └──────────────────────┬────────────────────────┘     │
//! │                         │                              │
//! │  ┌──────────────────────┴────────────────────────┐     │
//! │  │              MemoryStore                       │     │
//! │  │  (In-memory + EventStoreDB persistence)       │     │
//! │  └───────────────────────────────────────────────┘     │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## References
//!
//! Based on: "Real-Time Procedural Learning From Experience for AI Agents"
//! (Bi, Hu, Nasir - Altrina, 2025)

pub mod config;
pub mod domain;
pub mod generated;
pub mod grpc;
pub mod infra;

// Re-export core types
pub use domain::memory::{
    EnvironmentState, InternalState, PraxisMemory, PraxisMemoryBuilder, StateFeatures,
};
pub use domain::retrieval::{MemoryRetriever, RetrievalConfig, RetrievedMemory};
pub use domain::competence::{ProceduralCompetence, LearningMetrics};
pub use domain::augmentation::ActionAugmentation;

// Re-export gRPC service
pub use grpc::PraxisGrpcService;

// Re-export generated proto types
pub use generated::praxis::v1 as proto;

// Re-export infrastructure
pub use infra::memory_store::{InMemoryStore, MemoryStore, StoreError};

/// PRAXIS version
pub const PRAXIS_VERSION: &str = "0.1.0";

/// Default retrieval breadth (k parameter from paper)
pub const DEFAULT_RETRIEVAL_BREADTH: usize = 10;

/// Default similarity threshold (τ parameter from paper)
pub const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.3;

/// Maximum memories per agent (prevents unbounded growth)
pub const MAX_MEMORIES_PER_AGENT: usize = 10000;

/// Memory decay half-life in days (for time-based relevance)
pub const MEMORY_DECAY_HALF_LIFE_DAYS: f64 = 30.0;
