//! PRAXIS Memory Types
//!
//! Core data structures for procedural memory storage.
//!
//! Based on the PRAXIS paper, each memory entry contains:
//! - M_env-pre: Environmental state before action
//! - M_int: Internal state (goal/directive)
//! - a_i: Action taken
//! - M_env-post: Environmental state after action

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// State features for IoU matching
///
/// Key-value pairs representing observable state features.
/// The IoU (Intersection over Union) is computed over the keys.
pub type StateFeatures = HashMap<String, String>;

/// Environmental state representation
///
/// Captures the state of the environment at a point in time.
/// Used for both pre-action and post-action states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentState {
    /// Compressed textual representation (e.g., DOM elements, page structure)
    pub textual_repr: String,

    /// Visual feature hash for coarse matching
    pub visual_hash: Option<[u8; 32]>,

    /// Key-value state features for IoU matching
    /// Example: {"page_url": "https://...", "form_visible": "true", "button_count": "3"}
    pub state_features: StateFeatures,

    /// Raw element identifiers present in the state
    pub element_ids: Vec<String>,

    /// Timestamp when state was captured
    pub captured_at: DateTime<Utc>,

    /// Optional embedding vector (precomputed for fast retrieval)
    pub embedding: Option<Vec<f32>>,
}

impl EnvironmentState {
    /// Create a new environment state
    pub fn new(textual_repr: impl Into<String>) -> Self {
        Self {
            textual_repr: textual_repr.into(),
            visual_hash: None,
            state_features: HashMap::new(),
            element_ids: Vec::new(),
            captured_at: Utc::now(),
            embedding: None,
        }
    }

    /// Create environment state with features
    pub fn with_features(textual_repr: impl Into<String>, features: StateFeatures) -> Self {
        Self {
            textual_repr: textual_repr.into(),
            visual_hash: None,
            state_features: features,
            element_ids: Vec::new(),
            captured_at: Utc::now(),
            embedding: None,
        }
    }

    /// Add a state feature
    pub fn add_feature(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.state_features.insert(key.into(), value.into());
    }

    /// Add an element ID
    pub fn add_element(&mut self, element_id: impl Into<String>) {
        self.element_ids.push(element_id.into());
    }

    /// Set the visual hash
    pub fn set_visual_hash(&mut self, hash: [u8; 32]) {
        self.visual_hash = Some(hash);
    }

    /// Set the embedding vector
    pub fn set_embedding(&mut self, embedding: Vec<f32>) {
        self.embedding = Some(embedding);
    }

    /// Get the length of the textual representation (for length overlap calculation)
    pub fn text_length(&self) -> usize {
        self.textual_repr.len()
    }

    /// Get the number of features (for IoU calculation)
    pub fn feature_count(&self) -> usize {
        self.state_features.len()
    }
}

impl Default for EnvironmentState {
    fn default() -> Self {
        Self::new("")
    }
}

/// Agent's internal state
///
/// Represents the agent's goal, progress, and internal context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalState {
    /// High-level goal/directive the agent is trying to achieve
    pub directive: String,

    /// Current sub-task in a multi-step procedure
    pub sub_task: Option<String>,

    /// Progress indicator (0.0 - 1.0)
    pub progress: f32,

    /// Semantic tags for the task type
    pub task_tags: Vec<String>,

    /// Optional embedding of the internal state for similarity matching
    pub embedding: Option<Vec<f32>>,

    /// Additional context key-value pairs
    pub context: HashMap<String, String>,
}

impl InternalState {
    /// Create a new internal state with a directive
    pub fn new(directive: impl Into<String>) -> Self {
        Self {
            directive: directive.into(),
            sub_task: None,
            progress: 0.0,
            task_tags: Vec::new(),
            embedding: None,
            context: HashMap::new(),
        }
    }

    /// Create internal state with sub-task
    pub fn with_sub_task(directive: impl Into<String>, sub_task: impl Into<String>) -> Self {
        Self {
            directive: directive.into(),
            sub_task: Some(sub_task.into()),
            progress: 0.0,
            task_tags: Vec::new(),
            embedding: None,
            context: HashMap::new(),
        }
    }

