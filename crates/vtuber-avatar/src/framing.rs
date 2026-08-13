//! Avatar-aware viewport camera framing.

// This foundation is intentionally added before the camera integration in
// Issue #4, so its public(crate) API is unused until that follow-up lands.
#[allow(dead_code)]
pub(crate) mod head_subtree_bounds;

use bevy::prelude::*;
use bevy_vrm1::prelude::{HeadBoneEntity, HipsBoneEntity};

use crate::lifecycle::{AvatarGeneration, AvatarLifecycle, AvatarLifecycleState};

const DEFAULT_VERTICAL_FOV: f32 = std::f32::consts::FRAC_PI_4;
const TARGET_FROM_HIPS: f32 = 0.60;
const VERTICAL_HALF_EXTENT_IN_HIPS_HEADS: f32 = 0.75;
const MIN_HIPS_HEAD_HEIGHT: f32 = 0.05;

/// Marks the single camera used to render the avatar viewport.
#[derive(Component)]
pub(crate) struct AvatarViewportCamera;

/// Frames a newly-ready avatar once, keeping live head motion visible instead
/// of making the camera follow and cancel it.
pub(crate) fn frame_avatar_camera(
    lifecycle: Res<AvatarLifecycle>,
    roots: Query<(&HeadBoneEntity, &HipsBoneEntity)>,
    bones: Query<&GlobalTransform>,
    mut cameras: Query<(&mut Transform, &Projection), With<AvatarViewportCamera>>,
    mut framed_generation: Local<Option<AvatarGeneration>>,
) {
    if lifecycle.state() != AvatarLifecycleState::Ready {
        return;
    }
    let generation = lifecycle.current_generation();
    if *framed_generation == Some(generation) {
        return;
    }
    let Some(root) = lifecycle.active_root() else {
        return;
    };
    let Ok((head_entity, hips_entity)) = roots.get(root) else {
        return;
    };
    let (Ok(head), Ok(hips)) = (bones.get(**head_entity), bones.get(**hips_entity)) else {
        return;
    };

    let mut framed = false;
    for (mut camera, projection) in &mut cameras {
        let vertical_fov = match projection {
            Projection::Perspective(perspective) => perspective.fov,
            _ => DEFAULT_VERTICAL_FOV,
        };
        if let Some(transform) =
            upper_body_camera_transform(head.translation(), hips.translation(), vertical_fov)
        {
            *camera = transform;
            framed = true;
        }
    }
    if framed {
        *framed_generation = Some(generation);
    }
}

fn upper_body_camera_transform(head: Vec3, hips: Vec3, vertical_fov: f32) -> Option<Transform> {
    if !head.is_finite() || !hips.is_finite() || !vertical_fov.is_finite() {
        return None;
    }
    let hips_head_height = head.y - hips.y;
    if hips_head_height < MIN_HIPS_HEAD_HEIGHT
        || !(0.1..std::f32::consts::PI - 0.1).contains(&vertical_fov)
    {
        return None;
    }

    // With this target and extent, the head is at about the upper quarter and
    // the hips at about the lower tenth of the viewport.
    let target = hips.lerp(head, TARGET_FROM_HIPS);
    let half_extent = hips_head_height * VERTICAL_HALF_EXTENT_IN_HIPS_HEADS;
    let distance = half_extent / (vertical_fov * 0.5).tan();
    if !distance.is_finite() || distance <= 0.0 {
        return None;
    }
    Some(Transform::from_translation(target + Vec3::Z * distance).looking_at(target, Vec3::Y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_humanoid(app: &mut App, x: f32) -> Entity {
        let hips = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(x, 1.0, 0.0)))
            .id();
        let head = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(x, 1.8, 0.0)))
            .id();
        app.world_mut()
            .spawn((HeadBoneEntity(head), HipsBoneEntity(hips)))
            .id()
    }

    fn make_ready(app: &mut App, root: Entity) {
        let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
        lifecycle.request_load(root).expect("load from empty state");
        lifecycle.start_binding(root);
        lifecycle.finish_ready();
    }

    #[test]
    fn framing_places_head_high_and_hips_low() {
        let hips = Vec3::new(0.0, 1.0, 0.0);
        let head = Vec3::new(0.0, 1.8, 0.0);
        let transform = upper_body_camera_transform(head, hips, DEFAULT_VERTICAL_FOV)
            .expect("valid humanoid framing should be computed");
        let target_y = hips.lerp(head, TARGET_FROM_HIPS).y;
        let projected_half_extent = transform.translation.z * (DEFAULT_VERTICAL_FOV * 0.5).tan();
        let head_screen_y = 0.5 - (head.y - target_y) / projected_half_extent * 0.5;
        let hips_screen_y = 0.5 - (hips.y - target_y) / projected_half_extent * 0.5;

        assert!((0.15..0.35).contains(&head_screen_y), "{head_screen_y}");
        assert!((0.80..0.95).contains(&hips_screen_y), "{hips_screen_y}");
    }

    #[test]
    fn framing_centres_model_world_offset() {
        let hips = Vec3::new(2.0, 0.8, -3.0);
        let head = Vec3::new(2.2, 1.8, -2.8);
        let transform = upper_body_camera_transform(head, hips, DEFAULT_VERTICAL_FOV)
            .expect("valid offset humanoid framing should be computed");
        let target = hips.lerp(head, TARGET_FROM_HIPS);

        assert!((transform.translation.x - target.x).abs() < 1e-6);
        assert!(transform.translation.z > target.z);
        assert!(
            transform
                .forward()
                .dot((target - transform.translation).normalize())
                > 0.999
        );
    }

    #[test]
    fn invalid_bone_geometry_uses_existing_fixed_camera() {
        assert!(
            upper_body_camera_transform(Vec3::ZERO, Vec3::ZERO, DEFAULT_VERTICAL_FOV).is_none()
        );
        assert!(
            upper_body_camera_transform(Vec3::splat(f32::NAN), Vec3::ZERO, DEFAULT_VERTICAL_FOV)
                .is_none()
        );
    }

    #[test]
    fn replacement_generation_is_reframed() {
        let mut app = App::new();
        app.init_resource::<AvatarLifecycle>()
            .add_systems(Update, frame_avatar_camera);
        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                AvatarViewportCamera,
                Transform::from_xyz(0.0, 0.0, 2.5),
            ))
            .id();
        let first = spawn_humanoid(&mut app, 0.0);
        make_ready(&mut app, first);

        app.update();
        let first_x = app.world().get::<Transform>(camera).unwrap().translation.x;

        let replacement = spawn_humanoid(&mut app, 3.0);
        {
            let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
            lifecycle
                .request_replace(replacement)
                .expect("replace from ready state");
            lifecycle.finish_unload();
            lifecycle.start_binding(replacement);
            lifecycle.finish_ready();
        }
        app.update();
        let replacement_x = app.world().get::<Transform>(camera).unwrap().translation.x;

        assert!(first_x.abs() < 1e-6);
        assert!((replacement_x - 3.0).abs() < 1e-6);
    }
}
