//! Pure-function boundary between semantic head pose and VRM model-space delta.
//!
//! # Sign convention (ADR-004)
//!
//! | Semantic | Model-space axis |
//! |----------|-----------------|
//! | yaw > 0 (face right in unmirrored image) | +Y rotation |
//! | pitch > 0 (chin up) | -X rotation |
//! | roll > 0 (clockwise tilt, viewer perspective) | -Z rotation |
//!
//! Euler order: intrinsic Y-X-Z, i.e. `R = R_y(yaw) * R_x(-pitch) * R_z(-roll)`.
//!
//! # Units
//!
//! All angles are in **radians**.

use bevy::math::Quat;
use vtuber_core::types::HeadPose;

/// Maximum absolute yaw in radians (90 degrees).
pub const MAX_YAW_RAD: f32 = std::f32::consts::FRAC_PI_2;

/// Maximum absolute pitch in radians (90 degrees).
pub const MAX_PITCH_RAD: f32 = std::f32::consts::FRAC_PI_2;

/// Maximum absolute roll in radians (90 degrees).
pub const MAX_ROLL_RAD: f32 = std::f32::consts::FRAC_PI_2;

/// Post-clamp semantic head pose in radians.
///
/// Produced by [`clamp_head_pose`] from a raw [`HeadPose`].
/// All fields are guaranteed to be within `[-MAX_*_RAD, MAX_*_RAD]`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClampedHeadPose {
    /// Yaw in radians, clamped to `[-MAX_YAW_RAD, MAX_YAW_RAD]`.
    pub yaw_rad: f32,
    /// Pitch in radians, clamped to `[-MAX_PITCH_RAD, MAX_PITCH_RAD]`.
    pub pitch_rad: f32,
    /// Roll in radians, clamped to `[-MAX_ROLL_RAD, MAX_ROLL_RAD]`.
    pub roll_rad: f32,
}

/// VRM model-space delta rotation for head/neck bones.
///
/// This is the rotation delta to apply relative to the bone's rest orientation.
/// See ADR-004 for the conjugation formula that converts this model-space delta
/// into bone-local space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelSpaceDelta {
    /// Unit quaternion representing the model-space rotation delta.
    pub rotation: Quat,
}

/// Raw per-axis model-space deltas before Euler composition.
///
/// Each field is the rotation around a single model-space axis, derived from
/// the corresponding semantic axis. Useful for diagnostics and per-axis clamping.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawAxisDeltas {
    /// Rotation around model-space +Y axis (from semantic yaw).
    pub yaw_delta: Quat,
    /// Rotation around model-space -X axis (from semantic pitch).
    pub pitch_delta: Quat,
    /// Rotation around model-space -Z axis (from semantic roll).
    pub roll_delta: Quat,
}

/// Error returned when input contains non-finite values (NaN or infinity).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonFiniteInputError;

impl std::fmt::Display for NonFiniteInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "head pose contains non-finite value (NaN or infinity)")
    }
}

impl std::error::Error for NonFiniteInputError {}

/// Clamp a raw semantic head pose to valid ranges.
///
/// Each component is clamped independently to its maximum absolute value.
/// NaN values are replaced with 0.0.
#[must_use]
pub fn clamp_head_pose(raw: &HeadPose) -> ClampedHeadPose {
    ClampedHeadPose {
        yaw_rad: clamp_or_zero(raw.yaw_rad, MAX_YAW_RAD),
        pitch_rad: clamp_or_zero(raw.pitch_rad, MAX_PITCH_RAD),
        roll_rad: clamp_or_zero(raw.roll_rad, MAX_ROLL_RAD),
    }
}

/// Convert a clamped semantic head pose to a VRM model-space delta quaternion.
///
/// Follows ADR-004:
/// - yaw → +Y axis rotation
/// - pitch → -X axis rotation
/// - roll → -Z axis rotation
/// - Intrinsic Y-X-Z Euler order
///
/// The result is a normalized unit quaternion.
///
/// # Invariant
///
/// Neutral input (all zeros) produces [`Quat::IDENTITY`].
#[must_use]
pub fn semantic_to_model_delta(pose: &ClampedHeadPose) -> ModelSpaceDelta {
    // ADR-004: yaw → +Y, pitch → -X, roll → -Z
    let rotation = Quat::from_euler(
        bevy::math::EulerRot::YXZ,
        pose.yaw_rad,
        -pose.pitch_rad,
        -pose.roll_rad,
    );
    // Quat::from_euler always produces a unit quaternion, but normalize for safety.
    ModelSpaceDelta {
        rotation: rotation.normalize(),
    }
}

