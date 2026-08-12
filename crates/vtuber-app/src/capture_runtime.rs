//! Capture runtime — bridges the orchestrator to the real camera backend.
//!
//! Manages the [`CaptureController`] lifecycle and provides Bevy systems for
//! preview texture updates and diagnostics synchronisation.

use std::sync::Arc;

use bevy::prelude::*;
use bevy_egui::{EguiTextureHandle, EguiUserTextures};
use vtuber_camera::capture::{CaptureController, CaptureServiceState};
use vtuber_camera::device::{CameraBackend, CameraDescriptor, CameraRequest};
use vtuber_core::metrics::RateCounter;
use vtuber_core::{FrameSeq, LatestSlot, VideoFrame, monotonic_now};

use crate::diagnostics::DiagnosticsSnapshot;
use crate::preview::PreviewState;

/// Resource wrapping the production [`CaptureController`].
///
/// The controller is created at app startup and lives for the entire
/// application lifetime. Individual capture sessions are started/stopped
/// via the controller's methods.
#[derive(Resource)]
pub struct CaptureRuntime {
    /// The underlying capture controller.
    controller: CaptureController,
    /// Whether the worker thread has been started.
    worker_started: bool,
    /// Last-read generation for the frame slot.
    last_generation: u64,
}

impl Default for CaptureRuntime {
    fn default() -> Self {
        Self {
            controller: CaptureController::new(),
            worker_started: false,
            last_generation: 0,
        }
    }
}

impl CaptureRuntime {
    /// Returns a reference to the underlying controller.
    #[must_use]
    pub fn controller(&self) -> &CaptureController {
        &self.controller
    }

    /// Returns a mutable reference to the underlying controller.
    pub fn controller_mut(&mut self) -> &mut CaptureController {
        &mut self.controller
    }

    /// Ensures the capture worker thread is running.
    ///
    /// This is idempotent — calling it multiple times has no effect after the
    /// first successful call.
    pub fn ensure_worker_started(&mut self) -> Result<(), String> {
        if self.worker_started {
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            let backend = vtuber_camera::backend::msmf::MsmfBackend::new();
            self.controller
                .start_worker(backend)
                .map_err(|e| format!("{e}"))?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            let backend = vtuber_camera::mock::MockBackend::default();
            self.controller
                .start_worker(backend)
                .map_err(|e| format!("{e}"))?;
        }

        self.worker_started = true;
        Ok(())
    }

    /// Enumerates available cameras.
    #[must_use]
    pub fn enumerate_cameras(&self) -> Vec<CameraDescriptor> {
        #[cfg(target_os = "windows")]
        {
            let backend = vtuber_camera::backend::msmf::MsmfBackend::new();
            backend.enumerate().unwrap_or_default()
        }

        #[cfg(not(target_os = "windows"))]
        {
            let backend = vtuber_camera::mock::MockBackend::default();
            backend.enumerate().unwrap_or_default()
        }
    }

    /// Starts capture with the given device.
    pub fn start_capture(
        &mut self,
        device: CameraDescriptor,
        request: CameraRequest,
    ) -> Result<(), String> {
        self.ensure_worker_started()?;
        self.controller
            .select_and_start(device, request)
            .map_err(|e| format!("{e}"))
    }

    /// Stops capture.
    pub fn stop_capture(&mut self) {
        self.controller.stop();
    }

    /// Stops and joins the capture worker for application shutdown.
    ///
    /// The controller is replaced with a fresh idle controller so this method
    /// can be called from a Bevy exit system without moving the resource out
    /// of the world. Normal Stop uses [`Self::stop_capture`] and preserves the
    /// worker for a later Start.
    pub fn shutdown(&mut self) {
        let controller = std::mem::replace(&mut self.controller, CaptureController::new());
        let _ = controller.shutdown();
        self.worker_started = false;
        self.last_generation = 0;
    }

    /// Returns the current service state.
    #[must_use]
    pub fn state(&self) -> CaptureServiceState {
        self.controller.state()
    }

    /// Returns the frame slot for reading the latest captured frame.
    #[must_use]
    pub fn frame_slot(&self) -> Arc<LatestSlot<VideoFrame>> {
        self.controller.frame_slot()
    }

    /// Tries to read the latest frame from the slot.
    ///
    /// Returns `None` if no new frame is available.
    pub fn try_read_frame(&mut self) -> Option<VideoFrame> {
        let slot = self.controller.frame_slot();
        match slot.try_read_after(self.last_generation) {
            Some(vtuber_core::ReadResult::New(frame)) => {
                // The slot generation may advance by more than one when the
                // producer overwrites unread frames. Track the actual
                // generation so the consumer never re-reads an old value.
                self.last_generation = slot.generation();
                Some(frame)
            }
            Some(vtuber_core::ReadResult::Closed) | None => None,
        }
    }
}

/// Resource holding the latest captured video frame for preview display.
#[derive(Resource, Default)]
pub struct LatestVideoFrame {
    /// The most recent frame, if any.
    pub frame: Option<VideoFrame>,
}

