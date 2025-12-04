//! Protocol DNA - The Four Primitives
//!
//! Protocol DNA defines the foundational operations in the ACTORIS ecosystem:
//!
//! 1. **SPAWN**: Create new agents from parent entities
//!    - Inherits 30% of parent trust (τ)
//!    - Requires stake deposit
//!    - Creates identity lineage
//!
//! 2. **LEND**: Extend compute credits (HC) to agents
//!    - Credit line with interest rate based on trust
//!    - Collateral requirements for low-trust borrowers
//!    - Auto-repayment from agent earnings
//!
//! 3. **INSURE**: Protect against agent failures
//!    - Premium based on historical performance
//!    - Coverage for failed/malicious actions
//!    - Claims process with oracle verification
//!
//! 4. **DELEGATE**: Transfer authority between entities
//!    - Scoped permissions (actions, limits, duration)
//!    - Revocable at any time
//!    - Audit trail

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use parking_lot::RwLock;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Protocol DNA errors
#[derive(Debug, Error)]
pub enum DnaError {
    #[error("Insufficient trust: need τ >= {required}, have τ = {current}")]
    InsufficientTrust { required: f64, current: f64 },

    #[error("Insufficient funds: need {required} HC, have {available} HC")]
    InsufficientFunds {
        required: Decimal,
        available: Decimal,
    },

    #[error("Spawn limit exceeded: max depth is {max_depth}")]
    SpawnDepthExceeded { max_depth: u32 },

    #[error("Invalid delegation: {reason}")]
    InvalidDelegation { reason: String },

    #[error("Loan already exists: {loan_id}")]
    LoanExists { loan_id: String },

    #[error("Loan not found: {loan_id}")]
    LoanNotFound { loan_id: String },

    #[error("Insurance policy not found: {policy_id}")]
    PolicyNotFound { policy_id: String },

    #[error("Claim denied: {reason}")]
    ClaimDenied { reason: String },

    #[error("Delegation not found: {delegation_id}")]
    DelegationNotFound { delegation_id: String },

    #[error("Delegation expired")]
    DelegationExpired,

    #[error("Permission denied: {action}")]
    PermissionDenied { action: String },
}

/// DNA Primitive type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DnaPrimitive {
    Spawn,
    Lend,
    Insure,
    Delegate,
}

/// SPAWN request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Parent DID (spawner)
    pub parent_did: String,
    /// New agent DID
    pub child_did: String,
    /// Initial HC allocation
    pub initial_hc: Decimal,
    /// Stake deposit (locked)
    pub stake_amount: Decimal,
    /// Agent metadata
    pub metadata: HashMap<String, String>,
}

/// SPAWN result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnResult {
    pub spawn_id: String,
    pub child_did: String,
    pub inherited_tau: f64,
    pub initial_hc: Decimal,
    pub stake_locked: Decimal,
    pub timestamp: i64,
}

/// LEND request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LendRequest {
    /// Lender DID
    pub lender_did: String,
    /// Borrower DID
    pub borrower_did: String,
    /// Loan amount in HC
    pub amount: Decimal,
    /// Interest rate (annual, e.g., 0.05 = 5%)
    pub interest_rate: Option<f64>,
    /// Collateral percentage (0.0 - 1.0)
    pub collateral_pct: f64,
    /// Loan duration
    pub duration_days: u32,
    /// Auto-repay from borrower earnings
    pub auto_repay: bool,
}

/// Active loan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loan {
    pub loan_id: String,
    pub lender_did: String,
    pub borrower_did: String,
    pub principal: Decimal,
    pub interest_rate: f64,
    pub collateral_amount: Decimal,
    pub outstanding: Decimal,
    pub repaid: Decimal,
    pub started_at: i64,
    pub expires_at: i64,
    pub auto_repay: bool,
    pub status: LoanStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoanStatus {
    Active,
    Repaid,
    Defaulted,
    Liquidated,
}