    /// Set the progress
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    /// Add a task tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.task_tags.push(tag.into());
    }

    /// Set the embedding
    pub fn set_embedding(&mut self, embedding: Vec<f32>) {
        self.embedding = Some(embedding);
    }

    /// Add context
    pub fn add_context(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.context.insert(key.into(), value.into());
    }

    /// Get a combined text representation for embedding
    pub fn as_text(&self) -> String {
        let mut text = self.directive.clone();
        if let Some(sub) = &self.sub_task {
            text.push_str(" | ");
            text.push_str(sub);
        }
        if !self.task_tags.is_empty() {
            text.push_str(" [");
            text.push_str(&self.task_tags.join(", "));
            text.push(']');
        }
        text
    }
}

impl Default for InternalState {
    fn default() -> Self {
        Self::new("")
    }
}

/// Action taken by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    /// Type of action (e.g., "click", "type", "navigate", "scroll")
    pub action_type: String,

    /// Target element or location
    pub target: Option<String>,

    /// Parameters for the action
    pub parameters: HashMap<String, serde_json::Value>,

    /// Raw action string (for display/logging)
    pub raw_action: String,
}

impl AgentAction {
    /// Create a new action
    pub fn new(action_type: impl Into<String>, raw_action: impl Into<String>) -> Self {
        Self {
            action_type: action_type.into(),
            target: None,
            parameters: HashMap::new(),
            raw_action: raw_action.into(),
        }
    }

    /// Create a click action
    pub fn click(target: impl Into<String>) -> Self {
        let target_str = target.into();
        Self {
            action_type: "click".to_string(),
            target: Some(target_str.clone()),
            parameters: HashMap::new(),
            raw_action: format!("click({})", target_str),
        }
    }

    /// Create a type action
    pub fn type_text(target: impl Into<String>, text: impl Into<String>) -> Self {
        let target_str = target.into();
        let text_str = text.into();
        let mut params = HashMap::new();
        params.insert("text".to_string(), serde_json::Value::String(text_str.clone()));
        Self {
            action_type: "type".to_string(),
            target: Some(target_str.clone()),
            parameters: params,
            raw_action: format!("type({}, \"{}\")", target_str, text_str),
        }
    }

    /// Create a navigate action
    pub fn navigate(url: impl Into<String>) -> Self {
        let url_str = url.into();
        let mut params = HashMap::new();
        params.insert("url".to_string(), serde_json::Value::String(url_str.clone()));
        Self {
            action_type: "navigate".to_string(),
            target: None,
            parameters: params,
            raw_action: format!("navigate(\"{}\")", url_str),
        }
    }
}

/// Outcome of the action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionOutcome {
    /// Action succeeded
    Success {
        /// Description of what happened
        description: String,
    },
    /// Action failed
    Failure {
        /// Error code/type
        error_code: String,
        /// Error description
        description: String,
        /// Whether it might succeed on retry
        recoverable: bool,
    },
    /// Action partially succeeded
    Partial {
        /// Completion percentage
        completion_pct: f32,
        /// Description
        description: String,
    },
}

impl ActionOutcome {
    /// Create a success outcome
    pub fn success(description: impl Into<String>) -> Self {
        Self::Success {
            description: description.into(),
        }
    }

    /// Create a failure outcome
    pub fn failure(error_code: impl Into<String>, description: impl Into<String>) -> Self {
        Self::Failure {
            error_code: error_code.into(),
            description: description.into(),
            recoverable: false,
        }
    }

    /// Create a recoverable failure
    pub fn recoverable_failure(error_code: impl Into<String>, description: impl Into<String>) -> Self {
        Self::Failure {
            error_code: error_code.into(),
            description: description.into(),
            recoverable: true,
        }
    }

    /// Check if the outcome was successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if the outcome was a failure
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure { .. })
    }
}