/// System that reads the latest frame from the capture slot and stores it
/// for preview rendering.
pub fn read_latest_frame(
    mut capture: ResMut<CaptureRuntime>,
    mut latest: ResMut<LatestVideoFrame>,
) {
    if let Some(frame) = capture.try_read_frame() {
        latest.frame = Some(frame);
    }
}

/// System that synchronises capture metrics into [`DiagnosticsSnapshot`].
pub fn sync_capture_diagnostics(
    capture: Res<CaptureRuntime>,
    latest: Res<LatestVideoFrame>,
    mut diagnostics: ResMut<DiagnosticsSnapshot>,
    mut last_seq: Local<Option<FrameSeq>>,
    mut rate: Local<Option<RateCounter>>,
) {
    let metrics = capture.controller().metrics();
    let rate_counter = rate.get_or_insert_with(|| RateCounter::new(1_000_000_000));
    if let Some(frame) = latest.frame.as_ref()
        && *last_seq != Some(frame.seq)
    {
        rate_counter.record(frame.captured_at.0);
        *last_seq = Some(frame.seq);
    }
    diagnostics.capture_rate = rate_counter.rate_hz(monotonic_now().0) as f32;
    diagnostics.capture_state = format!("{:?}", capture.state());
    diagnostics.slot_overwrites = metrics.frames_dropped;
    diagnostics.camera_backend = Some(camera_backend_name().to_string());
    if let Some(err) = metrics.last_error {
        diagnostics.last_error = Some(err);
    }
}

fn camera_backend_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "MSMF"
    }
    #[cfg(target_os = "macos")]
    {
        "AVFoundation"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        "Mock"
    }
}

/// Updates one reusable Bevy image from the latest captured frame.
///
/// This system is display-only: it never mutates the source frame and does
/// not mirror or otherwise transform the bytes used by inference. Updates are
/// throttled by [`PreviewState::target_fps`], while capture continues when the
/// preview is hidden.
pub fn update_preview_texture_system(
    latest: Res<LatestVideoFrame>,
    mut preview: ResMut<PreviewState>,
    mut images: ResMut<Assets<Image>>,
    mut last_upload: Local<Option<std::time::Instant>>,
) {
    if !preview.visible {
        return;
    }
    let Some(frame) = latest.frame.as_ref() else {
        return;
    };
    let now = std::time::Instant::now();
    if last_upload.is_some_and(|previous| now.duration_since(previous) < preview.update_interval())
    {
        return;
    }

    let Some(image) = preview_image(frame) else {
        return;
    };

    if let Some(handle) = preview.image_handle.as_ref() {
        if let Some(mut existing) = images.get_mut(handle.id()) {
            *existing = image;
        }
    } else {
        preview.image_handle = Some(images.add(image));
    }
    *last_upload = Some(now);
}

/// Registers the reusable preview image with egui once it exists.
pub fn register_preview_texture_system(
    preview: Res<PreviewState>,
    mut textures: ResMut<EguiUserTextures>,
) {
    if let Some(handle) = preview.image_handle.as_ref()
        && textures.image_id(handle.id()).is_none()
    {
        textures.add_image(EguiTextureHandle::Weak(handle.id()));
    }
}

fn preview_image(frame: &VideoFrame) -> Option<Image> {
    let rgba = frame_to_rgba(frame)?;
    let size = bevy::render::render_resource::Extent3d {
        width: frame.width,
        height: frame.height,
        depth_or_array_layers: 1,
    };
    Some(Image::new_fill(
        size,
        bevy::render::render_resource::TextureDimension::D2,
        &rgba,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        // Preview pixels change every frame, so the CPU-side asset must stay
        // in the main world while also being extracted for rendering.
        bevy::asset::RenderAssetUsages::default(),
    ))
}

fn frame_to_rgba(frame: &VideoFrame) -> Option<Vec<u8>> {
    let channels = match frame.format {
        vtuber_core::PixelFormat::Gray8 => 1,
        vtuber_core::PixelFormat::Rgb8 | vtuber_core::PixelFormat::Bgr8 => 3,
        vtuber_core::PixelFormat::Rgba8 => 4,
    };
    let row_bytes = frame.width as usize * channels;
    if frame.stride_bytes < row_bytes
        || frame.data.len() < frame.stride_bytes.saturating_mul(frame.height as usize)
    {
        return None;
    }
    let mut rgba = Vec::with_capacity(frame.width as usize * frame.height as usize * 4);
    for y in 0..frame.height as usize {
        let row = &frame.data[y * frame.stride_bytes..y * frame.stride_bytes + row_bytes];
        for pixel in row.chunks_exact(channels) {
            match frame.format {
                vtuber_core::PixelFormat::Gray8 => {
                    rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 255])
                }
                vtuber_core::PixelFormat::Rgb8 => {
                    rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255])
                }
                vtuber_core::PixelFormat::Bgr8 => {
                    rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255])
                }
                vtuber_core::PixelFormat::Rgba8 => rgba.extend_from_slice(pixel),
            }
        }
    }
    Some(rgba)
}

