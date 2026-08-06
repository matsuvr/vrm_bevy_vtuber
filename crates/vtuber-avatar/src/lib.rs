//! `vtuber-avatar`: Bevy and `bevy_vrm1` adapter.
//!
//! This is the only crate that interacts with Bevy entities and `bevy_vrm1` APIs.
//! `bevy_vrm1` types must not leak into `vtuber-core` or `vtuber-tracking`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use bevy::prelude::*;
use bevy_vrm1::prelude::*;

pub mod compatibility;

/// Plugin that sets up the VRM avatar scene and diagnostics.
#[derive(Default)]
pub struct VtuberAvatarPlugin;

impl Plugin for VtuberAvatarPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(VrmPlugin)
            .add_plugins(compatibility::VrmCompatibilityPlugin)
            .add_systems(Startup, setup_scene)
            .add_systems(Update, log_loaded_vrm)
            .add_systems(Update, log_head_bone);
    }
}

/// Command-line / environment path to the VRM model to load.
#[derive(Resource, Debug, Clone, Default)]
pub struct StartupModelPath(pub Option<String>);

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    startup: Option<Res<StartupModelPath>>,
    asset_server: Res<AssetServer>,
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
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.5))
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    // Load the requested model, or nothing if no path was supplied.
    if let Some(path) = startup.and_then(|p| p.0.clone()) {
        commands.spawn((VrmHandle(asset_server.load(path)),));
    }
}

fn log_loaded_vrm(vrms: Query<(Entity, &VrmHandle), Added<Vrm>>) {
    for (entity, _) in vrms.iter() {
        info!("VRM entity loaded: {:?}", entity);
    }
}

fn log_head_bone(heads: Query<Entity, Added<HeadBoneEntity>>) {
    for entity in heads.iter() {
        info!("Head bone capability found: {:?}", entity);
    }
}

/// Placeholder module retained for the empty baseline.
pub mod placeholder;
