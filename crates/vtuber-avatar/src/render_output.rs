//! Transparent avatar-only render target and asynchronous GPU readback.
//!
//! This module owns the Bevy side of the output boundary. It deliberately
//! exposes only the transport-neutral [`vtuber_core::VideoOutputFrame`] and a
//! latest-value slot; no network or NDI type enters the avatar crate.

use bevy::camera::{CameraUpdateSystems, ClearColorConfig, RenderTarget, visibility::RenderLayers};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::render::renderer::RenderDevice;
use vtuber_core::{FrameSeq, VideoOutputFrame, VideoOutputProfile, monotonic_now};

use crate::lifecycle::AvatarGeneration;

/// The rendering layer containing avatar geometry and output lighting.
pub const AVATAR_RENDER_LAYER: usize = 0;
/// The main-window-only layer containing the ground plane.
pub const VIEWPORT_ONLY_RENDER_LAYER: usize = 1;

/// Fixed render target and profile used by the output camera.
#[derive(Resource, Clone, Debug)]
pub struct AvatarOutputTarget {
    image: Handle<Image>,
    profile: VideoOutputProfile,
}

impl AvatarOutputTarget {
    /// Returns the fixed transport-neutral output profile.
    #[must_use]
    pub const fn profile(&self) -> VideoOutputProfile {
        self.profile
    }

    /// Returns the render-target image handle for renderer integration.
    #[must_use]
    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }
}

/// Runtime activation state for the transparent output camera/readback.
///
/// The default is inactive. Toggling this resource does not affect camera
/// capture, tracking, or the main avatar viewport.
#[derive(Resource, Clone, Debug, Default)]
pub struct AvatarOutputState {
    active: bool,
    profile: VideoOutputProfile,
}

impl AvatarOutputState {
    /// Returns whether the offscreen camera and readback are active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// Returns the fixed output profile.
    #[must_use]
    pub const fn profile(&self) -> VideoOutputProfile {
        self.profile
    }

    /// Activates or deactivates the output camera/readback lifecycle.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Activates transparent output.
    pub fn activate(&mut self) {
        self.set_active(true);
    }

    /// Deactivates transparent output.
    pub fn deactivate(&mut self) {
        self.set_active(false);
    }
}

/// A latest-value slot for completed output frames.
///
/// A slow consumer can replace one pending frame, but cannot grow a queue.
#[derive(Resource, Default, Debug)]
pub struct AvatarOutputFrameSlot {
    latest: Option<VideoOutputFrame>,
    next_frame_seq: u64,
    received_frames: u64,
    replaced_frames: u64,
    rejected_frames: u64,
}

impl AvatarOutputFrameSlot {
    /// Takes the newest completed frame, if one is pending.
    pub fn take_latest(&mut self) -> Option<VideoOutputFrame> {
        self.latest.take()
    }

    /// Returns the newest completed frame without removing it.
    #[must_use]
    pub fn latest(&self) -> Option<&VideoOutputFrame> {
        self.latest.as_ref()
    }

    /// Number of successfully converted readback frames.
    #[must_use]
    pub const fn received_frames(&self) -> u64 {
        self.received_frames
    }

    /// Number of completed frames replaced before a consumer took them.
    #[must_use]
    pub const fn replaced_frames(&self) -> u64 {
        self.replaced_frames
    }

    /// Number of malformed readbacks rejected at the contract boundary.
    #[must_use]
    pub const fn rejected_frames(&self) -> u64 {
        self.rejected_frames
    }

    fn replace(&mut self, frame: VideoOutputFrame) {
        self.next_frame_seq = self.next_frame_seq.saturating_add(1);
        self.received_frames = self.received_frames.saturating_add(1);
        if self.latest.replace(frame).is_some() {
            self.replaced_frames = self.replaced_frames.saturating_add(1);
        }
    }

    fn reject(&mut self) {
        self.rejected_frames = self.rejected_frames.saturating_add(1);
    }

    fn next_frame_seq(&self) -> FrameSeq {
        FrameSeq(self.next_frame_seq)
    }
}

/// Read-only snapshot of the current main avatar viewport camera.
///
/// The snapshot contains no Bevy entity ID and is updated only after the
/// main camera's framing/manual controls have produced their current state.
#[derive(Resource, Clone, Debug, Default)]
pub struct AvatarViewportSnapshot {
    /// Avatar lifecycle generation associated with this camera state.
    pub generation: AvatarGeneration,
    /// Current viewport camera transform.
    pub transform: Option<Transform>,
    /// Current perspective projection values.
    pub projection: Option<PerspectiveProjection>,
}