/// PRAXIS Memory Entry
///
/// A complete procedural memory entry containing:
/// - Pre-action environmental state (M_env-pre)
/// - Internal state (M_int)
/// - Action taken (a_i)
/// - Post-action environmental state (M_env-post)
/// - Outcome of the action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PraxisMemory {
    /// Unique memory identifier
    pub id: Uuid,

    /// Agent that created this memory
    pub agent_id: Uuid,

    /// Environmental state BEFORE the action (M_env-pre)
    pub env_state_pre: EnvironmentState,

    /// Agent's internal state at the time (M_int)
    pub internal_state: InternalState,

    /// Action taken (a_i)
    pub action: AgentAction,

    /// Environmental state AFTER the action (M_env-post)
    pub env_state_post: EnvironmentState,

    /// Outcome of the action
    pub outcome: ActionOutcome,

    /// When this memory was created
    pub created_at: DateTime<Utc>,

    /// Number of times this memory has been retrieved
    pub retrieval_count: u64,

    /// Last time this memory was retrieved
    pub last_retrieved: Option<DateTime<Utc>>,

    /// Reinforcement score (increases with successful retrievals)
    pub reinforcement_score: f32,

    /// Source of this memory
    pub source: MemorySource,

    /// Optional link to related OutcomeRecord in TrustLedger
    pub outcome_record_id: Option<Uuid>,
}

impl PraxisMemory {
    /// Create a new PRAXIS memory entry
    pub fn new(
        agent_id: Uuid,
        env_state_pre: EnvironmentState,
        internal_state: InternalState,
        action: AgentAction,
        env_state_post: EnvironmentState,
        outcome: ActionOutcome,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            env_state_pre,
            internal_state,
            action,
            env_state_post,
            outcome,
            created_at: Utc::now(),
            retrieval_count: 0,
            last_retrieved: None,
            reinforcement_score: 1.0,
            source: MemorySource::AgentExperience,
            outcome_record_id: None,
        }
    }

    /// Create a builder for PraxisMemory
    pub fn builder(agent_id: Uuid) -> PraxisMemoryBuilder {
        PraxisMemoryBuilder::new(agent_id)
    }

    /// Check if this memory represents a successful action
    pub fn is_successful(&self) -> bool {
        self.outcome.is_success()
    }

    /// Record that this memory was retrieved
    pub fn record_retrieval(&mut self) {
        self.retrieval_count += 1;
        self.last_retrieved = Some(Utc::now());
        // Reinforce successful memories that get retrieved
        if self.is_successful() {
            self.reinforcement_score *= 1.1;
            self.reinforcement_score = self.reinforcement_score.min(10.0);
        }
    }

    /// Calculate age in days
    pub fn age_days(&self) -> f64 {
        let duration = Utc::now() - self.created_at;
        duration.num_seconds() as f64 / 86400.0
    }

    /// Calculate time-decayed relevance
    pub fn time_decay_factor(&self, half_life_days: f64) -> f32 {
        let age = self.age_days();
        (0.5_f64.powf(age / half_life_days)) as f32
    }
}

/// Source of a memory entry
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemorySource {
    /// Memory from agent's own experience
    AgentExperience,
    /// Memory from human demonstration
    HumanDemonstration,
    /// Memory imported from another agent
    AgentTransfer,
    /// Memory synthesized/generated
    Synthetic,
}

/// Builder for PraxisMemory
#[derive(Debug)]
pub struct PraxisMemoryBuilder {
    agent_id: Uuid,
    env_state_pre: Option<EnvironmentState>,
    internal_state: Option<InternalState>,
    action: Option<AgentAction>,
    env_state_post: Option<EnvironmentState>,
    outcome: Option<ActionOutcome>,
    source: MemorySource,
    outcome_record_id: Option<Uuid>,
}

impl PraxisMemoryBuilder {
    /// Create a new builder
    pub fn new(agent_id: Uuid) -> Self {
        Self {
            agent_id,
            env_state_pre: None,
            internal_state: None,
            action: None,
            env_state_post: None,
            outcome: None,
            source: MemorySource::AgentExperience,
            outcome_record_id: None,
        }
    }

