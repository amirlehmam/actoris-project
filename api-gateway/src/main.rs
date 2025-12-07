//! ACTORIS API Gateway v2
//!
//! Economic OS for AI Agents - Full API matching the Actoris vision:
//! - IdentityCloud: UnifiedID + TrustScore + HC Wallet
//! - TrustLedger: 3-of-N Oracle Consensus with FROST signatures
//! - OneBill: Price = Compute + Risk - Trust
//! - Darwinian: Fitness-based resource allocation

use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::{Method, StatusCode},
    response::{IntoResponse, Json, Response},
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

// ============ ERROR TYPE ============

struct AppError(StatusCode, String);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({"error": self.1}))).into_response()
    }
}

// ============ STATE ============

#[derive(Clone)]
struct AppState {
    agents: Arc<RwLock<HashMap<String, AgentV2>>>,
    actions: Arc<RwLock<HashMap<String, ActionV2>>>,
    // Protocol DNA primitives
    loans: Arc<RwLock<HashMap<String, Loan>>>,
    policies: Arc<RwLock<HashMap<String, InsurancePolicy>>>,
    delegations: Arc<RwLock<HashMap<String, Delegation>>>,
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

// ============ PROTOCOL DNA MODELS ============

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Loan {
    id: String,
    lender_id: String,
    borrower_id: String,
    principal: f64,        // HC amount
    interest_rate: f64,    // APR (based on trust)
    term_days: u32,
    status: String,        // "active" | "repaid" | "defaulted"
    created_at: DateTime<Utc>,
    due_at: DateTime<Utc>,
    repaid_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InsurancePolicy {
    id: String,
    insurer_id: String,
    insured_id: String,
    coverage: f64,         // HC coverage amount
    premium: f64,          // Premium paid
    premium_rate: f64,     // Rate based on trust
    action_type: String,   // Type of action insured
    status: String,        // "active" | "claimed" | "expired"
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Delegation {
    id: String,
    client_id: String,
    agent_id: String,
    task_description: String,
    escrow_amount: f64,    // HC locked in escrow
    status: String,        // "pending" | "active" | "completed" | "disputed" | "cancelled"
    created_at: DateTime<Utc>,
    deadline: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
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

// Protocol DNA Request Types
#[derive(Debug, Deserialize)]
struct SpawnAgentRequest {
    parent_id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct CreateLoanRequest {
    lender_id: String,
    borrower_id: String,
    principal: f64,
    term_days: u32,
}

#[derive(Debug, Deserialize)]
struct CreateInsuranceRequest {
    insurer_id: String,
    insured_id: String,
    coverage: f64,
    action_type: String,
    duration_days: u32,
}

#[derive(Debug, Deserialize)]
struct CreateDelegationRequest {
    client_id: String,
    agent_id: String,
    task_description: String,
    escrow_amount: f64,
    deadline_days: u32,
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
    // Generate all random values before any await points to avoid Send issues
    let id = format!("agent-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    let wallet_id = format!("wallet-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());

    let (initial_score, balance, revenue, cost, hc_allocation) = {
        let mut rng = rand::thread_rng();
        (
            rng.gen_range(400..700u16),
            rng.gen_range(100.0..5000.0f64),
            rng.gen_range(1000.0..20000.0f64),
            rng.gen_range(800.0..15000.0f64),
            rng.gen_range(100.0..1000.0f64),
        )
    };

    let initial_score: u16 = initial_score;
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
            id: wallet_id,
            balance,
            reserved: 0.0,
            expires_at: Utc::now() + chrono::Duration::days(30),
        },
        fitness: FitnessMetrics {
            eta: tau * (revenue / cost),
            revenue,
            cost,
            classification: if tau > 0.7 { "champion".to_string() }
                           else if tau > 0.5 { "neutral".to_string() }
                           else { "underperformer".to_string() },
            hc_allocation,
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
) -> Result<Json<AgentV2>, AppError> {
    let agents = state.agents.read().await;
    agents.get(&agent_id)
        .cloned()
        .map(Json)
        .ok_or(AppError(StatusCode::NOT_FOUND, "Agent not found".to_string()))
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
    let id = format!("act-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());

    // Calculate OneBill pricing: Price = Compute + Risk - Trust
    let (base_compute, risk_premium, trust_discount) = {
        let mut rng = rand::thread_rng();
        (
            rng.gen_range(0.05..0.15f64),
            rng.gen_range(0.01..0.05f64),
            rng.gen_range(0.005..0.03f64),
        )
    };
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
) -> Result<Json<ActionV2>, AppError> {
    // Generate all random values before any await points
    let oracle_names = ["Oracle-Alpha", "Oracle-Beta", "Oracle-Gamma", "Oracle-Delta", "Oracle-Epsilon"];
    let (oracle_votes_data, latency_ms) = {
        let mut rng = rand::thread_rng();
        let votes: Vec<bool> = oracle_names.iter().map(|_| rng.gen_bool(0.95)).collect();
        let latency = rng.gen_range(600.0..1400.0f64);
        (votes, latency)
    };

    let oracle_votes: Vec<OracleVoteV2> = oracle_names.iter().enumerate().map(|(i, name)| {
        OracleVoteV2 {
            oracle_id: format!("oracle-{}", i),
            oracle_name: name.to_string(),
            vote: oracle_votes_data[i],
            timestamp: Utc::now(),
        }
    }).collect();

    let yes_votes = oracle_votes.iter().filter(|v| v.vote).count();
    let quorum_reached = yes_votes >= 3;

    let mut actions = state.actions.write().await;

    let action = actions.get_mut(&action_id)
        .ok_or(AppError(StatusCode::NOT_FOUND, "Action not found".to_string()))?;

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

// ============ PROTOCOL DNA HANDLERS ============

// SPAWN: Create child agent with 30% trust inheritance
async fn spawn_agent(
    State(state): State<AppState>,
    Json(req): Json<SpawnAgentRequest>,
) -> Result<Json<AgentV2>, AppError> {
    // Generate random values before await
    let id = format!("agent-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    let wallet_id = format!("wallet-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    let hc_allocation = {
        let mut rng = rand::thread_rng();
        rng.gen_range(50.0..200.0f64)
    };

    let agents = state.agents.read().await;
    let parent = agents.get(&req.parent_id)
        .ok_or(AppError(StatusCode::NOT_FOUND, "Parent agent not found".to_string()))?;

    // 30% trust inheritance cap
    let inherited_score = (parent.trust_score.score as f64 * 0.30) as u16;
    let tau = inherited_score as f64 / 1000.0;

    let child = AgentV2 {
        id: id.clone(),
        name: req.name.clone(),
        agent_type: "agent".to_string(),
        trust_score: TrustScoreV2 {
            score: inherited_score,
            tau,
            tier: match inherited_score {
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
            id: wallet_id,
            balance: 100.0, // Starting balance
            reserved: 0.0,
            expires_at: Utc::now() + chrono::Duration::days(30),
        },
        fitness: FitnessMetrics {
            eta: tau * 0.5, // Start with low fitness
            revenue: 0.0,
            cost: 0.0,
            classification: "neutral".to_string(),
            hc_allocation,
        },
        status: "active".to_string(),
        created_at: Utc::now(),
    };

    drop(agents);
    state.agents.write().await.insert(id.clone(), child.clone());

    let _ = state.events_tx.send(Event {
        event_type: "agent_spawned".to_string(),
        payload: serde_json::json!({
            "parent_id": req.parent_id,
            "child": child
        }),
        timestamp: Utc::now(),
    });

    info!("Spawned agent {} from parent {} (inherited trust: {})", id, req.parent_id, inherited_score);
    Ok(Json(child))
}

// LEND: Create risk-priced loan based on TrustScore
async fn create_loan(
    State(state): State<AppState>,
    Json(req): Json<CreateLoanRequest>,
) -> Result<Json<Loan>, AppError> {
    let agents = state.agents.read().await;

    let borrower = agents.get(&req.borrower_id)
        .ok_or(AppError(StatusCode::NOT_FOUND, "Borrower not found".to_string()))?;

    let _lender = agents.get(&req.lender_id)
        .ok_or(AppError(StatusCode::NOT_FOUND, "Lender not found".to_string()))?;

    // Calculate interest rate based on borrower's trust score
    // Higher trust = lower rate. Base rate 3.2% APR
    let base_rate = 0.032;
    let tau = borrower.trust_score.tau;
    let interest_rate = base_rate * (2.0 - tau); // Range: 3.2% to 6.4%

    let id = format!("loan-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());

    let loan = Loan {
        id: id.clone(),
        lender_id: req.lender_id.clone(),
        borrower_id: req.borrower_id.clone(),
        principal: req.principal,
        interest_rate,
        term_days: req.term_days,
        status: "active".to_string(),
        created_at: Utc::now(),
        due_at: Utc::now() + chrono::Duration::days(req.term_days as i64),
        repaid_amount: 0.0,
    };

    drop(agents);
    state.loans.write().await.insert(id.clone(), loan.clone());

    let _ = state.events_tx.send(Event {
        event_type: "loan_created".to_string(),
        payload: serde_json::to_value(&loan).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Created loan {} (rate: {:.2}%)", id, interest_rate * 100.0);
    Ok(Json(loan))
}

async fn list_loans(State(state): State<AppState>) -> Json<Vec<Loan>> {
    let loans = state.loans.read().await;
    Json(loans.values().cloned().collect())
}

// INSURE: Create insurance policy with trust-based premium
async fn create_insurance(
    State(state): State<AppState>,
    Json(req): Json<CreateInsuranceRequest>,
) -> Result<Json<InsurancePolicy>, AppError> {
    let agents = state.agents.read().await;

    let insured = agents.get(&req.insured_id)
        .ok_or(AppError(StatusCode::NOT_FOUND, "Insured entity not found".to_string()))?;

    let _insurer = agents.get(&req.insurer_id)
        .ok_or(AppError(StatusCode::NOT_FOUND, "Insurer not found".to_string()))?;

    // Calculate premium based on trust score
    // Lower trust = higher premium. Base rate 8-12%
    let tau = insured.trust_score.tau;
    let risk_factor = 1.0 + (1.0 - tau); // 1.0 to 2.0
    let base_failure_prob = 0.05; // 5% base failure probability
    let premium_rate = base_failure_prob * risk_factor; // 5% to 10%
    let premium = req.coverage * premium_rate;

    let id = format!("policy-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());

    let policy = InsurancePolicy {
        id: id.clone(),
        insurer_id: req.insurer_id.clone(),
        insured_id: req.insured_id.clone(),
        coverage: req.coverage,
        premium,
        premium_rate,
        action_type: req.action_type,
        status: "active".to_string(),
        created_at: Utc::now(),
        expires_at: Utc::now() + chrono::Duration::days(req.duration_days as i64),
    };

    drop(agents);
    state.policies.write().await.insert(id.clone(), policy.clone());

    let _ = state.events_tx.send(Event {
        event_type: "policy_created".to_string(),
        payload: serde_json::to_value(&policy).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Created insurance policy {} (premium: {:.2} HC)", id, premium);
    Ok(Json(policy))
}

async fn list_policies(State(state): State<AppState>) -> Json<Vec<InsurancePolicy>> {
    let policies = state.policies.read().await;
    Json(policies.values().cloned().collect())
}

// DELEGATE: Create task delegation with escrow
async fn create_delegation(
    State(state): State<AppState>,
    Json(req): Json<CreateDelegationRequest>,
) -> Result<Json<Delegation>, AppError> {
    let mut agents = state.agents.write().await;

    let client = agents.get_mut(&req.client_id)
        .ok_or(AppError(StatusCode::NOT_FOUND, "Client not found".to_string()))?;

    // Check if client has enough balance
    if client.wallet.balance < req.escrow_amount {
        return Err(AppError(StatusCode::BAD_REQUEST, "Insufficient balance".to_string()));
    }

    // Lock escrow
    client.wallet.balance -= req.escrow_amount;
    client.wallet.reserved += req.escrow_amount;

    let id = format!("delegation-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());

    let delegation = Delegation {
        id: id.clone(),
        client_id: req.client_id.clone(),
        agent_id: req.agent_id.clone(),
        task_description: req.task_description,
        escrow_amount: req.escrow_amount,
        status: "pending".to_string(),
        created_at: Utc::now(),
        deadline: Utc::now() + chrono::Duration::days(req.deadline_days as i64),
        completed_at: None,
    };

    drop(agents);
    state.delegations.write().await.insert(id.clone(), delegation.clone());

    let _ = state.events_tx.send(Event {
        event_type: "delegation_created".to_string(),
        payload: serde_json::to_value(&delegation).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Created delegation {} (escrow: {} HC)", id, req.escrow_amount);
    Ok(Json(delegation))
}

async fn list_delegations(State(state): State<AppState>) -> Json<Vec<Delegation>> {
    let delegations = state.delegations.read().await;
    Json(delegations.values().cloned().collect())
}

async fn complete_delegation(
    State(state): State<AppState>,
    Path(delegation_id): Path<String>,
) -> Result<Json<Delegation>, AppError> {
    let mut delegations = state.delegations.write().await;
    let mut agents = state.agents.write().await;

    let delegation = delegations.get_mut(&delegation_id)
        .ok_or(AppError(StatusCode::NOT_FOUND, "Delegation not found".to_string()))?;

    if delegation.status != "pending" && delegation.status != "active" {
        return Err(AppError(StatusCode::BAD_REQUEST, "Delegation already completed".to_string()));
    }

    // Release escrow to agent
    if let Some(client) = agents.get_mut(&delegation.client_id) {
        client.wallet.reserved -= delegation.escrow_amount;
    }
    if let Some(agent) = agents.get_mut(&delegation.agent_id) {
        agent.wallet.balance += delegation.escrow_amount;
        agent.fitness.revenue += delegation.escrow_amount;
    }

    delegation.status = "completed".to_string();
    delegation.completed_at = Some(Utc::now());

    let delegation_clone = delegation.clone();

    let _ = state.events_tx.send(Event {
        event_type: "delegation_completed".to_string(),
        payload: serde_json::to_value(&delegation_clone).unwrap(),
        timestamp: Utc::now(),
    });

    info!("Completed delegation {}", delegation_id);
    Ok(Json(delegation_clone))
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
        loans: Arc::new(RwLock::new(HashMap::new())),
        policies: Arc::new(RwLock::new(HashMap::new())),
        delegations: Arc::new(RwLock::new(HashMap::new())),
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
        // Protocol DNA - Spawn
        .route("/spawn", post(spawn_agent))
        // Protocol DNA - Lend
        .route("/loans", get(list_loans).post(create_loan))
        // Protocol DNA - Insure
        .route("/policies", get(list_policies).post(create_insurance))
        // Protocol DNA - Delegate
        .route("/delegations", get(list_delegations).post(create_delegation))
        .route("/delegations/:delegation_id/complete", post(complete_delegation))
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
    info!("Protocol DNA: /spawn, /loans, /policies, /delegations");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
