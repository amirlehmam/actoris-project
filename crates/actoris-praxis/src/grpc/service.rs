//! PRAXIS gRPC service implementation
//!
//! Implements the PraxisService from proto/actoris/praxis/v1/praxis.proto

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::domain::augmentation::ActionAugmentation as DomainAugmentation;
use crate::domain::competence::{
    CompetenceSnapshot as DomainSnapshot, LearningMetrics as DomainLearningMetrics,
    ProceduralCompetence as DomainCompetence,
};
use crate::domain::memory::{
    ActionOutcome as DomainOutcome, AgentAction as DomainAction,
    EnvironmentState as DomainEnvState, InternalState as DomainInternal,
    MemorySource as DomainSource, PraxisMemory as DomainMemory,
};
use crate::domain::retrieval::{MemoryRetriever, RetrievalConfig, RetrievedMemory as DomainRetrieved};
use crate::generated::praxis::v1 as proto;
use crate::infra::memory_store::{InMemoryStore, MemoryStore, StoreStats};

/// PRAXIS gRPC service handler
pub struct PraxisGrpcService {
    /// Memory store (public for REST API access)
    pub store: Arc<InMemoryStore>,
    /// Memory retriever
    retriever: Arc<RwLock<MemoryRetriever>>,
    /// Learning metrics per agent
    learning_metrics: Arc<RwLock<HashMap<Uuid, DomainLearningMetrics>>>,
    /// Update subscribers per agent
    subscribers: Arc<RwLock<HashMap<Uuid, Vec<mpsc::Sender<proto::MemoryUpdate>>>>>,
}

