//! Action Augmentation
//!
//! Uses retrieved procedural memories to augment agent action selection.

use serde::{Deserialize, Serialize};

use super::memory::PraxisMemory;
use super::retrieval::RetrievedMemory;

/// Action augmentation context provided to the agent
///
/// This is injected into the agent's context to inform action selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionAugmentation {
    /// Retrieved relevant memories
    pub memories: Vec<AugmentedMemory>,

    /// Suggested action based on memories
    pub suggested_action: Option<SuggestedAction>,

    /// Confidence in the suggestion
    pub confidence: f32,

    /// Warning if conflicting memories exist
    pub warning: Option<String>,
}

/// A memory formatted for augmentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentedMemory {
    /// Relevance rank (1 = most relevant)
    pub rank: usize,

    /// Relevance score
    pub relevance: f32,

    /// The action that was taken
    pub action: String,

    /// Whether it was successful
    pub was_successful: bool,

    /// Brief description of the outcome
    pub outcome_summary: String,

    /// The directive/goal at the time
    pub directive: String,

    /// How many times this memory has been used
    pub usage_count: u64,
}

/// A suggested action based on memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    /// The suggested action string
    pub action: String,

    /// Why this action is suggested
    pub reasoning: String,

    /// Source memories that support this suggestion
    pub supporting_memories: Vec<usize>,

    /// Confidence (0.0-1.0)
    pub confidence: f32,
}

impl ActionAugmentation {
    /// Create augmentation from retrieved memories
    pub fn from_retrieved(retrieved: Vec<RetrievedMemory>) -> Self {
        if retrieved.is_empty() {
            return Self {
                memories: Vec::new(),
                suggested_action: None,
                confidence: 0.0,
                warning: None,
            };
        }

        // Convert to augmented format
        let memories: Vec<AugmentedMemory> = retrieved
            .iter()
            .map(|r| AugmentedMemory {
                rank: r.rank + 1,
                relevance: r.relevance_score,
                action: r.memory.action.raw_action.clone(),
                was_successful: r.memory.is_successful(),
                outcome_summary: Self::summarize_outcome(&r.memory),
                directive: r.memory.internal_state.directive.clone(),
                usage_count: r.memory.retrieval_count,
            })
            .collect();

        // Generate suggestion from successful memories
        let (suggested_action, confidence) = Self::generate_suggestion(&retrieved);

        // Check for conflicts
        let warning = Self::check_conflicts(&retrieved);

        Self {
            memories,
            suggested_action,
            confidence,
            warning,
        }
    }

    /// Summarize an outcome for display
    fn summarize_outcome(memory: &PraxisMemory) -> String {
        match &memory.outcome {
            super::memory::ActionOutcome::Success { description } => {
                format!("Success: {}", description)
            }
            super::memory::ActionOutcome::Failure { error_code, description, .. } => {
                format!("Failed ({}): {}", error_code, description)
            }
            super::memory::ActionOutcome::Partial { completion_pct, description } => {
                format!("Partial ({:.0}%): {}", completion_pct * 100.0, description)
            }
        }
    }

    /// Generate a suggestion based on retrieved memories
    fn generate_suggestion(retrieved: &[RetrievedMemory]) -> (Option<SuggestedAction>, f32) {
        // Find successful memories
        let successful: Vec<_> = retrieved
            .iter()
            .filter(|r| r.is_successful())
            .collect();

        if successful.is_empty() {
            return (None, 0.0);
        }

        // Use the most relevant successful memory
        let best = &successful[0];

        // Calculate confidence based on:
        // - Relevance score
        // - Number of supporting memories
        // - Reinforcement score
        let base_confidence = best.relevance_score;
        let support_bonus = ((successful.len() as f32 - 1.0) * 0.1).min(0.3);
        let reinforcement_bonus = (best.memory.reinforcement_score - 1.0).min(0.2);

        let confidence = (base_confidence + support_bonus + reinforcement_bonus).clamp(0.0, 0.95);

        let supporting_memories: Vec<usize> = successful
            .iter()
            .take(3)
            .map(|r| r.rank)
            .collect();

        let reasoning = format!(
            "Based on {} similar successful experience(s). Most relevant: {}",
            successful.len(),
            best.memory.internal_state.directive
        );

        (
            Some(SuggestedAction {
                action: best.memory.action.raw_action.clone(),
                reasoning,
                supporting_memories,
                confidence,
            }),
            confidence,
        )
    }

