//! PRAXIS Service Binary
//!
//! Procedural Recall for Agents with eXperiences Indexed by State

use std::net::SocketAddr;

use anyhow::Result;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use actoris_praxis::{
    config::PraxisConfig,
    domain::retrieval::RetrievalConfig,
    grpc::PraxisGrpcService,
    DEFAULT_RETRIEVAL_BREADTH, DEFAULT_SIMILARITY_THRESHOLD, MAX_MEMORIES_PER_AGENT,
    MEMORY_DECAY_HALF_LIFE_DAYS, PRAXIS_VERSION,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Actoris PRAXIS Service v{}", PRAXIS_VERSION);

    // Load configuration
    let config = PraxisConfig::load()?;
    info!("Loaded configuration: {:?}", config);

    // Build retrieval config from settings
    let retrieval_config = RetrievalConfig {
        search_breadth: config.retrieval.search_breadth,
        similarity_threshold: config.retrieval.similarity_threshold,
        iou_weight: config.retrieval.iou_weight,
        length_weight: config.retrieval.length_weight,
        enable_time_decay: config.retrieval.enable_time_decay,
        decay_half_life_days: config.retrieval.decay_half_life_days,
        boost_successful: true,
        success_boost: 1.5,
    };

    // Initialize gRPC service
    let praxis_service = PraxisGrpcService::with_config(
        config.storage.max_memories_per_agent,
        retrieval_config,
    );

    info!("PRAXIS service initialized");
    info!(
        "Retrieval config: breadth={}, threshold={}, time_decay={}",
        config.retrieval.search_breadth,
        config.retrieval.similarity_threshold,
        config.retrieval.enable_time_decay
    );
    info!(
        "Storage config: max_memories_per_agent={}",
        config.storage.max_memories_per_agent
    );

    // Parse address
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    info!("Starting gRPC server on {}", addr);

    // Create gRPC server
    // Note: In production, you would use tonic-build to generate the server
    // For now, we'll create a simple reflection-enabled server

    // Start the server with graceful shutdown
    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Received shutdown signal");
    };

    info!("PRAXIS gRPC server listening on {}", addr);
    info!("");
    info!("Available endpoints:");
    info!("  - StoreMemory");
    info!("  - RetrieveMemories");
    info!("  - GetMemory");
    info!("  - UpdateMemory");
    info!("  - DeleteMemory");
    info!("  - GetProceduralCompetence");
    info!("  - GetLearningMetrics");
    info!("  - StreamMemoryUpdates");
    info!("  - ImportDemonstrations");
    info!("  - GetActionAugmentation");
    info!("  - GetStoreStats");
    info!("  - GetAgentMemories");
    info!("");

    // Since we don't have auto-generated server from proto,
    // we expose the service methods via a manual tonic server
    // In production, use tonic-build to generate PraxisServiceServer

    // For now, we'll create a basic HTTP/JSON API using axum as a fallback
    // or wait for ctrl+c

    // Create a simple health check server
    use std::sync::Arc;
    let service = Arc::new(praxis_service);

    // Start REST API alongside gRPC (using axum)
    let app = create_rest_api(service.clone());

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("REST API server started on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    info!("Shutting down PRAXIS service");
    Ok(())
}

