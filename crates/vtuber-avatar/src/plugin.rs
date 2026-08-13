//! `VtuberAvatarPlugin` and top-level Bevy system registration.
//!
//! This is the only plugin that wires `bevy_vrm1` systems together with the
//! VTuber lifecycle domain. `bevy_vrm1` types are used internally and are not
//! re-exported from the crate facade.

use bevy::app::AnimationSystems;
use bevy::prelude::*;
use bevy_vrm1::prelude::*;
use bevy_vrm1::vrm::body_tracking::apply_direct_body_tracking;

use crate::bind::observe_initialized;
use crate::binding::bind_humanoid_bones;
use crate::expression::apply_tracked_expressions;
use crate::framing::{AvatarViewportCamera, frame_avatar_camera};
use crate::gaze::update_direct_look_at_input;
use crate::lifecycle::{
    AvatarLifecycle, LoadAvatarRequest, LoadAvatarResult, ReplaceAvatarRequest,
    ReplaceAvatarResult, UnloadAvatarRequest, UnloadAvatarResult, apply_avatar_request_events,
};
use crate::load::{
    LoadImportedAvatarRequest, LoadImportedAvatarResult, handle_load_imported_avatar_requests,
};
use crate::mirror::AvatarMotionMirror;
use crate::pose::{
    PoseApplyMetrics, reset_pose_metrics_on_lifecycle_change, update_body_tracking_pose_input,
};
use crate::unload::{
    ActiveControlFrame, clear_control_cache_on_lifecycle_change, despawn_unloading_avatar,
};

/// Plugin that sets up the VRM avatar scene, lifecycle, and diagnostics.
#[derive(Default)]
pub struct VtuberAvatarPlugin;

impl Plugin for VtuberAvatarPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(VrmPlugin)
            .add_plugins(crate::compatibility::VrmCompatibilityPlugin)
            .init_resource::<AvatarLifecycle>()
            .init_resource::<ActiveControlFrame>()
            .init_resource::<AvatarMotionMirror>()
            .init_resource::<PoseApplyMetrics>()
            .add_message::<LoadAvatarRequest>()
            .add_message::<LoadAvatarResult>()
            .add_message::<UnloadAvatarRequest>()
            .add_message::<UnloadAvatarResult>()
            .add_message::<ReplaceAvatarRequest>()
            .add_message::<ReplaceAvatarResult>()
            .add_message::<LoadImportedAvatarRequest>()
            .add_message::<LoadImportedAvatarResult>()
            .add_systems(Startup, setup_scene)
            .add_systems(
                Update,
                (
                    handle_load_imported_avatar_requests,
                    apply_avatar_request_events,
                    despawn_unloading_avatar,
                    observe_initialized,
                    bind_humanoid_bones,
                )
                    .chain(),
            )
            .add_systems(Update, clear_control_cache_on_lifecycle_change)
            .add_systems(Update, log_loaded_vrm)
            .add_systems(Update, log_head_bone)
            .add_systems(
                PostUpdate,
                frame_avatar_camera.after(TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                update_body_tracking_pose_input
                    .after(AnimationSystems)
                    .before(apply_direct_body_tracking)
                    .before(VrmSystemSets::Constraints),
            )
            .add_systems(
                PostUpdate,
                update_direct_look_at_input
                    .after(apply_direct_body_tracking)
                    .before(VrmSystemSets::GazeControl),
            )
            .add_systems(
                PostUpdate,
                apply_tracked_expressions
                    .after(VrmSystemSets::GazeControl)
                    .before(VrmSystemSets::Expressions),
            )
            .add_systems(Update, reset_pose_metrics_on_lifecycle_change);
    }
}

/// Command-line / environment path to the VRM model to load.
///
/// This resource is retained for backwards compatibility with the desktop
/// entry point. Startup model loading will be migrated to the lifecycle
/// request flow in a later subtask.
#[derive(Resource, Debug, Clone, Default)]
pub struct StartupModelPath(pub Option<String>);

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground plane for visual reference.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.3, 0.35),
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
    ));

    // Key light.
    commands.spawn((
        DirectionalLight {
            illuminance: 1500.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
    ));

    // Camera framing the upper body.
    commands.spawn((
        Camera3d::default(),
        AvatarViewportCamera,
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.5))
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));
}

fn log_loaded_vrm(vrms: Query<Entity, Added<Vrm>>) {
    for entity in vrms.iter() {
        info!("VRM runtime attached to root: {entity:?}");
    }
}

fn log_head_bone(heads: Query<Entity, Added<HeadBoneEntity>>) {
    for entity in heads.iter() {
        info!("Head bone capability found: {:?}", entity);
    }
}