impl PraxisGrpcService {
    /// Create a new PRAXIS gRPC service
    pub fn new(max_memories_per_agent: usize) -> Self {
        let store = Arc::new(InMemoryStore::new(max_memories_per_agent));
        let retriever = Arc::new(RwLock::new(MemoryRetriever::new(
            store.clone(),
            RetrievalConfig::default(),
        )));

        Self {
            store,
            retriever,
            learning_metrics: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with custom retrieval config
    pub fn with_config(max_memories_per_agent: usize, retrieval_config: RetrievalConfig) -> Self {
        let store = Arc::new(InMemoryStore::new(max_memories_per_agent));
        let retriever = Arc::new(RwLock::new(MemoryRetriever::new(
            store.clone(),
            retrieval_config,
        )));

        Self {
            store,
            retriever,
            learning_metrics: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Broadcast a memory update to subscribers
    async fn broadcast_update(&self, agent_id: Uuid, update: proto::MemoryUpdate) {
        let subs = self.subscribers.read().await;
        if let Some(channels) = subs.get(&agent_id) {
            for tx in channels {
                let _ = tx.send(update.clone()).await;
            }
        }
    }

    /// Convert domain memory to proto
    fn memory_to_proto(memory: &DomainMemory) -> proto::PraxisMemory {
        proto::PraxisMemory {
            id: memory.id.to_string(),
            agent_id: memory.agent_id.to_string(),
            env_state_pre: Some(Self::env_state_to_proto(&memory.env_state_pre)),
            internal_state: Some(Self::internal_state_to_proto(&memory.internal_state)),
            action: Some(Self::action_to_proto(&memory.action)),
            env_state_post: Some(Self::env_state_to_proto(&memory.env_state_post)),
            outcome: Some(Self::outcome_to_proto(&memory.outcome)),
            created_at: memory.created_at.timestamp_millis(),
            retrieval_count: memory.retrieval_count,
            last_retrieved: memory.last_retrieved.map(|t| t.timestamp_millis()),
            reinforcement_score: memory.reinforcement_score,
            source: Self::source_to_proto(&memory.source),
            outcome_record_id: memory.outcome_record_id.map(|id| id.to_string()),
        }
    }

    fn env_state_to_proto(state: &DomainEnvState) -> proto::EnvironmentState {
        proto::EnvironmentState {
            textual_repr: state.textual_repr.clone(),
            visual_hash: state.visual_hash.map(|h| h.to_vec()),
            state_features: state.state_features.clone(),
            element_ids: state.element_ids.clone(),
            captured_at: state.captured_at.timestamp_millis(),
            embedding: state.embedding.clone().unwrap_or_default(),
        }
    }

    fn internal_state_to_proto(state: &DomainInternal) -> proto::InternalState {
        proto::InternalState {
            directive: state.directive.clone(),
            sub_task: state.sub_task.clone(),
            progress: state.progress,
            task_tags: state.task_tags.clone(),
            embedding: state.embedding.clone().unwrap_or_default(),
            context: state.context.clone(),
        }
    }

    fn action_to_proto(action: &DomainAction) -> proto::AgentAction {
        proto::AgentAction {
            action_type: action.action_type.clone(),
            target: action.target.clone(),
            parameters: action
                .parameters
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
            raw_action: action.raw_action.clone(),
        }
    }

    fn outcome_to_proto(outcome: &DomainOutcome) -> proto::ActionOutcome {
        let outcome_type = match outcome {
            DomainOutcome::Success { description } => {
                Some(proto::action_outcome::Outcome::Success(
                    proto::action_outcome::SuccessOutcome {
                        description: description.clone(),
                    },
                ))
            }
            DomainOutcome::Failure {
                error_code,
                description,
                recoverable,
            } => Some(proto::action_outcome::Outcome::Failure(
                proto::action_outcome::FailureOutcome {
                    error_code: error_code.clone(),
                    description: description.clone(),
                    recoverable: *recoverable,
                },
            )),
            DomainOutcome::Partial {
                completion_pct,
                description,
            } => Some(proto::action_outcome::Outcome::Partial(
                proto::action_outcome::PartialOutcome {
                    completion_pct: *completion_pct,
                    description: description.clone(),
                },
            )),
        };

        proto::ActionOutcome {
            outcome: outcome_type,
        }
    }

    fn source_to_proto(source: &DomainSource) -> i32 {
        match source {
            DomainSource::AgentExperience => proto::MemorySource::AgentExperience as i32,
            DomainSource::HumanDemonstration => proto::MemorySource::HumanDemonstration as i32,
            DomainSource::AgentTransfer => proto::MemorySource::AgentTransfer as i32,
            DomainSource::Synthetic => proto::MemorySource::Synthetic as i32,
        }
    }

    /// Convert proto to domain types
    fn proto_to_env_state(state: &proto::EnvironmentState) -> DomainEnvState {
        let mut env = DomainEnvState::with_features(
            state.textual_repr.clone(),
            state.state_features.clone(),
        );
        env.element_ids = state.element_ids.clone();
        if !state.embedding.is_empty() {
            env.embedding = Some(state.embedding.clone());
        }
        if let Some(hash) = &state.visual_hash {
            if hash.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(hash);
                env.visual_hash = Some(arr);
            }
        }
        env
    }

    fn proto_to_internal_state(state: &proto::InternalState) -> DomainInternal {
        let mut internal = if let Some(sub) = &state.sub_task {
            DomainInternal::with_sub_task(&state.directive, sub)
        } else {
            DomainInternal::new(&state.directive)
        };
        internal.progress = state.progress;
        internal.task_tags = state.task_tags.clone();
        internal.context = state.context.clone();
        if !state.embedding.is_empty() {
            internal.embedding = Some(state.embedding.clone());
        }
        internal
    }

    fn proto_to_action(action: &proto::AgentAction) -> DomainAction {
        DomainAction {
            action_type: action.action_type.clone(),
            target: action.target.clone(),
            parameters: action
                .parameters
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect(),
            raw_action: action.raw_action.clone(),
        }
    }

    fn proto_to_outcome(outcome: &proto::ActionOutcome) -> DomainOutcome {
        match &outcome.outcome {
            Some(proto::action_outcome::Outcome::Success(s)) => {
                DomainOutcome::success(&s.description)
            }
            Some(proto::action_outcome::Outcome::Failure(f)) => {
                if f.recoverable {
                    DomainOutcome::recoverable_failure(&f.error_code, &f.description)
                } else {
                    DomainOutcome::failure(&f.error_code, &f.description)
                }
            }
            Some(proto::action_outcome::Outcome::Partial(p)) => DomainOutcome::Partial {
                completion_pct: p.completion_pct,
                description: p.description.clone(),
            },
            None => DomainOutcome::failure("UNKNOWN", "No outcome provided"),
        }
    }

    fn proto_to_source(source: i32) -> DomainSource {
        match proto::MemorySource::try_from(source) {
            Ok(proto::MemorySource::AgentExperience) => DomainSource::AgentExperience,
            Ok(proto::MemorySource::HumanDemonstration) => DomainSource::HumanDemonstration,
            Ok(proto::MemorySource::AgentTransfer) => DomainSource::AgentTransfer,
            Ok(proto::MemorySource::Synthetic) => DomainSource::Synthetic,
            _ => DomainSource::AgentExperience,
        }
    }

    fn competence_to_proto(comp: &DomainCompetence) -> proto::ProceduralCompetence {
        proto::ProceduralCompetence {
            total_memories: comp.total_memories,
            successful_memories: comp.successful_memories,
            success_rate: comp.success_rate,
            diversity_score: comp.diversity_score,
            generalization_score: comp.generalization_score,
            learning_velocity: comp.learning_velocity,
            retrieval_accuracy: comp.retrieval_accuracy,
            memory_utilization: comp.memory_utilization,
            fitness_multiplier: comp.fitness_multiplier(),
            calculated_at: comp.calculated_at.timestamp_millis(),
        }
    }

    fn snapshot_to_proto(snap: &DomainSnapshot) -> proto::CompetenceSnapshot {
        proto::CompetenceSnapshot {
            timestamp: snap.timestamp.timestamp_millis(),
            success_rate: snap.success_rate,
            total_memories: snap.total_memories,
            fitness_multiplier: snap.fitness_multiplier,
        }
    }
}

// ============================================================================
// Service Trait Implementation
// ============================================================================

#[tonic::async_trait]
impl PraxisService for PraxisGrpcService {
    /// Store a new procedural memory
    #[instrument(skip(self, request))]
    async fn store_memory(
        &self,
        request: Request<proto::StoreMemoryRequest>,
    ) -> Result<Response<proto::StoreMemoryResponse>, Status> {
        let req = request.into_inner();

        let agent_id = Uuid::parse_str(&req.agent_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid agent_id: {}", e)))?;

        let env_pre = req
            .env_state_pre
            .as_ref()
            .map(Self::proto_to_env_state)
            .ok_or_else(|| Status::invalid_argument("env_state_pre is required"))?;

        let internal = req
            .internal_state
            .as_ref()
            .map(Self::proto_to_internal_state)
            .ok_or_else(|| Status::invalid_argument("internal_state is required"))?;

        let action = req
            .action
            .as_ref()
            .map(Self::proto_to_action)
            .ok_or_else(|| Status::invalid_argument("action is required"))?;

        let env_post = req
            .env_state_post
            .as_ref()
            .map(Self::proto_to_env_state)
            .ok_or_else(|| Status::invalid_argument("env_state_post is required"))?;

        let outcome = req
            .outcome
            .as_ref()
            .map(Self::proto_to_outcome)
            .ok_or_else(|| Status::invalid_argument("outcome is required"))?;

        let source = Self::proto_to_source(req.source);

        let mut memory = DomainMemory::new(agent_id, env_pre, internal, action, env_post, outcome);
        memory.source = source;
        if let Some(record_id) = &req.outcome_record_id {
            memory.outcome_record_id = Uuid::parse_str(record_id).ok();
        }

        let memory_id = self
            .store
            .store(memory.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to store memory: {}", e)))?;

        info!(memory_id = %memory_id, agent_id = %agent_id, "Memory stored");

        // Broadcast update
        self.broadcast_update(
            agent_id,
            proto::MemoryUpdate {
                memory_id: memory_id.to_string(),
                agent_id: agent_id.to_string(),
                update_type: proto::MemoryUpdateType::Created as i32,
                memory: Some(Self::memory_to_proto(&memory)),
                timestamp: Utc::now().timestamp_millis(),
            },
        )
        .await;

        Ok(Response::new(proto::StoreMemoryResponse {
            memory_id: memory_id.to_string(),
            created_at: memory.created_at.timestamp_millis(),
        }))
    }

    /// Retrieve relevant memories for action selection
    #[instrument(skip(self, request))]
    async fn retrieve_memories(
        &self,
        request: Request<proto::RetrieveMemoriesRequest>,
    ) -> Result<Response<proto::RetrieveMemoriesResponse>, Status> {
        let req = request.into_inner();
        let start = Instant::now();

        let agent_id = Uuid::parse_str(&req.agent_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid agent_id: {}", e)))?;

        let current_env = req
            .current_env
            .as_ref()
            .map(Self::proto_to_env_state)
            .ok_or_else(|| Status::invalid_argument("current_env is required"))?;

        let current_internal = req
            .current_internal
            .as_ref()
            .map(Self::proto_to_internal_state)
            .ok_or_else(|| Status::invalid_argument("current_internal is required"))?;

        let retriever = self.retriever.read().await;
        let total_searched = self.store.count_agent_memories(&agent_id).await;

        let results = if req.include_global.unwrap_or(false) {
            retriever
                .retrieve_global(
                    &current_env,
                    &current_internal,
                    req.max_results.unwrap_or(10) as usize,
                )
                .await
        } else {
            retriever
                .retrieve(&agent_id, &current_env, &current_internal)
                .await
        };

        // Filter by successful_only if requested
        let results: Vec<_> = if req.successful_only.unwrap_or(false) {
            results.into_iter().filter(|r| r.is_successful()).collect()
        } else {
            results
        };

        // Apply max_results limit
        let max_results = req.max_results.unwrap_or(10) as usize;
        let results: Vec<_> = results.into_iter().take(max_results).collect();

        // Record retrievals
        for result in &results {
            let mut memory = result.memory.clone();
            memory.record_retrieval();
            let _ = self.store.update(memory).await;

            // Broadcast retrieval update
            self.broadcast_update(
                agent_id,
                proto::MemoryUpdate {
                    memory_id: result.memory.id.to_string(),
                    agent_id: agent_id.to_string(),
                    update_type: proto::MemoryUpdateType::Retrieved as i32,
                    memory: None,
                    timestamp: Utc::now().timestamp_millis(),
                },
            )
            .await;
        }

        let search_time = start.elapsed().as_secs_f32() * 1000.0;

        debug!(
            agent_id = %agent_id,
            results = results.len(),
            searched = total_searched,
            time_ms = search_time,
            "Memories retrieved"
        );

        Ok(Response::new(proto::RetrieveMemoriesResponse {
            memories: results
                .iter()
                .map(|r| proto::RetrievedMemory {
                    memory: Some(Self::memory_to_proto(&r.memory)),
                    env_similarity: r.env_similarity,
                    internal_similarity: r.internal_similarity,
                    relevance_score: r.relevance_score,
                    rank: r.rank as u32 + 1,
                })
                .collect(),
            total_searched: total_searched as u32,
            search_time_ms: search_time,
        }))
    }

    /// Get a specific memory by ID
    #[instrument(skip(self, request))]
    async fn get_memory(
        &self,
        request: Request<proto::GetMemoryRequest>,
    ) -> Result<Response<proto::GetMemoryResponse>, Status> {
        let req = request.into_inner();

        let memory_id = Uuid::parse_str(&req.memory_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid memory_id: {}", e)))?;

        let memory = self.store.get(&memory_id).await;

        Ok(Response::new(proto::GetMemoryResponse {
            memory: memory.map(|m| Self::memory_to_proto(&m)),
        }))
    }

    /// Update a memory (e.g., record retrieval)
    #[instrument(skip(self, request))]
    async fn update_memory(
        &self,
        request: Request<proto::UpdateMemoryRequest>,
    ) -> Result<Response<proto::UpdateMemoryResponse>, Status> {
        let req = request.into_inner();

        let memory_id = Uuid::parse_str(&req.memory_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid memory_id: {}", e)))?;

        let mut memory = self
            .store
            .get(&memory_id)
            .await
            .ok_or_else(|| Status::not_found("Memory not found"))?;

        if req.record_retrieval.unwrap_or(false) {
            memory.record_retrieval();
        }

        if let Some(delta) = req.reinforcement_delta {
            memory.reinforcement_score = (memory.reinforcement_score + delta).clamp(0.1, 10.0);
        }

        self.store
            .update(memory.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to update: {}", e)))?;

        // Broadcast update
        self.broadcast_update(
            memory.agent_id,
            proto::MemoryUpdate {
                memory_id: memory_id.to_string(),
                agent_id: memory.agent_id.to_string(),
                update_type: proto::MemoryUpdateType::Reinforced as i32,
                memory: Some(Self::memory_to_proto(&memory)),
                timestamp: Utc::now().timestamp_millis(),
            },
        )
        .await;

        Ok(Response::new(proto::UpdateMemoryResponse {
            memory: Some(Self::memory_to_proto(&memory)),
        }))
    }

    /// Delete a memory
    #[instrument(skip(self, request))]
    async fn delete_memory(
        &self,
        request: Request<proto::DeleteMemoryRequest>,
    ) -> Result<Response<proto::DeleteMemoryResponse>, Status> {
        let req = request.into_inner();

        let memory_id = Uuid::parse_str(&req.memory_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid memory_id: {}", e)))?;

        // Get agent_id before deletion for broadcast
        let agent_id = self.store.get(&memory_id).await.map(|m| m.agent_id);

        let deleted = self.store.delete(&memory_id).await.is_ok();

        if deleted {
            if let Some(agent_id) = agent_id {
                self.broadcast_update(
                    agent_id,
                    proto::MemoryUpdate {
                        memory_id: memory_id.to_string(),
                        agent_id: agent_id.to_string(),
                        update_type: proto::MemoryUpdateType::Deleted as i32,
                        memory: None,
                        timestamp: Utc::now().timestamp_millis(),
                    },
                )
                .await;
            }
        }

        Ok(Response::new(proto::DeleteMemoryResponse { deleted }))
    }

    /// Get agent's procedural competence metrics
    #[instrument(skip(self, request))]
    async fn get_procedural_competence(
        &self,
        request: Request<proto::GetProceduralCompetenceRequest>,
    ) -> Result<Response<proto::GetProceduralCompetenceResponse>, Status> {
        let req = request.into_inner();

        let agent_id = Uuid::parse_str(&req.agent_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid agent_id: {}", e)))?;

        let memories = self.store.get_agent_memories(&agent_id).await;
        let competence = DomainCompetence::from_memories(&memories);

        Ok(Response::new(proto::GetProceduralCompetenceResponse {
            competence: Some(Self::competence_to_proto(&competence)),
        }))
    }

    /// Get learning metrics over time
    #[instrument(skip(self, request))]
    async fn get_learning_metrics(
        &self,
        request: Request<proto::GetLearningMetricsRequest>,
    ) -> Result<Response<proto::GetLearningMetricsResponse>, Status> {
        let req = request.into_inner();

        let agent_id = Uuid::parse_str(&req.agent_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid agent_id: {}", e)))?;

        // Calculate current competence
        let memories = self.store.get_agent_memories(&agent_id).await;
        let competence = DomainCompetence::from_memories(&memories);

        // Get or create learning metrics
        let mut metrics_map = self.learning_metrics.write().await;
        let metrics = metrics_map
            .entry(agent_id)
            .or_insert_with(|| DomainLearningMetrics::new(agent_id));

        // Update with current competence
        metrics.update(competence.clone());

        Ok(Response::new(proto::GetLearningMetricsResponse {
            metrics: Some(proto::LearningMetrics {
                agent_id: agent_id.to_string(),
                current: Some(Self::competence_to_proto(&metrics.current)),
                history: metrics.history.iter().map(Self::snapshot_to_proto).collect(),
                trend: metrics.trend as i32,
                days_since_improvement: metrics.days_since_improvement,
                is_actively_learning: metrics.is_actively_learning(),
                should_protect_for_learning: metrics.should_protect_for_learning(),
            }),
        }))
    }

    type StreamMemoryUpdatesStream =
        Pin<Box<dyn Stream<Item = Result<proto::MemoryUpdate, Status>> + Send>>;

    /// Stream memory updates for an agent
    #[instrument(skip(self, request))]
    async fn stream_memory_updates(
        &self,
        request: Request<proto::StreamMemoryUpdatesRequest>,
    ) -> Result<Response<Self::StreamMemoryUpdatesStream>, Status> {
        let req = request.into_inner();

        let agent_id = Uuid::parse_str(&req.agent_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid agent_id: {}", e)))?;

        let (tx, rx) = mpsc::channel(100);

        // Register subscriber
        {
            let mut subs = self.subscribers.write().await;
            subs.entry(agent_id).or_default().push(tx);
        }

        let stream = ReceiverStream::new(rx).map(Ok);
        Ok(Response::new(Box::pin(stream)))
    }

    /// Batch import memories (for human demonstrations)
    #[instrument(skip(self, request))]
    async fn import_demonstrations(
        &self,
        request: Request<proto::ImportDemonstrationsRequest>,
    ) -> Result<Response<proto::ImportDemonstrationsResponse>, Status> {
        let req = request.into_inner();

        let agent_id = Uuid::parse_str(&req.agent_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid agent_id: {}", e)))?;

        let mut imported_count = 0u32;
        let mut memory_ids = Vec::new();
        let mut failed_reasons = Vec::new();

        for (i, demo) in req.demonstrations.iter().enumerate() {
            let result = (|| -> Result<Uuid, String> {
                let env_pre = demo
                    .env_state_pre
                    .as_ref()
                    .map(Self::proto_to_env_state)
                    .ok_or("env_state_pre is required")?;

                let internal = demo
                    .internal_state
                    .as_ref()
                    .map(Self::proto_to_internal_state)
                    .ok_or("internal_state is required")?;

                let action = demo
                    .action
                    .as_ref()
                    .map(Self::proto_to_action)
                    .ok_or("action is required")?;

                let env_post = demo
                    .env_state_post
                    .as_ref()
                    .map(Self::proto_to_env_state)
                    .ok_or("env_state_post is required")?;

                let outcome = demo
                    .outcome
                    .as_ref()
                    .map(Self::proto_to_outcome)
                    .ok_or("outcome is required")?;

                let mut memory =
                    DomainMemory::new(agent_id, env_pre, internal, action, env_post, outcome);
                memory.source = DomainSource::HumanDemonstration;

                Ok(memory.id)
            })();

            match result {
                Ok(id) => {
                    // Build and store the memory
                    let demo = &req.demonstrations[i];
                    let env_pre = Self::proto_to_env_state(demo.env_state_pre.as_ref().unwrap());
                    let internal =
                        Self::proto_to_internal_state(demo.internal_state.as_ref().unwrap());
                    let action = Self::proto_to_action(demo.action.as_ref().unwrap());
                    let env_post = Self::proto_to_env_state(demo.env_state_post.as_ref().unwrap());
                    let outcome = Self::proto_to_outcome(demo.outcome.as_ref().unwrap());

                    let mut memory =
                        DomainMemory::new(agent_id, env_pre, internal, action, env_post, outcome);
                    memory.source = DomainSource::HumanDemonstration;

                    match self.store.store(memory).await {
                        Ok(stored_id) => {
                            memory_ids.push(stored_id.to_string());
                            imported_count += 1;
                        }
                        Err(e) => {
                            failed_reasons.push(format!("Demo {}: {}", i, e));
                        }
                    }
                }
                Err(reason) => {
                    failed_reasons.push(format!("Demo {}: {}", i, reason));
                }
            }
        }

        info!(
            agent_id = %agent_id,
            imported = imported_count,
            failed = failed_reasons.len(),
            "Demonstrations imported"
        );

        Ok(Response::new(proto::ImportDemonstrationsResponse {
            imported_count,
            memory_ids,
            failed_reasons,
        }))
    }

    /// Get action augmentation for current state
    #[instrument(skip(self, request))]
    async fn get_action_augmentation(
        &self,
        request: Request<proto::GetActionAugmentationRequest>,
    ) -> Result<Response<proto::GetActionAugmentationResponse>, Status> {
        let req = request.into_inner();

        let agent_id = Uuid::parse_str(&req.agent_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid agent_id: {}", e)))?;

        let current_env = req
            .current_env
            .as_ref()
            .map(Self::proto_to_env_state)
            .ok_or_else(|| Status::invalid_argument("current_env is required"))?;

        let current_internal = req
            .current_internal
            .as_ref()
            .map(Self::proto_to_internal_state)
            .ok_or_else(|| Status::invalid_argument("current_internal is required"))?;

        let max_memories = req.max_memories.unwrap_or(5) as usize;

        // Retrieve memories
        let retriever = self.retriever.read().await;
        let results = retriever
            .retrieve(&agent_id, &current_env, &current_internal)
            .await;
        let results: Vec<_> = results.into_iter().take(max_memories).collect();

        // Build augmentation
        let augmentation = DomainAugmentation::from_retrieved(results);

        // Compute context_string before moving fields
        let context_string = augmentation.as_context_string();

        Ok(Response::new(proto::GetActionAugmentationResponse {
            augmentation: Some(proto::ActionAugmentation {
                memories: augmentation
                    .memories
                    .iter()
                    .map(|m| proto::AugmentedMemory {
                        rank: m.rank as u32,
                        relevance: m.relevance,
                        action: m.action.clone(),
                        was_successful: m.was_successful,
                        outcome_summary: m.outcome_summary.clone(),
                        directive: m.directive.clone(),
                        usage_count: m.usage_count,
                    })
                    .collect(),
                suggested_action: augmentation.suggested_action.map(|s| proto::SuggestedAction {
                    action: s.action,
                    reasoning: s.reasoning,
                    supporting_memories: s.supporting_memories.iter().map(|&i| i as u32).collect(),
                    confidence: s.confidence,
                }),
                confidence: augmentation.confidence,
                warning: augmentation.warning,
                context_string,
            }),
        }))
    }

    /// Get store statistics
    #[instrument(skip(self, _request))]
    async fn get_store_stats(
        &self,
        _request: Request<proto::GetStoreStatsRequest>,
    ) -> Result<Response<proto::GetStoreStatsResponse>, Status> {
        let stats = self.store.stats();

        Ok(Response::new(proto::GetStoreStatsResponse {
            stats: Some(proto::StoreStats {
                total_memories: stats.total_memories as u64,
                unique_agents: stats.unique_agents as u32,
                avg_memories_per_agent: stats.avg_memories_per_agent,
                max_memories_per_agent: stats.max_memories_per_agent as u64,
                successful_memories: stats.successful_memories as u64,
            }),
        }))
    }

    /// Get all memories for an agent
    #[instrument(skip(self, request))]
    async fn get_agent_memories(
        &self,
        request: Request<proto::GetAgentMemoriesRequest>,
    ) -> Result<Response<proto::GetAgentMemoriesResponse>, Status> {
        let req = request.into_inner();

        let agent_id = Uuid::parse_str(&req.agent_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid agent_id: {}", e)))?;

        let memories = self.store.get_agent_memories(&agent_id).await;
        let total = memories.len() as u32;

        // Apply pagination
        let offset = req.offset.unwrap_or(0) as usize;
        let limit = req.limit.unwrap_or(50) as usize;

        let memories: Vec<_> = memories
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|m| Self::memory_to_proto(&m))
            .collect();

        Ok(Response::new(proto::GetAgentMemoriesResponse {
            memories,
            total,
        }))
    }
}

// ============================================================================
// Service Trait Definition
// ============================================================================

/// PraxisService trait (matching proto definition)
#[tonic::async_trait]
pub trait PraxisService: Send + Sync + 'static {
    async fn store_memory(
        &self,
        request: Request<proto::StoreMemoryRequest>,
    ) -> Result<Response<proto::StoreMemoryResponse>, Status>;

    async fn retrieve_memories(
        &self,
        request: Request<proto::RetrieveMemoriesRequest>,
    ) -> Result<Response<proto::RetrieveMemoriesResponse>, Status>;

    async fn get_memory(
        &self,
        request: Request<proto::GetMemoryRequest>,
    ) -> Result<Response<proto::GetMemoryResponse>, Status>;

    async fn update_memory(
        &self,
        request: Request<proto::UpdateMemoryRequest>,
    ) -> Result<Response<proto::UpdateMemoryResponse>, Status>;

    async fn delete_memory(
        &self,
        request: Request<proto::DeleteMemoryRequest>,
    ) -> Result<Response<proto::DeleteMemoryResponse>, Status>;

    async fn get_procedural_competence(
        &self,
        request: Request<proto::GetProceduralCompetenceRequest>,
    ) -> Result<Response<proto::GetProceduralCompetenceResponse>, Status>;

    async fn get_learning_metrics(
        &self,
        request: Request<proto::GetLearningMetricsRequest>,
    ) -> Result<Response<proto::GetLearningMetricsResponse>, Status>;

    type StreamMemoryUpdatesStream: Stream<Item = Result<proto::MemoryUpdate, Status>> + Send + 'static;

    async fn stream_memory_updates(
        &self,
        request: Request<proto::StreamMemoryUpdatesRequest>,
    ) -> Result<Response<Self::StreamMemoryUpdatesStream>, Status>;

    async fn import_demonstrations(
        &self,
        request: Request<proto::ImportDemonstrationsRequest>,
    ) -> Result<Response<proto::ImportDemonstrationsResponse>, Status>;

    async fn get_action_augmentation(
        &self,
        request: Request<proto::GetActionAugmentationRequest>,
    ) -> Result<Response<proto::GetActionAugmentationResponse>, Status>;

    async fn get_store_stats(
        &self,
        request: Request<proto::GetStoreStatsRequest>,
    ) -> Result<Response<proto::GetStoreStatsResponse>, Status>;

    async fn get_agent_memories(
        &self,
        request: Request<proto::GetAgentMemoriesRequest>,
    ) -> Result<Response<proto::GetAgentMemoriesResponse>, Status>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> PraxisGrpcService {
        PraxisGrpcService::new(1000)
    }

    #[tokio::test]
    async fn test_store_and_get_memory() {
        let service = create_test_service();
        let agent_id = Uuid::new_v4().to_string();

        let store_req = Request::new(proto::StoreMemoryRequest {
            agent_id: agent_id.clone(),
            env_state_pre: Some(proto::EnvironmentState {
                textual_repr: "Test page".to_string(),
                visual_hash: None,
                state_features: HashMap::new(),
                element_ids: vec![],
                captured_at: Utc::now().timestamp_millis(),
                embedding: vec![],
            }),
            internal_state: Some(proto::InternalState {
                directive: "Test task".to_string(),
                sub_task: None,
                progress: 0.0,
                task_tags: vec![],
                embedding: vec![],
                context: HashMap::new(),
            }),
            action: Some(proto::AgentAction {
                action_type: "click".to_string(),
                target: Some("button".to_string()),
                parameters: HashMap::new(),
                raw_action: "click(button)".to_string(),
            }),
            env_state_post: Some(proto::EnvironmentState {
                textual_repr: "Result page".to_string(),
                visual_hash: None,
                state_features: HashMap::new(),
                element_ids: vec![],
                captured_at: Utc::now().timestamp_millis(),
                embedding: vec![],
            }),
            outcome: Some(proto::ActionOutcome {
                outcome: Some(proto::action_outcome::Outcome::Success(
                    proto::action_outcome::SuccessOutcome {
                        description: "Clicked successfully".to_string(),
                    },
                )),
            }),
            source: proto::MemorySource::AgentExperience as i32,
            outcome_record_id: None,
        });

        let store_resp = service.store_memory(store_req).await.unwrap();
        let memory_id = store_resp.into_inner().memory_id;

        // Get the memory
        let get_req = Request::new(proto::GetMemoryRequest {
            memory_id: memory_id.clone(),
        });
        let get_resp = service.get_memory(get_req).await.unwrap();
        let memory = get_resp.into_inner().memory.unwrap();

        assert_eq!(memory.id, memory_id);
        assert_eq!(memory.agent_id, agent_id);
    }

    #[tokio::test]
    async fn test_get_store_stats() {
        let service = create_test_service();

        let req = Request::new(proto::GetStoreStatsRequest {});
        let resp = service.get_store_stats(req).await.unwrap();
        let stats = resp.into_inner().stats.unwrap();

        assert_eq!(stats.total_memories, 0);
        assert_eq!(stats.unique_agents, 0);
    }
}