/// Marks the dedicated transparent output camera.
#[derive(Component, Debug)]
pub struct AvatarOutputCamera;

/// Internal gate that allows only one GPU readback to be in flight.
#[derive(Component, Debug)]
struct AvatarOutputReadbackInFlight;

#[derive(SystemParam)]
struct OutputCameraQuery<'w, 's> {
    // This tuple intentionally keeps all output-camera writes in one query so
    // the camera, projection, transform, and bounded-readback gate change as
    // one synchronized boundary.
    #[allow(clippy::type_complexity)]
    cameras: Query<
        'w,
        's,
        (
            Entity,
            &'static mut Camera,
            &'static mut Projection,
            &'static mut Transform,
            &'static mut GlobalTransform,
            Option<&'static AvatarOutputReadbackInFlight>,
        ),
        (
            With<AvatarOutputCamera>,
            Without<crate::framing::AvatarViewportCamera>,
        ),
    >,
}

/// Creates the fixed transparent target and output camera.
pub fn setup_output_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<AvatarOutputState>,
) {
    let profile = state.profile();
    let mut image = Image::new_target_texture(
        profile.width,
        profile.height,
        TextureFormat::Bgra8UnormSrgb,
        None,
    );
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image_handle = images.add(image);
    let camera_transform =
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.5)).looking_at(Vec3::ZERO, Vec3::Y);

    commands.insert_resource(AvatarOutputTarget {
        image: image_handle.clone(),
        profile,
    });
    let mut camera = Camera {
        is_active: false,
        clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ..default()
    };
    camera.order = -1;
    commands
        .spawn((
            Camera3d::default(),
            camera,
            RenderTarget::Image(image_handle.into()),
            Projection::Perspective(PerspectiveProjection {
                fov: crate::framing::fixed_fov_fit::FIXED_VERTICAL_FOV,
                aspect_ratio: profile.width as f32 / profile.height as f32,
                ..default()
            }),
            camera_transform,
            RenderLayers::layer(AVATAR_RENDER_LAYER),
            AvatarOutputCamera,
        ))
        .observe(handle_output_readback);
    state.deactivate();
}

/// Mirrors the main viewport camera into the fixed offscreen camera.
///
/// This runs after avatar framing and after transform propagation. The output
/// camera has no parent, so its global transform can be committed immediately
/// and the render extractor sees the same state in this frame.
#[allow(clippy::type_complexity)]
fn sync_output_camera(
    lifecycle: Res<crate::lifecycle::AvatarLifecycle>,
    state: Res<AvatarOutputState>,
    target: Res<AvatarOutputTarget>,
    mut snapshot: ResMut<AvatarViewportSnapshot>,
    main_cameras: Query<
        (&Transform, &Projection),
        (
            With<crate::framing::AvatarViewportCamera>,
            Without<AvatarOutputCamera>,
        ),
    >,
    mut output_cameras: OutputCameraQuery,
    mut commands: Commands,
) {
    snapshot.generation = lifecycle.current_generation();
    let Ok((main_transform, main_projection)) = main_cameras.single() else {
        snapshot.transform = None;
        snapshot.projection = None;
        return;
    };
    let Projection::Perspective(main_projection) = main_projection else {
        snapshot.transform = None;
        snapshot.projection = None;
        return;
    };

    snapshot.transform = Some(*main_transform);
    snapshot.projection = Some(main_projection.clone());

    for (entity, mut camera, mut projection, mut transform, mut global_transform, in_flight) in
        &mut output_cameras.cameras
    {
        *transform = *main_transform;
        *global_transform = GlobalTransform::from(*main_transform);
        *projection = Projection::Perspective(main_projection.clone());
        camera.is_active = state.is_active();
        if state.is_active() && in_flight.is_none() {
            commands.entity(entity).insert((
                Readback::texture(target.image().clone()),
                AvatarOutputReadbackInFlight,
            ));
        } else if !state.is_active() {
            commands
                .entity(entity)
                .remove::<Readback>()
                .remove::<AvatarOutputReadbackInFlight>();
        }
    }
}

