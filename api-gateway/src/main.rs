//! ACTORIS API Gateway
//!
//! REST API Gateway connecting the frontend to all backend services.
//! Provides unified endpoints for identity, trust, billing, and verification.

use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post, put, delete},
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{broadcast, RwLock};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing::{info, error, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

// ============ STATE ============

#[derive(Clone)]
struct AppState {
    // In-memory stores for demo (replace with service clients in production)
    agents: Arc<RwLock<HashMap<String, Agent>>>,
    actions: Arc<RwLock<HashMap<String, Action>>>,
    wallets: Arc<RwLock<HashMap<String, Wallet>>>,
    // Broadcast channel for real-time updates
    events_tx: broadcast::Sender<Event>,
}

// ============ MODELS ============

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Agent {
    id: String,
    name: String,
    agent_type: AgentType,
    trust_score: TrustScore,
    wallet_id: String,
    created_at: DateTime<Utc>,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentType {
    Human,
    Ai,
    Hybrid,
    Contract,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrustScore {
    tau: f64,           // 0.0 to 1.0
    raw_score: u16,     // 0 to 1000
    tier: u8,           // 0, 1, 2, 3
    verifications: u64,
    disputes: u64,
    last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Wallet {
    id: String,
    agent_id: String,
    balance: Decimal,
    locked: Decimal,
    pending: Decimal,
    transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transaction {
    id: String,
    tx_type: TransactionType,
    amount: Decimal,
    from: Option<String>,
    to: Option<String>,
    action_id: Option<String>,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TransactionType {
    Deposit,
    Withdrawal,
    ActionPayment,
    VerificationReward,
    DisputePenalty,
    Stake,
    Unstake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Action {
    id: String,
    producer_id: String,
    consumer_id: String,
    action_type: String,
    status: ActionStatus,
    input_hash: String,
    output_hash: Option<String>,
    price: Option<Decimal>,
    created_at: DateTime<Utc>,
    verified_at: Option<DateTime<Utc>>,
    verification_proof: Option<VerificationProof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ActionStatus {
    Pending,
    Processing,
    Verified,
    Disputed,
    Settled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerificationProof {
    oracle_votes: Vec<OracleVote>,
    quorum_reached: bool,
    aggregate_signature: String,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OracleVote {
    oracle_id: String,
    vote: bool,
    signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Event {
    event_type: String,
    payload: serde_json::Value,
    timestamp: DateTime<Utc>,
}

// ============ REQUEST/RESPONSE TYPES ============

#[derive(Debug, Deserialize)]
struct CreateAgentRequest {
    name: String,
    agent_type: AgentType,
    metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
struct CreateAgentResponse {
    agent: Agent,
    wallet: Wallet,
}

#[derive(Debug, Deserialize)]
struct SubmitActionRequest {
    producer_id: String,
    consumer_id: String,
    action_type: String,
    input_data: String,
}

#[derive(Debug, Serialize)]
struct SubmitActionResponse {
    action: Action,
    estimated_price: Decimal,
}

#[derive(Debug, Deserialize)]
struct VerifyActionRequest {
    output_data: String,
}

#[derive(Debug, Serialize)]
struct VerifyActionResponse {
    action: Action,
    proof: VerificationProof,
}

#[derive(Debug, Deserialize)]
struct DepositRequest {
    amount: Decimal,
}

#[derive(Debug, Serialize)]
struct StatsResponse {
    total_agents: usize,
    total_actions: usize,
    total_verified: usize,
    total_hc_volume: Decimal,
    average_trust_score: f64,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    services: HashMap<String, bool>,
}

// ============ HANDLERS ============

async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let services = HashMap::from([
        ("identity_cloud".to_string(), true),
        ("trustledger".to_string(), true),
        ("onebill".to_string(), true),
        ("darwinian".to_string(), true),
        ("redis".to_string(), true),
        ("nats".to_string(), true),
    ]);

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        services,
    })
}

async fn get_stats(State(state): State<AppState>) -> Json<StatsResponse> {
    let agents = state.agents.read().await;
    let actions = state.actions.read().await;
    let wallets = state.wallets.read().await;

    let total_verified = actions.values()
        .filter(|a| matches!(a.status, ActionStatus::Verified | ActionStatus::Settled))
        .count();

    let total_volume: Decimal = wallets.values()
        .flat_map(|w| w.transactions.iter())
        .filter(|t| matches!(t.tx_type, TransactionType::ActionPayment))
        .map(|t| t.amount)
        .sum();

    let avg_trust = if agents.is_empty() {
        0.0
    } else {
        agents.values().map(|a| a.trust_score.tau).sum::<f64>() / agents.len() as f64
    };

    Json(StatsResponse {
        total_agents: agents.len(),
        total_actions: actions.len(),
        total_verified,
        total_hc_volume: total_volume,
        average_trust_score: avg_trust,
    })
}

// Agent endpoints
async fn create_agent(
    State(state): State<AppState>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<CreateAgentResponse>, (StatusCode, String)> {
    let agent_id = Uuid::now_v7().to_string();
    let wallet_id = Uuid::now_v7().to_string();

    let agent = Agent {
        id: agent_id.clone(),
        name: req.name,
        agent_type: req.agent_type,
        trust_score: TrustScore {
            tau: 0.5, // Start at neutral
            raw_score: 500,
            tier: 1,
            verifications: 0,
            disputes: 0,
            last_updated: Utc::now(),
        },
        wallet_id: wallet_id.clone(),
        created_at: Utc::now(),
        metadata: req.metadata.unwrap_or_default(),
    };

    let wallet = Wallet {
        id: wallet_id,
        agent_id: agent_id.clone(),
        balance: Decimal::ZERO,
        locked: Decimal::ZERO,
        pending: Decimal::ZERO,
        transactions: vec![],
    };

    state.agents.write().await.insert(agent_id.clone(), agent.clone());
    state.wallets.write().await.insert(agent.wallet_id.clone(), wallet.clone());

    // Broadcast event
    let _ = state.events_tx.send(Event {
        event_type: "agent_created".to_string(),
        payload: serde_json::to_value(&agent).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Created agent: {}", agent_id);

    Ok(Json(CreateAgentResponse { agent, wallet }))
}

async fn get_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<Agent>, (StatusCode, String)> {
    let agents = state.agents.read().await;
    agents.get(&agent_id)
        .cloned()
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))
}

async fn list_agents(State(state): State<AppState>) -> Json<Vec<Agent>> {
    let agents = state.agents.read().await;
    Json(agents.values().cloned().collect())
}

async fn get_trust_score(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<TrustScore>, (StatusCode, String)> {
    let agents = state.agents.read().await;
    agents.get(&agent_id)
        .map(|a| Json(a.trust_score.clone()))
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))
}

// Wallet endpoints
async fn get_wallet(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<Wallet>, (StatusCode, String)> {
    let agents = state.agents.read().await;
    let agent = agents.get(&agent_id)
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))?;

    let wallets = state.wallets.read().await;
    wallets.get(&agent.wallet_id)
        .cloned()
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Wallet not found".to_string()))
}

async fn deposit(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(req): Json<DepositRequest>,
) -> Result<Json<Wallet>, (StatusCode, String)> {
    let agents = state.agents.read().await;
    let agent = agents.get(&agent_id)
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))?
        .clone();
    drop(agents);

    let mut wallets = state.wallets.write().await;
    let wallet = wallets.get_mut(&agent.wallet_id)
        .ok_or((StatusCode::NOT_FOUND, "Wallet not found".to_string()))?;

    wallet.balance += req.amount;
    wallet.transactions.push(Transaction {
        id: Uuid::now_v7().to_string(),
        tx_type: TransactionType::Deposit,
        amount: req.amount,
        from: None,
        to: Some(agent_id.clone()),
        action_id: None,
        timestamp: Utc::now(),
    });

    let _ = state.events_tx.send(Event {
        event_type: "deposit".to_string(),
        payload: serde_json::json!({
            "agent_id": agent_id,
            "amount": req.amount.to_string(),
        }),
        timestamp: Utc::now(),
    });

    Ok(Json(wallet.clone()))
}

// Action endpoints
async fn submit_action(
    State(state): State<AppState>,
    Json(req): Json<SubmitActionRequest>,
) -> Result<Json<SubmitActionResponse>, (StatusCode, String)> {
    // Verify agents exist
    let agents = state.agents.read().await;
    if !agents.contains_key(&req.producer_id) {
        return Err((StatusCode::NOT_FOUND, "Producer not found".to_string()));
    }
    if !agents.contains_key(&req.consumer_id) {
        return Err((StatusCode::NOT_FOUND, "Consumer not found".to_string()));
    }

    let producer = agents.get(&req.producer_id).unwrap();
    let consumer_tau = agents.get(&req.consumer_id).unwrap().trust_score.tau;
    drop(agents);

    // Calculate estimated price based on trust scores
    let base_price = Decimal::from(100); // Base HC
    let trust_discount = Decimal::from_f64_retain(consumer_tau * 0.20).unwrap_or(Decimal::ZERO);
    let estimated_price = base_price * (Decimal::ONE - trust_discount);

    let input_hash = blake3::hash(req.input_data.as_bytes()).to_hex().to_string();

    let action = Action {
        id: Uuid::now_v7().to_string(),
        producer_id: req.producer_id.clone(),
        consumer_id: req.consumer_id.clone(),
        action_type: req.action_type,
        status: ActionStatus::Pending,
        input_hash,
        output_hash: None,
        price: Some(estimated_price),
        created_at: Utc::now(),
        verified_at: None,
        verification_proof: None,
    };

    state.actions.write().await.insert(action.id.clone(), action.clone());

    let _ = state.events_tx.send(Event {
        event_type: "action_submitted".to_string(),
        payload: serde_json::to_value(&action).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Submitted action: {}", action.id);

    Ok(Json(SubmitActionResponse {
        action,
        estimated_price,
    }))
}

async fn verify_action(
    State(state): State<AppState>,
    Path(action_id): Path<String>,
    Json(req): Json<VerifyActionRequest>,
) -> Result<Json<VerifyActionResponse>, (StatusCode, String)> {
    let mut actions = state.actions.write().await;
    let action = actions.get_mut(&action_id)
        .ok_or((StatusCode::NOT_FOUND, "Action not found".to_string()))?;

    if !matches!(action.status, ActionStatus::Pending | ActionStatus::Processing) {
        return Err((StatusCode::BAD_REQUEST, "Action cannot be verified".to_string()));
    }

    // Simulate oracle verification
    action.status = ActionStatus::Processing;
    let output_hash = blake3::hash(req.output_data.as_bytes()).to_hex().to_string();
    action.output_hash = Some(output_hash.clone());

    // Simulate FROST threshold signature (3 of 5 oracles)
    let oracle_votes: Vec<OracleVote> = (0..5).map(|i| {
        let oracle_id = format!("oracle-{}", i);
        let vote_data = format!("{}:{}:{}", action_id, output_hash, oracle_id);
        OracleVote {
            oracle_id,
            vote: true, // All vote yes for demo
            signature: blake3::hash(vote_data.as_bytes()).to_hex().to_string()[..64].to_string(),
        }
    }).collect();

    let aggregate_sig = blake3::hash(
        oracle_votes.iter()
            .map(|v| v.signature.as_str())
            .collect::<Vec<_>>()
            .join("")
            .as_bytes()
    ).to_hex().to_string();

    let proof = VerificationProof {
        oracle_votes,
        quorum_reached: true,
        aggregate_signature: aggregate_sig,
        timestamp: Utc::now(),
    };

    action.status = ActionStatus::Verified;
    action.verified_at = Some(Utc::now());
    action.verification_proof = Some(proof.clone());

    let action_clone = action.clone();
    drop(actions);

    // Update trust scores
    let mut agents = state.agents.write().await;
    if let Some(producer) = agents.get_mut(&action_clone.producer_id) {
        producer.trust_score.verifications += 1;
        producer.trust_score.raw_score = (producer.trust_score.raw_score + 10).min(1000);
        producer.trust_score.tau = producer.trust_score.raw_score as f64 / 1000.0;
        producer.trust_score.tier = match producer.trust_score.raw_score {
            0..=250 => 0,
            251..=500 => 1,
            501..=750 => 2,
            _ => 3,
        };
        producer.trust_score.last_updated = Utc::now();
    }
    drop(agents);

    // Process payment
    if let Some(price) = action_clone.price {
        let agents = state.agents.read().await;
        let consumer_wallet_id = agents.get(&action_clone.consumer_id)
            .map(|a| a.wallet_id.clone());
        let producer_wallet_id = agents.get(&action_clone.producer_id)
            .map(|a| a.wallet_id.clone());
        drop(agents);

        if let (Some(consumer_wid), Some(producer_wid)) = (consumer_wallet_id, producer_wallet_id) {
            let mut wallets = state.wallets.write().await;

            // Debit consumer
            if let Some(consumer_wallet) = wallets.get_mut(&consumer_wid) {
                consumer_wallet.balance -= price;
                consumer_wallet.transactions.push(Transaction {
                    id: Uuid::now_v7().to_string(),
                    tx_type: TransactionType::ActionPayment,
                    amount: price,
                    from: Some(action_clone.consumer_id.clone()),
                    to: Some(action_clone.producer_id.clone()),
                    action_id: Some(action_clone.id.clone()),
                    timestamp: Utc::now(),
                });
            }

            // Credit producer
            if let Some(producer_wallet) = wallets.get_mut(&producer_wid) {
                producer_wallet.balance += price;
                producer_wallet.transactions.push(Transaction {
                    id: Uuid::now_v7().to_string(),
                    tx_type: TransactionType::VerificationReward,
                    amount: price,
                    from: Some(action_clone.consumer_id.clone()),
                    to: Some(action_clone.producer_id.clone()),
                    action_id: Some(action_clone.id.clone()),
                    timestamp: Utc::now(),
                });
            }
        }
    }

    let _ = state.events_tx.send(Event {
        event_type: "action_verified".to_string(),
        payload: serde_json::to_value(&action_clone).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Verified action: {}", action_clone.id);

    Ok(Json(VerifyActionResponse {
        action: action_clone,
        proof,
    }))
}

async fn get_action(
    State(state): State<AppState>,
    Path(action_id): Path<String>,
) -> Result<Json<Action>, (StatusCode, String)> {
    let actions = state.actions.read().await;
    actions.get(&action_id)
        .cloned()
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Action not found".to_string()))
}

async fn list_actions(State(state): State<AppState>) -> Json<Vec<Action>> {
    let actions = state.actions.read().await;
    let mut all: Vec<_> = actions.values().cloned().collect();
    all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(all)
}

async fn get_agent_actions(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Json<Vec<Action>> {
    let actions = state.actions.read().await;
    let agent_actions: Vec<_> = actions.values()
        .filter(|a| a.producer_id == agent_id || a.consumer_id == agent_id)
        .cloned()
        .collect();
    Json(agent_actions)
}

// WebSocket for real-time events
async fn websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.events_tx.subscribe();

    // Send events to client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let msg = serde_json::to_string(&event).unwrap();
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Receive messages from client (for future commands)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    info!("Received: {}", text);
                }
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

// ============ MAIN ============

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("api_gateway=info".parse()?)
                .add_directive("tower_http=debug".parse()?)
        )
        .json()
        .init();

    // Load environment
    dotenvy::dotenv().ok();

    // Create broadcast channel for events
    let (events_tx, _) = broadcast::channel::<Event>(1000);

    // Create app state
    let state = AppState {
        agents: Arc::new(RwLock::new(HashMap::new())),
        actions: Arc::new(RwLock::new(HashMap::new())),
        wallets: Arc::new(RwLock::new(HashMap::new())),
        events_tx,
    };

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health & stats
        .route("/health", get(health_check))
        .route("/stats", get(get_stats))
        // Agents
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/:agent_id", get(get_agent))
        .route("/agents/:agent_id/trust", get(get_trust_score))
        .route("/agents/:agent_id/wallet", get(get_wallet))
        .route("/agents/:agent_id/wallet/deposit", post(deposit))
        .route("/agents/:agent_id/actions", get(get_agent_actions))
        // Actions
        .route("/actions", get(list_actions).post(submit_action))
        .route("/actions/:action_id", get(get_action))
        .route("/actions/:action_id/verify", post(verify_action))
        // WebSocket
        .route("/ws", get(websocket_handler))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors)
        .with_state(state);

    // Use PORT env var (Railway) or default to 8080
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("ACTORIS API Gateway starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