    /// Set the pre-action environment state
    pub fn env_pre(mut self, state: EnvironmentState) -> Self {
        self.env_state_pre = Some(state);
        self
    }

    /// Set the internal state
    pub fn internal(mut self, state: InternalState) -> Self {
        self.internal_state = Some(state);
        self
    }

    /// Set the action
    pub fn action(mut self, action: AgentAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Set the post-action environment state
    pub fn env_post(mut self, state: EnvironmentState) -> Self {
        self.env_state_post = Some(state);
        self
    }

    /// Set the outcome
    pub fn outcome(mut self, outcome: ActionOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// Set the source
    pub fn source(mut self, source: MemorySource) -> Self {
        self.source = source;
        self
    }

    /// Link to an OutcomeRecord
    pub fn outcome_record(mut self, record_id: Uuid) -> Self {
        self.outcome_record_id = Some(record_id);
        self
    }

    /// Build the PraxisMemory
    pub fn build(self) -> Result<PraxisMemory, &'static str> {
        let env_state_pre = self.env_state_pre.ok_or("env_state_pre is required")?;
        let internal_state = self.internal_state.ok_or("internal_state is required")?;
        let action = self.action.ok_or("action is required")?;
        let env_state_post = self.env_state_post.ok_or("env_state_post is required")?;
        let outcome = self.outcome.ok_or("outcome is required")?;

        let mut memory = PraxisMemory::new(
            self.agent_id,
            env_state_pre,
            internal_state,
            action,
            env_state_post,
            outcome,
        );
        memory.source = self.source;
        memory.outcome_record_id = self.outcome_record_id;

        Ok(memory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_state_creation() {
        let mut state = EnvironmentState::new("Login page with form");
        state.add_feature("page_url", "https://example.com/login");
        state.add_feature("form_visible", "true");
        state.add_element("username_input");
        state.add_element("password_input");

        assert_eq!(state.feature_count(), 2);
        assert_eq!(state.element_ids.len(), 2);
    }

    #[test]
    fn test_internal_state_as_text() {
        let mut state = InternalState::new("Complete user registration");
        state.sub_task = Some("Fill in email field".to_string());
        state.add_tag("registration");
        state.add_tag("form-filling");

        let text = state.as_text();
        assert!(text.contains("Complete user registration"));
        assert!(text.contains("Fill in email field"));
        assert!(text.contains("registration"));
    }

    #[test]
    fn test_praxis_memory_builder() {
        let agent_id = Uuid::new_v4();

        let memory = PraxisMemory::builder(agent_id)
            .env_pre(EnvironmentState::new("Before state"))
            .internal(InternalState::new("Test directive"))
            .action(AgentAction::click("button#submit"))
            .env_post(EnvironmentState::new("After state"))
            .outcome(ActionOutcome::success("Button clicked"))
            .build();

        assert!(memory.is_ok());
        let memory = memory.unwrap();
        assert!(memory.is_successful());
        assert_eq!(memory.agent_id, agent_id);
    }

    #[test]
    fn test_memory_retrieval_reinforcement() {
        let agent_id = Uuid::new_v4();
        let mut memory = PraxisMemory::new(
            agent_id,
            EnvironmentState::default(),
            InternalState::default(),
            AgentAction::new("test", "test"),
            EnvironmentState::default(),
            ActionOutcome::success("ok"),
        );

        let initial_score = memory.reinforcement_score;
        memory.record_retrieval();

        assert!(memory.reinforcement_score > initial_score);
        assert_eq!(memory.retrieval_count, 1);
        assert!(memory.last_retrieved.is_some());
    }

    #[test]
    fn test_time_decay() {
        let agent_id = Uuid::new_v4();
        let memory = PraxisMemory::new(
            agent_id,
            EnvironmentState::default(),
            InternalState::default(),
            AgentAction::new("test", "test"),
            EnvironmentState::default(),
            ActionOutcome::success("ok"),
        );

        // Fresh memory should have decay factor close to 1.0
        let decay = memory.time_decay_factor(30.0);
        assert!(decay > 0.99);
    }
}
