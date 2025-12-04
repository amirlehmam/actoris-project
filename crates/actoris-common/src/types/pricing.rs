//! Pricing Types - Price = Compute + Risk - Trust
//!
//! The Actoris pricing formula balances three factors:
//! - C (Compute): Base cost in PFLOP-hours
//! - R (Risk): Premium based on task/data sensitivity
//! - T (Trust): Discount for high-trust actors (max 20%)
//!
//! Final Price: P = C + R - T

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Task complexity levels affecting risk premium
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskComplexity {
    /// Simple, well-defined tasks (+0% risk)
    Low,
    /// Moderate complexity, some uncertainty (+10% risk)
    Medium,
    /// Complex tasks, significant uncertainty (+25% risk)
    High,
    /// Mission-critical, failure has severe consequences (+50% risk)
    Critical,
}

impl TaskComplexity {
    /// Get the risk multiplier for this complexity level
    pub fn risk_multiplier(&self) -> f64 {
        match self {
            TaskComplexity::Low => 0.0,
            TaskComplexity::Medium => 0.10,
            TaskComplexity::High => 0.25,
            TaskComplexity::Critical => 0.50,
        }
    }
}

impl Default for TaskComplexity {
    fn default() -> Self {
        TaskComplexity::Medium
    }
}

/// Data sensitivity levels affecting risk premium
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSensitivity {
    /// Public data, no privacy concerns (+0% risk)
    Public,
    /// Internal business data (+5% risk)
    Internal,
    /// Customer/personal data (+15% risk)
    Confidential,
    /// Regulated data (PII, PHI, financial) (+30% risk)
    Restricted,
}

impl DataSensitivity {
    /// Get the risk multiplier for this sensitivity level
    pub fn risk_multiplier(&self) -> f64 {
        match self {
            DataSensitivity::Public => 0.0,
            DataSensitivity::Internal => 0.05,
            DataSensitivity::Confidential => 0.15,
            DataSensitivity::Restricted => 0.30,
        }
    }
}

impl Default for DataSensitivity {
    fn default() -> Self {
        DataSensitivity::Internal
    }
}

/// Individual risk factor with explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Factor name (e.g., "task_complexity", "data_sensitivity")
    pub name: String,
    /// Risk multiplier (e.g., 0.25 for 25%)
    pub multiplier: f64,
    /// Human-readable explanation
    pub description: String,
}

impl RiskFactor {
    pub fn new(name: impl Into<String>, multiplier: f64, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            multiplier,
            description: description.into(),
        }
    }
}

/// Request for price calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingRequest {
    /// Actor's DID (for trust lookup)
    pub actor_did: String,

    /// Action type identifier
    pub action_type: String,

    /// Estimated compute cost (PFLOP-hours)
    pub compute_hc: Decimal,

    /// Actor's current trust score
    pub trust_score: u16,

    /// Task complexity classification
    pub task_complexity: TaskComplexity,

    /// Data sensitivity classification
    pub data_sensitivity: DataSensitivity,

    /// Optional custom risk factors from rules engine
    pub custom_factors: Vec<RiskFactor>,

    /// Request timestamp
    pub timestamp: i64,
}

impl PricingRequest {
    /// Create a new pricing request
    pub fn new(
        actor_did: impl Into<String>,
        action_type: impl Into<String>,
        compute_hc: Decimal,
        trust_score: u16,
    ) -> Self {
        Self {
            actor_did: actor_did.into(),
            action_type: action_type.into(),
            compute_hc,
            trust_score,
            task_complexity: TaskComplexity::default(),
            data_sensitivity: DataSensitivity::default(),
            custom_factors: Vec::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Set task complexity
    pub fn with_complexity(mut self, complexity: TaskComplexity) -> Self {
        self.task_complexity = complexity;
        self
    }

    /// Set data sensitivity
    pub fn with_sensitivity(mut self, sensitivity: DataSensitivity) -> Self {
        self.data_sensitivity = sensitivity;
        self
    }

    /// Add custom risk factor
    pub fn with_risk_factor(mut self, factor: RiskFactor) -> Self {
        self.custom_factors.push(factor);
        self
    }
}

/// Detailed pricing breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingBreakdown {
    /// Base rate per PFLOP-hour
    pub compute_rate: Decimal,

    /// All applied risk factors
    pub risk_factors: Vec<RiskFactor>,

    /// Total risk multiplier (sum of factors)
    pub total_risk_multiplier: f64,

    /// Trust-based discount rate (0.0 - 0.20)
    pub discount_rate: f64,

    /// Calculation timestamp
    pub calculated_at: i64,

    /// Rules engine decision ID (for audit)
    pub decision_id: Option<String>,
}

/// Price calculation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingResponse {
    /// C: Base compute cost
    pub base_cost: Decimal,

    /// R: Risk premium
    pub risk_premium: Decimal,

    /// T: Trust discount
    pub trust_discount: Decimal,

    /// P: Final price (C + R - T)
    pub final_price: Decimal,

    /// Detailed breakdown
    pub breakdown: PricingBreakdown,

    /// Whether price is within budget (if budget was specified)
    pub within_budget: Option<bool>,

    /// Price validity period (milliseconds)
    pub valid_for_ms: u64,

    /// Expiration timestamp
    pub expires_at: i64,
}

impl PricingResponse {
    /// Default price validity period (5 minutes)
    pub const DEFAULT_VALIDITY_MS: u64 = 5 * 60 * 1000;

    /// Check if the price quote is still valid
    pub fn is_valid(&self) -> bool {
        chrono::Utc::now().timestamp_millis() < self.expires_at
    }