/// Bridge system that connects the orchestrator's capture intent to the
/// [`CaptureRuntime`].
///
/// - Enumerates cameras when the orchestrator signals a refresh.
/// - Starts capture when the orchestrator transitions to `Starting`.
/// - Stops capture when the orchestrator transitions to `Stopping`.
/// - Updates the orchestrator's pipeline state based on capture state.
pub fn capture_bridge_system(
    mut capture: ResMut<CaptureRuntime>,
    mut orchestrator: ResMut<crate::orchestrator::Orchestrator>,
) {
    use crate::orchestrator::PipelineState;

    if capture.controller().worker_finished()
        && matches!(
            capture.state(),
            CaptureServiceState::Starting | CaptureServiceState::Running
        )
        && orchestrator.capture_desired()
    {
        orchestrator.fail_camera("capture worker exited unexpectedly".to_string());
    }

    // Handle camera enumeration refresh independently from start/stop
    // acknowledgements. Refreshing while the pipeline is running must not
    // accidentally stop or restart the worker.
    if orchestrator.camera_refresh_requested() {
        let cameras = capture.enumerate_cameras();
        orchestrator.set_camera_list(cameras);
        orchestrator.clear_camera_refresh_request();
    }

    // Handle capture start.
    if orchestrator.capture_desired()
        && orchestrator.pipeline_state() == PipelineState::Starting
        && !orchestrator.capture_ack()
    {
        if let Some(device) = orchestrator.selected_camera_descriptor() {
            let request = vtuber_camera::CameraRequest::default();
            match capture.start_capture(device, request) {
                Ok(()) => {
                    orchestrator.set_capture_ack(true);
                    orchestrator.set_last_error(None);
                }
                Err(e) => {
                    orchestrator.set_pipeline_state(PipelineState::Failed);
                    orchestrator.set_capture_ack(true);
                    orchestrator.set_last_error(Some(
                        crate::orchestrator::OrchestratorError::CameraFailed(e),
                    ));
                }
            }
        } else {
            orchestrator.set_pipeline_state(PipelineState::Failed);
            orchestrator.set_capture_ack(true);
            orchestrator.set_last_error(Some(
                crate::orchestrator::OrchestratorError::NoCameraSelected,
            ));
        }
    }

    // Reflect the asynchronous worker state after a start command. A
    // successful command only means that opening was requested; Running is
    // reported after the worker has opened the selected device and produced
    // its first frame.
    if orchestrator.capture_desired()
        && orchestrator.pipeline_state() == PipelineState::Starting
        && capture.state() == CaptureServiceState::Running
    {
        orchestrator.set_pipeline_state(PipelineState::Running);
    } else if orchestrator.capture_desired() && capture.state() == CaptureServiceState::BackOff {
        orchestrator.set_pipeline_state(PipelineState::Failed);
        orchestrator.set_last_error(Some(crate::orchestrator::OrchestratorError::CameraFailed(
            "camera entered back-off".to_string(),
        )));
    }

    // Handle capture stop.
    if !orchestrator.capture_desired()
        && orchestrator.pipeline_state() == PipelineState::Stopping
        && !orchestrator.capture_ack()
    {
        capture.stop_capture();
        orchestrator.set_capture_ack(true);
    }

    if !orchestrator.capture_desired()
        && orchestrator.pipeline_state() == PipelineState::Stopping
        && matches!(
            capture.state(),
            CaptureServiceState::Selected | CaptureServiceState::Idle
        )
    {
        orchestrator.complete_capture_stop();
    }
}

#[cfg(test)]
mod preview_tests {
    use std::sync::Arc;

    use super::*;
    use vtuber_core::{MonoTimeNs, PixelFormat};

    fn rgb_frame() -> VideoFrame {
        VideoFrame {
            seq: FrameSeq(1),
            captured_at: MonoTimeNs(1),
            width: 1,
            height: 1,
            stride_bytes: 3,
            format: PixelFormat::Rgb8,
            data: Arc::from([10, 20, 30]),
        }
    }

    #[test]
    fn preview_image_remains_mutable_in_main_world() {
        let image = preview_image(&rgb_frame()).expect("valid RGB frame should produce an image");
        assert!(
            image
                .asset_usage
                .contains(bevy::asset::RenderAssetUsages::MAIN_WORLD)
        );
        assert!(
            image
                .asset_usage
                .contains(bevy::asset::RenderAssetUsages::RENDER_WORLD)
        );
    }

    #[test]
    fn preview_handle_is_registered_for_egui() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .init_resource::<PreviewState>()
            .init_resource::<EguiUserTextures>()
            .add_systems(Update, register_preview_texture_system);
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(preview_image(&rgb_frame()).expect("valid preview image"));
        app.world_mut().resource_mut::<PreviewState>().image_handle = Some(handle.clone());

        app.update();

        assert!(
            app.world()
                .resource::<EguiUserTextures>()
                .image_id(handle.id())
                .is_some()
        );
    }
}
