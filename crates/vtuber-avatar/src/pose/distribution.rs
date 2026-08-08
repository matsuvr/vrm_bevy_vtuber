//! Head/neck weight distribution and per-axis range clamping.
//!
//! Distributes a semantic head pose between head and neck bones according
//! to configurable weights. When the neck bone is absent, all rotation is
//! applied to the head.

use bevy::math::Quat;
use bevy::prelude::Resource;

use super::binding::RestOrientationCache;
use super::types::{ClampedHeadPose, ModelSpaceDelta, semantic_to_model_delta};
use crate::pose::math::apply_model_delta_to_bone;

/// Weight distribution between head and neck bones.
///
/// Weights must sum to 1.0. When `neck_weight` is 0.0 (or the neck bone is
/// absent), all rotation goes to the head.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadNeckWeights {
    /// Fraction of rotation applied to the head bone. Range [0, 1].
    pub head: f32,
    /// Fraction of rotation applied to the neck bone. Range [0, 1].
    pub neck: f32,
}

impl Default for HeadNeckWeights {
    fn default() -> Self {
        // 60% head, 40% neck — a common VTuber rigging ratio.
        Self {
            head: 0.6,
            neck: 0.4,
        }
    }
}

impl HeadNeckWeights {
    /// Create weights that send everything to the head (no neck).
    #[must_use]
    pub fn head_only() -> Self {
        Self {
            head: 1.0,
            neck: 0.0,
        }
    }

    /// Normalize so that head + neck == 1.0.
    fn normalized(self) -> Self {
        let sum = self.head + self.neck;
        if sum <= 0.0 {
            return Self::head_only();
        }
        Self {
            head: self.head / sum,
            neck: self.neck / sum,
        }
    }
}

/// Per-axis maximum angle clamp settings in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseClampSettings {
    /// Maximum absolute yaw in radians.
    pub max_yaw_rad: f32,
    /// Maximum absolute pitch in radians.
    pub max_pitch_rad: f32,
    /// Maximum absolute roll in radians.
    pub max_roll_rad: f32,
}

impl Default for PoseClampSettings {
    fn default() -> Self {
        Self {
            max_yaw_rad: std::f32::consts::FRAC_PI_2,
            max_pitch_rad: std::f32::consts::FRAC_PI_2,
            max_roll_rad: std::f32::consts::FRAC_PI_2,
        }
    }
}

/// Diagnostic output from clamped distribution, showing before/after values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistributionDiagnostic {
    /// Input yaw before clamping (radians).
    pub raw_yaw_rad: f32,
    /// Input pitch before clamping (radians).
    pub raw_pitch_rad: f32,
    /// Input roll before clamping (radians).
    pub raw_roll_rad: f32,
    /// Yaw after clamping (radians).
    pub clamped_yaw_rad: f32,
    /// Pitch after clamping (radians).
    pub clamped_pitch_rad: f32,
    /// Roll after clamping (radians).
    pub clamped_roll_rad: f32,
    /// Whether any value was clamped.
    pub was_clamped: bool,
}

/// Result of distributing a pose to head and neck.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistributedPose {
    /// Model-space delta for the head bone.
    pub head_delta: ModelSpaceDelta,
    /// Model-space delta for the neck bone (None if neck is absent).
    pub neck_delta: Option<ModelSpaceDelta>,
    /// Diagnostic information about the distribution.
    pub diagnostic: DistributionDiagnostic,
}

/// Settings for the pose distribution pipeline.
///
/// Combines weights and clamp settings into a single configurable type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Resource)]
pub struct PoseDistributionSettings {
    /// Head/neck weight distribution.
    pub weights: HeadNeckWeights,
    /// Per-axis clamp limits.
    pub clamp: PoseClampSettings,
}

