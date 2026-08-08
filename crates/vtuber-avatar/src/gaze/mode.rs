//! Gaze mode selection from avatar capabilities.
//!
//! Determines the appropriate gaze control method based on what the model
//! supports: expression-based look directions, eye bone rotation, or disabled.
//!
//! # Priority
//!
//! 1. Expression look directions (if any direction preset exists)
//! 2. Eye bone rotation (if both eye bones are present)
//! 3. Disabled (no gaze control)
//!
//! The mode is determined once at binding time and cached.

use crate::capabilities::{AvatarCapabilities, GazeMode, LookDirectionSet};

/// Result of gaze mode selection with diagnostic information.
#[derive(Clone, Debug, PartialEq)]
pub struct GazeModeSelection {
    /// The selected gaze mode.
    pub mode: GazeMode,
    /// Whether the model has lookAt.type set (for diagnostics only).
    /// This does NOT affect mode selection — we never insert bevy_vrm1::LookAt.
    pub model_has_look_at_type: bool,
    /// Available look direction presets.
    pub available_directions: LookDirectionSet,
    /// Whether eye bones are present.
    pub has_eye_bones: bool,
}

/// Select the appropriate gaze mode based on avatar capabilities.
///
/// # Priority
///
/// 1. If any look direction expression exists → `GazeMode::Expression` or
///    `GazeMode::ExpressionAndEyeBones`
/// 2. If eye bones exist but no expressions → `GazeMode::EyeBones`
/// 3. Otherwise → `GazeMode::None`
///
/// The `model_has_look_at_type` flag is preserved for diagnostics but does
/// NOT influence the selection — we never insert `bevy_vrm1::LookAt`.
#[must_use]
pub fn select_gaze_mode(
    capabilities: &AvatarCapabilities,
    model_has_look_at_type: bool,
) -> GazeModeSelection {
    let available_directions = capabilities.look_directions;
    let has_eye_bones = capabilities.bones.left_eye && capabilities.bones.right_eye;
    let has_any_direction = available_directions.any();

    let mode = if has_any_direction && has_eye_bones {
        GazeMode::ExpressionAndEyeBones
    } else if has_any_direction {
        GazeMode::Expression
    } else if has_eye_bones {
        GazeMode::EyeBones
    } else {
        GazeMode::None
    };

    GazeModeSelection {
        mode,
        model_has_look_at_type,
        available_directions,
        has_eye_bones,
    }
}

/// Check if a gaze mode supports expression-based gaze.
#[must_use]
pub fn supports_expression_gaze(mode: GazeMode) -> bool {
    matches!(mode, GazeMode::Expression | GazeMode::ExpressionAndEyeBones)
}

/// Check if a gaze mode supports eye bone gaze.
#[must_use]
pub fn supports_eye_bone_gaze(mode: GazeMode) -> bool {
    matches!(mode, GazeMode::EyeBones | GazeMode::ExpressionAndEyeBones)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::{BlinkMode, BonePresence, MouthMode};

    fn make_caps(
        look_directions: LookDirectionSet,
        has_left_eye: bool,
        has_right_eye: bool,
    ) -> AvatarCapabilities {
        AvatarCapabilities {
            bones: BonePresence {
                head: true,
                neck: false,
                left_eye: has_left_eye,
                right_eye: has_right_eye,
                upper_chest: false,
                chest: false,
                spine: false,
            },
            blink: BlinkMode::None,
            mouth: MouthMode::None,
            look_directions,
            spring_bone: false,
            gaze: GazeMode::None, // Will be overridden by selection
            unknown_expressions: Vec::new(),
        }
    }

    #[test]
    fn gaze_mode_full_expressions_and_bones() {
        let caps = make_caps(
            LookDirectionSet {
                left: true,
                right: true,
                up: true,
                down: true,
            },
            true,
            true,
        );
        let selection = select_gaze_mode(&caps, false);

        assert_eq!(selection.mode, GazeMode::ExpressionAndEyeBones);
        assert!(selection.has_eye_bones);
        assert!(selection.available_directions.any());
    }

    #[test]
    fn gaze_mode_expressions_only() {
        let caps = make_caps(
            LookDirectionSet {
                left: true,
                right: false,
                up: false,
                down: false,
            },
            false,
            false,
        );
        let selection = select_gaze_mode(&caps, false);

        assert_eq!(selection.mode, GazeMode::Expression);
        assert!(!selection.has_eye_bones);
    }

    #[test]
    fn gaze_mode_eye_bones_only() {
        let caps = make_caps(
            LookDirectionSet {
                left: false,
                right: false,
                up: false,
                down: false,
            },
            true,
            true,
        );
        let selection = select_gaze_mode(&caps, false);

        assert_eq!(selection.mode, GazeMode::EyeBones);
        assert!(selection.has_eye_bones);
    }

    #[test]
    fn gaze_mode_none_when_nothing_available() {
        let caps = make_caps(
            LookDirectionSet {
                left: false,
                right: false,
                up: false,
                down: false,
            },
            false,
            false,
        );
        let selection = select_gaze_mode(&caps, false);

        assert_eq!(selection.mode, GazeMode::None);
    }

    #[test]
    fn gaze_mode_partial_directions_still_expression() {
        // Even with only one direction, it's still expression mode.
        let caps = make_caps(
            LookDirectionSet {
                left: true,
                right: false,
                up: false,
                down: false,
            },
            true,
            true,
        );
        let selection = select_gaze_mode(&caps, false);

        assert_eq!(selection.mode, GazeMode::ExpressionAndEyeBones);
    }

    #[test]
    fn gaze_mode_look_at_type_preserved_for_diagnostics() {
        let caps = make_caps(LookDirectionSet::default(), false, false);
        let selection = select_gaze_mode(&caps, true);

        assert!(selection.model_has_look_at_type);
        // But mode is still None because no expressions or bones.
        assert_eq!(selection.mode, GazeMode::None);
    }

    #[test]
    fn gaze_mode_one_eye_missing_disables_bone_gaze() {
        let caps = make_caps(
            LookDirectionSet::default(),
            true,
            false, // right eye missing
        );
        let selection = select_gaze_mode(&caps, false);

        // Without both eyes, bone gaze is disabled.
        assert_eq!(selection.mode, GazeMode::None);
        assert!(!selection.has_eye_bones);
    }

    #[test]
    fn gaze_mode_supports_expression_gaze() {
        assert!(supports_expression_gaze(GazeMode::Expression));
        assert!(supports_expression_gaze(GazeMode::ExpressionAndEyeBones));
        assert!(!supports_expression_gaze(GazeMode::EyeBones));
        assert!(!supports_expression_gaze(GazeMode::None));
    }

    #[test]
    fn gaze_mode_supports_eye_bone_gaze() {
        assert!(!supports_eye_bone_gaze(GazeMode::Expression));
        assert!(supports_eye_bone_gaze(GazeMode::ExpressionAndEyeBones));
        assert!(supports_eye_bone_gaze(GazeMode::EyeBones));
        assert!(!supports_eye_bone_gaze(GazeMode::None));
    }

    #[test]
    fn gaze_mode_expression_look_at_model_no_panic() {
        // A model with lookAt.type = expression but no actual direction presets
        // should not panic — it just gets GazeMode::None.
        let caps = make_caps(LookDirectionSet::default(), false, false);
        let selection = select_gaze_mode(&caps, true);

        assert_eq!(selection.mode, GazeMode::None);
        assert!(selection.model_has_look_at_type);
    }
}
