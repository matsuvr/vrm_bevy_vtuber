//! Expression event coalescing and change epsilon.
//!
//! Ensures at most one `ModifyExpressions` event per avatar per frame,
//! skips sending when the change is below epsilon, and resets state
//! on avatar generation change.

use std::collections::HashMap;

use crate::expression::command::ExpressionCommand;

/// Default epsilon for change detection.
pub const DEFAULT_CHANGE_EPSILON: f32 = 0.01;

/// Tracks the previous expression state for coalescing.
#[derive(Clone, Debug, Default)]
pub struct ExpressionStateTracker {
    /// Previous frame's expression weights.
    previous: HashMap<String, f32>,
    /// Current avatar generation.
    generation: u64,
    /// Change epsilon threshold.
    epsilon: f32,
}

impl ExpressionStateTracker {
    /// Create a new tracker with default epsilon.
    #[must_use]
    pub fn new() -> Self {
        Self {
            previous: HashMap::new(),
            generation: 0,
            epsilon: DEFAULT_CHANGE_EPSILON,
        }
    }

    /// Create a new tracker with custom epsilon.
    #[must_use]
    pub fn with_epsilon(epsilon: f32) -> Self {
        Self {
            previous: HashMap::new(),
            generation: 0,
            epsilon,
        }
    }

    /// Reset the tracker for a new avatar generation.
    pub fn reset_for_generation(&mut self, generation: u64) {
        self.previous.clear();
        self.generation = generation;
    }

    /// Get the current generation.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Compute the commands to send, applying coalescing and epsilon.
    ///
    /// Returns `None` if no changes exceed epsilon (skip sending).
    /// Returns `Some(commands)` with the full set of commands to send,
    /// including explicit zeros for expressions that were previously
    /// non-zero but are now zero.
    #[must_use]
    pub fn compute_commands(
        &mut self,
        new_commands: &[ExpressionCommand],
        current_generation: u64,
    ) -> Option<Vec<ExpressionCommand>> {
        // Reset if generation changed.
        if current_generation != self.generation {
            self.reset_for_generation(current_generation);
        }

        // Build the new weight map.
        let mut new_weights: HashMap<String, f32> = HashMap::new();
        for cmd in new_commands {
            new_weights.insert(cmd.name.clone(), cmd.weight);
        }

        // Check if any change exceeds epsilon.
        let mut has_significant_change = false;

        // Check new/changed values.
        for (name, &weight) in &new_weights {
            let prev = self.previous.get(name).copied().unwrap_or(0.0);
            if (weight - prev).abs() > self.epsilon {
                has_significant_change = true;
                break;
            }
        }

        // Check for values that disappeared (need explicit zero).
        if !has_significant_change {
            for (name, &prev_weight) in &self.previous {
                if prev_weight.abs() > self.epsilon && !new_weights.contains_key(name) {
                    has_significant_change = true;
                    break;
                }
            }
        }

        if !has_significant_change {
            return None;
        }

        // Build the output commands, including explicit zeros for disappeared expressions.
        let mut output: Vec<ExpressionCommand> = new_commands.to_vec();

        // Add explicit zeros for previously non-zero expressions that are now gone.
        for (name, &prev_weight) in &self.previous {
            if prev_weight.abs() > self.epsilon && !new_weights.contains_key(name) {
                output.push(ExpressionCommand {
                    name: name.clone(),
                    weight: 0.0,
                });
            }
        }

        // Update previous state.
        self.previous = new_weights;

        Some(output)
    }

    /// Force a reset of the previous state (e.g., on avatar unload).
    pub fn force_reset(&mut self) {
        self.previous.clear();
    }
}

