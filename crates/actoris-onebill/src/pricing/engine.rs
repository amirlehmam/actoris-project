//! Zen-Engine based pricing engine
//!
//! Uses Zen-Engine for declarative business rules to calculate:
//! - Base compute costs
//! - Risk premiums based on task complexity and data sensitivity
//! - Trust-based discounts

use actoris_common::{
    ActorisError, DataSensitivity, PricingBreakdown, PricingRequest, PricingResponse, Result,
    RiskFactor, TaskComplexity,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, instrument};
use zen_engine::{DecisionEngine, DecisionGraphResponse};

/// Zen-Engine based pricing engine with configurable rules
pub struct PricingEngine {
    /// The decision engine instance
    engine: DecisionEngine,
    /// Base rate per PFLOP-hour
    base_rate: Decimal,
    /// Loaded decision graph
    decision_graph: Option<Value>,
}

/// Input for pricing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingInput {
    /// Base compute cost in HC
    pub compute_hc: String,
    /// Trust score (0-1000)
    pub trust_score: u32,
    /// Task complexity level (0-4)
    pub task_complexity: u32,
    /// Data sensitivity level (0-4)
    pub data_sensitivity: u32,
    /// Action type for rule matching
    pub action_type: String,
    /// Actor DID for custom rules
    pub actor_did: String,
}

/// Output from pricing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingOutput {
    /// Final price in HC
    pub final_price: String,
    /// Base compute cost
    pub compute_cost: String,
    /// Risk premium amount
    pub risk_premium: String,
    /// Trust discount amount
    pub trust_discount: String,
    /// Risk premium multiplier applied
    pub risk_multiplier: f64,
    /// Discount rate applied (0.0 - 0.20)
    pub discount_rate: f64,
    /// Rule path taken
    pub rule_path: String,
}

impl PricingEngine {
    /// Create a new pricing engine with default rules
    pub fn new(base_rate: Decimal) -> Self {
        let engine = DecisionEngine::default();
        let decision_graph = Self::default_pricing_rules();

        Self {
            engine,
            base_rate,
            decision_graph: Some(decision_graph),
        }
    }

    /// Load rules from a JSON file
    pub fn with_rules_file(base_rate: Decimal, rules_path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(rules_path)
            .map_err(|e| ActorisError::Config(format!("Failed to read rules file: {}", e)))?;

        let decision_graph: Value = serde_json::from_str(&content)
            .map_err(|e| ActorisError::Config(format!("Failed to parse rules JSON: {}", e)))?;

        let engine = DecisionEngine::default();

        Ok(Self {
            engine,
            base_rate,
            decision_graph: Some(decision_graph),
        })
    }

    /// Calculate price for a request using Zen-Engine rules
    #[instrument(skip(self))]
    pub async fn calculate(&self, request: &PricingRequest) -> Result<PricingResponse> {
        let input = PricingInput {
            compute_hc: request.compute_hc.to_string(),
            trust_score: request.trust_score,
            task_complexity: request.task_complexity.clone() as u32,
            data_sensitivity: request.data_sensitivity.clone() as u32,
            action_type: request.action_type.clone(),
            actor_did: request.actor_did.clone(),
        };

        let output = self.evaluate_rules(&input).await?;
        let now = chrono::Utc::now().timestamp_millis();

        // Parse output values
        let final_price = output
            .final_price
            .parse::<Decimal>()
            .unwrap_or(request.compute_hc);
        let compute_cost = output
            .compute_cost
            .parse::<Decimal>()
            .unwrap_or(request.compute_hc);
        let risk_premium = output
            .risk_premium
            .parse::<Decimal>()
            .unwrap_or(Decimal::ZERO);
        let trust_discount = output
            .trust_discount
            .parse::<Decimal>()
            .unwrap_or(Decimal::ZERO);

        Ok(PricingResponse {
            quote_id: uuid::Uuid::new_v4(),
            final_price,
            breakdown: PricingBreakdown {
                compute_cost,
                risk_premium,
                trust_discount,
                risk_factor: Self::risk_factor_from_level(request.task_complexity.clone()),
            },
            currency: "HC".to_string(),
            valid_for_ms: PricingResponse::DEFAULT_VALIDITY_MS,
            expires_at: now + PricingResponse::DEFAULT_VALIDITY_MS as i64,
            computed_at: now,
        })
    }