/// INSURE request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsureRequest {
    /// Insured DID (the agent being insured)
    pub insured_did: String,
    /// Insurer DID (the entity providing insurance)
    pub insurer_did: String,
    /// Coverage amount in HC
    pub coverage_amount: Decimal,
    /// Premium percentage (of coverage)
    pub premium_pct: Option<f64>,
    /// Coverage duration
    pub duration_days: u32,
    /// Covered action types (empty = all)
    pub covered_actions: Vec<String>,
}

/// Insurance policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsurancePolicy {
    pub policy_id: String,
    pub insured_did: String,
    pub insurer_did: String,
    pub coverage_amount: Decimal,
    pub premium_paid: Decimal,
    pub premium_rate: f64,
    pub started_at: i64,
    pub expires_at: i64,
    pub covered_actions: Vec<String>,
    pub claims_made: u32,
    pub claims_paid: Decimal,
    pub status: PolicyStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyStatus {
    Active,
    Expired,
    Cancelled,
    ClaimsPending,
}

/// Insurance claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsuranceClaim {
    pub claim_id: String,
    pub policy_id: String,
    pub action_id: String,
    pub claimed_amount: Decimal,
    pub reason: String,
    pub evidence_hash: [u8; 32],
    pub submitted_at: i64,
    pub status: ClaimStatus,
    pub payout: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimStatus {
    Pending,
    UnderReview,
    Approved,
    Denied,
    Paid,
}

/// DELEGATE request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateRequest {
    /// Delegator DID (granting authority)
    pub delegator_did: String,
    /// Delegate DID (receiving authority)
    pub delegate_did: String,
    /// Allowed actions (empty = all)
    pub allowed_actions: Vec<String>,
    /// Maximum HC per action
    pub max_hc_per_action: Option<Decimal>,
    /// Maximum total HC
    pub max_total_hc: Option<Decimal>,
    /// Delegation duration
    pub duration_days: u32,
    /// Sub-delegation allowed
    pub allow_subdelegation: bool,
}

/// Active delegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub delegation_id: String,
    pub delegator_did: String,
    pub delegate_did: String,
    pub allowed_actions: Vec<String>,
    pub max_hc_per_action: Option<Decimal>,
    pub max_total_hc: Option<Decimal>,
    pub hc_used: Decimal,
    pub started_at: i64,
    pub expires_at: i64,
    pub allow_subdelegation: bool,
    pub revoked: bool,
    pub parent_delegation_id: Option<String>,
}

/// Protocol DNA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnaConfig {
    /// Maximum spawn depth (generations from root)
    pub max_spawn_depth: u32,
    /// Inherited trust percentage (e.g., 0.30 = 30%)
    pub inherited_trust_pct: f64,
    /// Minimum trust for spawning
    pub min_spawn_tau: f64,
    /// Minimum trust for lending
    pub min_lend_tau: f64,
    /// Base interest rate for loans
    pub base_interest_rate: f64,
    /// Interest discount per 0.1 tau
    pub interest_discount_per_tau: f64,
    /// Base insurance premium rate
    pub base_premium_rate: f64,
    /// Premium discount per 0.1 tau
    pub premium_discount_per_tau: f64,
    /// Minimum stake for spawning
    pub min_spawn_stake: Decimal,
}

impl Default for DnaConfig {
    fn default() -> Self {
        Self {
            max_spawn_depth: 5,
            inherited_trust_pct: 0.30,
            min_spawn_tau: 0.3,
            min_lend_tau: 0.5,
            base_interest_rate: 0.10, // 10% annual
            interest_discount_per_tau: 0.01, // 1% discount per 0.1 tau
            base_premium_rate: 0.05, // 5% of coverage
            premium_discount_per_tau: 0.005, // 0.5% discount per 0.1 tau
            min_spawn_stake: Decimal::new(100, 0), // 100 HC
        }
    }
}

/// Protocol DNA executor
pub struct ProtocolDna {
    config: DnaConfig,

    /// Active loans
    loans: Arc<RwLock<HashMap<String, Loan>>>,

    /// Insurance policies
    policies: Arc<RwLock<HashMap<String, InsurancePolicy>>>,

    /// Insurance claims
    claims: Arc<RwLock<HashMap<String, InsuranceClaim>>>,

    /// Active delegations
    delegations: Arc<RwLock<HashMap<String, Delegation>>>,