/// Validate that a head pose contains only finite values.
///
/// Returns `Err(NonFiniteInputError)` if any component is NaN or infinity.
pub fn validate_head_pose(pose: &HeadPose) -> Result<(), NonFiniteInputError> {
    if pose.yaw_rad.is_finite() && pose.pitch_rad.is_finite() && pose.roll_rad.is_finite() {
        Ok(())
    } else {
        Err(NonFiniteInputError)
    }
}

/// Compute raw per-axis model-space deltas before Euler composition.
///
/// Each axis produces an independent quaternion. The final composed rotation
/// is `yaw_delta * pitch_delta * roll_delta` (Y-X-Z intrinsic order).
///
/// This is useful for diagnostics and per-axis clamping strategies.
#[must_use]
pub fn raw_axis_deltas(pose: &ClampedHeadPose) -> RawAxisDeltas {
    RawAxisDeltas {
        yaw_delta: Quat::from_axis_angle(bevy::math::Vec3::Y, pose.yaw_rad),
        pitch_delta: Quat::from_axis_angle(bevy::math::Vec3::X, -pose.pitch_rad),
        roll_delta: Quat::from_axis_angle(bevy::math::Vec3::Z, -pose.roll_rad),
    }
}

/// Convert a clamped head pose to model-space delta using explicit axis composition.
///
/// This is equivalent to [`semantic_to_model_delta`] but makes the per-axis
/// composition explicit: `R = R_y(yaw) * R_x(-pitch) * R_z(-roll)`.
///
/// Useful for testing that the Euler order is correct.
#[must_use]
pub fn semantic_to_model_delta_explicit(pose: &ClampedHeadPose) -> ModelSpaceDelta {
    let raw = raw_axis_deltas(pose);
    // Y-X-Z intrinsic: R = R_y * R_x * R_z
    let rotation = (raw.yaw_delta * raw.pitch_delta * raw.roll_delta).normalize();
    ModelSpaceDelta { rotation }
}

