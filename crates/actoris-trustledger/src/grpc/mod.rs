//! gRPC service implementation for TrustLedger
//!
//! Provides:
//! - TrustLedgerService for action verification
//! - OracleService for oracle consensus participation

pub mod service;

pub use service::{
    OracleGrpcService, OracleService, TrustLedgerGrpcService, TrustLedgerService,
};
