//! Procedural Competence Metrics
//!
//! Metrics for measuring an agent's procedural learning ability,
//! used in Darwinian fitness calculations.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::memory::PraxisMemory;

/// Procedural competence metrics for an agent
///
/// These metrics measure how well an agent learns and applies
/// procedural knowledge, used as a factor in Darwinian fitness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralCompetence {
    /// Total number of memories
    pub total_memories: u64,

    /// Number of successful procedure memories
    pub successful_memories: u64,

    /// Success rate (successful / total)
    pub success_rate: f32,

    /// Diversity score (0.0-1.0) - how varied are the procedures?
    pub diversity_score: f32,

    /// Generalization score (0.0-1.0) - performance on novel states
    pub generalization_score: f32,

    /// Learning velocity - rate of new successful procedures
    pub learning_velocity: f32,

    /// Memory retrieval accuracy - how often retrieved memories help
    pub retrieval_accuracy: f32,

    /// Procedural memory utilization - how often memories are used
    pub memory_utilization: f32,

    /// When these metrics were calculated
    pub calculated_at: DateTime<Utc>,
}

impl Default for ProceduralCompetence {
    fn default() -> Self {
        Self {
            total_memories: 0,
            successful_memories: 0,
            success_rate: 0.0,
            diversity_score: 0.0,
            generalization_score: 0.0,
            learning_velocity: 0.0,
            retrieval_accuracy: 0.0,
            memory_utilization: 0.0,
            calculated_at: Utc::now(),
        }
    }
}

impl ProceduralCompetence {
    /// Calculate a composite competence score for fitness
    ///
    /// Returns a multiplier (0.5-2.0) for the base fitness score
    pub fn fitness_multiplier(&self) -> f32 {
        if self.total_memories == 0 {
            return 1.0; // Neutral for agents with no memories
        }

        // Weighted combination of factors
        let base = 1.0;

        // Success rate contribution (±0.3)
        let success_contribution = (self.success_rate - 0.5) * 0.6;

        // Diversity contribution (±0.2)
        let diversity_contribution = (self.diversity_score - 0.5) * 0.4;

        // Learning velocity contribution (±0.2)
        let velocity_contribution = (self.learning_velocity - 0.5) * 0.4;

        // Generalization contribution (±0.3)
        let generalization_contribution = (self.generalization_score - 0.5) * 0.6;

        let multiplier = base
            + success_contribution
            + diversity_contribution
            + velocity_contribution
            + generalization_contribution;

        multiplier.clamp(0.5, 2.0)
    }

    /// Calculate competence from a set of memories
    pub fn from_memories(memories: &[PraxisMemory]) -> Self {
        if memories.is_empty() {
            return Self::default();
        }

        let total = memories.len() as u64;
        let successful = memories.iter().filter(|m| m.is_successful()).count() as u64;
        let success_rate = successful as f32 / total as f32;

        // Calculate diversity based on action types and directives
        let diversity_score = Self::calculate_diversity(memories);

        // Calculate learning velocity (recent success rate vs overall)
        let learning_velocity = Self::calculate_learning_velocity(memories);

        // Calculate retrieval stats
        let (retrieval_accuracy, memory_utilization) = Self::calculate_retrieval_stats(memories);

        // Generalization is harder to calculate without test data
        // Use a proxy based on diversity and success across different contexts
        let generalization_score = (diversity_score + success_rate) / 2.0;

        Self {
            total_memories: total,
            successful_memories: successful,
            success_rate,
            diversity_score,
            generalization_score,
            learning_velocity,
            retrieval_accuracy,
            memory_utilization,
            calculated_at: Utc::now(),
        }
    }

