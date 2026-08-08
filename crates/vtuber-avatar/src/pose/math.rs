//! Conjugation math for converting model-space deltas to bone-local space.
//!
//! # ADR-004 formula
//!
//! ```text
//! R_bone_rest_model = inverse(R_root_rest_global) * R_bone_rest_global
//! R_delta_local     = inverse(R_bone_rest_model) * R_delta_model * R_bone_rest_model
//! R_output_local    = R_bone_rest_local * R_delta_local
//! ```
//!
//! All functions are pure and unit-testable without Bevy ECS.

use bevy::math::Quat;

/// Compute the bone rest orientation in model space.
///
/// `R_bone_rest_model = inverse(R_root_rest_global) * R_bone_rest_global`
///
/// This gives the bone's rest rotation relative to the avatar root,
/// expressed in model-space coordinates.
#[must_use]
pub fn bone_rest_model_rotation(root_rest_global: Quat, bone_rest_global: Quat) -> Quat {
    root_rest_global.inverse() * bone_rest_global
}

/// Convert a model-space delta rotation to bone-local delta via conjugation.
///
/// `R_delta_local = inverse(R_bone_rest_model) * R_delta_model * R_bone_rest_model`
///
/// This is the core of the rest-pose-aware pose application: it rotates the
/// model-space tracking delta into the bone's local rest frame so that the
/// rotation axes align with the bone's actual orientation in the model.
#[must_use]
pub fn model_delta_to_local_delta(model_delta: Quat, bone_rest_model: Quat) -> Quat {
    bone_rest_model.inverse() * model_delta * bone_rest_model
}

/// Compute the final bone-local output rotation.
///
/// `R_output_local = R_bone_rest_local * R_delta_local`
///
/// This is the rotation to set on the bone's `Transform.rotation`, combining
/// the rest pose with the tracking delta.
#[must_use]
pub fn compute_output_rotation(bone_rest_local: Quat, delta_local: Quat) -> Quat {
    (bone_rest_local * delta_local).normalize()
}

/// Full pipeline: model-space delta → bone-local output rotation.
///
/// Combines [`bone_rest_model_rotation`], [`model_delta_to_local_delta`],
/// and [`compute_output_rotation`] into a single call.
#[must_use]
pub fn apply_model_delta_to_bone(
    model_delta: Quat,
    root_rest_global: Quat,
    bone_rest_global: Quat,
    bone_rest_local: Quat,
) -> Quat {
    let bone_rest_model = bone_rest_model_rotation(root_rest_global, bone_rest_global);
    let delta_local = model_delta_to_local_delta(model_delta, bone_rest_model);
    compute_output_rotation(bone_rest_local, delta_local)
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

    // ---- identity rest: model delta == local delta ----

    #[test]
    fn local_pose_conjugation_identity_rest_preserves_delta() {
        let root_rest = Quat::IDENTITY;
        let bone_rest_global = Quat::IDENTITY;
        let bone_rest_local = Quat::IDENTITY;
        let model_delta = Quat::from_rotation_y(0.3);

        let bone_rest_model = bone_rest_model_rotation(root_rest, bone_rest_global);
        assert!(
            approx_eq(bone_rest_model, Quat::IDENTITY),
            "identity rest should give identity model rotation"
        );

        let delta_local = model_delta_to_local_delta(model_delta, bone_rest_model);
        assert!(
            approx_eq(delta_local, model_delta),
            "with identity rest, local delta should equal model delta"
        );

        let output = compute_output_rotation(bone_rest_local, delta_local);
        assert!(approx_eq(output, model_delta));
    }

    // ---- non-identity rest: world direction is correct ----

    #[test]
    fn local_pose_conjugation_rotated_root_preserves_world_direction() {
        // Root is rotated 90° around Y.
        let root_rest_global = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        // Head has no additional local rotation relative to root.
        let bone_rest_global = root_rest_global;
        let bone_rest_local = Quat::IDENTITY;

        // Model-space yaw: rotate 30° around model +Y.
        let model_delta = Quat::from_rotation_y(0.3);

        let output = apply_model_delta_to_bone(
            model_delta,
            root_rest_global,
            bone_rest_global,
            bone_rest_local,
        );

        // With identity local rest and bone aligned with root,
        // the local delta should equal the model delta.
        assert!(
            approx_eq(output, model_delta),
            "bone aligned with root should get model delta as local output"
        );
    }

    #[test]
    fn local_pose_conjugation_non_identity_bone_rest() {
        // Root is identity.
        let root_rest_global = Quat::IDENTITY;
        // Head bone has a rest rotation of 45° around X (tilted forward).
        let bone_rest_global = Quat::from_rotation_x(0.5);
        let _bone_rest_local = bone_rest_global; // root is identity

        // Model-space delta: 30° yaw.
        let model_delta = Quat::from_rotation_y(0.3);

        let bone_rest_model = bone_rest_model_rotation(root_rest_global, bone_rest_global);
        assert!(approx_eq(bone_rest_model, bone_rest_global));

        let delta_local = model_delta_to_local_delta(model_delta, bone_rest_model);

        // The local delta should NOT equal the model delta because the bone
        // has a non-identity rest orientation.
        assert!(
            !approx_eq(delta_local, model_delta),
            "non-identity bone rest should change the local delta"
        );

        // Verify the conjugation is correct by round-tripping.
        let round_trip = bone_rest_model * delta_local * bone_rest_model.inverse();
        assert!(
            approx_eq(round_trip, model_delta),
            "conjugation round-trip should recover model delta"
        );
    }

    // ---- quaternion multiplication order regression ----

    #[test]
    fn local_pose_conjugation_multiplication_order_matters() {
        let bone_rest_model = Quat::from_rotation_x(0.5);
        let model_delta = Quat::from_rotation_y(0.3);

        let correct = model_delta_to_local_delta(model_delta, bone_rest_model);

        // Wrong order: delta * rest instead of rest_inv * delta * rest.
        let _wrong = bone_rest_model.inverse() * (model_delta * bone_rest_model);
        // This is actually the same as correct due to associativity.
        // Let's test a truly wrong order.
        let truly_wrong = model_delta * bone_rest_model;

        assert!(
            !approx_eq(correct, truly_wrong),
            "wrong multiplication order must produce different result"
        );
    }

    #[test]
    fn local_pose_conjugation_output_is_unit_quaternion() {
        let root_rest = Quat::from_euler(bevy::math::EulerRot::XYZ, 0.1, 0.2, 0.3);
        let bone_rest_global = root_rest * Quat::from_rotation_x(0.4);
        let bone_rest_local = Quat::from_rotation_x(0.4);
        let model_delta = Quat::from_euler(bevy::math::EulerRot::YXZ, 0.5, -0.3, -0.2);

        let output =
            apply_model_delta_to_bone(model_delta, root_rest, bone_rest_global, bone_rest_local);

        let norm_sq = output.length_squared();
        assert!(
            (norm_sq - 1.0).abs() < EPSILON,
            "output must be a unit quaternion, got norm_sq={norm_sq}"
        );
    }

    #[test]
    fn local_pose_conjugation_neutral_delta_gives_rest() {
        let root_rest = Quat::from_rotation_z(0.3);
        let bone_rest_global = root_rest * Quat::from_rotation_x(0.2);
        let bone_rest_local = Quat::from_rotation_x(0.2);

        let output =
            apply_model_delta_to_bone(Quat::IDENTITY, root_rest, bone_rest_global, bone_rest_local);

        assert!(
            approx_eq(output, bone_rest_local),
            "neutral model delta should produce bone rest local rotation"
        );
    }
}
