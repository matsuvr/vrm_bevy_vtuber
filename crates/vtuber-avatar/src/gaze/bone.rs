//! Eye bone gaze fallback.
//!
//! When the model has eye bones but no (or insufficient) look direction
//! expressions, gaze is applied by rotating the eye bones directly.
//!
//! The rotation is computed from rest orientation + yaw/pitch delta,
//! with per-axis clamping.

use bevy::math::Quat;

use crate::gaze::expression::RawGazeInput;

/// Settings for eye bone gaze rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EyeBoneGazeSettings {
    /// Maximum absolute yaw in radians.
    pub max_yaw_rad: f32,
    /// Maximum absolute pitch in radians.
    pub max_pitch_rad: f32,
}

impl Default for EyeBoneGazeSettings {
    fn default() -> Self {
        Self {
            max_yaw_rad: 0.5,   // ~30 degrees
            max_pitch_rad: 0.4, // ~23 degrees
        }
    }
}

/// Compute the eye bone rotation delta from gaze input.
///
/// Returns a quaternion representing the rotation to apply to the eye bone.
/// The rotation is in model space and must be conjugated to bone-local space
/// by the caller using the rest orientation cache.
///
/// # Clamping
///
/// Yaw and pitch are clamped independently to their maximum values.
#[must_use]
pub fn compute_eye_bone_rotation(input: &RawGazeInput, settings: &EyeBoneGazeSettings) -> Quat {
    let yaw = input
        .yaw_rad
        .clamp(-settings.max_yaw_rad, settings.max_yaw_rad);
    let pitch = input
        .pitch_rad
        .clamp(-settings.max_pitch_rad, settings.max_pitch_rad);

    // In model space: yaw → +Y, pitch → -X (same convention as head pose).
    Quat::from_euler(bevy::math::EulerRot::YXZ, yaw, -pitch, 0.0)
}

/// Compute eye bone local rotation from rest + gaze delta.
///
/// This is a convenience function that combines the rest orientation
/// with the gaze delta using the conjugation formula from ADR-004.
///
/// `R_output = R_rest * R_delta`
///
/// For proper bone-local application, use the full conjugation pipeline
/// from `pose::math` instead.
#[must_use]
pub fn compute_eye_bone_local_rotation(
    rest_local: Quat,
    gaze_delta_model: Quat,
    rest_global: Quat,
    root_rest_global: Quat,
) -> Quat {
    // Convert model-space delta to bone-local via conjugation.
    let bone_rest_model = root_rest_global.inverse() * rest_global;
    let delta_local = bone_rest_model.inverse() * gaze_delta_model * bone_rest_model;
    (rest_local * delta_local).normalize()
}

/// Policy for handling missing eye bones.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MissingEyePolicy {
    /// Disable gaze entirely if either eye is missing.
    #[default]
    Disabled,
    /// Apply gaze to the available eye only.
    AvailableOnly,
}

/// Check if both eye bones are present.
#[must_use]
pub fn has_both_eyes(
    left_eye: Option<bevy::prelude::Entity>,
    right_eye: Option<bevy::prelude::Entity>,
) -> bool {
    left_eye.is_some() && right_eye.is_some()
}

/// Determine which eyes to apply gaze to based on policy and availability.
#[must_use]
pub fn resolve_eye_targets(
    left_eye: Option<bevy::prelude::Entity>,
    right_eye: Option<bevy::prelude::Entity>,
    policy: MissingEyePolicy,
) -> (Option<bevy::prelude::Entity>, Option<bevy::prelude::Entity>) {
    match policy {
        MissingEyePolicy::Disabled => {
            if has_both_eyes(left_eye, right_eye) {
                (left_eye, right_eye)
            } else {
                (None, None)
            }
        }
        MissingEyePolicy::AvailableOnly => (left_eye, right_eye),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eye_bone_gaze_center_preserves_rest() {
        let input = RawGazeInput {
            yaw_rad: 0.0,
            pitch_rad: 0.0,
        };
        let delta = compute_eye_bone_rotation(&input, &EyeBoneGazeSettings::default());

        assert!(
            delta.abs_diff_eq(Quat::IDENTITY, 1e-5),
            "center gaze should produce identity delta"
        );
    }

    #[test]
    fn eye_bone_gaze_positive_yaw_rotates_right() {
        let input = RawGazeInput {
            yaw_rad: 0.3,
            pitch_rad: 0.0,
        };
        let delta = compute_eye_bone_rotation(&input, &EyeBoneGazeSettings::default());

        let (axis, angle) = delta.to_axis_angle();
        assert!(axis.y > 0.0, "positive yaw should rotate around +Y");
        assert!(angle > 0.0, "rotation angle should be positive");
    }

    #[test]
    fn eye_bone_gaze_positive_pitch_rotates_up() {
        let input = RawGazeInput {
            yaw_rad: 0.0,
            pitch_rad: 0.2,
        };
        let delta = compute_eye_bone_rotation(&input, &EyeBoneGazeSettings::default());

        let (axis, angle) = delta.to_axis_angle();
        // pitch > 0 → -X rotation (ADR-004 convention)
        assert!(axis.x < 0.0, "positive pitch should rotate around -X");
        assert!(angle > 0.0, "rotation angle should be positive");
    }

    #[test]
    fn eye_bone_gaze_extreme_values_clamped() {
        let input = RawGazeInput {
            yaw_rad: 10.0,
            pitch_rad: -10.0,
        };
        let settings = EyeBoneGazeSettings::default();
        let delta = compute_eye_bone_rotation(&input, &settings);

        // The delta should be finite and within expected range.
        assert!(delta.is_finite());

        // Verify the clamped values produce the expected rotation.
        let clamped_input = RawGazeInput {
            yaw_rad: settings.max_yaw_rad,
            pitch_rad: -settings.max_pitch_rad,
        };
        let expected = compute_eye_bone_rotation(&clamped_input, &settings);
        assert!(delta.abs_diff_eq(expected, 1e-5));
    }

    #[test]
    fn eye_bone_gaze_local_rotation_with_identity_rest() {
        let rest_local = Quat::IDENTITY;
        let rest_global = Quat::IDENTITY;
        let root_rest = Quat::IDENTITY;
        let gaze_delta = Quat::from_rotation_y(0.3);

        let output =
            compute_eye_bone_local_rotation(rest_local, gaze_delta, rest_global, root_rest);

        // With identity rest, output should equal the gaze delta.
        assert!(output.abs_diff_eq(gaze_delta, 1e-4));
    }

    #[test]
    fn eye_bone_gaze_missing_eye_policy_disabled() {
        let left = Some(bevy::prelude::Entity::PLACEHOLDER);
        let right = None;

        let (l, r) = resolve_eye_targets(left, right, MissingEyePolicy::Disabled);
        assert!(l.is_none());
        assert!(r.is_none());
    }

    #[test]
    fn eye_bone_gaze_missing_eye_policy_available_only() {
        let left = Some(bevy::prelude::Entity::PLACEHOLDER);
        let right = None;

        let (l, r) = resolve_eye_targets(left, right, MissingEyePolicy::AvailableOnly);
        assert!(l.is_some());
        assert!(r.is_none());
    }

    #[test]
    fn eye_bone_gaze_has_both_eyes() {
        let left = Some(bevy::prelude::Entity::PLACEHOLDER);
        let right = Some(bevy::prelude::Entity::from_bits(2));
        assert!(has_both_eyes(left, right));

        assert!(!has_both_eyes(left, None));
        assert!(!has_both_eyes(None, right));
        assert!(!has_both_eyes(None, None));
    }
}
