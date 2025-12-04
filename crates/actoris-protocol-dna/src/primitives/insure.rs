//! Insure primitive - Outcome guarantees

use rust_decimal::Decimal;

/// Insure outcomes with premium pricing
pub struct InsurePrimitive;

impl InsurePrimitive {
    /// Calculate insurance premium
    pub fn calculate_premium(
        coverage: Decimal,
        trust_score: u16,
        failure_probability: f64,
    ) -> Decimal {
        let tau = trust_score as f64 / 1000.0;
        // Lower trust = higher premium
        let risk_factor = 1.0 + (1.0 - tau);
        let base_premium = failure_probability * risk_factor;
        coverage * Decimal::try_from(base_premium).unwrap_or(Decimal::ONE)
    }
}