    /// Check for conflicting memories
    fn check_conflicts(retrieved: &[RetrievedMemory]) -> Option<String> {
        if retrieved.len() < 2 {
            return None;
        }

        // Check if top memories have conflicting outcomes for similar actions
        let top_successful = retrieved.iter().take(3).filter(|r| r.is_successful()).count();
        let top_failed = retrieved.iter().take(3).filter(|r| !r.is_successful()).count();

        if top_successful > 0 && top_failed > 0 {
            Some(format!(
                "Conflicting experiences: {} successful, {} failed. Review carefully.",
                top_successful,
                top_failed
            ))
        } else {
            None
        }
    }

    /// Format as context string for injection into agent prompt
    pub fn as_context_string(&self) -> String {
        if self.memories.is_empty() {
            return String::from("No relevant procedural memories found.");
        }

        let mut ctx = String::from("## Relevant Procedural Memories\n\n");

        for mem in &self.memories {
            ctx.push_str(&format!(
                "### Memory #{} (relevance: {:.2})\n",
                mem.rank,
                mem.relevance
            ));
            ctx.push_str(&format!("- **Directive**: {}\n", mem.directive));
            ctx.push_str(&format!("- **Action**: `{}`\n", mem.action));
            ctx.push_str(&format!(
                "- **Result**: {} {}\n",
                if mem.was_successful { "[SUCCESS]" } else { "[FAILED]" },
                mem.outcome_summary
            ));
            ctx.push_str(&format!("- **Used {} times**\n\n", mem.usage_count));
        }

        if let Some(suggestion) = &self.suggested_action {
            ctx.push_str("## Suggested Action\n\n");
            ctx.push_str(&format!(
                "Based on past experience, consider: `{}`\n",
                suggestion.action
            ));
            ctx.push_str(&format!("Confidence: {:.0}%\n", suggestion.confidence * 100.0));
            ctx.push_str(&format!("Reasoning: {}\n", suggestion.reasoning));
        }

        if let Some(warning) = &self.warning {
            ctx.push_str(&format!("\n**Warning**: {}\n", warning));
        }

        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::memory::*;
    use uuid::Uuid;

    fn create_retrieved_memory(successful: bool, relevance: f32, rank: usize) -> RetrievedMemory {
        let memory = PraxisMemory::new(
            Uuid::new_v4(),
            EnvironmentState::new("test env"),
            InternalState::new("test directive"),
            AgentAction::click("button#submit"),
            EnvironmentState::new("result env"),
            if successful {
                ActionOutcome::success("Clicked successfully")
            } else {
                ActionOutcome::failure("CLICK_FAILED", "Element not found")
            },
        );

        RetrievedMemory {
            memory,
            env_similarity: relevance,
            internal_similarity: relevance,
            relevance_score: relevance,
            rank,
        }
    }

    #[test]
    fn test_augmentation_from_retrieved() {
        let retrieved = vec![
            create_retrieved_memory(true, 0.9, 0),
            create_retrieved_memory(true, 0.7, 1),
            create_retrieved_memory(false, 0.6, 2),
        ];

        let augmentation = ActionAugmentation::from_retrieved(retrieved);

        assert_eq!(augmentation.memories.len(), 3);
        assert!(augmentation.suggested_action.is_some());
        assert!(augmentation.confidence > 0.5);
        assert!(augmentation.warning.is_some()); // Conflict exists
    }

    #[test]
    fn test_augmentation_context_string() {
        let retrieved = vec![
            create_retrieved_memory(true, 0.9, 0),
        ];

        let augmentation = ActionAugmentation::from_retrieved(retrieved);
        let ctx = augmentation.as_context_string();

        assert!(ctx.contains("Memory #1"));
        assert!(ctx.contains("SUCCESS"));
        assert!(ctx.contains("Suggested Action"));
    }

    #[test]
    fn test_empty_augmentation() {
        let augmentation = ActionAugmentation::from_retrieved(vec![]);

        assert!(augmentation.memories.is_empty());
        assert!(augmentation.suggested_action.is_none());
        assert!((augmentation.confidence - 0.0).abs() < 0.01);
    }
}