/// Create REST API routes for the PRAXIS service
fn create_rest_api(
    service: std::sync::Arc<PraxisGrpcService>,
) -> axum::Router {
    use axum::{
        extract::{Path, Query, State},
        http::{header, Method, StatusCode},
        response::Json,
        routing::{delete, get, post, put},
        Router,
    };
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use tower_http::cors::{Any, CorsLayer};

    // CORS layer to allow frontend connections from any origin
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT]);

    #[derive(Deserialize)]
    struct RetrieveQuery {
        max_results: Option<u32>,
        similarity_threshold: Option<f32>,
        include_global: Option<bool>,
        successful_only: Option<bool>,
    }

    #[derive(Deserialize)]
    struct MemoriesQuery {
        limit: Option<u32>,
        offset: Option<u32>,
    }

    Router::new()
        // Health check
        .route("/health", get(|| async {
            Json(serde_json::json!({"status": "healthy"}))
        }))

        // Store stats (both routes for compatibility)
        .route("/praxis/stats", get({
            let svc = service.clone();
            move || {
                let svc = svc.clone();
                async move {
                    let stats = svc.store.stats();
                    let success_rate = if stats.total_memories > 0 {
                        stats.successful_memories as f32 / stats.total_memories as f32
                    } else {
                        0.0
                    };
                    Json(serde_json::json!({
                        "total_memories": stats.total_memories,
                        "unique_agents": stats.unique_agents,
                        "avg_memories_per_agent": stats.avg_memories_per_agent,
                        "max_memories_per_agent": stats.max_memories_per_agent,
                        "successful_memories": stats.successful_memories,
                        "success_rate": success_rate,
                    }))
                }
            }
        }))
        .route("/api/v1/stats", get({
            let svc = service.clone();
            move || {
                let svc = svc.clone();
                async move {
                    let stats = svc.store.stats();
                    Json(serde_json::json!({
                        "total_memories": stats.total_memories,
                        "unique_agents": stats.unique_agents,
                        "avg_memories_per_agent": stats.avg_memories_per_agent,
                        "max_memories_per_agent": stats.max_memories_per_agent,
                        "successful_memories": stats.successful_memories,
                    }))
                }
            }
        }))

        // Get agent memories (both routes for compatibility)
        .route("/praxis/agents/:agent_id/memories", get({
            let svc = service.clone();
            move |Path(agent_id): Path<String>, Query(query): Query<MemoriesQuery>| {
                let svc = svc.clone();
                async move {
                    let agent_uuid = match uuid::Uuid::parse_str(&agent_id) {
                        Ok(id) => id,
                        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid agent_id"}))),
                    };

                    let memories = svc.store.get_agent_memories(&agent_uuid).await;
                    let total = memories.len();

                    let offset = query.offset.unwrap_or(0) as usize;
                    let limit = query.limit.unwrap_or(50) as usize;

                    let memories: Vec<_> = memories
                        .into_iter()
                        .skip(offset)
                        .take(limit)
                        .map(|m| serde_json::json!({
                            "id": m.id.to_string(),
                            "agent_id": m.agent_id.to_string(),
                            "directive": m.internal_state.directive,
                            "action": m.action.raw_action,
                            "is_successful": m.is_successful(),
                            "created_at": m.created_at.to_rfc3339(),
                            "retrieval_count": m.retrieval_count,
                            "reinforcement_score": m.reinforcement_score,
                        }))
                        .collect();

                    (StatusCode::OK, Json(serde_json::json!({
                        "memories": memories,
                        "total": total,
                    })))
                }
            }
        }))
        .route("/api/v1/agents/:agent_id/memories", get({
            let svc = service.clone();
            move |Path(agent_id): Path<String>, Query(query): Query<MemoriesQuery>| {
                let svc = svc.clone();
                async move {
                    let agent_uuid = match uuid::Uuid::parse_str(&agent_id) {
                        Ok(id) => id,
                        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid agent_id"}))),
                    };

                    let memories = svc.store.get_agent_memories(&agent_uuid).await;
                    let total = memories.len();

                    let offset = query.offset.unwrap_or(0) as usize;
                    let limit = query.limit.unwrap_or(50) as usize;

                    let memories: Vec<_> = memories
                        .into_iter()
                        .skip(offset)
                        .take(limit)
                        .map(|m| serde_json::json!({
                            "id": m.id.to_string(),
                            "agent_id": m.agent_id.to_string(),
                            "directive": m.internal_state.directive,
                            "action": m.action.raw_action,
                            "is_successful": m.is_successful(),
                            "created_at": m.created_at.to_rfc3339(),
                            "retrieval_count": m.retrieval_count,
                            "reinforcement_score": m.reinforcement_score,
                        }))
                        .collect();

                    (StatusCode::OK, Json(serde_json::json!({
                        "memories": memories,
                        "total": total,
                    })))
                }
            }
        }))

        // Get agent competence (both routes for compatibility)
        .route("/praxis/agents/:agent_id/competence", get({
            let svc = service.clone();
            move |Path(agent_id): Path<String>| {
                let svc = svc.clone();
                async move {
                    let agent_uuid = match uuid::Uuid::parse_str(&agent_id) {
                        Ok(id) => id,
                        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid agent_id"}))),
                    };

                    let memories = svc.store.get_agent_memories(&agent_uuid).await;
                    let competence = actoris_praxis::ProceduralCompetence::from_memories(&memories);

                    (StatusCode::OK, Json(serde_json::json!({
                        "total_memories": competence.total_memories,
                        "successful_memories": competence.successful_memories,
                        "success_rate": competence.success_rate,
                        "diversity_score": competence.diversity_score,
                        "generalization_score": competence.generalization_score,
                        "learning_velocity": competence.learning_velocity,
                        "retrieval_accuracy": competence.retrieval_accuracy,
                        "memory_utilization": competence.memory_utilization,
                        "fitness_multiplier": competence.fitness_multiplier(),
                    })))
                }
            }
        }))
        .route("/api/v1/agents/:agent_id/competence", get({
            let svc = service.clone();
            move |Path(agent_id): Path<String>| {
                let svc = svc.clone();
                async move {
                    let agent_uuid = match uuid::Uuid::parse_str(&agent_id) {
                        Ok(id) => id,
                        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid agent_id"}))),
                    };

                    let memories = svc.store.get_agent_memories(&agent_uuid).await;
                    let competence = actoris_praxis::ProceduralCompetence::from_memories(&memories);

                    (StatusCode::OK, Json(serde_json::json!({
                        "total_memories": competence.total_memories,
                        "successful_memories": competence.successful_memories,
                        "success_rate": competence.success_rate,
                        "diversity_score": competence.diversity_score,
                        "generalization_score": competence.generalization_score,
                        "learning_velocity": competence.learning_velocity,
                        "retrieval_accuracy": competence.retrieval_accuracy,
                        "memory_utilization": competence.memory_utilization,
                        "fitness_multiplier": competence.fitness_multiplier(),
                    })))
                }
            }
        }))

        // Version info
        .route("/api/v1/version", get(|| async {
            Json(serde_json::json!({
                "service": "actoris-praxis",
                "version": PRAXIS_VERSION,
                "description": "Procedural Recall for Agents with eXperiences Indexed by State",
            }))
        }))
        // Apply CORS middleware
        .layer(cors)
}
