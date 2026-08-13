//! Integration tests for humanoid bone binding.

use bevy::prelude::*;
use bevy_vrm1::prelude::*;

use vtuber_avatar::bind::BindTriggered;
use vtuber_avatar::binding::{AvatarBinding, bind_humanoid_bones};
use vtuber_avatar::lifecycle::{ActiveAvatar, AvatarLifecycle, AvatarLifecycleState};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<AvatarLifecycle>()
        .add_systems(Update, bind_humanoid_bones);
    app
}

fn spawn_bone(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            RestTransform(Transform::IDENTITY),
            RestGlobalTransform(GlobalTransform::IDENTITY),
        ))
        .id()
}

fn enter_binding(app: &mut App, root: Entity) {
    let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
    lifecycle.request_load(root).unwrap();
    lifecycle.start_binding(root);
}

#[test]
fn humanoid_binding_head_only_ready() {
    let mut app = test_app();
    let head = spawn_bone(&mut app);
    let root = app
        .world_mut()
        .spawn((
            ActiveAvatar,
            BindTriggered,
            HeadBoneEntity(head),
            Visibility::Hidden,
        ))
        .id();

    enter_binding(&mut app, root);
    app.update();

    let lifecycle = app.world().resource::<AvatarLifecycle>();
    assert_eq!(lifecycle.state(), AvatarLifecycleState::Ready);
    assert_eq!(lifecycle.active_root(), Some(root));

    let binding = app
        .world()
        .get::<AvatarBinding>(root)
        .expect("AvatarBinding should be cached on the root");
    assert_eq!(binding.head, head);
    assert!(binding.neck.is_none());
    assert!(binding.upper_chest.is_none());
    assert!(binding.chest.is_none());
    assert!(binding.spine.is_none());
    assert!(binding.left_upper_arm.is_none());
    assert!(binding.right_upper_arm.is_none());
    assert!(binding.left_eye.is_none());
    assert!(binding.right_eye.is_none());
    assert!(app.world().get::<BodyTracking>(root).is_some());
    assert!(app.world().get::<BodyTrackingPoseInput>(root).is_some());
    assert!(app.world().get::<BodyTrackingProfile>(root).is_some());

    assert_eq!(
        app.world().get::<Visibility>(root),
        Some(&Visibility::Inherited)
    );
    let capabilities = app
        .world()
        .resource::<AvatarLifecycle>()
        .capabilities()
        .expect("successful binding should publish capabilities");
    assert!(capabilities.bones.head);
    assert!(!capabilities.bones.neck);
}

#[test]
fn humanoid_binding_waits_for_rest_global_transform() {
    let mut app = test_app();
    let head = app
        .world_mut()
        .spawn((Transform::IDENTITY, RestTransform(Transform::IDENTITY)))
        .id();
    app.world_mut().entity_mut(head).remove::<GlobalTransform>();
    let root = app
        .world_mut()
        .spawn((ActiveAvatar, BindTriggered, HeadBoneEntity(head)))
        .id();

    enter_binding(&mut app, root);
    app.update();
    assert_eq!(
        app.world().resource::<AvatarLifecycle>().state(),
        AvatarLifecycleState::Binding
    );
    assert!(app.world().get::<AvatarBinding>(root).is_none());

    app.world_mut()
        .entity_mut(head)
        .insert(RestGlobalTransform(GlobalTransform::IDENTITY));
    app.update();

    assert_eq!(
        app.world().resource::<AvatarLifecycle>().state(),
        AvatarLifecycleState::Ready
    );
    assert!(app.world().get::<BodyTrackingPoseInput>(root).is_some());
}

#[test]
fn humanoid_binding_missing_head_fails() {
    let mut app = test_app();
    let root = app.world_mut().spawn((ActiveAvatar, BindTriggered)).id();

    enter_binding(&mut app, root);
    app.update();

    let lifecycle = app.world().resource::<AvatarLifecycle>();
    assert_eq!(lifecycle.state(), AvatarLifecycleState::Failed);
    assert!(lifecycle.active_root().is_none());
    assert!(!app.world().entity(root).contains::<ActiveAvatar>());
    assert!(
        app.world().get::<AvatarBinding>(root).is_none(),
        "no binding should be cached when the head is missing"
    );
}

#[test]
fn humanoid_binding_despawned_head_fails() {
    let mut app = test_app();
    let head = spawn_bone(&mut app);
    let root = app
        .world_mut()
        .spawn((ActiveAvatar, BindTriggered, HeadBoneEntity(head)))
        .id();

    enter_binding(&mut app, root);
    app.world_mut().entity_mut(head).despawn();
    app.update();

    let lifecycle = app.world().resource::<AvatarLifecycle>();
    assert_eq!(lifecycle.state(), AvatarLifecycleState::Failed);
    assert!(lifecycle.active_root().is_none());
}

#[test]
fn humanoid_binding_missing_rest_transform_fails() {
    let mut app = test_app();
    let head = app.world_mut().spawn(Transform::IDENTITY).id();
    let root = app
        .world_mut()
        .spawn((ActiveAvatar, BindTriggered, HeadBoneEntity(head)))
        .id();

    enter_binding(&mut app, root);
    app.update();

    let lifecycle = app.world().resource::<AvatarLifecycle>();
    assert_eq!(lifecycle.state(), AvatarLifecycleState::Failed);
}

