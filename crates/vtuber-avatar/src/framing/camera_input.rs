//! Main viewport mouse input routing for camera controls.

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};

use super::camera_control::{AvatarCameraControl, CameraControlGeometryError, geometry};
use crate::lifecycle::{AvatarGeneration, AvatarLifecycle, AvatarLifecycleState};

/// System set for main viewport input before transform propagation.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CameraInputSet;

/// Deterministic mouse gesture ownership for the main viewport.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraPointerGesture {
    /// No camera drag owns the pointer.
    #[default]
    None,
    /// The left-button gesture owns the pointer for this generation.
    Orbit {
        /// Avatar generation captured at gesture start.
        generation: AvatarGeneration,
    },
    /// The right-button gesture owns the pointer for this generation.
    Pan {
        /// Avatar generation captured at gesture start.
        generation: AvatarGeneration,
    },
}

/// Converts Bevy's line/pixel scroll units into one normalized dolly input.
///
/// Horizontal scroll is intentionally ignored by the caller. Bevy's official
/// `MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR` is used for pixel input so
/// line and high-resolution trackpad input share the same scale.
pub fn normalized_vertical_scroll(
    scroll: Vec2,
    unit: MouseScrollUnit,
) -> Result<f32, CameraControlGeometryError> {
    if !scroll.is_finite() {
        return Err(CameraControlGeometryError::NonFiniteInput);
    }
    let normalized = match unit {
        MouseScrollUnit::Line => scroll.y,
        MouseScrollUnit::Pixel => scroll.y / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR,
    };
    if normalized.is_finite() {
        Ok(normalized)
    } else {
        Err(CameraControlGeometryError::NonFiniteInput)
    }
}

/// Applies captured orbit/pan and background-only wheel dolly to the viewport.
///
/// The system reads only `Camera` and `Transform`; perspective projection and
/// FOV are deliberately absent from the query. Mouse deltas are already frame
/// accumulations, so no delta-time factor is applied.
#[derive(SystemParam)]
pub(crate) struct CameraInputWorld<'w, 's> {
    gate: Res<'w, super::camera_control::CameraPointerInputGate>,
    lifecycle: Res<'w, AvatarLifecycle>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras:
        Query<'w, 's, (&'static Camera, &'static mut Transform), With<super::AvatarViewportCamera>>,
}

