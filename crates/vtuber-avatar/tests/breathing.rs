//! Always-on procedural breathing integration tests.
//!
//! These tests cover transform composition, animation-base detection,
//! lifecycle independence, replacement cleanup, frame-rate equivalence, and
//! the same-frame interaction with direct-pose body tracking.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimePlugin;
use bevy_vrm1::prelude::*;
use bevy_vrm1::vrm::body_tracking::apply_direct_body_tracking;
use vtuber_avatar::{
    ActiveAvatar, AvatarBinding, AvatarGeneration, AvatarLifecycle, BreathingBinding,
    BreathingProfile, BreathingState, apply_breathing_hips_translation, breathing_envelope,
    breathing_phase, resolve_breathing_amplitudes, resolve_breathing_binding,
};

const EPSILON: f32 = 1.0e-4;
/// A scene root -> intermediate -> hips (plus an optional spine -> head
/// chain) with immutable rest data, resolved breathing geometry, and a Ready
/// lifecycle. The app deliberately has no ActiveControlFrame resource and no
/// active BodyTracking input: breathing must advance regardless of tracking.
struct Scene {
    app: App,
    root: Entity,
    intermediate: Option<Entity>,
    ancestors: Vec<Entity>,
    hips: Entity,
    head: Entity,
    spine: Option<Entity>,
    rest_hips_global: GlobalTransform,
    vertical_amplitude: f32,
    forward_amplitude: f32,
    profile: BreathingProfile,
}

impl Scene {
    fn hips_translation(&self) -> Vec3 {
        self.app
            .world()
            .get::<Transform>(self.hips)
            .expect("hips has a Transform")
            .translation
    }

    fn hips_global(&self) -> GlobalTransform {
        *self
            .app
            .world()
            .get::<GlobalTransform>(self.hips)
            .expect("hips has a GlobalTransform")
    }
    /// Mirrors the system's own f64 phase accumulation for exact expectations.
    /// The system evaluates the envelope before advancing the accumulator.
    fn expected_delta(&self, dt: f64, frames: usize) -> Vec3 {
        let mut elapsed = 0.0;
        let mut breath = 0.0f64;
        for _ in 0..frames {
            breath = breathing_envelope(breathing_phase(elapsed, self.profile.period_seconds));
            elapsed += dt;
        }
        let breath = breath as f32;
        let binding = self
            .app
            .world()
            .get::<BreathingBinding>(self.root)
            .expect("root has BreathingBinding");
        binding.up_local * (self.vertical_amplitude * breath)
            + binding.forward_local * (self.forward_amplitude * breath)
    }

    /// Runs frames updates with the given frame time, accumulating phase the
    /// same way the system does.
    fn advance(&mut self, dt: Duration, frames: usize) {
        for _ in 0..frames {
            self.app.world_mut().resource_mut::<Time>().advance_by(dt);
            self.app.update();
        }
    }
}

fn ready_lifecycle(app: &mut App, root: Entity) -> AvatarGeneration {
    let mut lifecycle = AvatarLifecycle::new();
    lifecycle
        .request_load(root)
        .expect("fresh lifecycle accepts load");
    lifecycle.start_binding(root);
    lifecycle.finish_ready();
    let generation = lifecycle.current_generation();
    app.insert_resource(lifecycle);
    generation
}
fn spawn_transform(
    app: &mut App,
    transform: Transform,
    parent_global: GlobalTransform,
    parent: Option<Entity>,
) -> Entity {
    let global = parent_global.mul_transform(transform);
    let mut builder = app.world_mut().spawn((transform, global));
    if let Some(parent) = parent {
        builder.insert(ChildOf(parent));
    }
    builder.id()
}

#[allow(clippy::too_many_arguments)]
fn build_scene(
    root_rotation: Quat,
    intermediate_rotation: Option<Quat>,
    intermediate_translation: Vec3,
    hips_rest_rotation: Quat,
    base: Vec3,
    with_body_chain: bool,
) -> Scene {
    let ancestor_specs = intermediate_rotation
        .map(|rotation| (rotation, intermediate_translation))
        .into_iter()
        .collect::<Vec<_>>();
    build_scene_with_ancestors(
        root_rotation,
        &ancestor_specs,
        hips_rest_rotation,
        base,
        with_body_chain,
    )
}