#[test]
fn humanoid_binding_optional_bones_cached() {
    let mut app = test_app();
    let head = spawn_bone(&mut app);
    let neck = spawn_bone(&mut app);
    let upper_chest = spawn_bone(&mut app);
    let chest = spawn_bone(&mut app);
    let spine = spawn_bone(&mut app);
    let left_upper_arm = spawn_bone(&mut app);
    let right_upper_arm = spawn_bone(&mut app);
    let left_rest_rotation = Quat::from_rotation_x(0.13);
    let right_rest_rotation = Quat::from_rotation_x(-0.17);
    app.world_mut().entity_mut(left_upper_arm).insert((
        Transform::from_rotation(Quat::from_rotation_y(0.4)),
        RestTransform(Transform::from_rotation(left_rest_rotation)),
    ));
    app.world_mut().entity_mut(right_upper_arm).insert((
        Transform::from_rotation(Quat::from_rotation_y(-0.3)),
        RestTransform(Transform::from_rotation(right_rest_rotation)),
    ));
    let left_eye = spawn_bone(&mut app);
    let right_eye = spawn_bone(&mut app);

    let root = app
        .world_mut()
        .spawn((
            ActiveAvatar,
            BindTriggered,
            HeadBoneEntity(head),
            NeckBoneEntity(neck),
            UpperChestBoneEntity(upper_chest),
            ChestBoneEntity(chest),
            SpineBoneEntity(spine),
            LeftUpperArmBoneEntity(left_upper_arm),
            RightUpperArmBoneEntity(right_upper_arm),
            LeftEyeBoneEntity(left_eye),
            RightEyeBoneEntity(right_eye),
        ))
        .id();

    enter_binding(&mut app, root);
    app.update();

    let lifecycle = app.world().resource::<AvatarLifecycle>();
    assert_eq!(lifecycle.state(), AvatarLifecycleState::Ready);

    let binding = app
        .world()
        .get::<AvatarBinding>(root)
        .expect("AvatarBinding should be cached");
    assert_eq!(binding.head, head);
    assert_eq!(binding.neck, Some(neck));
    assert_eq!(binding.upper_chest, Some(upper_chest));
    assert_eq!(binding.chest, Some(chest));
    assert_eq!(binding.spine, Some(spine));
    assert_eq!(binding.left_upper_arm, Some(left_upper_arm));
    assert_eq!(binding.right_upper_arm, Some(right_upper_arm));
    assert_eq!(binding.left_eye, Some(left_eye));
    assert_eq!(binding.right_eye, Some(right_eye));

    let left_rotation = app
        .world()
        .get::<Transform>(left_upper_arm)
        .unwrap()
        .rotation;
    let right_rotation = app
        .world()
        .get::<Transform>(right_upper_arm)
        .unwrap()
        .rotation;
    let left_rest = app
        .world()
        .get::<RestTransform>(left_upper_arm)
        .expect("upper arm rest transform remains model-authored");
    let right_rest = app
        .world()
        .get::<RestTransform>(right_upper_arm)
        .expect("upper arm rest transform remains model-authored");
    let relaxed_drop = 55.0_f32.to_radians();
    let expected_left = left_rest_rotation * Quat::from_rotation_z(-relaxed_drop);
    let expected_right = right_rest_rotation * Quat::from_rotation_z(relaxed_drop);
    assert!(
        left_rotation.dot(expected_left).abs() > 0.999_999,
        "left upper arm should use the rest rotation plus the downward offset: actual={left_rotation:?}"
    );
    assert!(
        right_rotation.dot(expected_right).abs() > 0.999_999,
        "right upper arm should use the rest rotation plus the downward offset: actual={right_rotation:?}"
    );
    assert!((left_rotation * Vec3::X).y < 0.0);
    assert!((right_rotation * -Vec3::X).y < 0.0);
    assert_eq!(left_rest.0.rotation, left_rest_rotation);
    assert_eq!(right_rest.0.rotation, right_rest_rotation);
}

#[test]
fn humanoid_binding_no_repeated_lookup_after_ready() {
    let mut app = test_app();
    let head = spawn_bone(&mut app);
    let left_upper_arm = spawn_bone(&mut app);
    let right_upper_arm = spawn_bone(&mut app);
    let root = app
        .world_mut()
        .spawn((
            ActiveAvatar,
            BindTriggered,
            HeadBoneEntity(head),
            LeftUpperArmBoneEntity(left_upper_arm),
            RightUpperArmBoneEntity(right_upper_arm),
        ))
        .id();

    enter_binding(&mut app, root);
    app.update();
    assert_eq!(
        app.world().resource::<AvatarLifecycle>().state(),
        AvatarLifecycleState::Ready
    );

    let before = app.world().get::<AvatarBinding>(root).copied();
    let replacement_rotation = Quat::from_rotation_y(0.2);
    app.world_mut()
        .get_mut::<Transform>(left_upper_arm)
        .expect("bound left upper arm remains available")
        .rotation = replacement_rotation;

    // A second update must not invalidate the ready state, reapply the default
    // arm delta, or change the cached binding.
    app.update();

    let lifecycle = app.world().resource::<AvatarLifecycle>();
    assert_eq!(lifecycle.state(), AvatarLifecycleState::Ready);
    assert_eq!(
        app.world().get::<AvatarBinding>(root),
        before.as_ref(),
        "AvatarBinding should not change after the avatar is ready"
    );
    assert_eq!(
        app.world()
            .get::<Transform>(left_upper_arm)
            .unwrap()
            .rotation,
        replacement_rotation,
        "the relaxed-arm default is a one-shot binding operation"
    );
}
