//! ACTORIS API Gateway v2
//!
//! Economic OS for AI Agents - Full API matching the Actoris vision:
//! - IdentityCloud: UnifiedID + TrustScore + HC Wallet
//! - TrustLedger: 3-of-N Oracle Consensus with FROST signatures
//! - OneBill: Price = Compute + Risk - Trust
//! - Darwinian: Fitness-based resource allocation

use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::Method,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing::info;
use chrono::{DateTime, Utc};
use rand::Rng;

// ============ STATE ============

#[derive(Clone)]
struct AppState {
    agents: Arc<RwLock<HashMap<String, AgentV2>>>,
    actions: Arc<RwLock<HashMap<String, ActionV2>>>,
    events_tx: broadcast::Sender<Event>,
}

// ============ MODELS - Matching UI v2 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentV2 {
    id: String,
    name: String,
    #[serde(rename = "type")]
    agent_type: String, // "human" | "agent" | "organization"
    trust_score: TrustScoreV2,
    wallet: HCWallet,
    fitness: FitnessMetrics,
    status: String, // "active" | "warning" | "culled"
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrustScoreV2 {
    score: u16,      // 0-1000
    tau: f64,        // 0.0-1.0
    tier: u8,        // 0, 1, 2, 3
    verifications: u64,
    disputes: u64,
    last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HCWallet {
    id: String,
    balance: f64,    // PFLOP-hours
    reserved: f64,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FitnessMetrics {
    eta: f64,        // η = τ × (Revenue/Cost)
    revenue: f64,
    cost: f64,
    classification: String, // "champion" | "neutral" | "underperformer"
    hc_allocation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionV2 {
    id: String,
    producer_id: String,
    consumer_id: String,
    action_type: String,
    status: String, // "pending" | "processing" | "verified" | "disputed" | "failed"
    pricing: ActionPricing,
    verification: Option<VerificationProofV2>,
    created_at: DateTime<Utc>,
    verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionPricing {
    base_compute: f64,
    risk_premium: f64,
    trust_discount: f64,
    final_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerificationProofV2 {
    oracle_votes: Vec<OracleVoteV2>,
    quorum_reached: bool,
    quorum_threshold: String,
    aggregate_signature: String,
    latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OracleVoteV2 {
    oracle_id: String,
    oracle_name: String,
    vote: bool,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AGDPMetrics {
    total_agdp: f64,
    actions_count: usize,
    verified_count: usize,
    disputed_count: usize,
    dispute_rate: f64,
    avg_verification_latency: f64,
    compute_efficiency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemStats {
    agdp: AGDPMetrics,
    total_entities: usize,
    total_actions: usize,
    total_verified: usize,
    avg_trust_score: f64,
    avg_fitness: f64,
    culled_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Event {
    event_type: String,
    payload: serde_json::Value,
    timestamp: DateTime<Utc>,
}

// ============ REQUEST TYPES ============

#[derive(Debug, Deserialize)]
struct CreateAgentRequest {
    name: String,
    #[serde(rename = "type")]
    agent_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubmitActionRequest {
    producer_id: String,
    consumer_id: String,
    action_type: String,
    input_data: String,
}

#[derive(Debug, Deserialize)]
struct VerifyActionRequest {
    output_data: String,
}

// ============ HANDLERS ============

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "services": {
            "identity_cloud": true,
            "trust_ledger": true,
            "onebill": true,
            "darwinian": true
        }
    }))
}

async fn get_stats(State(state): State<AppState>) -> Json<SystemStats> {
    let agents = state.agents.read().await;
    let actions = state.actions.read().await;

    let verified_count = actions.values().filter(|a| a.status == "verified").count();
    let disputed_count = actions.values().filter(|a| a.status == "disputed").count();
    let total_actions = actions.len();

    let total_agdp: f64 = actions.values()
        .filter(|a| a.status == "verified")
        .map(|a| a.pricing.final_price)
        .sum();

    let avg_latency = actions.values()
        .filter_map(|a| a.verification.as_ref())
        .map(|v| v.latency_ms)
        .sum::<f64>() / verified_count.max(1) as f64;

    let avg_trust = if agents.is_empty() {
        0.0
    } else {
        agents.values().map(|a| a.trust_score.score as f64).sum::<f64>() / agents.len() as f64
    };

    let avg_fitness = if agents.is_empty() {
        0.0
    } else {
        agents.values().map(|a| a.fitness.eta).sum::<f64>() / agents.len() as f64
    };

    let culled_count = agents.values().filter(|a| a.status == "culled").count();

    Json(SystemStats {
        agdp: AGDPMetrics {
            total_agdp,
            actions_count: total_actions,
            verified_count,
            disputed_count,
            dispute_rate: if total_actions > 0 { disputed_count as f64 / total_actions as f64 } else { 0.0 },
            avg_verification_latency: avg_latency,
            compute_efficiency: 1.23, // Simulated CRI
        },
        total_entities: agents.len(),
        total_actions,
        total_verified: verified_count,
        avg_trust_score: avg_trust,
        avg_fitness,
        culled_count,
    })
}

async fn list_agents(State(state): State<AppState>) -> Json<Vec<AgentV2>> {
    let agents = state.agents.read().await;
    Json(agents.values().cloned().collect())
}

async fn create_agent(
    State(state): State<AppState>,
    Json(req): Json<CreateAgentRequest>,
) -> Json<AgentV2> {
    let mut rng = rand::thread_rng();
    let id = format!("agent-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());

    let initial_score: u16 = rng.gen_range(400..700);
    let tau = initial_score as f64 / 1000.0;

    let agent = AgentV2 {
        id: id.clone(),
        name: req.name,
        agent_type: req.agent_type.unwrap_or_else(|| "agent".to_string()),
        trust_score: TrustScoreV2 {
            score: initial_score,
            tau,
            tier: match initial_score {
                0..=250 => 0,
                251..=500 => 1,
                501..=750 => 2,
                _ => 3,
            },
            verifications: 0,
            disputes: 0,
            last_updated: Utc::now(),
        },
        wallet: HCWallet {
            id: format!("wallet-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            balance: rng.gen_range(100.0..5000.0),
            reserved: 0.0,
            expires_at: Utc::now() + chrono::Duration::days(30),
        },
        fitness: FitnessMetrics {
            eta: tau * rng.gen_range(0.8..1.5),
            revenue: rng.gen_range(1000.0..20000.0),
            cost: rng.gen_range(800.0..15000.0),
            classification: if tau > 0.7 { "champion".to_string() }
                           else if tau > 0.5 { "neutral".to_string() }
                           else { "underperformer".to_string() },
            hc_allocation: rng.gen_range(100.0..1000.0),
        },
        status: "active".to_string(),
        created_at: Utc::now(),
    };

    state.agents.write().await.insert(id.clone(), agent.clone());

    let _ = state.events_tx.send(Event {
        event_type: "agent_created".to_string(),
        payload: serde_json::to_value(&agent).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Created agent: {}", id);
    Json(agent)
}

async fn get_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<AgentV2>, (axum::http::StatusCode, String)> {
    let agents = state.agents.read().await;
    agents.get(&agent_id)
        .cloned()
        .map(Json)
        .ok_or((axum::http::StatusCode::NOT_FOUND, "Agent not found".to_string()))
}

async fn list_actions(State(state): State<AppState>) -> Json<Vec<ActionV2>> {
    let actions = state.actions.read().await;
    let mut all: Vec<_> = actions.values().cloned().collect();
    all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(all)
}

async fn submit_action(
    State(state): State<AppState>,
    Json(req): Json<SubmitActionRequest>,
) -> Json<ActionV2> {
    let mut rng = rand::thread_rng();
    let id = format!("act-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());

    // Calculate OneBill pricing: Price = Compute + Risk - Trust
    let base_compute = rng.gen_range(0.05..0.15);
    let risk_premium = rng.gen_range(0.01..0.05);
    let trust_discount = rng.gen_range(0.005..0.03);
    let final_price = base_compute + risk_premium - trust_discount;

    let action = ActionV2 {
        id: id.clone(),
        producer_id: req.producer_id,
        consumer_id: req.consumer_id,
        action_type: req.action_type,
        status: "pending".to_string(),
        pricing: ActionPricing {
            base_compute,
            risk_premium,
            trust_discount,
            final_price,
        },
        verification: None,
        created_at: Utc::now(),
        verified_at: None,
    };

    state.actions.write().await.insert(id.clone(), action.clone());

    let _ = state.events_tx.send(Event {
        event_type: "action_submitted".to_string(),
        payload: serde_json::to_value(&action).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Submitted action: {}", id);
    Json(action)
}

async fn verify_action(
    State(state): State<AppState>,
    Path(action_id): Path<String>,
    Json(_req): Json<VerifyActionRequest>,
) -> Result<Json<ActionV2>, (axum::http::StatusCode, String)> {
    let mut rng = rand::thread_rng();
    let mut actions = state.actions.write().await;

    let action = actions.get_mut(&action_id)
        .ok_or((axum::http::StatusCode::NOT_FOUND, "Action not found".to_string()))?;

    // Simulate 3-of-5 oracle consensus
    let oracle_names = ["Oracle-Alpha", "Oracle-Beta", "Oracle-Gamma", "Oracle-Delta", "Oracle-Epsilon"];
    let oracle_votes: Vec<OracleVoteV2> = oracle_names.iter().enumerate().map(|(i, name)| {
        OracleVoteV2 {
            oracle_id: format!("oracle-{}", i),
            oracle_name: name.to_string(),
            vote: rng.gen_bool(0.95), // 95% chance of yes
            timestamp: Utc::now(),
        }
    }).collect();

    let yes_votes = oracle_votes.iter().filter(|v| v.vote).count();
    let quorum_reached = yes_votes >= 3;

    let latency_ms = rng.gen_range(600.0..1400.0);

    action.verification = Some(VerificationProofV2 {
        oracle_votes,
        quorum_reached,
        quorum_threshold: "3-of-5".to_string(),
        aggregate_signature: format!("0x{}", hex::encode(&rand::random::<[u8; 32]>())),
        latency_ms,
    });

    action.status = if quorum_reached { "verified".to_string() } else { "disputed".to_string() };
    action.verified_at = Some(Utc::now());

    let action_clone = action.clone();

    // Update producer trust score
    if quorum_reached {
        drop(actions);
        let mut agents = state.agents.write().await;
        if let Some(producer) = agents.get_mut(&action_clone.producer_id) {
            producer.trust_score.verifications += 1;
            producer.trust_score.score = (producer.trust_score.score + 5).min(1000);
            producer.trust_score.tau = producer.trust_score.score as f64 / 1000.0;
            producer.trust_score.last_updated = Utc::now();

            // Update fitness
            producer.fitness.revenue += action_clone.pricing.final_price * 100.0;
            producer.fitness.eta = producer.trust_score.tau * (producer.fitness.revenue / producer.fitness.cost.max(1.0));
            producer.fitness.classification = if producer.fitness.eta >= 1.0 { "champion".to_string() }
                                              else if producer.fitness.eta >= 0.7 { "neutral".to_string() }
                                              else { "underperformer".to_string() };
        }
    }

    let _ = state.events_tx.send(Event {
        event_type: "action_verified".to_string(),
        payload: serde_json::to_value(&action_clone).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Verified action: {} (quorum: {})", action_id, quorum_reached);
    Ok(Json(action_clone))
}

async fn get_leaderboard(State(state): State<AppState>) -> Json<Vec<AgentV2>> {
    let agents = state.agents.read().await;
    let mut sorted: Vec<_> = agents.values().cloned().collect();
    sorted.sort_by(|a, b| b.fitness.eta.partial_cmp(&a.fitness.eta).unwrap_or(std::cmp::Ordering::Equal));
    Json(sorted)
}

// WebSocket handler
async fn websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.events_tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let msg = serde_json::to_string(&event).unwrap();
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

// Seed demo data
async fn seed_demo_data(state: &AppState) {
    let demo_agents = vec![
        ("Alpha-7 Analyst", "agent", 892, 1.45),
        ("DataBot-X9", "agent", 756, 1.12),
        ("ProcessorUnit-3", "agent", 534, 0.68),
        ("Acme Corp", "organization", 945, 1.67),
        ("John Smith", "human", 823, 1.34),
    ];

    let mut rng = rand::thread_rng();

    for (name, agent_type, score, eta) in demo_agents {
        let id = format!("agent-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        let tau = score as f64 / 1000.0;

        let agent = AgentV2 {
            id: id.clone(),
            name: name.to_string(),
            agent_type: agent_type.to_string(),
            trust_score: TrustScoreV2 {
                score,
                tau,
                tier: match score {
                    0..=250 => 0,
                    251..=500 => 1,
                    501..=750 => 2,
                    _ => 3,
                },
                verifications: rng.gen_range(100..2000),
                disputes: rng.gen_range(0..30),
                last_updated: Utc::now(),
            },
            wallet: HCWallet {
                id: format!("wallet-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
                balance: rng.gen_range(500.0..50000.0),
                reserved: rng.gen_range(0.0..500.0),
                expires_at: Utc::now() + chrono::Duration::days(30),
            },
            fitness: FitnessMetrics {
                eta,
                revenue: rng.gen_range(5000.0..100000.0),
                cost: rng.gen_range(3000.0..80000.0),
                classification: if eta >= 1.0 { "champion".to_string() }
                               else if eta >= 0.7 { "neutral".to_string() }
                               else { "underperformer".to_string() },
                hc_allocation: rng.gen_range(100.0..2000.0),
            },
            status: if eta < 0.7 { "warning".to_string() } else { "active".to_string() },
            created_at: Utc::now() - chrono::Duration::days(rng.gen_range(1..90)),
        };

        state.agents.write().await.insert(id, agent);
    }

    // Seed some actions
    let agents: Vec<String> = state.agents.read().await.keys().cloned().collect();
    let action_types = ["inference", "analysis", "generation", "classification", "embedding"];

    for i in 0..15 {
        let id = format!("act-{:04}", 1000 + i);
        let producer_id = agents[rng.gen_range(0..agents.len())].clone();
        let consumer_id = agents[rng.gen_range(0..agents.len())].clone();

        let base_compute = rng.gen_range(0.05..0.15);
        let risk_premium = rng.gen_range(0.01..0.05);
        let trust_discount = rng.gen_range(0.005..0.03);

        let statuses = ["verified", "verified", "verified", "processing", "pending", "disputed"];
        let status = statuses[rng.gen_range(0..statuses.len())].to_string();

        let verification = if status == "verified" || status == "disputed" {
            let oracle_names = ["Oracle-Alpha", "Oracle-Beta", "Oracle-Gamma", "Oracle-Delta", "Oracle-Epsilon"];
            Some(VerificationProofV2 {
                oracle_votes: oracle_names.iter().enumerate().map(|(j, name)| {
                    OracleVoteV2 {
                        oracle_id: format!("oracle-{}", j),
                        oracle_name: name.to_string(),
                        vote: status == "verified" || rng.gen_bool(0.6),
                        timestamp: Utc::now(),
                    }
                }).collect(),
                quorum_reached: status == "verified",
                quorum_threshold: "3-of-5".to_string(),
                aggregate_signature: format!("0x{}", hex::encode(&rand::random::<[u8; 32]>())),
                latency_ms: rng.gen_range(600.0..1400.0),
            })
        } else {
            None
        };

        let action = ActionV2 {
            id: id.clone(),
            producer_id,
            consumer_id,
            action_type: action_types[rng.gen_range(0..action_types.len())].to_string(),
            status,
            pricing: ActionPricing {
                base_compute,
                risk_premium,
                trust_discount,
                final_price: base_compute + risk_premium - trust_discount,
            },
            verification,
            created_at: Utc::now() - chrono::Duration::hours(rng.gen_range(0..48)),
            verified_at: Some(Utc::now() - chrono::Duration::hours(rng.gen_range(0..24))),
        };

        state.actions.write().await.insert(id, action);
    }

    info!("Seeded demo data: {} agents, {} actions",
          state.agents.read().await.len(),
          state.actions.read().await.len());
}

// ============ MAIN ============

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("api_gateway=info".parse()?)
        )
        .json()
        .init();

    dotenvy::dotenv().ok();

    let (events_tx, _) = broadcast::channel::<Event>(1000);

    let state = AppState {
        agents: Arc::new(RwLock::new(HashMap::new())),
        actions: Arc::new(RwLock::new(HashMap::new())),
        events_tx,
    };

    // Seed demo data on startup
    seed_demo_data(&state).await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        // Health & Stats
        .route("/health", get(health_check))
        .route("/stats", get(get_stats))
        // IdentityCloud
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/:agent_id", get(get_agent))
        // TrustLedger
        .route("/actions", get(list_actions).post(submit_action))
        .route("/actions/:action_id/verify", post(verify_action))
        // Darwinian
        .route("/darwinian/leaderboard", get(get_leaderboard))
        // WebSocket
        .route("/ws", get(websocket_handler))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors)
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("ACTORIS API Gateway v2 starting on {}", addr);
    info!("Endpoints: /health, /stats, /agents, /actions, /darwinian/leaderboard");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
