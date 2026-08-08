//! Gaze expression mapping.
//!
//! Converts gaze yaw/pitch into look direction expression weights
//! (lookLeft, lookRight, lookUp, lookDown) with dead zone, per-axis max,
//! and normalization.

use crate::capabilities::LookDirectionSet;

/// Raw gaze input from tracking (yaw/pitch in radians).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawGazeInput {
    /// Horizontal gaze angle in radians. Positive = right.
    pub yaw_rad: f32,
    /// Vertical gaze angle in radians. Positive = up.
    pub pitch_rad: f32,
}

/// Settings for gaze expression mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GazeExpressionSettings {
    /// Dead zone in radians. Gaze within this range produces zero weight.
    pub dead_zone_rad: f32,
    /// Maximum absolute yaw in radians for full weight.
    pub max_yaw_rad: f32,
    /// Maximum absolute pitch in radians for full weight.
    pub max_pitch_rad: f32,
}

impl Default for GazeExpressionSettings {
    fn default() -> Self {
        Self {
            dead_zone_rad: 0.05, // ~3 degrees
            max_yaw_rad: 0.5,    // ~30 degrees
            max_pitch_rad: 0.4,  // ~23 degrees
        }
    }
}

/// Map raw gaze input to look direction expression weights.
///
/// Applies dead zone, per-axis max, and normalization.
/// Only outputs weights for available direction presets.
///
/// Left/right and up/down are mutually exclusive: only one of each pair
/// can be non-zero at a time.
#[must_use]
pub fn map_gaze_to_expressions(
    input: &RawGazeInput,
    available: &LookDirectionSet,
    settings: &GazeExpressionSettings,
) -> Vec<(String, f32)> {
    let mut result = Vec::new();

    // Horizontal: yaw → lookLeft / lookRight
    let yaw_abs = input.yaw_rad.abs();
    if yaw_abs > settings.dead_zone_rad {
        let normalized = ((yaw_abs - settings.dead_zone_rad)
            / (settings.max_yaw_rad - settings.dead_zone_rad))
            .clamp(0.0, 1.0);

        if input.yaw_rad > 0.0 {
            // Positive yaw → look right
            if available.right {
                result.push(("lookRight".to_string(), normalized));
            }
        } else {
            // Negative yaw → look left
            if available.left {
                result.push(("lookLeft".to_string(), normalized));
            }
        }
    }

    // Vertical: pitch → lookUp / lookDown
    let pitch_abs = input.pitch_rad.abs();
    if pitch_abs > settings.dead_zone_rad {
        let normalized = ((pitch_abs - settings.dead_zone_rad)
            / (settings.max_pitch_rad - settings.dead_zone_rad))
            .clamp(0.0, 1.0);

        if input.pitch_rad > 0.0 {
            // Positive pitch → look up
            if available.up {
                result.push(("lookUp".to_string(), normalized));
            }
        } else {
            // Negative pitch → look down
            if available.down {
                result.push(("lookDown".to_string(), normalized));
            }
        }
    }

    result
}

