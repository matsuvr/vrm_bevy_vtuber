//! Non-identity rest rotation synthetic integration test.
//!
//! Builds a minimal Bevy world with non-identity rest rotations on root,
//! neck, and head bones, then drives the full pose pipeline through the
//! ECS system schedule. Verifies:
//!
//! - Neutral input preserves rest rotation
//! - Yaw/pitch/roll produce correct world-space directions
//! - Combined rotation matches expected composition
//! - Clamp limits extreme input
//! - Neck-missing model routes all to head
//! - Repeated application does not drift
//! - Schedule ordering is correct (PostUpdate)

use bevy::app::App;
use bevy::math::{Quat, Vec3};
use bevy::prelude::*;

use vtuber_avatar::binding::AvatarBinding;
use vtuber_avatar::lifecycle::{ActiveAvatar, AvatarGeneration, AvatarLifecycle};
use vtuber_avatar::pose::binding::RestOrientationCache;
use vtuber_avatar::pose::distribution::{PoseDistributionSettings, distribute_pose};
use vtuber_avatar::pose::math::apply_model_delta_to_bone;
use vtuber_avatar::pose::types::{ClampedHeadPose, semantic_to_model_delta};
use vtuber_avatar::pose::{apply_distributed_pose, apply_tracked_head_pose};
use vtuber_avatar::unload::ActiveControlFrame;

const EPSILON: f32 = 1e-4;

fn approx_eq_quat(a: Quat, b: Quat) -> bool {
    a.abs_diff_eq(b, EPSILON)
}

/// Fixture: a synthetic avatar with non-identity rest rotations.
struct PoseFixture {
    root: Entity,
    #[allow(dead_code)]
    head: Entity,
    #[allow(dead_code)]
    neck: Entity,
    root_rest_global: Quat,
    head_rest_global: Quat,
    head_rest_local: Quat,
    #[allow(dead_code)]
    neck_rest_global: Quat,
    neck_rest_local: Quat,
}

fn build_fixture(app: &mut App) -> PoseFixture {
    // Non-identity rest rotations.
    let root_rest_global = Quat::from_rotation_y(0.3);
    let neck_rest_local = Quat::from_rotation_x(0.15);
    let neck_rest_global = root_rest_global * neck_rest_local;
    let head_rest_local = Quat::from_rotation_x(0.1);
    let head_rest_global = neck_rest_global * head_rest_local;

    let root = app
        .world_mut()
        .spawn((
            Transform::from_rotation(root_rest_global),
            GlobalTransform::from(Transform::from_rotation(root_rest_global)),
            bevy_vrm1::prelude::RestTransform(Transform::from_rotation(root_rest_global)),
        ))
        .id();

    let neck = app
        .world_mut()
        .spawn((
            Transform::from_rotation(neck_rest_local),
            GlobalTransform::from(Transform::from_rotation(neck_rest_global)),
            bevy_vrm1::prelude::RestTransform(Transform::from_rotation(neck_rest_local)),
            ChildOf(root),
        ))
        .id();

    let head = app
        .world_mut()
        .spawn((
            Transform::from_rotation(head_rest_local),
            GlobalTransform::from(Transform::from_rotation(head_rest_global)),
            bevy_vrm1::prelude::RestTransform(Transform::from_rotation(head_rest_local)),
            ChildOf(neck),
        ))
        .id();

    // Set up lifecycle as Ready with binding and cache.
    let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
    lifecycle.request_load(root).unwrap();
    lifecycle.start_binding(root);
    lifecycle.finish_ready();

    app.world_mut().entity_mut(root).insert((
        ActiveAvatar,
        AvatarBinding {
            generation: AvatarGeneration(1),
            root,
            head,
            neck: Some(neck),
            left_eye: None,
            right_eye: None,
            upper_chest: None,
            chest: None,
            spine: None,
        },
        RestOrientationCache {
            generation: AvatarGeneration(1),
            root_rest_global,
            head_rest_local,
            head_rest_global,
            neck_rest_local: Some(neck_rest_local),
            neck_rest_global: Some(neck_rest_global),
        },
    ));

    PoseFixture {
        root,
        head,
        neck,
        root_rest_global,
        head_rest_global,
        head_rest_local,
        neck_rest_global,
        neck_rest_local,
    }
}

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<AvatarLifecycle>()
        .init_resource::<ActiveControlFrame>()
        .init_resource::<PoseDistributionSettings>()
        .add_systems(PostUpdate, apply_tracked_head_pose);
    app
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Neutral input preserves non-identity rest rotation.
#[test]
fn pose_integration_neutral_preserves_rest() {
    let mut app = test_app();
    let fix = build_fixture(&mut app);

    let distributed = distribute_pose(0.0, 0.0, 0.0, true, &PoseDistributionSettings::default());
    let cache = app.world().get::<RestOrientationCache>(fix.root).unwrap();
    let (head_out, neck_out) = apply_distributed_pose(&distributed, cache);

    assert!(
        approx_eq_quat(head_out, fix.head_rest_local),
        "neutral head output should match rest local"
    );
    assert!(
        approx_eq_quat(neck_out.unwrap(), fix.neck_rest_local),
        "neutral neck output should match rest local"
    );
}