fn build_scene_with_ancestors(
    root_rotation: Quat,
    ancestor_specs: &[(Quat, Vec3)],
    hips_rest_rotation: Quat,
    base: Vec3,
    with_body_chain: bool,
) -> Scene {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<TimePlugin>())
        .insert_resource(Time::<()>::default())
        .add_systems(PostUpdate, apply_breathing_hips_translation);

    let root_transform = Transform::from_rotation(root_rotation);
    let root = spawn_transform(&mut app, root_transform, GlobalTransform::IDENTITY, None);

    let mut ancestors = Vec::with_capacity(ancestor_specs.len());
    let mut hips_parent = root;
    let mut hips_parent_global = GlobalTransform::from(root_transform);
    for &(rotation, translation) in ancestor_specs {
        let transform = Transform {
            translation,
            rotation,
            scale: Vec3::ONE,
        };
        let ancestor = spawn_transform(&mut app, transform, hips_parent_global, Some(hips_parent));
        ancestors.push(ancestor);
        hips_parent = ancestor;
        hips_parent_global = hips_parent_global.mul_transform(transform);
    }

    let hips_rest_local = Transform {
        translation: base,
        rotation: hips_rest_rotation,
        scale: Vec3::ONE,
    };
    let rest_hips_global = hips_parent_global.mul_transform(hips_rest_local);
    let hips = spawn_transform(
        &mut app,
        hips_rest_local,
        hips_parent_global,
        Some(hips_parent),
    );
    app.world_mut().entity_mut(hips).insert((
        RestTransform(hips_rest_local),
        RestGlobalTransform(rest_hips_global),
    ));
    let (spine, head) = if with_body_chain {
        let spine_rest = Transform {
            translation: Vec3::new(0.0, 0.2, 0.0),
            ..Transform::IDENTITY
        };
        let spine_global = rest_hips_global.mul_transform(spine_rest);
        let spine = spawn_transform(&mut app, spine_rest, rest_hips_global, Some(hips));
        app.world_mut()
            .entity_mut(spine)
            .insert((RestTransform(spine_rest), RestGlobalTransform(spine_global)));
        let head_rest = Transform {
            translation: Vec3::new(0.0, 0.3, 0.0),
            ..Transform::IDENTITY
        };
        let head_global = spine_global.mul_transform(head_rest);
        let head = spawn_transform(&mut app, head_rest, spine_global, Some(spine));
        app.world_mut()
            .entity_mut(head)
            .insert((RestTransform(head_rest), RestGlobalTransform(head_global)));
        (Some(spine), head)
    } else {
        let head = app.world_mut().spawn_empty().id();
        (None, head)
    };

    app.world_mut().entity_mut(root).insert((
        Vrm,
        BodyTracking::default(),
        BodyTrackingPoseInput::default(),
    ));
    if let Some(spine) = spine {
        app.world_mut()
            .entity_mut(root)
            .insert((HeadBoneEntity(head), SpineBoneEntity(spine)));
    }

    let profile = BreathingProfile::default();
    let ancestor_path = ancestors.iter().rev().copied().collect::<Vec<_>>();
    let binding = resolve_breathing_binding(
        AvatarGeneration(1),
        hips,
        &profile,
        Some(hips_rest_local),
        Some(rest_hips_global),
        ancestor_path,
    )
    .expect("scene rest data resolves");

    let generation = ready_lifecycle(&mut app, root);
    let binding = BreathingBinding {
        generation,
        ..binding
    };
    app.world_mut().entity_mut(root).insert((
        ActiveAvatar,
        AvatarBinding::head_only(root, head, generation),
        profile,
        binding,
        BreathingState::default(),
    ));

    let (vertical_amplitude, forward_amplitude) =
        resolve_breathing_amplitudes(&profile, rest_hips_global.translation().y)
            .expect("scene hips height is valid");

    Scene {
        app,
        root,
        intermediate: ancestors.first().copied(),
        ancestors,
        hips,
        head,
        spine,
        rest_hips_global,
        vertical_amplitude,
        forward_amplitude,
        profile,
    }
}

fn identity_quat() -> Quat {
    Quat::IDENTITY
}