/// Check if gaze is within the dead zone (center gaze).
#[must_use]
pub fn is_gaze_in_dead_zone(input: &RawGazeInput, dead_zone_rad: f32) -> bool {
    input.yaw_rad.abs() <= dead_zone_rad && input.pitch_rad.abs() <= dead_zone_rad
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_directions() -> LookDirectionSet {
        LookDirectionSet {
            left: true,
            right: true,
            up: true,
            down: true,
        }
    }

    #[test]
    fn gaze_expression_center_gaze_all_zero() {
        let input = RawGazeInput {
            yaw_rad: 0.0,
            pitch_rad: 0.0,
        };
        let result = map_gaze_to_expressions(
            &input,
            &all_directions(),
            &GazeExpressionSettings::default(),
        );

        assert!(
            result.is_empty(),
            "center gaze should produce no expressions"
        );
    }

    #[test]
    fn gaze_expression_positive_yaw_looks_right() {
        let input = RawGazeInput {
            yaw_rad: 0.3,
            pitch_rad: 0.0,
        };
        let result = map_gaze_to_expressions(
            &input,
            &all_directions(),
            &GazeExpressionSettings::default(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "lookRight");
        assert!(result[0].1 > 0.0 && result[0].1 <= 1.0);
    }

    #[test]
    fn gaze_expression_negative_yaw_looks_left() {
        let input = RawGazeInput {
            yaw_rad: -0.3,
            pitch_rad: 0.0,
        };
        let result = map_gaze_to_expressions(
            &input,
            &all_directions(),
            &GazeExpressionSettings::default(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "lookLeft");
    }

    #[test]
    fn gaze_expression_positive_pitch_looks_up() {
        let input = RawGazeInput {
            yaw_rad: 0.0,
            pitch_rad: 0.2,
        };
        let result = map_gaze_to_expressions(
            &input,
            &all_directions(),
            &GazeExpressionSettings::default(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "lookUp");
    }

    #[test]
    fn gaze_expression_negative_pitch_looks_down() {
        let input = RawGazeInput {
            yaw_rad: 0.0,
            pitch_rad: -0.2,
        };
        let result = map_gaze_to_expressions(
            &input,
            &all_directions(),
            &GazeExpressionSettings::default(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "lookDown");
    }

    #[test]
    fn gaze_expression_combined_yaw_and_pitch() {
        let input = RawGazeInput {
            yaw_rad: 0.3,
            pitch_rad: 0.2,
        };
        let result = map_gaze_to_expressions(
            &input,
            &all_directions(),
            &GazeExpressionSettings::default(),
        );

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|(n, _)| n == "lookRight"));
        assert!(result.iter().any(|(n, _)| n == "lookUp"));
    }

    #[test]
    fn gaze_expression_dead_zone_suppresses_small_values() {
        let input = RawGazeInput {
            yaw_rad: 0.03, // within default dead zone of 0.05
            pitch_rad: 0.0,
        };
        let result = map_gaze_to_expressions(
            &input,
            &all_directions(),
            &GazeExpressionSettings::default(),
        );

        assert!(result.is_empty(), "within dead zone should produce nothing");
    }

    #[test]
    fn gaze_expression_max_clamps_weight() {
        let input = RawGazeInput {
            yaw_rad: 10.0, // way beyond max
            pitch_rad: 0.0,
        };
        let result = map_gaze_to_expressions(
            &input,
            &all_directions(),
            &GazeExpressionSettings::default(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 1.0, "weight should be clamped to 1.0");
    }

    #[test]
    fn gaze_expression_partial_directions() {
        let available = LookDirectionSet {
            left: true,
            right: false, // no right
            up: false,
            down: true,
        };
        let input = RawGazeInput {
            yaw_rad: 0.3,    // wants right, but not available
            pitch_rad: -0.2, // wants down, available
        };
        let result =
            map_gaze_to_expressions(&input, &available, &GazeExpressionSettings::default());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "lookDown");
    }

    #[test]
    fn gaze_expression_left_right_mutually_exclusive() {
        // Positive yaw should only produce lookRight, not lookLeft.
        let input = RawGazeInput {
            yaw_rad: 0.3,
            pitch_rad: 0.0,
        };
        let result = map_gaze_to_expressions(
            &input,
            &all_directions(),
            &GazeExpressionSettings::default(),
        );

        assert!(!result.iter().any(|(n, _)| n == "lookLeft"));
        assert!(result.iter().any(|(n, _)| n == "lookRight"));
    }

    #[test]
    fn gaze_expression_is_in_dead_zone() {
        let center = RawGazeInput {
            yaw_rad: 0.0,
            pitch_rad: 0.0,
        };
        assert!(is_gaze_in_dead_zone(&center, 0.05));

        let edge = RawGazeInput {
            yaw_rad: 0.05,
            pitch_rad: 0.0,
        };
        assert!(is_gaze_in_dead_zone(&edge, 0.05));

        let outside = RawGazeInput {
            yaw_rad: 0.1,
            pitch_rad: 0.0,
        };
        assert!(!is_gaze_in_dead_zone(&outside, 0.05));
    }

    #[test]
    fn gaze_expression_partial_preset_finite_output() {
        // Even with only one direction available, output should be finite.
        let available = LookDirectionSet {
            left: true,
            right: false,
            up: false,
            down: false,
        };
        let input = RawGazeInput {
            yaw_rad: -0.3,
            pitch_rad: 0.0,
        };
        let result =
            map_gaze_to_expressions(&input, &available, &GazeExpressionSettings::default());

        assert_eq!(result.len(), 1);
        assert!(result[0].1.is_finite());
    }
}
