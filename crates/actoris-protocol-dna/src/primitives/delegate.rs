//! Delegate primitive - Escrow + verification

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Task delegation with escrow
pub struct DelegatePrimitive;

/// Delegation contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationContract {
    pub id: String,
    pub client_did: String,
    pub agent_did: String,
    pub task_description: String,
    pub escrow_amount: Decimal,
    pub deadline: i64,
    pub status: DelegationStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DelegationStatus {
    Pending,
    Active,
    Completed,
    Disputed,
    Cancelled,
}

impl DelegatePrimitive {
    /// Create a new delegation (locks escrow)
    pub fn create_delegation(
        client_did: String,
        agent_did: String,
        task_description: String,
        escrow_amount: Decimal,
        deadline: i64,
    ) -> DelegationContract {
        DelegationContract {
            id: uuid::Uuid::now_v7().to_string(),
            client_did,
            agent_did,
            task_description,
            escrow_amount,
            deadline,
            status: DelegationStatus::Pending,
        }
    }
}
