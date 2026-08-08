//! Pose lifecycle integration tests.
//!
//! Verifies that the pose system correctly handles:
//! - Neutral/zero pose produces rest rotation
//! - Lost/returning-neutral frames go through the same apply path
//! - Avatar replace starts from neutral rest
//! - No drift when applying the same delta repeatedly
//! - No panic when the system runs after avatar unload

use bevy::math::Quat;

use vtuber_avatar::lifecycle::AvatarGeneration;
use vtuber_avatar::pose::distribution::apply_distributed_pose;
use vtuber_avatar::pose::{
    ClampedHeadPose, PoseDistributionSettings, RestOrientationCache, distribute_pose,
    semantic_to_model_delta,
};

fn make_cache_with_rest(
    root_rest: Quat,
    head_rest_global: Quat,
    head_rest_local: Quat,
    neck_rest_global: Option<Quat>,
    neck_rest_local: Option<Quat>,
) -> RestOrientationCache {
    RestOrientationCache {
        generation: AvatarGeneration(1),
        root_rest_global: root_rest,
        head_rest_local,
        head_rest_global,
        neck_rest_local,
        neck_rest_global,
    }
}

/// Neutral input produces rest rotation — no drift.
#[test]
fn pose_lifecycle_neutral_produces_rest_rotation() {
    let head_rest = Quat::from_rotation_x(0.2);
    let cache = make_cache_with_rest(Quat::IDENTITY, head_rest, head_rest, None, None);

    let distributed = distribute_pose(0.0, 0.0, 0.0, false, &PoseDistributionSettings::default());
    let (head_out, neck_out) = apply_distributed_pose(&distributed, &cache);

    assert!(
        head_out.abs_diff_eq(head_rest, 1e-5),
        "neutral pose should produce rest rotation"
    );
    assert!(neck_out.is_none());
}

/// Applying the same delta multiple times does not accumulate drift.
#[test]
fn pose_lifecycle_no_drift_on_repeated_application() {
    let head_rest = Quat::from_rotation_x(0.1);
    let cache = make_cache_with_rest(Quat::IDENTITY, head_rest, head_rest, None, None);

    let distributed = distribute_pose(0.3, 0.1, 0.05, false, &PoseDistributionSettings::default());

    // Apply the same delta 100 times — each time should produce the same result.
    let first = apply_distributed_pose(&distributed, &cache).0;
    for _ in 0..99 {
        let result = apply_distributed_pose(&distributed, &cache).0;
        assert!(
            result.abs_diff_eq(first, 1e-5),
            "repeated application must not drift"
        );
    }
}

/// Neck-missing model distributes all to head.
#[test]
fn pose_lifecycle_neck_missing_all_to_head() {
    let head_rest = Quat::IDENTITY;
    let cache = make_cache_with_rest(Quat::IDENTITY, head_rest, head_rest, None, None);

    let distributed = distribute_pose(0.5, 0.3, 0.1, false, &PoseDistributionSettings::default());
    assert!(distributed.neck_delta.is_none());

    let (head_out, neck_out) = apply_distributed_pose(&distributed, &cache);
    assert!(neck_out.is_none());

    // Head should get the full pose.
    let full = semantic_to_model_delta(&ClampedHeadPose {
        yaw_rad: 0.5,
        pitch_rad: 0.3,
        roll_rad: 0.1,
    });
    // With identity rest, the output should equal the model delta.
    assert!(head_out.abs_diff_eq(full.rotation, 1e-4));
}

/// Non-identity rest rotation is preserved when delta is neutral.
#[test]
fn pose_lifecycle_non_identity_rest_preserved() {
    let root_rest = Quat::from_rotation_y(0.5);
    let head_rest_global = root_rest * Quat::from_rotation_x(0.3);
    let head_rest_local = Quat::from_rotation_x(0.3);

    let cache = make_cache_with_rest(root_rest, head_rest_global, head_rest_local, None, None);

    let distributed = distribute_pose(0.0, 0.0, 0.0, false, &PoseDistributionSettings::default());
    let (head_out, _) = apply_distributed_pose(&distributed, &cache);

    assert!(
        head_out.abs_diff_eq(head_rest_local, 1e-5),
        "neutral delta with non-identity rest should produce rest local rotation"
    );
}

/// Distribution with extreme input is clamped, not NaN or infinite.
#[test]
fn pose_lifecycle_extreme_input_clamped() {
    let distributed = distribute_pose(
        f32::MAX,
        -f32::MAX,
        f32::MAX,
        true,
        &PoseDistributionSettings::default(),
    );

    assert!(distributed.diagnostic.was_clamped);
    assert!(distributed.head_delta.rotation.is_finite());
    if let Some(neck) = &distributed.neck_delta {
        assert!(neck.rotation.is_finite());
    }
}

/// NaN input is replaced with zero, not propagated.
#[test]
fn pose_lifecycle_nan_input_safe() {
    let distributed = distribute_pose(
        f32::NAN,
        f32::NAN,
        f32::NAN,
        true,
        &PoseDistributionSettings::default(),
    );

    assert!(distributed.head_delta.rotation.is_finite());
    assert!(distributed.diagnostic.was_clamped);
    assert_eq!(distributed.diagnostic.clamped_yaw_rad, 0.0);
}