    /// Calculate diversity of procedures
    fn calculate_diversity(memories: &[PraxisMemory]) -> f32 {
        if memories.is_empty() {
            return 0.0;
        }

        // Count unique action types
        let action_types: std::collections::HashSet<_> = memories
            .iter()
            .map(|m| &m.action.action_type)
            .collect();

        // Count unique directives (simplified)
        let directives: std::collections::HashSet<_> = memories
            .iter()
            .map(|m| &m.internal_state.directive)
            .collect();

        // Count unique task tags
        let mut all_tags = std::collections::HashSet::new();
        for m in memories {
            for tag in &m.internal_state.task_tags {
                all_tags.insert(tag);
            }
        }

        // Normalize diversity scores
        let action_diversity = (action_types.len() as f32).ln_1p() / (memories.len() as f32).ln_1p();
        let directive_diversity = (directives.len() as f32).ln_1p() / (memories.len() as f32).ln_1p();
        let tag_diversity = if all_tags.is_empty() {
            0.5
        } else {
            (all_tags.len() as f32).ln_1p() / (memories.len() as f32 * 2.0).ln_1p()
        };

        // Combine
        (action_diversity * 0.4 + directive_diversity * 0.4 + tag_diversity * 0.2).clamp(0.0, 1.0)
    }

    /// Calculate learning velocity (improvement over time)
    fn calculate_learning_velocity(memories: &[PraxisMemory]) -> f32 {
        if memories.len() < 10 {
            return 0.5; // Not enough data
        }

        // Sort by creation time
        let mut sorted: Vec<_> = memories.iter().collect();
        sorted.sort_by_key(|m| m.created_at);

        // Calculate success rate in first half vs second half
        let mid = sorted.len() / 2;
        let first_half = &sorted[..mid];
        let second_half = &sorted[mid..];

        let first_success_rate = first_half.iter().filter(|m| m.is_successful()).count() as f32
            / first_half.len() as f32;
        let second_success_rate = second_half.iter().filter(|m| m.is_successful()).count() as f32
            / second_half.len() as f32;

        // Velocity is the improvement (clamped to 0-1)
        let improvement = second_success_rate - first_success_rate;

        // Map improvement (-1 to 1) to velocity (0 to 1)
        ((improvement + 1.0) / 2.0).clamp(0.0, 1.0)
    }

    /// Calculate retrieval statistics
    fn calculate_retrieval_stats(memories: &[PraxisMemory]) -> (f32, f32) {
        if memories.is_empty() {
            return (0.5, 0.0);
        }

        let total_retrievals: u64 = memories.iter().map(|m| m.retrieval_count).sum();
        let retrieved_count = memories.iter().filter(|m| m.retrieval_count > 0).count();

        // Memory utilization: what fraction of memories have been retrieved?
        let utilization = retrieved_count as f32 / memories.len() as f32;

        // Retrieval accuracy: are successful memories being retrieved more?
        // (This is a proxy - real accuracy would need outcome tracking)
        let successful_retrievals: u64 = memories
            .iter()
            .filter(|m| m.is_successful())
            .map(|m| m.retrieval_count)
            .sum();

        let accuracy = if total_retrievals > 0 {
            successful_retrievals as f32 / total_retrievals as f32
        } else {
            0.5
        };

        (accuracy, utilization)
    }
}

/// Learning metrics over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMetrics {
    /// Agent ID
    pub agent_id: uuid::Uuid,

    /// Competence snapshots over time
    pub history: Vec<CompetenceSnapshot>,

    /// Current competence
    pub current: ProceduralCompetence,

    /// Trend direction (-1, 0, 1)
    pub trend: i8,

    /// Days since last improvement
    pub days_since_improvement: u32,
}

/// A snapshot of competence at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetenceSnapshot {
    /// When this snapshot was taken
    pub timestamp: DateTime<Utc>,
    /// Success rate at this time
    pub success_rate: f32,
    /// Total memories at this time
    pub total_memories: u64,
    /// Fitness multiplier at this time
    pub fitness_multiplier: f32,
}

impl LearningMetrics {
    /// Create new learning metrics for an agent
    pub fn new(agent_id: uuid::Uuid) -> Self {
        Self {
            agent_id,
            history: Vec::new(),
            current: ProceduralCompetence::default(),
            trend: 0,
            days_since_improvement: 0,
        }
    }