#[test]
fn invalid_profiles_are_safe_no_ops_at_both_resolution_boundaries() {
    let hips = Entity::from_raw_u32(1).expect("test entity index is valid");
    let rest_local = Transform::from_translation(Vec3::new(0.0, 1.0, 0.0));
    let rest_global = GlobalTransform::from(rest_local);
    let invalid_profiles = [
        (
            "reversed vertical bounds",
            BreathingProfile {
                vertical_min_meters: 1.0,
                vertical_max_meters: 0.0,
                ..BreathingProfile::default()
            },
        ),
        (
            "NaN lower bound",
            BreathingProfile {
                vertical_min_meters: f32::NAN,
                ..BreathingProfile::default()
            },
        ),
        (
            "NaN upper bound",
            BreathingProfile {
                forward_max_meters: f32::NAN,
                ..BreathingProfile::default()
            },
        ),
        (
            "NaN factor",
            BreathingProfile {
                vertical_height_factor: f32::NAN,
                ..BreathingProfile::default()
            },
        ),
        (
            "infinite factor",
            BreathingProfile {
                forward_height_factor: f32::INFINITY,
                ..BreathingProfile::default()
            },
        ),
    ];

    for (label, profile) in invalid_profiles {
        assert!(
            resolve_breathing_amplitudes(&profile, 1.0).is_none(),
            "{label}: amplitude resolution must be a safe no-op"
        );
        assert!(
            resolve_breathing_binding(
                AvatarGeneration(1),
                hips,
                &profile,
                Some(rest_local),
                Some(rest_global),
                Vec::new(),
            )
            .is_none(),
            "{label}: binding resolution must be a safe no-op"
        );
    }
}

// --- transform composition ---

#[test]
fn first_ready_frame_is_exactly_the_authored_base() {
    let base = Vec3::new(0.1, 1.0, 0.2);
    let mut scene = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    // The very first update advances phase 0 -> 0 with delta 0: no pop.
    scene.advance(Duration::from_secs_f64(1.0 / 60.0), 1);
    assert_eq!(scene.hips_translation(), base);
}

#[test]
fn breathing_never_accumulates_and_wraps_exactly_to_base() {
    let base = Vec3::new(0.1, 1.0, 0.2);
    let mut scene = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    // 1/64 s is exact in binary, so pre-evaluation elapsed hits exactly 5.0 s
    // at frame 321 (5.0 s) and 641 (10.0 s): the hips return exactly to base.
    let dt = 1.0 / 64.0;
    for frames in 1..=641 {
        scene.advance(Duration::from_secs_f64(dt), 1);
        let expected = base + scene.expected_delta(dt, frames);
        let actual = scene.hips_translation();
        assert!(
            (actual - expected).length() < 1.0e-6,
            "frame {frames}: {actual} vs {expected}"
        );
        if frames == 321 || frames == 641 {
            assert_eq!(
                actual, base,
                "frame {frames} is exactly at a cycle boundary and must equal the base"
            );
        }
    }
    let state = scene
        .app
        .world()
        .get::<BreathingState>(scene.root)
        .expect("state exists");
    assert_eq!(state.base(), Some(base));
    assert!((state.elapsed_seconds() - 641.0 * dt).abs() < 1.0e-6);
}

