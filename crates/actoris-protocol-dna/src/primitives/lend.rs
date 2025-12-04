//! Lend primitive - Risk-priced credit extension

use rust_decimal::Decimal;

/// Lend HC credits with risk-adjusted pricing
pub struct LendPrimitive;

impl LendPrimitive {
    /// Calculate credit limit based on trust score
    pub fn calculate_credit_limit(base_limit: Decimal, trust_score: u16) -> Decimal {
        let multiplier = Self::credit_multiplier(trust_score);
        base_limit * Decimal::try_from(multiplier).unwrap_or(Decimal::ONE)
    }

    /// Credit multiplier: 0.1x at 0 trust, 3x at 1000 trust
    fn credit_multiplier(trust_score: u16) -> f64 {
        let tau = trust_score as f64 / 1000.0;
        0.1 + (tau.powf(2.0) * 2.9)
    }

    /// Calculate interest rate based on trust (inverse relationship)
    pub fn calculate_interest_rate(base_rate: f64, trust_score: u16) -> f64 {
        let tau = trust_score as f64 / 1000.0;
        base_rate * (2.0 - tau) // High trust = lower rate
    }
}