/// Distribute a raw head pose between head and neck bones.
///
/// 1. Clamp the input to per-axis limits.
/// 2. Split the clamped pose between head and neck according to weights.
/// 3. Convert each portion to a model-space delta.
///
/// When `has_neck` is false, all rotation goes to the head regardless of
/// the configured neck weight.
#[must_use]
pub fn distribute_pose(
    raw_yaw: f32,
    raw_pitch: f32,
    raw_roll: f32,
    has_neck: bool,
    settings: &PoseDistributionSettings,
) -> DistributedPose {
    // Step 1: Clamp.
    let clamped_yaw = clamp_axis(raw_yaw, settings.clamp.max_yaw_rad);
    let clamped_pitch = clamp_axis(raw_pitch, settings.clamp.max_pitch_rad);
    let clamped_roll = clamp_axis(raw_roll, settings.clamp.max_roll_rad);

    let was_clamped =
        clamped_yaw != raw_yaw || clamped_pitch != raw_pitch || clamped_roll != raw_roll;

    let diagnostic = DistributionDiagnostic {
        raw_yaw_rad: raw_yaw,
        raw_pitch_rad: raw_pitch,
        raw_roll_rad: raw_roll,
        clamped_yaw_rad: clamped_yaw,
        clamped_pitch_rad: clamped_pitch,
        clamped_roll_rad: clamped_roll,
        was_clamped,
    };

    // Step 2: Determine effective weights.
    let weights = if has_neck {
        settings.weights.normalized()
    } else {
        HeadNeckWeights::head_only()
    };

    // Step 3: Split and convert.
    let head_pose = ClampedHeadPose {
        yaw_rad: clamped_yaw * weights.head,
        pitch_rad: clamped_pitch * weights.head,
        roll_rad: clamped_roll * weights.head,
    };
    let head_delta = semantic_to_model_delta(&head_pose);

    let neck_delta = if has_neck && weights.neck > 0.0 {
        let neck_pose = ClampedHeadPose {
            yaw_rad: clamped_yaw * weights.neck,
            pitch_rad: clamped_pitch * weights.neck,
            roll_rad: clamped_roll * weights.neck,
        };
        Some(semantic_to_model_delta(&neck_pose))
    } else {
        None
    };

    DistributedPose {
        head_delta,
        neck_delta,
        diagnostic,
    }
}

/// Apply a distributed pose to head and neck bone transforms.
///
/// Uses the rest orientation cache to convert model-space deltas to
/// bone-local rotations via conjugation (ADR-004).
///
/// Returns the new local rotations for head and optionally neck.
pub fn apply_distributed_pose(
    distributed: &DistributedPose,
    cache: &RestOrientationCache,
) -> (Quat, Option<Quat>) {
    let head_output = apply_model_delta_to_bone(
        distributed.head_delta.rotation,
        cache.root_rest_global,
        cache.head_rest_global,
        cache.head_rest_local,
    );

    let neck_output = if let (Some(neck_delta), Some(neck_rest_global), Some(neck_rest_local)) = (
        distributed.neck_delta,
        cache.neck_rest_global,
        cache.neck_rest_local,
    ) {
        Some(apply_model_delta_to_bone(
            neck_delta.rotation,
            cache.root_rest_global,
            neck_rest_global,
            neck_rest_local,
        ))
    } else {
        None
    };

    (head_output, neck_output)
}