    /// Spawn registry (child -> parent)
    spawn_registry: Arc<RwLock<HashMap<String, String>>>,

    /// Spawn depths (did -> depth)
    spawn_depths: Arc<RwLock<HashMap<String, u32>>>,

    /// Trust scores (did -> tau)
    trust_scores: Arc<RwLock<HashMap<String, f64>>>,

    /// HC balances (did -> available)
    hc_balances: Arc<RwLock<HashMap<String, Decimal>>>,
}

impl ProtocolDna {
    /// Create new Protocol DNA executor
    pub fn new(config: DnaConfig) -> Self {
        Self {
            config,
            loans: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            claims: Arc::new(RwLock::new(HashMap::new())),
            delegations: Arc::new(RwLock::new(HashMap::new())),
            spawn_registry: Arc::new(RwLock::new(HashMap::new())),
            spawn_depths: Arc::new(RwLock::new(HashMap::new())),
            trust_scores: Arc::new(RwLock::new(HashMap::new())),
            hc_balances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set trust score for a DID
    pub fn set_trust(&self, did: &str, tau: f64) {
        self.trust_scores.write().insert(did.to_string(), tau.clamp(0.0, 1.0));
    }

    /// Get trust score for a DID
    pub fn get_trust(&self, did: &str) -> f64 {
        self.trust_scores.read().get(did).copied().unwrap_or(0.0)
    }

    /// Set HC balance for a DID
    pub fn set_balance(&self, did: &str, balance: Decimal) {
        self.hc_balances.write().insert(did.to_string(), balance);
    }

    /// Get HC balance for a DID
    pub fn get_balance(&self, did: &str) -> Decimal {
        self.hc_balances.read().get(did).copied().unwrap_or(Decimal::ZERO)
    }

    // ============ SPAWN ============

    /// Execute SPAWN primitive
    pub fn spawn(&self, request: SpawnRequest) -> Result<SpawnResult, DnaError> {
        let parent_tau = self.get_trust(&request.parent_did);

        // Check minimum trust
        if parent_tau < self.config.min_spawn_tau {
            return Err(DnaError::InsufficientTrust {
                required: self.config.min_spawn_tau,
                current: parent_tau,
            });
        }

        // Check spawn depth
        let parent_depth = self.spawn_depths.read().get(&request.parent_did).copied().unwrap_or(0);
        if parent_depth >= self.config.max_spawn_depth {
            return Err(DnaError::SpawnDepthExceeded {
                max_depth: self.config.max_spawn_depth,
            });
        }

        // Check funds
        let parent_balance = self.get_balance(&request.parent_did);
        let required = request.initial_hc + request.stake_amount;
        if parent_balance < required {
            return Err(DnaError::InsufficientFunds {
                required,
                available: parent_balance,
            });
        }

        // Check minimum stake
        if request.stake_amount < self.config.min_spawn_stake {
            return Err(DnaError::InsufficientFunds {
                required: self.config.min_spawn_stake,
                available: request.stake_amount,
            });
        }

        // Calculate inherited trust
        let inherited_tau = parent_tau * self.config.inherited_trust_pct;

        // Deduct from parent
        {
            let mut balances = self.hc_balances.write();
            if let Some(balance) = balances.get_mut(&request.parent_did) {
                *balance -= required;
            }
        }

        // Set child trust and balance
        self.set_trust(&request.child_did, inherited_tau);
        self.set_balance(&request.child_did, request.initial_hc);

        // Record lineage
        self.spawn_registry
            .write()
            .insert(request.child_did.clone(), request.parent_did.clone());
        self.spawn_depths
            .write()
            .insert(request.child_did.clone(), parent_depth + 1);

        let spawn_id = Uuid::now_v7().to_string();
        let timestamp = chrono::Utc::now().timestamp_millis();

        info!(
            spawn_id = %spawn_id,
            parent = %request.parent_did,
            child = %request.child_did,
            tau = inherited_tau,
            hc = %request.initial_hc,
            "Agent spawned"
        );

        Ok(SpawnResult {
            spawn_id,
            child_did: request.child_did,
            inherited_tau,
            initial_hc: request.initial_hc,
            stake_locked: request.stake_amount,
            timestamp,
        })
    }

    // ============ LEND ============

    /// Execute LEND primitive
    pub fn lend(&self, request: LendRequest) -> Result<Loan, DnaError> {
        let lender_tau = self.get_trust(&request.lender_did);
        let borrower_tau = self.get_trust(&request.borrower_did);

        // Check lender trust
        if lender_tau < self.config.min_lend_tau {
            return Err(DnaError::InsufficientTrust {
                required: self.config.min_lend_tau,
                current: lender_tau,
            });
        }

        // Check lender funds
        let lender_balance = self.get_balance(&request.lender_did);
        if lender_balance < request.amount {
            return Err(DnaError::InsufficientFunds {
                required: request.amount,
                available: lender_balance,
            });
        }

        // Calculate collateral
        let collateral_amount = request.amount * Decimal::try_from(request.collateral_pct).unwrap();
        let borrower_balance = self.get_balance(&request.borrower_did);
        if borrower_balance < collateral_amount {
            return Err(DnaError::InsufficientFunds {
                required: collateral_amount,
                available: borrower_balance,
            });
        }

        // Calculate interest rate (discount based on borrower trust)
        let interest_rate = request.interest_rate.unwrap_or_else(|| {
            let discount = borrower_tau * 10.0 * self.config.interest_discount_per_tau;
            (self.config.base_interest_rate - discount).max(0.01)
        });

        // Transfer funds
        {
            let mut balances = self.hc_balances.write();
            if let Some(lender_bal) = balances.get_mut(&request.lender_did) {
                *lender_bal -= request.amount;
            }
            if let Some(borrower_bal) = balances.get_mut(&request.borrower_did) {
                *borrower_bal -= collateral_amount;
                *borrower_bal += request.amount;
            }
        }

        let loan_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let duration_ms = (request.duration_days as i64) * 24 * 60 * 60 * 1000;

        let loan = Loan {
            loan_id: loan_id.clone(),
            lender_did: request.lender_did.clone(),
            borrower_did: request.borrower_did.clone(),
            principal: request.amount,
            interest_rate,
            collateral_amount,
            outstanding: request.amount,
            repaid: Decimal::ZERO,
            started_at: now,
            expires_at: now + duration_ms,
            auto_repay: request.auto_repay,
            status: LoanStatus::Active,
        };

        self.loans.write().insert(loan_id.clone(), loan.clone());

        info!(
            loan_id = %loan_id,
            lender = %request.lender_did,
            borrower = %request.borrower_did,
            amount = %request.amount,
            rate = interest_rate,
            "Loan created"
        );

        Ok(loan)
    }

    /// Repay loan
    pub fn repay_loan(&self, loan_id: &str, amount: Decimal) -> Result<Loan, DnaError> {
        let mut loans = self.loans.write();
        let loan = loans
            .get_mut(loan_id)
            .ok_or_else(|| DnaError::LoanNotFound {
                loan_id: loan_id.to_string(),
            })?;

        if loan.status != LoanStatus::Active {
            return Err(DnaError::LoanNotFound {
                loan_id: loan_id.to_string(),
            });
        }

        // Check borrower funds
        let borrower_balance = self.get_balance(&loan.borrower_did);
        let repay_amount = amount.min(loan.outstanding);
        if borrower_balance < repay_amount {
            return Err(DnaError::InsufficientFunds {
                required: repay_amount,
                available: borrower_balance,
            });
        }

        // Transfer repayment
        {
            let mut balances = self.hc_balances.write();
            if let Some(borrower_bal) = balances.get_mut(&loan.borrower_did) {
                *borrower_bal -= repay_amount;
            }
            let lender_bal = balances.entry(loan.lender_did.clone()).or_insert(Decimal::ZERO);
            *lender_bal += repay_amount;
        }

        loan.repaid += repay_amount;
        loan.outstanding -= repay_amount;

        if loan.outstanding <= Decimal::ZERO {
            loan.status = LoanStatus::Repaid;
            // Return collateral
            let mut balances = self.hc_balances.write();
            if let Some(borrower_bal) = balances.get_mut(&loan.borrower_did) {
                *borrower_bal += loan.collateral_amount;
            }
        }

        info!(
            loan_id = %loan_id,
            repaid = %repay_amount,
            outstanding = %loan.outstanding,
            status = ?loan.status,
            "Loan repayment"
        );

        Ok(loan.clone())
    }

    // ============ INSURE ============

    /// Execute INSURE primitive
    pub fn insure(&self, request: InsureRequest) -> Result<InsurancePolicy, DnaError> {
        let insured_tau = self.get_trust(&request.insured_did);
        let insurer_balance = self.get_balance(&request.insurer_did);

        // Check insurer has enough to cover
        if insurer_balance < request.coverage_amount {
            return Err(DnaError::InsufficientFunds {
                required: request.coverage_amount,
                available: insurer_balance,
            });
        }

        // Calculate premium (discount based on insured trust)
        let premium_rate = request.premium_pct.unwrap_or_else(|| {
            let discount = insured_tau * 10.0 * self.config.premium_discount_per_tau;
            (self.config.base_premium_rate - discount).max(0.01)
        });
        let premium = request.coverage_amount * Decimal::try_from(premium_rate).unwrap();

        // Check insured can pay premium
        let insured_balance = self.get_balance(&request.insured_did);
        if insured_balance < premium {
            return Err(DnaError::InsufficientFunds {
                required: premium,
                available: insured_balance,
            });
        }

        // Transfer premium
        {
            let mut balances = self.hc_balances.write();
            if let Some(insured_bal) = balances.get_mut(&request.insured_did) {
                *insured_bal -= premium;
            }
            let insurer_bal = balances.entry(request.insurer_did.clone()).or_insert(Decimal::ZERO);
            *insurer_bal += premium;
        }

        let policy_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let duration_ms = (request.duration_days as i64) * 24 * 60 * 60 * 1000;

        let policy = InsurancePolicy {
            policy_id: policy_id.clone(),
            insured_did: request.insured_did.clone(),
            insurer_did: request.insurer_did.clone(),
            coverage_amount: request.coverage_amount,
            premium_paid: premium,
            premium_rate,
            started_at: now,
            expires_at: now + duration_ms,
            covered_actions: request.covered_actions,
            claims_made: 0,
            claims_paid: Decimal::ZERO,
            status: PolicyStatus::Active,
        };

        self.policies.write().insert(policy_id.clone(), policy.clone());

        info!(
            policy_id = %policy_id,
            insured = %request.insured_did,
            insurer = %request.insurer_did,
            coverage = %request.coverage_amount,
            premium = %premium,
            "Insurance policy created"
        );

        Ok(policy)
    }

    /// File insurance claim
    pub fn file_claim(
        &self,
        policy_id: &str,
        action_id: &str,
        amount: Decimal,
        reason: &str,
        evidence: &[u8],
    ) -> Result<InsuranceClaim, DnaError> {
        let mut policies = self.policies.write();
        let policy = policies
            .get_mut(policy_id)
            .ok_or_else(|| DnaError::PolicyNotFound {
                policy_id: policy_id.to_string(),
            })?;

        if policy.status != PolicyStatus::Active {
            return Err(DnaError::PolicyNotFound {
                policy_id: policy_id.to_string(),
            });
        }

        // Check expiry
        let now = chrono::Utc::now().timestamp_millis();
        if now > policy.expires_at {
            policy.status = PolicyStatus::Expired;
            return Err(DnaError::PolicyNotFound {
                policy_id: policy_id.to_string(),
            });
        }

        // Check coverage
        if amount > policy.coverage_amount - policy.claims_paid {
            return Err(DnaError::ClaimDenied {
                reason: "Claim exceeds remaining coverage".to_string(),
            });
        }

        let claim_id = Uuid::now_v7().to_string();
        let evidence_hash = *blake3::hash(evidence).as_bytes();

        let claim = InsuranceClaim {
            claim_id: claim_id.clone(),
            policy_id: policy_id.to_string(),
            action_id: action_id.to_string(),
            claimed_amount: amount,
            reason: reason.to_string(),
            evidence_hash,
            submitted_at: now,
            status: ClaimStatus::Pending,
            payout: None,
        };

        policy.claims_made += 1;
        policy.status = PolicyStatus::ClaimsPending;

        self.claims.write().insert(claim_id.clone(), claim.clone());

        info!(
            claim_id = %claim_id,
            policy_id = %policy_id,
            amount = %amount,
            "Insurance claim filed"
        );

        Ok(claim)
    }

    /// Process insurance claim (oracle verified)
    pub fn process_claim(&self, claim_id: &str, approved: bool, payout: Option<Decimal>) -> Result<InsuranceClaim, DnaError> {
        let mut claims = self.claims.write();
        let claim = claims
            .get_mut(claim_id)
            .ok_or_else(|| DnaError::ClaimDenied {
                reason: "Claim not found".to_string(),
            })?;

        if claim.status != ClaimStatus::Pending && claim.status != ClaimStatus::UnderReview {
            return Err(DnaError::ClaimDenied {
                reason: "Claim already processed".to_string(),
            });
        }

        if approved {
            let payout_amount = payout.unwrap_or(claim.claimed_amount);
            claim.status = ClaimStatus::Approved;
            claim.payout = Some(payout_amount);

            // Update policy
            let mut policies = self.policies.write();
            if let Some(policy) = policies.get_mut(&claim.policy_id) {
                policy.claims_paid += payout_amount;

                // Transfer payout
                let mut balances = self.hc_balances.write();
                if let Some(insurer_bal) = balances.get_mut(&policy.insurer_did) {
                    *insurer_bal -= payout_amount;
                }
                let insured_bal = balances.entry(policy.insured_did.clone()).or_insert(Decimal::ZERO);
                *insured_bal += payout_amount;

                if policy.claims_paid >= policy.coverage_amount {
                    policy.status = PolicyStatus::Expired;
                } else {
                    policy.status = PolicyStatus::Active;
                }
            }

            claim.status = ClaimStatus::Paid;
        } else {
            claim.status = ClaimStatus::Denied;

            // Reset policy status
            let mut policies = self.policies.write();
            if let Some(policy) = policies.get_mut(&claim.policy_id) {
                if policy.status == PolicyStatus::ClaimsPending {
                    policy.status = PolicyStatus::Active;
                }
            }
        }

        info!(
            claim_id = %claim_id,
            approved = approved,
            payout = ?claim.payout,
            "Claim processed"
        );

        Ok(claim.clone())
    }

    // ============ DELEGATE ============

    /// Execute DELEGATE primitive
    pub fn delegate(&self, request: DelegateRequest) -> Result<Delegation, DnaError> {
        let delegator_tau = self.get_trust(&request.delegator_did);

        // Must have some trust to delegate
        if delegator_tau < 0.1 {
            return Err(DnaError::InsufficientTrust {
                required: 0.1,
                current: delegator_tau,
            });
        }

        let delegation_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let duration_ms = (request.duration_days as i64) * 24 * 60 * 60 * 1000;

        let delegation = Delegation {
            delegation_id: delegation_id.clone(),
            delegator_did: request.delegator_did.clone(),
            delegate_did: request.delegate_did.clone(),
            allowed_actions: request.allowed_actions,
            max_hc_per_action: request.max_hc_per_action,
            max_total_hc: request.max_total_hc,
            hc_used: Decimal::ZERO,
            started_at: now,
            expires_at: now + duration_ms,
            allow_subdelegation: request.allow_subdelegation,
            revoked: false,
            parent_delegation_id: None,
        };

        self.delegations.write().insert(delegation_id.clone(), delegation.clone());

        info!(
            delegation_id = %delegation_id,
            delegator = %request.delegator_did,
            delegate = %request.delegate_did,
            "Delegation created"
        );

        Ok(delegation)
    }

    /// Check if action is authorized by delegation
    pub fn check_delegation(
        &self,
        delegate_did: &str,
        action: &str,
        hc_amount: Decimal,
    ) -> Result<String, DnaError> {
        let delegations = self.delegations.read();
        let now = chrono::Utc::now().timestamp_millis();

        for (id, delegation) in delegations.iter() {
            if delegation.delegate_did != delegate_did {
                continue;
            }
            if delegation.revoked {
                continue;
            }
            if now > delegation.expires_at {
                continue;
            }

            // Check action allowed
            if !delegation.allowed_actions.is_empty()
                && !delegation.allowed_actions.contains(&action.to_string())
            {
                continue;
            }

            // Check per-action limit
            if let Some(max_per) = delegation.max_hc_per_action {
                if hc_amount > max_per {
                    continue;
                }
            }

            // Check total limit
            if let Some(max_total) = delegation.max_total_hc {
                if delegation.hc_used + hc_amount > max_total {
                    continue;
                }
            }

            return Ok(id.clone());
        }

        Err(DnaError::PermissionDenied {
            action: action.to_string(),
        })
    }

    /// Use HC under delegation
    pub fn use_delegation(&self, delegation_id: &str, hc_amount: Decimal) -> Result<(), DnaError> {
        let mut delegations = self.delegations.write();
        let delegation = delegations
            .get_mut(delegation_id)
            .ok_or_else(|| DnaError::DelegationNotFound {
                delegation_id: delegation_id.to_string(),
            })?;

        let now = chrono::Utc::now().timestamp_millis();
        if now > delegation.expires_at {
            return Err(DnaError::DelegationExpired);
        }

        if delegation.revoked {
            return Err(DnaError::DelegationNotFound {
                delegation_id: delegation_id.to_string(),
            });
        }

        delegation.hc_used += hc_amount;

        debug!(
            delegation_id = %delegation_id,
            used = %hc_amount,
            total_used = %delegation.hc_used,
            "Delegation HC used"
        );

        Ok(())
    }

    /// Revoke delegation
    pub fn revoke_delegation(&self, delegation_id: &str, revoker_did: &str) -> Result<(), DnaError> {
        let mut delegations = self.delegations.write();
        let delegation = delegations
            .get_mut(delegation_id)
            .ok_or_else(|| DnaError::DelegationNotFound {
                delegation_id: delegation_id.to_string(),
            })?;

        if delegation.delegator_did != revoker_did {
            return Err(DnaError::PermissionDenied {
                action: "revoke delegation".to_string(),
            });
        }

        delegation.revoked = true;

        info!(
            delegation_id = %delegation_id,
            "Delegation revoked"
        );

        Ok(())
    }

    /// Get active loans for a borrower
    pub fn get_borrower_loans(&self, borrower_did: &str) -> Vec<Loan> {
        self.loans
            .read()
            .values()
            .filter(|l| l.borrower_did == borrower_did && l.status == LoanStatus::Active)
            .cloned()
            .collect()
    }

    /// Get active policies for an insured
    pub fn get_insured_policies(&self, insured_did: &str) -> Vec<InsurancePolicy> {
        self.policies
            .read()
            .values()
            .filter(|p| p.insured_did == insured_did && p.status == PolicyStatus::Active)
            .cloned()
            .collect()
    }

    /// Get active delegations for a delegate
    pub fn get_delegate_permissions(&self, delegate_did: &str) -> Vec<Delegation> {
        let now = chrono::Utc::now().timestamp_millis();
        self.delegations
            .read()
            .values()
            .filter(|d| d.delegate_did == delegate_did && !d.revoked && now <= d.expires_at)
            .cloned()
            .collect()
    }
}

impl Default for ProtocolDna {
    fn default() -> Self {
        Self::new(DnaConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_spawn() {
        let dna = ProtocolDna::default();

        // Set up parent
        dna.set_trust("did:key:parent", 0.8);
        dna.set_balance("did:key:parent", dec!(1000));

        // Spawn child
        let result = dna.spawn(SpawnRequest {
            parent_did: "did:key:parent".to_string(),
            child_did: "did:key:child".to_string(),
            initial_hc: dec!(100),
            stake_amount: dec!(100),
            metadata: HashMap::new(),
        });

        assert!(result.is_ok());
        let spawn = result.unwrap();
        assert_eq!(spawn.inherited_tau, 0.8 * 0.30); // 30% inheritance

        // Child should have balance
        assert_eq!(dna.get_balance("did:key:child"), dec!(100));

        // Parent balance reduced
        assert_eq!(dna.get_balance("did:key:parent"), dec!(800));
    }

    #[test]
    fn test_lend_and_repay() {
        let dna = ProtocolDna::default();

        // Set up lender and borrower
        dna.set_trust("did:key:lender", 0.9);
        dna.set_trust("did:key:borrower", 0.5);
        dna.set_balance("did:key:lender", dec!(1000));
        dna.set_balance("did:key:borrower", dec!(200)); // For collateral

        // Create loan
        let loan = dna.lend(LendRequest {
            lender_did: "did:key:lender".to_string(),
            borrower_did: "did:key:borrower".to_string(),
            amount: dec!(500),
            interest_rate: None, // Auto-calculate
            collateral_pct: 0.2,
            duration_days: 30,
            auto_repay: true,
        }).unwrap();

        assert_eq!(loan.principal, dec!(500));
        assert_eq!(loan.collateral_amount, dec!(100));
        assert!(loan.interest_rate < 0.10); // Should have discount

        // Borrower balance should be initial + loan - collateral
        assert_eq!(dna.get_balance("did:key:borrower"), dec!(600)); // 200 + 500 - 100

        // Repay loan
        dna.set_balance("did:key:borrower", dec!(600));
        let repaid = dna.repay_loan(&loan.loan_id, dec!(500)).unwrap();
        assert_eq!(repaid.status, LoanStatus::Repaid);

        // Collateral returned
        assert_eq!(dna.get_balance("did:key:borrower"), dec!(200)); // 600 - 500 + 100 collateral
    }

    #[test]
    fn test_insure_and_claim() {
        let dna = ProtocolDna::default();

        // Set up parties
        dna.set_trust("did:key:insured", 0.7);
        dna.set_balance("did:key:insured", dec!(100));
        dna.set_balance("did:key:insurer", dec!(1000));

        // Create policy
        let policy = dna.insure(InsureRequest {
            insured_did: "did:key:insured".to_string(),
            insurer_did: "did:key:insurer".to_string(),
            coverage_amount: dec!(500),
            premium_pct: None,
            duration_days: 30,
            covered_actions: vec![],
        }).unwrap();

        assert!(policy.premium_paid > Decimal::ZERO);
        assert!(policy.premium_paid < dec!(50)); // Should be reasonable

        // File claim
        let claim = dna.file_claim(
            &policy.policy_id,
            "action_123",
            dec!(100),
            "Service failure",
            b"evidence data",
        ).unwrap();

        assert_eq!(claim.status, ClaimStatus::Pending);

        // Process claim
        let processed = dna.process_claim(&claim.claim_id, true, Some(dec!(100))).unwrap();
        assert_eq!(processed.status, ClaimStatus::Paid);
        assert_eq!(processed.payout, Some(dec!(100)));
    }

    #[test]
    fn test_delegate() {
        let dna = ProtocolDna::default();

        // Set up delegator
        dna.set_trust("did:key:delegator", 0.8);

        // Create delegation
        let delegation = dna.delegate(DelegateRequest {
            delegator_did: "did:key:delegator".to_string(),
            delegate_did: "did:key:delegate".to_string(),
            allowed_actions: vec!["compute.*".to_string()],
            max_hc_per_action: Some(dec!(100)),
            max_total_hc: Some(dec!(1000)),
            duration_days: 7,
            allow_subdelegation: false,
        }).unwrap();

        assert!(!delegation.revoked);

        // Check delegation
        let check = dna.check_delegation("did:key:delegate", "compute.run", dec!(50));
        assert!(check.is_ok());

        // Use delegation
        dna.use_delegation(&delegation.delegation_id, dec!(50)).unwrap();

        // Get permissions
        let perms = dna.get_delegate_permissions("did:key:delegate");
        assert_eq!(perms.len(), 1);
        assert_eq!(perms[0].hc_used, dec!(50));

        // Revoke
        dna.revoke_delegation(&delegation.delegation_id, "did:key:delegator").unwrap();

        let perms = dna.get_delegate_permissions("did:key:delegate");
        assert!(perms.is_empty());
    }
}