    /// Evaluate pricing rules
    async fn evaluate_rules(&self, input: &PricingInput) -> Result<PricingOutput> {
        // If no decision graph loaded, use default calculation
        let Some(ref graph) = self.decision_graph else {
            return Ok(self.default_calculation(input));
        };

        // Create evaluation context
        let context = serde_json::to_value(input)
            .map_err(|e| ActorisError::Serialization(format!("Failed to serialize input: {}", e)))?;

        // Evaluate the decision graph
        let result = self
            .engine
            .evaluate(graph, &context)
            .await
            .map_err(|e| ActorisError::Pricing(format!("Rule evaluation failed: {}", e)))?;

        // Extract result
        match result.result {
            Some(output_value) => {
                let output: PricingOutput = serde_json::from_value(output_value).map_err(|e| {
                    ActorisError::Serialization(format!("Failed to parse rule output: {}", e))
                })?;
                Ok(output)
            }
            None => {
                debug!("No rule matched, using default calculation");
                Ok(self.default_calculation(input))
            }
        }
    }

    /// Default calculation when no rules match
    fn default_calculation(&self, input: &PricingInput) -> PricingOutput {
        let compute_hc: Decimal = input.compute_hc.parse().unwrap_or(Decimal::ZERO);

        // Calculate risk multiplier based on complexity and sensitivity
        let complexity_factor = match input.task_complexity {
            0 => dec!(1.0),
            1 => dec!(1.1),
            2 => dec!(1.25),
            3 => dec!(1.5),
            4 => dec!(2.0),
            _ => dec!(1.0),
        };

        let sensitivity_factor = match input.data_sensitivity {
            0 => dec!(1.0),
            1 => dec!(1.0),
            2 => dec!(1.1),
            3 => dec!(1.25),
            4 => dec!(1.5),
            _ => dec!(1.0),
        };

        let risk_multiplier = complexity_factor * sensitivity_factor;

        // Calculate trust discount (up to 20%)
        let tau = Decimal::from(input.trust_score) / dec!(1000);
        let max_discount = dec!(0.20);
        let discount_rate = tau * max_discount;

        // Apply formula: P = C + R - T
        let compute_cost = compute_hc * self.base_rate;
        let risk_premium = compute_cost * (risk_multiplier - dec!(1.0));
        let subtotal = compute_cost + risk_premium;
        let trust_discount = subtotal * discount_rate;
        let final_price = (subtotal - trust_discount).max(Decimal::ZERO);

        PricingOutput {
            final_price: final_price.to_string(),
            compute_cost: compute_cost.to_string(),
            risk_premium: risk_premium.to_string(),
            trust_discount: trust_discount.to_string(),
            risk_multiplier: risk_multiplier
                .try_into()
                .unwrap_or(1.0),
            discount_rate: discount_rate.try_into().unwrap_or(0.0),
            rule_path: "default".to_string(),
        }
    }

    /// Convert task complexity to risk factor
    fn risk_factor_from_level(complexity: TaskComplexity) -> RiskFactor {
        match complexity {
            TaskComplexity::Low => RiskFactor::Low,
            TaskComplexity::Medium => RiskFactor::Medium,
            TaskComplexity::High => RiskFactor::High,
            TaskComplexity::Critical => RiskFactor::Critical,
            TaskComplexity::Unspecified => RiskFactor::Low,
        }
    }