    /// Calculate the effective rate (price per PFLOP-hour)
    pub fn effective_rate(&self, compute_hc: Decimal) -> Option<Decimal> {
        if compute_hc > Decimal::ZERO {
            Some(self.final_price / compute_hc)
        } else {
            None
        }
    }

    /// Get savings from trust discount as percentage
    pub fn savings_percentage(&self) -> f64 {
        if self.base_cost > Decimal::ZERO {
            let savings = self.trust_discount / self.base_cost;
            savings.try_into().unwrap_or(0.0) * 100.0
        } else {
            0.0
        }
    }
}

/// Simple pricing calculator (without rules engine)
///
/// For production use, OneBill service with Zen-Engine should be used
pub struct SimplePricingCalculator {
    /// Base rate per PFLOP-hour
    pub rate_per_hc: Decimal,
}

impl SimplePricingCalculator {
    pub fn new(rate_per_hc: Decimal) -> Self {
        Self { rate_per_hc }
    }

    /// Calculate price using the formula: P = C + R - T
    pub fn calculate(&self, request: &PricingRequest) -> PricingResponse {
        let now = chrono::Utc::now().timestamp_millis();

        // C: Base compute cost
        let base_cost = request.compute_hc * self.rate_per_hc;

        // R: Risk premium
        let mut risk_factors = vec![
            RiskFactor::new(
                "task_complexity",
                request.task_complexity.risk_multiplier(),
                format!("{:?} task complexity", request.task_complexity),
            ),
            RiskFactor::new(
                "data_sensitivity",
                request.data_sensitivity.risk_multiplier(),
                format!("{:?} data sensitivity", request.data_sensitivity),
            ),
        ];
        risk_factors.extend(request.custom_factors.clone());

        let total_risk_multiplier: f64 = risk_factors.iter().map(|f| f.multiplier).sum();
        let risk_premium = base_cost * Decimal::try_from(total_risk_multiplier).unwrap_or_default();

        // T: Trust discount (max 20%)
        let trust_tau = request.trust_score as f64 / 1000.0;
        let discount_rate = (trust_tau * 0.20).min(0.20);
        let trust_discount = base_cost * Decimal::try_from(discount_rate).unwrap_or_default();

        // P: Final price
        let final_price = base_cost + risk_premium - trust_discount;

        PricingResponse {
            base_cost,
            risk_premium,
            trust_discount,
            final_price,
            breakdown: PricingBreakdown {
                compute_rate: self.rate_per_hc,
                risk_factors,
                total_risk_multiplier,
                discount_rate,
                calculated_at: now,
                decision_id: None,
            },
            within_budget: None,
            valid_for_ms: PricingResponse::DEFAULT_VALIDITY_MS,
            expires_at: now + PricingResponse::DEFAULT_VALIDITY_MS as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_task_complexity_multipliers() {
        assert_eq!(TaskComplexity::Low.risk_multiplier(), 0.0);
        assert_eq!(TaskComplexity::Medium.risk_multiplier(), 0.10);
        assert_eq!(TaskComplexity::High.risk_multiplier(), 0.25);
        assert_eq!(TaskComplexity::Critical.risk_multiplier(), 0.50);
    }

    #[test]
    fn test_data_sensitivity_multipliers() {
        assert_eq!(DataSensitivity::Public.risk_multiplier(), 0.0);
        assert_eq!(DataSensitivity::Internal.risk_multiplier(), 0.05);
        assert_eq!(DataSensitivity::Confidential.risk_multiplier(), 0.15);
        assert_eq!(DataSensitivity::Restricted.risk_multiplier(), 0.30);
    }

    #[test]
    fn test_simple_pricing() {
        let calculator = SimplePricingCalculator::new(dec!(1.00));

        let request = PricingRequest::new("did:key:test", "test.action", dec!(100), 1000)
            .with_complexity(TaskComplexity::Low)
            .with_sensitivity(DataSensitivity::Public);

        let response = calculator.calculate(&request);

        // Base cost = 100 * 1.00 = 100
        assert_eq!(response.base_cost, dec!(100));
        // Risk = 0 (low complexity, public data)
        assert_eq!(response.risk_premium, dec!(0));
        // Trust discount = 100 * 0.20 = 20 (max score)
        assert_eq!(response.trust_discount, dec!(20));
        // Final = 100 + 0 - 20 = 80
        assert_eq!(response.final_price, dec!(80));
    }

    #[test]
    fn test_pricing_with_risk() {
        let calculator = SimplePricingCalculator::new(dec!(1.00));

        let request = PricingRequest::new("did:key:test", "test.action", dec!(100), 500) // 50% trust
            .with_complexity(TaskComplexity::High) // +25%
            .with_sensitivity(DataSensitivity::Confidential); // +15%

        let response = calculator.calculate(&request);

        // Base cost = 100
        assert_eq!(response.base_cost, dec!(100));
        // Risk = 100 * 0.40 = 40
        assert_eq!(response.risk_premium, dec!(40));
        // Trust discount = 100 * 0.10 = 10 (50% trust = 10% discount)
        assert_eq!(response.trust_discount, dec!(10));
        // Final = 100 + 40 - 10 = 130
        assert_eq!(response.final_price, dec!(130));
    }

    #[test]
    fn test_pricing_validity() {
        let calculator = SimplePricingCalculator::new(dec!(1.00));
        let request = PricingRequest::new("did:key:test", "test.action", dec!(100), 500);
        let response = calculator.calculate(&request);

        assert!(response.is_valid());
        assert_eq!(response.valid_for_ms, PricingResponse::DEFAULT_VALIDITY_MS);
    }
}