#[test]
fn animation_base_replacement_is_detected_and_preserved() {
    let base = Vec3::new(0.1, 1.0, 0.2);
    let mut scene = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    let dt = Duration::from_secs_f64(1.0 / 60.0);
    scene.advance(dt, 60);

    // Animation writes a completely new hips translation mid-cycle.
    let new_base = base + Vec3::new(0.2, 0.1, 0.3);
    scene
        .app
        .world_mut()
        .get_mut::<Transform>(scene.hips)
        .expect("hips transform")
        .translation = new_base;
    scene.advance(dt, 1);

    let expected = new_base + scene.expected_delta(1.0 / 60.0, 61);
    let actual = scene.hips_translation();
    assert!(
        (actual - expected).length() < EPSILON,
        "new animation base must be captured: {actual} vs {expected}"
    );
    let state = scene
        .app
        .world()
        .get::<BreathingState>(scene.root)
        .expect("state exists");
    assert_eq!(state.base(), Some(new_base));
}
#[test]
fn non_identity_root_parent_and_rest_rotations_preserve_model_space_motion() {
    let root_rotation = Quat::from_euler(EulerRot::XYZ, 0.3, 0.7, 0.2).normalize();
    let intermediate_rotation = Quat::from_euler(EulerRot::ZYX, 0.4, -0.2, 0.5).normalize();
    let hips_rest_rotation = Quat::from_euler(EulerRot::YXZ, -0.5, 0.1, 0.9).normalize();
    let base = Vec3::new(0.15, 1.1, 0.05);
    let mut scene = build_scene(
        root_rotation,
        Some(intermediate_rotation),
        Vec3::new(0.1, 0.2, 0.3),
        hips_rest_rotation,
        base,
        false,
    );

    let dt = Duration::from_secs_f64(1.0 / 60.0);
    // Peak inhale after 2.5 seconds.
    scene.advance(dt, 150);

    // The breathing delta is model-space (+Y * vertical) + (+Z * forward) at
    // peak, independent of the rotated root, intermediate, and hips rest
    // rotations. The peak envelope is essentially 1.0.
    let global_delta = scene.hips_global().translation() - scene.rest_hips_global.translation();
    assert!(
        (global_delta - Vec3::new(0.0, scene.vertical_amplitude, scene.forward_amplitude)).length()
            < 2.0 * EPSILON,
        "global delta {global_delta} must equal model-space up/forward at peak"
    );
}

#[test]
fn multiple_intermediate_globals_are_composed_from_root_toward_hips() {
    let ancestor_specs = [
        (
            Quat::from_euler(EulerRot::XYZ, 0.35, -0.2, 0.15).normalize(),
            Vec3::new(0.2, -0.1, 0.3),
        ),
        (
            Quat::from_euler(EulerRot::ZYX, -0.45, 0.25, 0.55).normalize(),
            Vec3::new(-0.15, 0.25, 0.1),
        ),
    ];
    let mut scene = build_scene_with_ancestors(
        Quat::from_rotation_y(0.4).normalize(),
        &ancestor_specs,
        Quat::from_rotation_z(-0.3).normalize(),
        Vec3::new(0.1, 1.0, -0.2),
        false,
    );

    // Use a non-neutral phase so the hips output is part of the composed
    // chain, not just the authored rest transform.
    scene.advance(Duration::from_secs_f64(1.0 / 60.0), 150);

    let root_global = *scene
        .app
        .world()
        .get::<GlobalTransform>(scene.root)
        .expect("root global");
    let mut expected_global = root_global;
    for &ancestor in &scene.ancestors {
        let transform = *scene
            .app
            .world()
            .get::<Transform>(ancestor)
            .expect("ancestor transform");
        expected_global = expected_global.mul_transform(transform);
    }
    let hips_transform = *scene
        .app
        .world()
        .get::<Transform>(scene.hips)
        .expect("hips transform");
    expected_global = expected_global.mul_transform(hips_transform);

    let actual_global = scene.hips_global();
    assert!(
        (actual_global.translation() - expected_global.translation()).length() < EPSILON,
        "hips global translation must use root-to-hips order: actual {}, expected {}",
        actual_global.translation(),
        expected_global.translation()
    );
    assert!(
        actual_global
            .rotation()
            .angle_between(expected_global.rotation())
            < EPSILON,
        "hips global rotation must use root-to-hips order"
    );
}

#[test]
fn intermediate_hierarchy_receives_fresh_globals_and_body_tracking_consumes_same_frame() {
    let mut scene = build_scene(
        identity_quat(),
        Some(Quat::from_rotation_y(0.4).normalize()),
        Vec3::new(0.05, 0.0, 0.0),
        identity_quat(),
        Vec3::new(0.0, 1.0, 0.0),
        true,
    );
    scene.app.add_systems(
        PostUpdate,
        (apply_breathing_hips_translation, apply_direct_body_tracking).chain(),
    );

    let dt = Duration::from_secs_f64(1.0 / 60.0);
    scene.advance(dt, 150);

    let spine = scene.spine.expect("spine exists");
    let hips_transform = *scene
        .app
        .world()
        .get::<Transform>(scene.hips)
        .expect("hips transform");
    let root_global = *scene
        .app
        .world()
        .get::<GlobalTransform>(scene.root)
        .expect("root global");
    let intermediate = scene.intermediate.expect("intermediate exists");
    let intermediate_transform = *scene
        .app
        .world()
        .get::<Transform>(intermediate)
        .expect("intermediate transform");
    let spine_transform = *scene
        .app
        .world()
        .get::<Transform>(spine)
        .expect("spine transform");

    let expected_spine_global = root_global
        .mul_transform(intermediate_transform)
        .mul_transform(hips_transform)
        .mul_transform(spine_transform);
    let spine_global = *scene
        .app
        .world()
        .get::<GlobalTransform>(spine)
        .expect("spine global");
    assert!(
        (spine_global.translation() - expected_spine_global.translation()).length() < EPSILON,
        "spine global must consume the same-frame breathing hips translation"
    );

    // Breathing and body tracking must not change the spine local rotation.
    assert!(
        spine_transform.rotation.angle_between(Quat::IDENTITY) < EPSILON,
        "spine local rotation must stay identity"
    );
}
// --- lifecycle and coexistence ---