pub(crate) fn apply_camera_pointer_input(
    mut input: CameraInputWorld,
    mut gesture: ResMut<CameraPointerGesture>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut camera_control: ResMut<AvatarCameraControl>,
) {
    let Some(generation) = camera_control.active_generation() else {
        *gesture = CameraPointerGesture::None;
        return;
    };
    if input.lifecycle.state() != AvatarLifecycleState::Ready
        || input.lifecycle.current_generation() != generation
    {
        *gesture = CameraPointerGesture::None;
        return;
    }
    if input
        .windows
        .iter()
        .next()
        .is_some_and(|window| !window.focused)
    {
        *gesture = CameraPointerGesture::None;
        return;
    }

    let Some((camera, mut transform)) = input.cameras.iter_mut().next() else {
        return;
    };
    let Some(current) = camera_control.current_for(generation) else {
        *gesture = CameraPointerGesture::None;
        return;
    };

    let active_gesture = match *gesture {
        CameraPointerGesture::None if input.gate.allows_camera_input() => {
            // Left wins if both buttons arrive in one frame. Once selected,
            // the mode cannot switch until its corresponding release.
            if mouse_buttons.just_pressed(MouseButton::Left) {
                *gesture = CameraPointerGesture::Orbit { generation };
            } else if mouse_buttons.just_pressed(MouseButton::Right) {
                *gesture = CameraPointerGesture::Pan { generation };
            }
            *gesture
        }
        existing => existing,
    };

    let mut next = current;
    match active_gesture {
        CameraPointerGesture::Orbit {
            generation: captured,
        } if captured == generation => {
            let sensitivity = camera_control.config().orbit_radians_per_pixel;
            if sensitivity.is_finite() && sensitivity > 0.0 && mouse_motion.delta.is_finite() {
                // Positive screen Y is downward, so dragging upward raises the
                // orbit camera and produces a positive pitch delta.
                if let Ok(candidate) = geometry::orbit(
                    next,
                    mouse_motion.delta.x * sensitivity,
                    -mouse_motion.delta.y * sensitivity,
                ) {
                    next = candidate;
                }
            }
        }
        CameraPointerGesture::Pan {
            generation: captured,
        } if captured == generation => {
            if let Some(viewport_size) = camera.logical_viewport_size()
                && let Ok(candidate) = geometry::pan(next, mouse_motion.delta, viewport_size)
            {
                next = candidate;
            }
        }
        CameraPointerGesture::None => {}
        CameraPointerGesture::Orbit { .. } | CameraPointerGesture::Pan { .. } => {
            *gesture = CameraPointerGesture::None;
            return;
        }
    }

    if input.gate.allows_camera_input()
        && let Ok(scroll) = normalized_vertical_scroll(mouse_scroll.delta, mouse_scroll.unit)
        && scroll != 0.0
        && let Ok(candidate) = geometry::dolly(next, scroll, camera_control.config())
    {
        next = candidate;
    }

    if next != current {
        *transform = next.transform();
        if !camera_control.set_current(generation, next) {
            *gesture = CameraPointerGesture::None;
            return;
        }
    }

    match active_gesture {
        CameraPointerGesture::Orbit { .. } if mouse_buttons.just_released(MouseButton::Left) => {
            *gesture = CameraPointerGesture::None;
        }
        CameraPointerGesture::Pan { .. } if mouse_buttons.just_released(MouseButton::Right) => {
            *gesture = CameraPointerGesture::None;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::camera_control::{
        AvatarCameraControlState, CameraControlPose, CameraPointerInputGate, FIXED_VERTICAL_FOV,
    };

    fn ready_app() -> (App, Entity, AvatarGeneration) {
        let mut app = App::new();
        app.init_resource::<CameraPointerInputGate>()
            .init_resource::<CameraPointerGesture>()
            .init_resource::<AvatarLifecycle>()
            .init_resource::<AvatarCameraControl>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, apply_camera_pointer_input);

        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                Camera {
                    viewport: Some(bevy::camera::Viewport {
                        physical_size: UVec2::new(1600, 900),
                        ..default()
                    }),
                    ..default()
                },
                super::super::AvatarViewportCamera,
                Transform::from_translation(Vec3::new(0.0, 1.0, 5.0))
                    .looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
            ))
            .id();
        let root = app.world_mut().spawn_empty().id();
        let generation = {
            let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
            lifecycle.request_load(root).expect("test load is valid");
            lifecycle.start_binding(root);
            lifecycle.finish_ready();
            lifecycle.current_generation()
        };
        let transform = *app
            .world()
            .get::<Transform>(camera)
            .expect("camera transform");
        let pose = CameraControlPose::new(transform, Vec3::new(0.0, 1.0, 0.0))
            .expect("test pose is valid");
        app.world_mut()
            .resource_mut::<AvatarCameraControl>()
            .initialize(generation, pose);
        (app, camera, generation)
    }

    fn press(app: &mut App, button: MouseButton) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(button);
    }

    fn release(app: &mut App, button: MouseButton) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(button);
    }

    fn clear_buttons(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
    }

    #[test]
    fn line_and_pixel_scroll_normalize_to_the_same_physical_intent() {
        let line = normalized_vertical_scroll(Vec2::new(3.0, 2.0), MouseScrollUnit::Line)
            .expect("line scroll is valid");
        let pixel = normalized_vertical_scroll(Vec2::new(100.0, 200.0), MouseScrollUnit::Pixel)
            .expect("pixel scroll is valid");
        assert!((line - pixel).abs() < f32::EPSILON);
        assert_eq!(
            normalized_vertical_scroll(Vec2::new(f32::NAN, 0.0), MouseScrollUnit::Line),
            Err(CameraControlGeometryError::NonFiniteInput)
        );
    }

    #[test]
    fn background_left_drag_captures_orbit_and_release_clears_it() {
        let (mut app, camera, generation) = ready_app();
        press(&mut app, MouseButton::Left);
        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = Vec2::new(20.0, -10.0);
        app.update();

        assert_eq!(
            *app.world().resource::<CameraPointerGesture>(),
            CameraPointerGesture::Orbit { generation }
        );
        assert_ne!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Vec3::new(0.0, 1.0, 5.0)
        );

        clear_buttons(&mut app);
        release(&mut app, MouseButton::Left);
        app.update();
        assert_eq!(
            *app.world().resource::<CameraPointerGesture>(),
            CameraPointerGesture::None
        );
    }

    #[test]
    fn simultaneous_buttons_choose_orbit_deterministically() {
        let (mut app, _, generation) = ready_app();
        press(&mut app, MouseButton::Left);
        press(&mut app, MouseButton::Right);
        app.update();

        assert_eq!(
            *app.world().resource::<CameraPointerGesture>(),
            CameraPointerGesture::Orbit { generation }
        );
    }

    #[test]
    fn right_press_wins_only_when_left_is_absent_and_wheel_changes_distance() {
        let (mut app, _, generation) = ready_app();
        press(&mut app, MouseButton::Right);
        app.update();
        assert_eq!(
            *app.world().resource::<CameraPointerGesture>(),
            CameraPointerGesture::Pan { generation }
        );

        clear_buttons(&mut app);
        release(&mut app, MouseButton::Right);
        app.update();

        app.world_mut()
            .resource_mut::<AccumulatedMouseScroll>()
            .delta = Vec2::new(12.0, 1.0);
        let before = app
            .world()
            .resource::<AvatarCameraControl>()
            .current_for(generation)
            .expect("control pose");
        app.update();
        let after = app
            .world()
            .resource::<AvatarCameraControl>()
            .current_for(generation)
            .expect("control pose");
        assert!(after.distance() < before.distance());
        assert_eq!(after.transform().rotation, before.transform().rotation);
    }

    #[test]
    fn egui_owned_start_is_blocked_but_captured_drag_continues_over_ui() {
        let (mut app, camera, generation) = ready_app();
        app.world_mut()
            .resource_mut::<CameraPointerInputGate>()
            .set_egui_owns_pointer(true);
        press(&mut app, MouseButton::Left);
        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = Vec2::new(20.0, 0.0);
        let before = app.world().get::<Transform>(camera).unwrap().translation;
        app.update();
        assert_eq!(
            *app.world().resource::<CameraPointerGesture>(),
            CameraPointerGesture::None
        );
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            before
        );

        clear_buttons(&mut app);
        app.world_mut()
            .resource_mut::<CameraPointerInputGate>()
            .set_egui_owns_pointer(false);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .reset_all();
        press(&mut app, MouseButton::Left);
        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = Vec2::X;
        app.update();
        assert_eq!(
            *app.world().resource::<CameraPointerGesture>(),
            CameraPointerGesture::Orbit { generation }
        );

        clear_buttons(&mut app);
        app.world_mut()
            .resource_mut::<CameraPointerInputGate>()
            .set_egui_owns_pointer(true);
        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = Vec2::new(10.0, 0.0);
        let during_ui = app.world().get::<Transform>(camera).unwrap().translation;
        app.update();
        assert_ne!(
            app.world().get::<Transform>(camera).unwrap().translation,
            during_ui
        );
        assert_eq!(
            *app.world().resource::<CameraPointerGesture>(),
            CameraPointerGesture::Orbit { generation }
        );
    }

    #[test]
    fn lifecycle_invalidation_clears_capture_without_changing_fov() {
        let (mut app, _, generation) = ready_app();
        press(&mut app, MouseButton::Left);
        app.update();
        assert_eq!(
            *app.world().resource::<CameraPointerGesture>(),
            CameraPointerGesture::Orbit { generation }
        );
        app.world_mut()
            .resource_mut::<AvatarCameraControl>()
            .invalidate();
        app.update();
        assert_eq!(
            *app.world().resource::<CameraPointerGesture>(),
            CameraPointerGesture::None
        );
        assert_eq!(
            app.world().resource::<AvatarCameraControl>().state(),
            AvatarCameraControlState::Unavailable
        );
        assert!((FIXED_VERTICAL_FOV - 12.0_f32.to_radians()).abs() < f32::EPSILON);
    }
}