/// Clamp a value to `[-max, max]`, replacing NaN with 0.0.
fn clamp_or_zero(value: f32, max: f32) -> f32 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(-max, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    const EPSILON: f32 = 1e-6;

    fn approx_eq(a: Quat, b: Quat) -> bool {
        // Quaternions q and -q represent the same rotation.
        let dot = a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
        dot.abs() > 1.0 - EPSILON
    }

    #[test]
    fn pose_semantics_neutral_produces_identity() {
        let raw = HeadPose {
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            roll_rad: 0.0,
        };
        let clamped = clamp_head_pose(&raw);
        let delta = semantic_to_model_delta(&clamped);
        assert!(
            approx_eq(delta.rotation, Quat::IDENTITY),
            "neutral pose must produce identity delta"
        );
    }

    #[test]
    fn pose_semantics_positive_yaw_rotates_around_positive_y() {
        let raw = HeadPose {
            yaw_rad: 0.5,
            pitch_rad: 0.0,
            roll_rad: 0.0,
        };
        let clamped = clamp_head_pose(&raw);
        let delta = semantic_to_model_delta(&clamped);

        // Pure yaw should produce rotation around +Y axis.
        let axis = delta.rotation.to_axis_angle();
        assert!(axis.1.abs() > EPSILON, "rotation angle should be non-zero");
        assert!(
            axis.0.y > 0.0 && axis.0.x.abs() < EPSILON && axis.0.z.abs() < EPSILON,
            "positive yaw should rotate around +Y, got axis {:?}",
            axis.0
        );
    }

    #[test]
    fn pose_semantics_negative_yaw_rotates_around_negative_y() {
        let raw = HeadPose {
            yaw_rad: -0.5,
            pitch_rad: 0.0,
            roll_rad: 0.0,
        };
        let clamped = clamp_head_pose(&raw);
        let delta = semantic_to_model_delta(&clamped);

        let axis = delta.rotation.to_axis_angle();
        assert!(
            axis.0.y < 0.0 && axis.0.x.abs() < EPSILON && axis.0.z.abs() < EPSILON,
            "negative yaw should rotate around -Y, got axis {:?}",
            axis.0
        );
    }

    #[test]
    fn pose_semantics_positive_pitch_rotates_around_negative_x() {
        let raw = HeadPose {
            yaw_rad: 0.0,
            pitch_rad: 0.5,
            roll_rad: 0.0,
        };
        let clamped = clamp_head_pose(&raw);
        let delta = semantic_to_model_delta(&clamped);

        // ADR-004: pitch > 0 → -X rotation.
        let axis = delta.rotation.to_axis_angle();
        assert!(
            axis.0.x < 0.0 && axis.0.y.abs() < EPSILON && axis.0.z.abs() < EPSILON,
            "positive pitch should rotate around -X, got axis {:?}",
            axis.0
        );
    }

    #[test]
    fn pose_semantics_negative_pitch_rotates_around_positive_x() {
        let raw = HeadPose {
            yaw_rad: 0.0,
            pitch_rad: -0.5,
            roll_rad: 0.0,
        };
        let clamped = clamp_head_pose(&raw);
        let delta = semantic_to_model_delta(&clamped);

        let axis = delta.rotation.to_axis_angle();
        assert!(
            axis.0.x > 0.0 && axis.0.y.abs() < EPSILON && axis.0.z.abs() < EPSILON,
            "negative pitch should rotate around +X, got axis {:?}",
            axis.0
        );
    }

    #[test]
    fn pose_semantics_positive_roll_rotates_around_negative_z() {
        let raw = HeadPose {
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            roll_rad: 0.5,
        };
        let clamped = clamp_head_pose(&raw);
        let delta = semantic_to_model_delta(&clamped);

        // ADR-004: roll > 0 → -Z rotation.
        let axis = delta.rotation.to_axis_angle();
        assert!(
            axis.0.z < 0.0 && axis.0.x.abs() < EPSILON && axis.0.y.abs() < EPSILON,
            "positive roll should rotate around -Z, got axis {:?}",
            axis.0
        );
    }

    #[test]
    fn pose_semantics_negative_roll_rotates_around_positive_z() {
        let raw = HeadPose {
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            roll_rad: -0.5,
        };
        let clamped = clamp_head_pose(&raw);
        let delta = semantic_to_model_delta(&clamped);

        let axis = delta.rotation.to_axis_angle();
        assert!(
            axis.0.z > 0.0 && axis.0.x.abs() < EPSILON && axis.0.y.abs() < EPSILON,
            "negative roll should rotate around +Z, got axis {:?}",
            axis.0
        );
    }

    #[test]
    fn pose_semantics_units_are_radians() {
        // 1 radian should produce a different result than 1 degree.
        let one_radian = HeadPose {
            yaw_rad: 1.0,
            pitch_rad: 0.0,
            roll_rad: 0.0,
        };
        let one_degree = HeadPose {
            yaw_rad: std::f32::consts::PI / 180.0,
            pitch_rad: 0.0,
            roll_rad: 0.0,
        };

        let delta_rad = semantic_to_model_delta(&clamp_head_pose(&one_radian));
        let delta_deg = semantic_to_model_delta(&clamp_head_pose(&one_degree));

        assert!(
            !approx_eq(delta_rad.rotation, delta_deg.rotation),
            "1 radian and 1 degree must produce different rotations"
        );
    }

    #[test]
    fn pose_semantics_clamp_limits_values() {
        let raw = HeadPose {
            yaw_rad: 10.0,
            pitch_rad: -10.0,
            roll_rad: 10.0,
        };
        let clamped = clamp_head_pose(&raw);

        assert_eq!(clamped.yaw_rad, MAX_YAW_RAD);
        assert_eq!(clamped.pitch_rad, -MAX_PITCH_RAD);
        assert_eq!(clamped.roll_rad, MAX_ROLL_RAD);
    }

    #[test]
    fn pose_semantics_clamp_replaces_nan() {
        let raw = HeadPose {
            yaw_rad: f32::NAN,
            pitch_rad: f32::NAN,
            roll_rad: f32::NAN,
        };
        let clamped = clamp_head_pose(&raw);

        assert_eq!(clamped.yaw_rad, 0.0);
        assert_eq!(clamped.pitch_rad, 0.0);
        assert_eq!(clamped.roll_rad, 0.0);
    }

    #[test]
    fn pose_semantics_combined_rotation_is_yxz_order() {
        // Verify that the combined rotation matches Y-X-Z intrinsic order.
        let yaw = 0.3_f32;
        let pitch = 0.2_f32;
        let roll = 0.1_f32;

        let raw = HeadPose {
            yaw_rad: yaw,
            pitch_rad: pitch,
            roll_rad: roll,
        };
        let clamped = clamp_head_pose(&raw);
        let delta = semantic_to_model_delta(&clamped);

        // Manual construction with ADR-004 signs.
        let expected = Quat::from_euler(bevy::math::EulerRot::YXZ, yaw, -pitch, -roll);

        assert!(
            approx_eq(delta.rotation, expected),
            "combined rotation should match YXZ Euler with ADR-004 signs"
        );
    }

    #[test]
    fn pose_semantics_delta_is_unit_quaternion() {
        let raw = HeadPose {
            yaw_rad: 0.7,
            pitch_rad: -0.3,
            roll_rad: 0.5,
        };
        let clamped = clamp_head_pose(&raw);
        let delta = semantic_to_model_delta(&clamped);

        let norm = Vec3::new(delta.rotation.x, delta.rotation.y, delta.rotation.z).length();
        let w_sq = delta.rotation.w * delta.rotation.w;
        assert!(
            (norm * norm + w_sq - 1.0).abs() < EPSILON,
            "delta must be a unit quaternion"
        );
    }

    // ---- M1-05-003: non-finite rejection and raw axis deltas ----

    #[test]
    fn model_space_pose_validate_rejects_nan() {
        let pose = HeadPose {
            yaw_rad: f32::NAN,
            pitch_rad: 0.0,
            roll_rad: 0.0,
        };
        assert_eq!(validate_head_pose(&pose), Err(NonFiniteInputError));
    }

    #[test]
    fn model_space_pose_validate_rejects_infinity() {
        let pose = HeadPose {
            yaw_rad: 0.0,
            pitch_rad: f32::INFINITY,
            roll_rad: 0.0,
        };
        assert_eq!(validate_head_pose(&pose), Err(NonFiniteInputError));
    }

    #[test]
    fn model_space_pose_validate_accepts_finite() {
        let pose = HeadPose {
            yaw_rad: 0.5,
            pitch_rad: -0.3,
            roll_rad: 0.1,
        };
        assert!(validate_head_pose(&pose).is_ok());
    }

    #[test]
    fn model_space_pose_raw_axis_deltas_neutral() {
        let pose = ClampedHeadPose {
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            roll_rad: 0.0,
        };
        let raw = raw_axis_deltas(&pose);
        assert!(approx_eq(raw.yaw_delta, Quat::IDENTITY));
        assert!(approx_eq(raw.pitch_delta, Quat::IDENTITY));
        assert!(approx_eq(raw.roll_delta, Quat::IDENTITY));
    }

    #[test]
    fn model_space_pose_raw_axis_deltas_yaw_only() {
        let pose = ClampedHeadPose {
            yaw_rad: 0.5,
            pitch_rad: 0.0,
            roll_rad: 0.0,
        };
        let raw = raw_axis_deltas(&pose);

        // Yaw should produce rotation around +Y.
        let (axis, angle) = raw.yaw_delta.to_axis_angle();
        assert!(axis.y > 0.0 && axis.x.abs() < EPSILON && axis.z.abs() < EPSILON);
        assert!((angle - 0.5).abs() < EPSILON);

        // Pitch and roll should be identity.
        assert!(approx_eq(raw.pitch_delta, Quat::IDENTITY));
        assert!(approx_eq(raw.roll_delta, Quat::IDENTITY));
    }

    #[test]
    fn model_space_pose_explicit_matches_euler() {
        let pose = ClampedHeadPose {
            yaw_rad: 0.3,
            pitch_rad: 0.2,
            roll_rad: 0.1,
        };
        let euler_delta = semantic_to_model_delta(&pose);
        let explicit_delta = semantic_to_model_delta_explicit(&pose);

        assert!(
            approx_eq(euler_delta.rotation, explicit_delta.rotation),
            "explicit axis composition must match Euler result"
        );
    }

    #[test]
    fn model_space_pose_wrong_order_detected() {
        // Verify that swapping the Euler order produces a different result.
        let pose = ClampedHeadPose {
            yaw_rad: 0.5,
            pitch_rad: 0.3,
            roll_rad: 0.2,
        };
        let correct = semantic_to_model_delta(&pose);

        // Wrong order: Z-X-Y instead of Y-X-Z.
        let wrong = Quat::from_euler(
            bevy::math::EulerRot::ZXY,
            -pose.roll_rad,
            -pose.pitch_rad,
            pose.yaw_rad,
        );

        assert!(
            !approx_eq(correct.rotation, wrong),
            "wrong Euler order must be detected by test"
        );
    }

    #[test]
    fn model_space_pose_result_determinant_is_one() {
        let pose = ClampedHeadPose {
            yaw_rad: 0.7,
            pitch_rad: -0.4,
            roll_rad: 0.3,
        };
        let delta = semantic_to_model_delta(&pose);

        // Unit quaternion has norm 1, which implies determinant 1 for the rotation matrix.
        let norm_sq = delta.rotation.length_squared();
        assert!(
            (norm_sq - 1.0).abs() < EPSILON,
            "quaternion norm must be 1.0, got {norm_sq}"
        );
    }
}