#[test]
fn breathing_continues_without_control_frame_and_inactive_tracking() {
    // The app has no ActiveControlFrame resource at all and the root
    // BodyTrackingPoseInput stays inactive; breathing must still advance.
    let base = Vec3::new(0.0, 1.0, 0.0);
    let mut scene = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    let dt = Duration::from_secs_f64(1.0 / 60.0);
    scene.advance(dt, 150);
    let peak = scene.hips_translation();
    assert!(
        (peak - base).length() > 0.005,
        "breathing must move the hips without any tracking input: {peak} vs {base}"
    );
}

#[test]
fn tracking_transitions_do_not_reset_or_snap_phase() {
    let base = Vec3::new(0.0, 1.0, 0.0);
    let mut reference = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    let mut toggled = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    let dt = Duration::from_secs_f64(1.0 / 60.0);
    for frame in 0..240 {
        if frame % 40 == 0 {
            let active = frame % 80 == 0;
            toggled
                .app
                .world_mut()
                .get_mut::<BodyTrackingPoseInput>(toggled.root)
                .expect("input exists")
                .active = active;
        }
        reference.advance(dt, 1);
        toggled.advance(dt, 1);
        let reference_value = reference.hips_translation();
        let toggled_value = toggled.hips_translation();
        assert!(
            (reference_value - toggled_value).length() < 1.0e-6,
            "tracking active transitions must not disturb breathing at frame {frame}"
        );
    }
}

#[test]
fn non_ready_lifecycle_stops_writing() {
    let base = Vec3::new(0.0, 1.0, 0.0);
    let mut scene = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    let dt = Duration::from_secs_f64(1.0 / 60.0);
    scene.advance(dt, 90);

    scene
        .app
        .world_mut()
        .resource_mut::<AvatarLifecycle>()
        .request_unload()
        .expect("unload from Ready is valid");
    let frozen = scene.hips_translation();
    scene.advance(dt, 30);
    assert_eq!(
        scene.hips_translation(),
        frozen,
        "outside Ready the breathing system must not write hips translation"
    );
}
#[test]
fn unload_replacement_clears_state_and_replacement_begins_neutral() {
    let base = Vec3::new(0.0, 1.0, 0.0);
    let mut scene = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    let dt = Duration::from_secs_f64(1.0 / 60.0);
    scene.advance(dt, 100);
    let old_root = scene.root;
    let old_hips = scene.hips;
    let old_intermediate = scene.intermediate;
    let old_head = scene.head;

    // Drive the replacement through the real lifecycle transitions.
    let new_root = scene.app.world_mut().spawn_empty().id();
    let mut lifecycle = scene
        .app
        .world_mut()
        .remove_resource::<AvatarLifecycle>()
        .expect("lifecycle exists");
    lifecycle
        .request_replace(new_root)
        .expect("replace from Ready is valid");
    lifecycle.finish_unload();
    lifecycle.start_binding(new_root);
    lifecycle.finish_ready();
    let generation = lifecycle.current_generation();
    scene.app.world_mut().insert_resource(lifecycle);

    // Spawn the replacement hierarchy with fresh rest data and components.
    let base2 = Vec3::new(0.3, 0.9, -0.2);
    let hips_rest_local = Transform {
        translation: base2,
        ..Transform::IDENTITY
    };
    let rest_hips_global = GlobalTransform::from(hips_rest_local);
    let new_hips = spawn_transform(
        &mut scene.app,
        hips_rest_local,
        GlobalTransform::IDENTITY,
        Some(new_root),
    );
    scene.app.world_mut().entity_mut(new_hips).insert((
        RestTransform(hips_rest_local),
        RestGlobalTransform(rest_hips_global),
    ));
    let new_head = scene.app.world_mut().spawn_empty().id();
    let binding = resolve_breathing_binding(
        generation,
        new_hips,
        &BreathingProfile::default(),
        Some(hips_rest_local),
        Some(rest_hips_global),
        Vec::new(),
    )
    .expect("replacement rest data resolves");
    scene.app.world_mut().entity_mut(new_root).insert((
        ActiveAvatar,
        AvatarBinding::head_only(new_root, new_head, generation),
        BreathingProfile::default(),
        binding,
        BreathingState::default(),
    ));

    // Remove the old hierarchy like the real unload path does. Despawning
    // the root cascades to its children, so only the detached head is
    // despawned explicitly.
    scene
        .app
        .world_mut()
        .entity_mut(old_root)
        .remove::<ActiveAvatar>();
    let _ = old_hips;
    let _ = old_intermediate;
    scene.app.world_mut().entity_mut(old_root).despawn();
    scene.app.world_mut().entity_mut(old_head).despawn();

    scene.advance(dt, 1);
    let new_translation = scene
        .app
        .world()
        .get::<Transform>(new_hips)
        .expect("replacement hips transform")
        .translation;
    assert_eq!(
        new_translation, base2,
        "replacement must begin at neutral phase 0 with no inherited delta"
    );
    assert!(
        scene.app.world().get::<BreathingState>(new_root).is_some(),
        "replacement root carries fresh breathing state"
    );
}
// --- robustness ---