/// Coalesce multiple expression command lists into a single list.
///
/// Later entries override earlier ones for the same expression name.
#[must_use]
pub fn coalesce_commands(command_lists: &[Vec<ExpressionCommand>]) -> Vec<ExpressionCommand> {
    let mut merged: HashMap<String, f32> = HashMap::new();

    for list in command_lists {
        for cmd in list {
            merged.insert(cmd.name.clone(), cmd.weight);
        }
    }

    merged
        .into_iter()
        .map(|(name, weight)| ExpressionCommand { name, weight })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(name: &str, weight: f32) -> ExpressionCommand {
        ExpressionCommand {
            name: name.to_string(),
            weight,
        }
    }

    #[test]
    fn expression_coalescing_first_frame_always_sends() {
        let mut tracker = ExpressionStateTracker::new();
        let commands = vec![cmd("blink", 0.5)];

        let result = tracker.compute_commands(&commands, 1);
        assert!(result.is_some(), "first frame should always send");
    }

    #[test]
    fn expression_coalescing_steady_state_skips() {
        let mut tracker = ExpressionStateTracker::new();
        let commands = vec![cmd("blink", 0.5)];

        // First frame sends.
        let _ = tracker.compute_commands(&commands, 1);

        // Same commands next frame should skip.
        let result = tracker.compute_commands(&commands, 1);
        assert!(result.is_none(), "unchanged commands should be skipped");
    }

    #[test]
    fn expression_coalescing_small_change_skips() {
        let mut tracker = ExpressionStateTracker::new();
        let commands1 = vec![cmd("blink", 0.5)];
        let commands2 = vec![cmd("blink", 0.505)]; // within epsilon

        let _ = tracker.compute_commands(&commands1, 1);
        let result = tracker.compute_commands(&commands2, 1);
        assert!(result.is_none(), "change within epsilon should be skipped");
    }

    #[test]
    fn expression_coalescing_large_change_sends() {
        let mut tracker = ExpressionStateTracker::new();
        let commands1 = vec![cmd("blink", 0.5)];
        let commands2 = vec![cmd("blink", 0.7)]; // exceeds epsilon

        let _ = tracker.compute_commands(&commands1, 1);
        let result = tracker.compute_commands(&commands2, 1);
        assert!(result.is_some(), "change exceeding epsilon should send");
    }

    #[test]
    fn expression_coalescing_disappeared_expression_gets_zero() {
        let mut tracker = ExpressionStateTracker::new();
        let commands1 = vec![cmd("blink", 0.5), cmd("aa", 0.3)];
        let commands2 = vec![cmd("blink", 0.5)]; // aa disappeared

        let _ = tracker.compute_commands(&commands1, 1);
        let result = tracker.compute_commands(&commands2, 1).unwrap();

        // Should include explicit zero for aa.
        assert!(result.iter().any(|c| c.name == "aa" && c.weight == 0.0));
    }

    #[test]
    fn expression_coalescing_generation_change_resets() {
        let mut tracker = ExpressionStateTracker::new();
        let commands = vec![cmd("blink", 0.5)];

        let _ = tracker.compute_commands(&commands, 1);

        // Generation change should reset and send again.
        let result = tracker.compute_commands(&commands, 2);
        assert!(result.is_some(), "generation change should force send");
        assert_eq!(tracker.generation(), 2);
    }

    #[test]
    fn expression_coalescing_empty_commands_steady() {
        let mut tracker = ExpressionStateTracker::new();
        let empty: Vec<ExpressionCommand> = vec![];

        // First frame with empty commands.
        let result1 = tracker.compute_commands(&empty, 1);
        // Empty first frame should still be None (nothing to send).
        assert!(result1.is_none());

        // Second frame also empty.
        let result2 = tracker.compute_commands(&empty, 1);
        assert!(result2.is_none());
    }

    #[test]
    fn expression_coalescing_merge_lists() {
        let list1 = vec![cmd("blink", 0.5), cmd("aa", 0.3)];
        let list2 = vec![cmd("blink", 0.7), cmd("lookLeft", 0.4)];

        let merged = coalesce_commands(&[list1, list2]);

        // Later list should override.
        assert!(merged.iter().any(|c| c.name == "blink" && c.weight == 0.7));
        assert!(merged.iter().any(|c| c.name == "aa" && c.weight == 0.3));
        assert!(
            merged
                .iter()
                .any(|c| c.name == "lookLeft" && c.weight == 0.4)
        );
    }

    #[test]
    fn expression_coalescing_one_event_per_frame() {
        // The compute_commands function returns a single Vec,
        // ensuring only one event is sent per frame.
        let mut tracker = ExpressionStateTracker::new();
        let commands = vec![cmd("blink", 0.5), cmd("aa", 0.3)];

        let result = tracker.compute_commands(&commands, 1);
        assert!(result.is_some());
        // The result is a single Vec, not multiple.
        let output = result.unwrap();
        assert_eq!(output.len(), 2);
    }
}