    /// Default pricing rules as Zen-Engine decision graph
    fn default_pricing_rules() -> Value {
        json!({
            "contentType": "application/vnd.gorules.decision",
            "nodes": [
                {
                    "id": "input",
                    "type": "inputNode",
                    "name": "Input",
                    "position": {"x": 0, "y": 0}
                },
                {
                    "id": "output",
                    "type": "outputNode",
                    "name": "Output",
                    "position": {"x": 600, "y": 0}
                },
                {
                    "id": "base_pricing",
                    "type": "decisionTableNode",
                    "name": "Base Pricing",
                    "position": {"x": 200, "y": 0},
                    "content": {
                        "hitPolicy": "first",
                        "inputs": [
                            {"field": "task_complexity", "name": "Complexity"},
                            {"field": "data_sensitivity", "name": "Sensitivity"}
                        ],
                        "outputs": [
                            {"field": "risk_multiplier", "name": "Risk Multiplier"}
                        ],
                        "rules": [
                            {"_id": "r1", "task_complexity": "4", "data_sensitivity": "-", "risk_multiplier": "2.5"},
                            {"_id": "r2", "task_complexity": "3", "data_sensitivity": "4", "risk_multiplier": "2.0"},
                            {"_id": "r3", "task_complexity": "3", "data_sensitivity": "-", "risk_multiplier": "1.5"},
                            {"_id": "r4", "task_complexity": "2", "data_sensitivity": ">= 3", "risk_multiplier": "1.4"},
                            {"_id": "r5", "task_complexity": "2", "data_sensitivity": "-", "risk_multiplier": "1.25"},
                            {"_id": "r6", "task_complexity": "1", "data_sensitivity": "-", "risk_multiplier": "1.1"},
                            {"_id": "r7", "task_complexity": "-", "data_sensitivity": "-", "risk_multiplier": "1.0"}
                        ]
                    }
                },
                {
                    "id": "trust_discount",
                    "type": "decisionTableNode",
                    "name": "Trust Discount",
                    "position": {"x": 400, "y": 0},
                    "content": {
                        "hitPolicy": "first",
                        "inputs": [
                            {"field": "trust_score", "name": "Trust Score"}
                        ],
                        "outputs": [
                            {"field": "discount_rate", "name": "Discount Rate"}
                        ],
                        "rules": [
                            {"_id": "t1", "trust_score": ">= 900", "discount_rate": "0.20"},
                            {"_id": "t2", "trust_score": ">= 800", "discount_rate": "0.16"},
                            {"_id": "t3", "trust_score": ">= 700", "discount_rate": "0.14"},
                            {"_id": "t4", "trust_score": ">= 600", "discount_rate": "0.12"},
                            {"_id": "t5", "trust_score": ">= 500", "discount_rate": "0.10"},
                            {"_id": "t6", "trust_score": ">= 400", "discount_rate": "0.08"},
                            {"_id": "t7", "trust_score": ">= 300", "discount_rate": "0.06"},
                            {"_id": "t8", "trust_score": ">= 200", "discount_rate": "0.04"},
                            {"_id": "t9", "trust_score": ">= 100", "discount_rate": "0.02"},
                            {"_id": "t10", "trust_score": "-", "discount_rate": "0.00"}
                        ]
                    }
                },
                {
                    "id": "calculator",
                    "type": "expressionNode",
                    "name": "Calculate Final Price",
                    "position": {"x": 500, "y": 100},
                    "content": {
                        "expressions": [
                            {
                                "key": "compute_cost",
                                "value": "number(input.compute_hc)"
                            },
                            {
                                "key": "risk_premium",
                                "value": "compute_cost * (base_pricing.risk_multiplier - 1)"
                            },
                            {
                                "key": "subtotal",
                                "value": "compute_cost + risk_premium"
                            },
                            {
                                "key": "trust_discount",
                                "value": "subtotal * trust_discount.discount_rate"
                            },
                            {
                                "key": "final_price",
                                "value": "max(subtotal - trust_discount, 0)"
                            },
                            {
                                "key": "rule_path",
                                "value": "'zen-engine'"
                            }
                        ]
                    }
                }
            ],
            "edges": [
                {"id": "e1", "sourceId": "input", "targetId": "base_pricing"},
                {"id": "e2", "sourceId": "input", "targetId": "trust_discount"},
                {"id": "e3", "sourceId": "base_pricing", "targetId": "calculator"},
                {"id": "e4", "sourceId": "trust_discount", "targetId": "calculator"},
                {"id": "e5", "sourceId": "calculator", "targetId": "output"}
            ]
        })
    }
}

impl Default for PricingEngine {
    fn default() -> Self {
        Self::new(Decimal::ONE)
    }
}

/// Custom pricing rules builder
pub struct PricingRulesBuilder {
    complexity_multipliers: Vec<(u32, Decimal)>,
    sensitivity_multipliers: Vec<(u32, Decimal)>,
    trust_discounts: Vec<(u32, Decimal)>,
    action_type_overrides: Vec<(String, Decimal)>,
}

impl PricingRulesBuilder {
    pub fn new() -> Self {
        Self {
            complexity_multipliers: vec![
                (4, dec!(2.0)),
                (3, dec!(1.5)),
                (2, dec!(1.25)),
                (1, dec!(1.1)),
                (0, dec!(1.0)),
            ],
            sensitivity_multipliers: vec![
                (4, dec!(1.5)),
                (3, dec!(1.25)),
                (2, dec!(1.1)),
                (1, dec!(1.0)),
                (0, dec!(1.0)),
            ],
            trust_discounts: vec![
                (900, dec!(0.20)),
                (800, dec!(0.16)),
                (700, dec!(0.14)),
                (600, dec!(0.12)),
                (500, dec!(0.10)),
            ],
            action_type_overrides: vec![],
        }
    }