#[test]
fn non_finite_hips_translation_is_a_bounded_safe_no_op() {
    let base = Vec3::new(0.0, 1.0, 0.0);
    let mut scene = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    let dt = Duration::from_secs_f64(1.0 / 60.0);
    scene.advance(dt, 30);

    scene
        .app
        .world_mut()
        .get_mut::<Transform>(scene.hips)
        .expect("hips transform")
        .translation = Vec3::new(f32::NAN, f32::NAN, f32::NAN);
    scene.advance(dt, 5);

    // Recovery: a finite animation value is captured as a fresh base and
    // composed with the current breathing delta.
    let recovered_base = Vec3::new(0.0, 1.0, 0.0);
    scene
        .app
        .world_mut()
        .get_mut::<Transform>(scene.hips)
        .expect("hips transform")
        .translation = recovered_base;
    scene.advance(dt, 1);
    let state = *scene
        .app
        .world()
        .get::<BreathingState>(scene.root)
        .expect("state exists");
    assert_eq!(state.base(), Some(recovered_base));
    // The system evaluates the envelope before advancing the accumulator, so
    // the written delta used the pre-evaluation elapsed time.
    let pre_evaluation_elapsed = state.elapsed_seconds() - 1.0 / 60.0;
    let breath = breathing_envelope(breathing_phase(
        pre_evaluation_elapsed,
        scene.profile.period_seconds,
    )) as f32;
    let binding = scene
        .app
        .world()
        .get::<BreathingBinding>(scene.root)
        .expect("binding exists");
    let expected_delta = binding.up_local * (scene.vertical_amplitude * breath)
        + binding.forward_local * (scene.forward_amplitude * breath);
    assert!(
        (scene.hips_translation() - (recovered_base + expected_delta)).length() < 1.0e-6,
        "recovered base must be finite and composed with the current delta"
    );
}

