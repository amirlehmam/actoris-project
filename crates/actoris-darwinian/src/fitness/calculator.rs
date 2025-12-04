//! Fitness calculation: η = τ × (revenue / cost)

use rust_decimal::Decimal;

pub struct FitnessCalculator;

impl FitnessCalculator {
    /// Calculate fitness: η = τ × (revenue / cost)
    pub fn calculate(tau: f64, revenue: Decimal, cost: Decimal) -> f64 {
        if cost.is_zero() {
            return 0.0;
        }
        let roi: f64 = (revenue / cost).try_into().unwrap_or(0.0);
        tau * roi
    }
}