    /// Set complexity multiplier for a level
    pub fn with_complexity_multiplier(mut self, level: u32, multiplier: Decimal) -> Self {
        self.complexity_multipliers
            .retain(|(l, _)| *l != level);
        self.complexity_multipliers.push((level, multiplier));
        self.complexity_multipliers.sort_by(|a, b| b.0.cmp(&a.0));
        self
    }

    /// Set trust discount threshold
    pub fn with_trust_discount(mut self, min_score: u32, discount_rate: Decimal) -> Self {
        self.trust_discounts.push((min_score, discount_rate));
        self.trust_discounts.sort_by(|a, b| b.0.cmp(&a.0));
        self
    }

    /// Add action type override multiplier
    pub fn with_action_override(mut self, action_prefix: &str, multiplier: Decimal) -> Self {
        self.action_type_overrides
            .push((action_prefix.to_string(), multiplier));
        self
    }

    /// Build the rules as JSON for Zen-Engine
    pub fn build_rules(&self) -> Value {
        // Build complexity rules
        let complexity_rules: Vec<Value> = self
            .complexity_multipliers
            .iter()
            .enumerate()
            .map(|(i, (level, mult))| {
                json!({
                    "_id": format!("c{}", i),
                    "task_complexity": level.to_string(),
                    "multiplier": mult.to_string()
                })
            })
            .collect();

        // Build trust rules
        let trust_rules: Vec<Value> = self
            .trust_discounts
            .iter()
            .enumerate()
            .map(|(i, (score, rate))| {
                json!({
                    "_id": format!("t{}", i),
                    "trust_score": format!(">= {}", score),
                    "discount_rate": rate.to_string()
                })
            })
            .collect();

        json!({
            "complexity_rules": complexity_rules,
            "trust_rules": trust_rules,
            "action_overrides": self.action_type_overrides
        })
    }
}

impl Default for PricingRulesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_default_pricing() {
        let engine = PricingEngine::new(dec!(1.0));

        let request = PricingRequest::new("did:key:test", "test.action", dec!(100), 500)
            .with_task_complexity(TaskComplexity::Medium)
            .with_data_sensitivity(DataSensitivity::Internal);

        let response = engine.calculate(&request).await.unwrap();

        // Should have some risk premium for medium complexity
        assert!(response.breakdown.risk_premium > Decimal::ZERO);
        // Should have some discount for 500 trust score
        assert!(response.breakdown.trust_discount > Decimal::ZERO);
        // Final price should be positive
        assert!(response.final_price > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_high_trust_discount() {
        let engine = PricingEngine::new(dec!(1.0));

        let request = PricingRequest::new("did:key:test", "test.action", dec!(100), 950)
            .with_task_complexity(TaskComplexity::Low)
            .with_data_sensitivity(DataSensitivity::Public);

        let response = engine.calculate(&request).await.unwrap();

        // High trust should get close to max discount (20%)
        let discount_rate = response.breakdown.trust_discount
            / (response.breakdown.compute_cost + response.breakdown.risk_premium);
        assert!(discount_rate >= dec!(0.15));
    }

    #[tokio::test]
    async fn test_critical_complexity_risk() {
        let engine = PricingEngine::new(dec!(1.0));

        let request = PricingRequest::new("did:key:test", "critical.action", dec!(100), 500)
            .with_task_complexity(TaskComplexity::Critical)
            .with_data_sensitivity(DataSensitivity::Restricted);

        let response = engine.calculate(&request).await.unwrap();

        // Critical complexity with restricted data should have high risk premium
        assert!(response.breakdown.risk_premium > response.breakdown.compute_cost * dec!(0.5));
    }

    #[test]
    fn test_rules_builder() {
        let rules = PricingRulesBuilder::new()
            .with_complexity_multiplier(4, dec!(3.0))
            .with_trust_discount(950, dec!(0.25))
            .with_action_override("critical.", dec!(2.0))
            .build_rules();

        assert!(rules.get("complexity_rules").is_some());
        assert!(rules.get("trust_rules").is_some());
    }
}
