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
    assert_eq!(binding.left_eye, Some(left_eye));
    assert_eq!(binding.right_eye, Some(right_eye));
}

#[test]
fn humanoid_binding_no_repeated_lookup_after_ready() {
    let mut app = test_app();
    let head = spawn_bone(&mut app);
    let root = app
        .world_mut()
        .spawn((ActiveAvatar, BindTriggered, HeadBoneEntity(head)))
        .id();

    enter_binding(&mut app, root);
    app.update();
    assert_eq!(
        app.world().resource::<AvatarLifecycle>().state(),
        AvatarLifecycleState::Ready
    );

    let before = app.world().get::<AvatarBinding>(root).copied();

    // A second update must not invalidate the ready state or the cached binding.
    app.update();

    let lifecycle = app.world().resource::<AvatarLifecycle>();
    assert_eq!(lifecycle.state(), AvatarLifecycleState::Ready);
    assert_eq!(
        app.world().get::<AvatarBinding>(root),
        before.as_ref(),
        "AvatarBinding should not change after the avatar is ready"
    );
}