/// Clamp a single axis value to [-max, max], replacing NaN with 0.
fn clamp_axis(value: f32, max: f32) -> f32 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(-max, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Quat;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: Quat, b: Quat) -> bool {
        let dot = a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
        dot.abs() > 1.0 - EPSILON
    }

    #[test]
    fn pose_distribution_neutral_input() {
        let settings = PoseDistributionSettings::default();
        let result = distribute_pose(0.0, 0.0, 0.0, true, &settings);

        assert!(approx_eq(result.head_delta.rotation, Quat::IDENTITY));
        assert!(result.neck_delta.is_some());
        assert!(approx_eq(
            result.neck_delta.unwrap().rotation,
            Quat::IDENTITY
        ));
        assert!(!result.diagnostic.was_clamped);
    }

    #[test]
    fn pose_distribution_head_neck_split() {
        let settings = PoseDistributionSettings {
            weights: HeadNeckWeights {
                head: 0.6,
                neck: 0.4,
            },
            ..Default::default()
        };
        let result = distribute_pose(0.5, 0.0, 0.0, true, &settings);

        // Head should get 60% of the yaw.
        let head_only = distribute_pose(0.5 * 0.6, 0.0, 0.0, false, &settings);
        assert!(approx_eq(
            result.head_delta.rotation,
            head_only.head_delta.rotation
        ));

        // Neck should get 40% of the yaw.
        let neck_only = distribute_pose(0.5 * 0.4, 0.0, 0.0, false, &settings);
        assert!(result.neck_delta.is_some());
        assert!(approx_eq(
            result.neck_delta.unwrap().rotation,
            neck_only.head_delta.rotation
        ));
    }

    #[test]
    fn pose_distribution_no_neck_all_to_head() {
        let settings = PoseDistributionSettings {
            weights: HeadNeckWeights {
                head: 0.6,
                neck: 0.4,
            },
            ..Default::default()
        };
        let result = distribute_pose(0.5, 0.3, 0.1, false, &settings);

        // All rotation should go to head.
        let all_head = distribute_pose(
            0.5,
            0.3,
            0.1,
            false,
            &PoseDistributionSettings {
                weights: HeadNeckWeights::head_only(),
                ..Default::default()
            },
        );
        assert!(approx_eq(
            result.head_delta.rotation,
            all_head.head_delta.rotation
        ));
        assert!(result.neck_delta.is_none());
    }

    #[test]
    fn pose_distribution_extreme_input_clamped() {
        let settings = PoseDistributionSettings::default();
        let result = distribute_pose(10.0, -10.0, 10.0, true, &settings);

        assert!(result.diagnostic.was_clamped);
        assert_eq!(
            result.diagnostic.clamped_yaw_rad,
            settings.clamp.max_yaw_rad
        );
        assert_eq!(
            result.diagnostic.clamped_pitch_rad,
            -settings.clamp.max_pitch_rad
        );
        assert_eq!(
            result.diagnostic.clamped_roll_rad,
            settings.clamp.max_roll_rad
        );
    }

    #[test]
    fn pose_distribution_nan_replaced_with_zero() {
        let settings = PoseDistributionSettings::default();
        let result = distribute_pose(f32::NAN, f32::NAN, f32::NAN, true, &settings);

        assert_eq!(result.diagnostic.clamped_yaw_rad, 0.0);
        assert_eq!(result.diagnostic.clamped_pitch_rad, 0.0);
        assert_eq!(result.diagnostic.clamped_roll_rad, 0.0);
        assert!(result.diagnostic.was_clamped);
    }

    #[test]
    fn pose_distribution_composed_delta_matches_total() {
        // The composition of head_delta * neck_delta should approximate
        // the full unsplit delta (within floating-point tolerance).
        let settings = PoseDistributionSettings {
            weights: HeadNeckWeights {
                head: 0.6,
                neck: 0.4,
            },
            ..Default::default()
        };
        let distributed = distribute_pose(0.4, 0.2, 0.1, true, &settings);
        let full = distribute_pose(
            0.4,
            0.2,
            0.1,
            false,
            &PoseDistributionSettings {
                weights: HeadNeckWeights::head_only(),
                ..Default::default()
            },
        );

        let composed = distributed.head_delta.rotation * distributed.neck_delta.unwrap().rotation;

        // The composed rotation should be close to the full rotation.
        // Note: due to non-commutativity of quaternion multiplication,
        // head * neck ≠ neck * head in general, but the total angle
        // should be preserved.
        let composed_angle = composed.to_axis_angle().1;
        let full_angle = full.head_delta.rotation.to_axis_angle().1;
        assert!(
            (composed_angle - full_angle).abs() < 0.01,
            "composed angle {composed_angle} should be close to full angle {full_angle}"
        );
    }

    #[test]
    fn pose_distribution_weights_normalize() {
        let w = HeadNeckWeights {
            head: 3.0,
            neck: 7.0,
        };
        let n = w.normalized();
        assert!((n.head - 0.3).abs() < EPSILON);
        assert!((n.neck - 0.7).abs() < EPSILON);
    }

    #[test]
    fn pose_distribution_zero_weights_fallback() {
        let w = HeadNeckWeights {
            head: 0.0,
            neck: 0.0,
        };
        let n = w.normalized();
        assert_eq!(n.head, 1.0);
        assert_eq!(n.neck, 0.0);
    }
}