#[test]
fn scale_and_rotation_channels_are_never_written() {
    let hips_rest_rotation = Quat::from_rotation_z(0.3).normalize();
    let base = Vec3::new(0.0, 1.0, 0.0);
    let mut scene = build_scene(
        identity_quat(),
        Some(Quat::from_rotation_x(0.25).normalize()),
        Vec3::ZERO,
        hips_rest_rotation,
        base,
        false,
    );
    let initial_hips_scale = scene
        .app
        .world()
        .get::<Transform>(scene.hips)
        .expect("hips transform")
        .scale;
    let initial_intermediate_rotation = scene
        .app
        .world()
        .get::<Transform>(scene.intermediate.expect("intermediate exists"))
        .expect("intermediate transform")
        .rotation;

    let dt = Duration::from_secs_f64(1.0 / 60.0);
    scene.advance(dt, 300);

    let hips_transform = scene
        .app
        .world()
        .get::<Transform>(scene.hips)
        .expect("hips transform");
    assert_eq!(hips_transform.scale, initial_hips_scale);
    assert!(
        hips_transform.rotation.angle_between(hips_rest_rotation) < EPSILON,
        "hips rotation must never be written by breathing"
    );
    let intermediate_rotation = scene
        .app
        .world()
        .get::<Transform>(scene.intermediate.expect("intermediate exists"))
        .expect("intermediate transform")
        .rotation;
    assert!(
        intermediate_rotation.angle_between(initial_intermediate_rotation) < EPSILON,
        "ancestor rotation must never be written by breathing"
    );
}
#[test]
fn root_and_camera_transforms_remain_unchanged() {
    let base = Vec3::new(0.0, 1.0, 0.0);
    let mut scene = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    let camera_transform = Transform::from_translation(Vec3::new(0.0, 0.0, 2.5));
    let camera = scene
        .app
        .world_mut()
        .spawn((camera_transform, GlobalTransform::from(camera_transform)))
        .id();
    let initial_root_transform = *scene
        .app
        .world()
        .get::<Transform>(scene.root)
        .expect("root transform");

    let dt = Duration::from_secs_f64(1.0 / 60.0);
    scene.advance(dt, 300);

    let root_transform = *scene
        .app
        .world()
        .get::<Transform>(scene.root)
        .expect("root transform");
    assert_eq!(root_transform, initial_root_transform);
    let camera_after = *scene
        .app
        .world()
        .get::<Transform>(camera)
        .expect("camera transform");
    assert_eq!(camera_after, camera_transform);
}

// --- frame-rate independence ---

#[test]
fn thirty_sixty_and_120_fps_produce_materially_equivalent_motion() {
    let base = Vec3::new(0.0, 1.0, 0.0);
    let run_at = |fps: f64| {
        let frames = (2.4 * fps).round() as usize;
        let mut scene = build_scene(
            identity_quat(),
            None,
            Vec3::ZERO,
            identity_quat(),
            base,
            false,
        );
        let dt = Duration::from_secs_f64(1.0 / fps);
        scene.advance(dt, frames);
        scene.hips_translation()
    };
    let at_30 = run_at(30.0);
    let at_60 = run_at(60.0);
    let at_120 = run_at(120.0);
    assert!(
        (at_30 - at_60).length() < 3.0e-3,
        "30fps {at_30} vs 60fps {at_60}"
    );
    assert!(
        (at_30 - at_120).length() < 3.0e-3,
        "30fps {at_30} vs 120fps {at_120}"
    );
    assert!(
        (at_60 - at_120).length() < 3.0e-3,
        "60fps {at_60} vs 120fps {at_120}"
    );
}

#[test]
fn repeated_evaluation_at_the_same_phase_produces_the_same_output() {
    let base = Vec3::new(0.0, 1.0, 0.0);
    let mut scene = build_scene(
        identity_quat(),
        None,
        Vec3::ZERO,
        identity_quat(),
        base,
        false,
    );
    let dt = Duration::from_secs_f64(1.0 / 60.0);
    scene.advance(dt, 90);
    // The system evaluates before advancing, so one zero-delta update moves
    // the display to the current accumulated phase; every further zero-delta
    // update must reproduce exactly the same output (no drift).
    scene.advance(Duration::ZERO, 1);
    let first = scene.hips_translation();
    scene.advance(Duration::ZERO, 5);
    let second = scene.hips_translation();
    assert_eq!(first, second);

    // The converged output equals the analytic waveform at the accumulated
    // phase (identity scene: up +Y, forward +Z, rest height 1.0).
    let breath = breathing_envelope(breathing_phase(90.0 / 60.0, 5.0)) as f32;
    let expected = base + Vec3::new(0.0, 0.010 * breath, 0.008 * breath);
    assert!(
        (first - expected).length() < 1.0e-6,
        "paused output {first} vs analytic {expected}"
    );
}