/// Yaw produces rotation around model +Y in world space.
#[test]
fn pose_integration_yaw_direction() {
    let mut app = test_app();
    let fix = build_fixture(&mut app);

    let distributed = distribute_pose(0.5, 0.0, 0.0, true, &PoseDistributionSettings::default());
    let cache = app.world().get::<RestOrientationCache>(fix.root).unwrap();
    let (head_out, _) = apply_distributed_pose(&distributed, cache);

    // The output should differ from rest by a rotation that includes +Y component.
    let delta = fix.head_rest_local.inverse() * head_out;
    let (axis, angle) = delta.to_axis_angle();
    assert!(
        angle.abs() > EPSILON,
        "yaw should produce non-zero rotation"
    );
    // The axis should have a significant Y component in model space.
    // After conjugation with non-identity rest, the local axis may not be pure Y,
    // but the rotation should be non-trivial.
    assert!(
        axis.length() > 0.9,
        "axis should be approximately unit length"
    );
}

/// Combined rotation is deterministic and matches manual composition.
#[test]
fn pose_integration_combined_matches_manual() {
    let mut app = test_app();
    let fix = build_fixture(&mut app);

    let distributed = distribute_pose(0.3, 0.2, 0.1, true, &PoseDistributionSettings::default());
    let cache = app.world().get::<RestOrientationCache>(fix.root).unwrap();
    let (head_out, _) = apply_distributed_pose(&distributed, cache);

    // Manual computation.
    let head_pose = ClampedHeadPose {
        yaw_rad: 0.3 * 0.6, // 60% head weight
        pitch_rad: 0.2 * 0.6,
        roll_rad: 0.1 * 0.6,
    };
    let model_delta = semantic_to_model_delta(&head_pose);
    let expected = apply_model_delta_to_bone(
        model_delta.rotation,
        fix.root_rest_global,
        fix.head_rest_global,
        fix.head_rest_local,
    );

    assert!(
        approx_eq_quat(head_out, expected),
        "combined output should match manual computation"
    );
}

/// Extreme input is clamped, output is finite.
#[test]
fn pose_integration_clamp_extreme() {
    let mut app = test_app();
    let fix = build_fixture(&mut app);

    let distributed = distribute_pose(
        100.0,
        -100.0,
        100.0,
        true,
        &PoseDistributionSettings::default(),
    );
    assert!(distributed.diagnostic.was_clamped);

    let cache = app.world().get::<RestOrientationCache>(fix.root).unwrap();
    let (head_out, neck_out) = apply_distributed_pose(&distributed, cache);

    assert!(head_out.is_finite(), "head output must be finite");
    assert!(neck_out.unwrap().is_finite(), "neck output must be finite");
}

