//! # Protocol DNA
//!
//! Programmable economic primitives for Actoris Economic OS.
//!
//! ## Primitives
//!
//! - **Spawn**: Create child agents (30% trust cap from parent)
//! - **Lend**: Risk-priced credit extension
//! - **Insure**: Outcome guarantees with premium pricing
//! - **Delegate**: Escrow-based task delegation with verification

pub mod primitives;
pub mod wasm;

// Re-export primitives
pub use primitives::{
    delegate::DelegatePrimitive,
    insure::InsurePrimitive,
    lend::LendPrimitive,
    spawn::SpawnPrimitive,
};