fn handle_output_readback(
    event: On<ReadbackComplete>,
    mut commands: Commands,
    state: Res<AvatarOutputState>,
    target: Res<AvatarOutputTarget>,
    mut slot: ResMut<AvatarOutputFrameSlot>,
) {
    commands
        .entity(event.entity)
        .remove::<Readback>()
        .remove::<AvatarOutputReadbackInFlight>();
    if !state.is_active() {
        return;
    }
    let profile = target.profile();
    let packed_stride = profile.packed_stride_bytes();
    let source_stride = RenderDevice::align_copy_bytes_per_row(packed_stride);
    match VideoOutputFrame::from_padded_bgra8(
        profile.width,
        profile.height,
        source_stride,
        slot.next_frame_seq(),
        monotonic_now(),
        &event.data,
    ) {
        Ok(frame) => slot.replace(frame),
        Err(error) => {
            slot.reject();
            warn!("discarding malformed avatar output readback: {error}");
        }
    }
}

/// Adds the output lifecycle systems to an existing avatar app.
pub fn register_output_systems(app: &mut App) {
    app.init_resource::<AvatarOutputState>()
        .init_resource::<AvatarOutputFrameSlot>()
        .init_resource::<AvatarViewportSnapshot>()
        .add_systems(Startup, setup_output_camera)
        .add_systems(
            PostUpdate,
            sync_output_camera
                .after(crate::framing::frame_avatar_camera)
                .after(TransformSystems::Propagate)
                .before(CameraUpdateSystems),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_target() -> AvatarOutputTarget {
        AvatarOutputTarget {
            image: Handle::default(),
            profile: VideoOutputProfile::default(),
        }
    }

    #[test]
    fn output_starts_inactive_and_has_fixed_profile() {
        let state = AvatarOutputState::default();
        assert!(!state.is_active());
        assert_eq!(state.profile(), VideoOutputProfile::DEFAULT);
    }

    #[test]
    fn frame_slot_keeps_only_the_latest_frame() {
        let mut slot = AvatarOutputFrameSlot::default();
        let make = |seq| {
            VideoOutputFrame::new_bgra8(
                1,
                1,
                FrameSeq(seq),
                vtuber_core::MonoTimeNs(seq),
                vec![0, 0, 0, 0],
            )
            .expect("one transparent pixel is valid")
        };
        slot.replace(make(0));
        slot.replace(make(1));
        assert_eq!(slot.replaced_frames(), 1);
        assert_eq!(
            slot.take_latest().expect("latest frame").frame_seq,
            FrameSeq(1)
        );
        assert!(slot.take_latest().is_none());
    }

    #[test]
    fn output_camera_mirrors_the_current_viewport_state() {
        let mut app = App::new();
        app.init_resource::<crate::lifecycle::AvatarLifecycle>()
            .insert_resource(AvatarOutputState::default())
            .insert_resource(test_target())
            .insert_resource(AvatarViewportSnapshot::default());

        let main_transform = Transform::from_xyz(1.0, 2.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y);
        let main_projection = Projection::Perspective(PerspectiveProjection {
            fov: 0.42,
            aspect_ratio: 1.7,
            ..default()
        });
        app.world_mut().spawn((
            main_transform,
            main_projection,
            crate::framing::AvatarViewportCamera::from_default_transform(main_transform),
        ));
        let output = app
            .world_mut()
            .spawn((
                Camera::default(),
                Projection::Perspective(PerspectiveProjection::default()),
                Transform::default(),
                GlobalTransform::default(),
                AvatarOutputCamera,
            ))
            .id();
        app.add_systems(Update, sync_output_camera);

        app.update();

        assert_eq!(app.world().get::<Transform>(output), Some(&main_transform));
        assert_eq!(
            app.world().get::<GlobalTransform>(output),
            Some(&GlobalTransform::from(main_transform))
        );
        let Projection::Perspective(projection) = app.world().get::<Projection>(output).unwrap()
        else {
            panic!("output camera must remain perspective");
        };
        assert_eq!(projection.fov, 0.42);
        assert_eq!(projection.aspect_ratio, 1.7);
        let snapshot = app.world().resource::<AvatarViewportSnapshot>();
        assert_eq!(snapshot.transform, Some(main_transform));
        let snapshot_projection = snapshot.projection.as_ref().expect("projection snapshot");
        assert_eq!(snapshot_projection.fov, 0.42);
        assert_eq!(snapshot_projection.aspect_ratio, 1.7);
    }

    #[test]
    fn layer_contract_excludes_ground_from_output() {
        let output = RenderLayers::layer(AVATAR_RENDER_LAYER);
        let ground = RenderLayers::layer(VIEWPORT_ONLY_RENDER_LAYER);
        let viewport =
            RenderLayers::from_layers(&[AVATAR_RENDER_LAYER, VIEWPORT_ONLY_RENDER_LAYER]);
        assert!(!output.intersects(&ground));
        assert!(output.intersects(&viewport));
        assert!(ground.intersects(&viewport));
    }
}