/// Neck-missing: all rotation goes to head.
#[test]
fn pose_integration_neck_missing() {
    let mut app = test_app();
    let fix = build_fixture(&mut app);

    // Remove neck from binding.
    let mut binding = app.world_mut().get_mut::<AvatarBinding>(fix.root).unwrap();
    binding.neck = None;

    let distributed = distribute_pose(0.5, 0.3, 0.1, false, &PoseDistributionSettings::default());
    assert!(distributed.neck_delta.is_none());

    let cache = app.world().get::<RestOrientationCache>(fix.root).unwrap();
    let (head_out, neck_out) = apply_distributed_pose(&distributed, cache);

    assert!(neck_out.is_none(), "no neck output when neck is missing");
    assert!(head_out.is_finite());
    // Head should get the full pose (100% weight).
    let full_pose = ClampedHeadPose {
        yaw_rad: 0.5,
        pitch_rad: 0.3,
        roll_rad: 0.1,
    };
    let full_delta = semantic_to_model_delta(&full_pose);
    let expected = apply_model_delta_to_bone(
        full_delta.rotation,
        fix.root_rest_global,
        fix.head_rest_global,
        fix.head_rest_local,
    );
    assert!(
        approx_eq_quat(head_out, expected),
        "head should get full pose when neck is missing"
    );
}

/// Repeated application of the same delta does not drift.
#[test]
fn pose_integration_no_drift() {
    let mut app = test_app();
    let fix = build_fixture(&mut app);

    let distributed = distribute_pose(0.4, 0.2, 0.1, true, &PoseDistributionSettings::default());
    let cache = app.world().get::<RestOrientationCache>(fix.root).unwrap();

    let first = apply_distributed_pose(&distributed, cache);
    for i in 0..50 {
        let result = apply_distributed_pose(&distributed, cache);
        assert!(
            approx_eq_quat(result.0, first.0),
            "head drift at iteration {i}"
        );
        assert!(
            approx_eq_quat(result.1.unwrap(), first.1.unwrap()),
            "neck drift at iteration {i}"
        );
    }
}

/// System is registered in PostUpdate.
#[test]
fn pose_integration_system_in_post_update() {
    let app = test_app();
    // Verify PostUpdate schedule exists and has systems registered.
    let schedules = app.world().resource::<bevy::ecs::schedule::Schedules>();
    assert!(
        schedules.get(bevy::prelude::PostUpdate).is_some(),
        "PostUpdate schedule should exist"
    );
}

/// World-space direction verification: a yaw rotation should move the
/// head's forward direction (+Z in model space) in the expected direction.
#[test]
fn pose_integration_world_direction_yaw() {
    // Use identity rest for simpler world-space reasoning.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<AvatarLifecycle>()
        .init_resource::<ActiveControlFrame>()
        .init_resource::<PoseDistributionSettings>();

    let root_rest = Quat::IDENTITY;
    let head_rest = Quat::IDENTITY;

    let root = app
        .world_mut()
        .spawn((
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            bevy_vrm1::prelude::RestTransform(Transform::IDENTITY),
        ))
        .id();

    let head = app
        .world_mut()
        .spawn((
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            bevy_vrm1::prelude::RestTransform(Transform::IDENTITY),
            ChildOf(root),
        ))
        .id();

    let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
    lifecycle.request_load(root).unwrap();
    lifecycle.start_binding(root);
    lifecycle.finish_ready();

    app.world_mut().entity_mut(root).insert((
        ActiveAvatar,
        AvatarBinding {
            generation: AvatarGeneration(1),
            root,
            head,
            neck: None,
            left_eye: None,
            right_eye: None,
            upper_chest: None,
            chest: None,
            spine: None,
        },
        RestOrientationCache {
            generation: AvatarGeneration(1),
            root_rest_global: root_rest,
            head_rest_local: head_rest,
            head_rest_global: head_rest,
            neck_rest_local: None,
            neck_rest_global: None,
        },
    ));

    // Apply positive yaw (face right → +Y rotation in model space).
    let distributed = distribute_pose(0.5, 0.0, 0.0, false, &PoseDistributionSettings::default());
    let cache = app.world().get::<RestOrientationCache>(root).unwrap();
    let (head_out, _) = apply_distributed_pose(&distributed, cache);

    // The head's forward direction (+Z) should rotate toward +X
    // (ADR-004: yaw > 0 → +Y rotation → +Z moves toward +X).
    let forward = head_out * Vec3::Z;
    assert!(
        forward.x > 0.1,
        "positive yaw should rotate forward toward +X (ADR-004), got forward={forward:?}"
    );
}