    /// Update with new competence measurement
    pub fn update(&mut self, competence: ProceduralCompetence) {
        // Add to history
        self.history.push(CompetenceSnapshot {
            timestamp: competence.calculated_at,
            success_rate: competence.success_rate,
            total_memories: competence.total_memories,
            fitness_multiplier: competence.fitness_multiplier(),
        });

        // Keep only last 100 snapshots
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        // Calculate trend
        self.trend = self.calculate_trend();

        // Update days since improvement
        if self.trend > 0 {
            self.days_since_improvement = 0;
        } else if let Some(last) = self.history.last() {
            let days = (Utc::now() - last.timestamp).num_days();
            self.days_since_improvement = days.max(0) as u32;
        }

        self.current = competence;
    }

    /// Calculate trend from history
    fn calculate_trend(&self) -> i8 {
        if self.history.len() < 3 {
            return 0;
        }

        let recent = &self.history[self.history.len().saturating_sub(5)..];
        if recent.len() < 2 {
            return 0;
        }

        let first_avg = recent[..recent.len()/2]
            .iter()
            .map(|s| s.fitness_multiplier)
            .sum::<f32>() / (recent.len() / 2) as f32;

        let second_avg = recent[recent.len()/2..]
            .iter()
            .map(|s| s.fitness_multiplier)
            .sum::<f32>() / (recent.len() - recent.len() / 2) as f32;

        let diff = second_avg - first_avg;

        if diff > 0.05 {
            1 // Improving
        } else if diff < -0.05 {
            -1 // Declining
        } else {
            0 // Stable
        }
    }

    /// Check if agent is actively learning
    pub fn is_actively_learning(&self) -> bool {
        self.trend >= 0 && self.days_since_improvement < 7
    }

    /// Check if agent should be protected from culling due to learning
    pub fn should_protect_for_learning(&self) -> bool {
        // Protect if:
        // 1. Trend is positive (improving)
        // 2. Recent improvement (within 14 days)
        // 3. High learning velocity
        self.trend > 0
            || self.days_since_improvement < 14
            || self.current.learning_velocity > 0.6
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::memory::*;

    fn create_test_memory(successful: bool, days_ago: i64) -> PraxisMemory {
        let mut memory = PraxisMemory::new(
            uuid::Uuid::new_v4(),
            EnvironmentState::new("test"),
            InternalState::new("test directive"),
            AgentAction::new("test", "test action"),
            EnvironmentState::new("test result"),
            if successful {
                ActionOutcome::success("ok")
            } else {
                ActionOutcome::failure("ERR", "failed")
            },
        );
        memory.created_at = Utc::now() - Duration::days(days_ago);
        memory
    }

    #[test]
    fn test_competence_from_memories() {
        let memories: Vec<_> = (0..20)
            .map(|i| create_test_memory(i % 2 == 0, i))
            .collect();

        let competence = ProceduralCompetence::from_memories(&memories);

        assert_eq!(competence.total_memories, 20);
        assert_eq!(competence.successful_memories, 10);
        assert!((competence.success_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_fitness_multiplier() {
        let mut competence = ProceduralCompetence::default();

        // Default should be neutral
        assert!((competence.fitness_multiplier() - 1.0).abs() < 0.01);

        // High performer
        competence.total_memories = 100;
        competence.success_rate = 0.9;
        competence.diversity_score = 0.8;
        competence.learning_velocity = 0.7;
        competence.generalization_score = 0.8;

        let multiplier = competence.fitness_multiplier();
        assert!(multiplier > 1.3);

        // Low performer
        competence.success_rate = 0.1;
        competence.diversity_score = 0.2;
        competence.learning_velocity = 0.3;
        competence.generalization_score = 0.2;

        let multiplier = competence.fitness_multiplier();
        assert!(multiplier < 0.8);
    }

    #[test]
    fn test_learning_metrics_trend() {
        let agent_id = uuid::Uuid::new_v4();
        let mut metrics = LearningMetrics::new(agent_id);

        // Add improving snapshots
        for i in 0..10 {
            let mut comp = ProceduralCompetence::default();
            comp.success_rate = 0.5 + (i as f32 * 0.05);
            comp.total_memories = (i + 1) * 10;
            metrics.update(comp);
        }

        assert!(metrics.trend >= 0);
        assert!(metrics.is_actively_learning());
    }
}
