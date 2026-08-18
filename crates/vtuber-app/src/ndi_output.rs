//! Orchestration bridge and UI snapshot for optional NDI output.
//!
//! This module owns no sender handles in the UI model. It translates explicit
//! UI intent into the `vtuber-ndi` controller, coordinates the avatar output
//! resources, and publishes a small immutable snapshot for egui.

use bevy::prelude::*;
use vtuber_avatar::{AvatarOutputFrameSlot, AvatarOutputState};
use vtuber_ndi::{NdiOutputConfig, NdiOutputController, NdiOutputStatus};

use crate::orchestrator::Orchestrator;
use crate::ui_model::{NdiOutputUiState, UiViewModel};

/// Session-local desired state emitted by explicit Start/Stop UI actions.
#[derive(Resource, Debug, Default)]
pub struct NdiOutputIntent {
    requested: bool,
    generation: u64,
}

impl NdiOutputIntent {
    /// Requests one sender start. Repeated requests in the same session are
    /// intentionally idempotent.
    pub fn request_start(&mut self) {
        if !self.requested {
            self.generation = self.generation.saturating_add(1);
            self.requested = true;
        }
    }

    /// Requests sender stop and deactivates future output submissions.
    pub fn request_stop(&mut self) {
        self.requested = false;
    }

    /// Returns whether the current intent wants a sender.
    #[must_use]
    pub const fn is_requested(&self) -> bool {
        self.requested
    }

    /// Returns the monotonically increasing start-intent generation.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

/// App-owned resource that retains the NDI controller and start-attempt state.
#[derive(Resource, Debug)]
pub struct NdiOutputRuntime {
    controller: NdiOutputController,
    attempted_generation: Option<u64>,
}

impl Default for NdiOutputRuntime {
    fn default() -> Self {
        Self {
            controller: NdiOutputController::new(),
            attempted_generation: None,
        }
    }
}

impl NdiOutputRuntime {
    /// Returns the transport-neutral backend status.
    #[must_use]
    pub fn status(&self) -> NdiOutputStatus {
        self.controller.status()
    }

    /// Returns the bounded sender metrics.
    #[must_use]
    pub fn metrics(&self) -> vtuber_ndi::NdiOutputMetrics {
        self.controller.metrics()
    }

    fn start_for_generation(
        &mut self,
        generation: u64,
        config: NdiOutputConfig,
    ) -> Result<(), vtuber_ndi::NdiOutputError> {
        self.attempted_generation = Some(generation);
        match self.controller.start(config) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.attempted_generation = None;
                Err(error)
            }
        }
    }

    fn stop(&mut self) {
        let _ = self.controller.stop();
        self.attempted_generation = None;
    }

    fn submit_frame(&self, frame: vtuber_core::VideoOutputFrame) {
        let _ = self.controller.submit_frame(frame);
    }
}

/// Converts backend state and small metrics into the UI-only snapshot.
pub fn sync_ndi_output_view_model_system(
    runtime: Res<NdiOutputRuntime>,
    mut view_model: ResMut<UiViewModel>,
) {
    let status = runtime.status();
    let metrics = runtime.metrics();
    let default_source = NdiOutputConfig::default().source_name;
    let ndi = &mut view_model.ndi_output;
    ndi.available = vtuber_ndi::is_sdk_feature_enabled();
    ndi.source_name = Some(default_source);
    ndi.connections = None;
    ndi.error_code = None;
    ndi.error_message = None;
    ndi.state = match status {
        NdiOutputStatus::Off => NdiOutputUiState::Off,
        NdiOutputStatus::Starting => NdiOutputUiState::Starting,
        NdiOutputStatus::Live {
            connections,
            source_name,
        } => {
            ndi.source_name = Some(source_name);
            ndi.connections = Some(connections);
            NdiOutputUiState::Live
        }
        NdiOutputStatus::Error { code, message } => {
            ndi.error_code = Some(code.as_str().to_owned());
            ndi.error_message = Some(message);
            NdiOutputUiState::Error
        }
    };
    ndi.dropped_frames = metrics.dropped_frames;
    ndi.replaced_frames = metrics.replaced_frames;
}

/// Coordinates UI intent, avatar readiness, readback activation, and sender
/// submission without coupling tracking/camera lifecycle to NDI output.
pub fn ndi_output_bridge_system(
    mut runtime: ResMut<NdiOutputRuntime>,
    mut intent: ResMut<NdiOutputIntent>,
    orchestrator: Res<Orchestrator>,
    lifecycle: Option<Res<vtuber_avatar::AvatarLifecycle>>,
    mut output_state: Option<ResMut<AvatarOutputState>>,
    mut frame_slot: Option<ResMut<AvatarOutputFrameSlot>>,
) {
    let (Some(lifecycle), Some(output_state), Some(frame_slot)) = (
        lifecycle,
        output_state.as_deref_mut(),
        frame_slot.as_deref_mut(),
    ) else {
        return;
    };
    let ready = lifecycle.state() == vtuber_avatar::AvatarLifecycleState::Ready;
    let has_model = orchestrator.has_imported_model();
    let status = runtime.status();

    // An explicit unload leaves no avatar to publish and therefore performs
    // the initial policy's automatic sender stop. A replacement keeps the
    // sender but deactivates readback until the new generation is ready.
    if !has_model {
        intent.request_stop();
        output_state.deactivate();
        runtime.stop();
        let _ = frame_slot.take_latest();
        return;
    }

    if !intent.is_requested() {
        output_state.deactivate();
        if matches!(
            status,
            NdiOutputStatus::Starting | NdiOutputStatus::Live { .. }
        ) {
            runtime.stop();
        }
        let _ = frame_slot.take_latest();
        return;
    }

    if !ready {
        // Keep a live session across VRM replacement, but never forward a
        // frame from the old generation while the replacement is loading.
        output_state.deactivate();
        let _ = frame_slot.take_latest();
        if matches!(status, NdiOutputStatus::Off) {
            intent.request_stop();
        }
        return;
    }

    if matches!(status, NdiOutputStatus::Off | NdiOutputStatus::Error { .. })
        && runtime.attempted_generation != Some(intent.generation())
        && runtime
            .start_for_generation(intent.generation(), NdiOutputConfig::default())
            .is_err()
    {
        // Keep the backend's typed error visible for the Live section but
        // prevent an automatic retry loop on every Bevy frame.
        intent.request_stop();
    }

    match runtime.status() {
        NdiOutputStatus::Live { .. } => {
            output_state.activate();
            if let Some(frame) = frame_slot.take_latest() {
                runtime.submit_frame(frame);
            }
        }
        NdiOutputStatus::Starting => output_state.deactivate(),
        NdiOutputStatus::Error { .. } => {
            output_state.deactivate();
            intent.request_stop();
            let _ = frame_slot.take_latest();
        }
        NdiOutputStatus::Off => output_state.deactivate(),
    }
}

/// Stops NDI before the app's other worker shutdown paths complete.
pub fn shutdown_ndi_output(
    mut runtime: Option<ResMut<NdiOutputRuntime>>,
    mut output_state: Option<ResMut<AvatarOutputState>>,
) {
    if let Some(runtime) = runtime.as_deref_mut() {
        runtime.stop();
    }
    if let Some(output_state) = output_state.as_deref_mut() {
        output_state.deactivate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_start_is_idempotent_and_stop_clears_request() {
        let mut intent = NdiOutputIntent::default();
        intent.request_start();
        let generation = intent.generation();
        intent.request_start();
        assert_eq!(intent.generation(), generation);
        assert!(intent.is_requested());
        intent.request_stop();
        assert!(!intent.is_requested());
    }

    #[test]
    fn default_view_model_is_off_and_small() {
        let vm = crate::ui_model::NdiOutputViewModel::default();
        assert_eq!(vm.state, NdiOutputUiState::Off);
        assert!(vm.source_name.is_none());
        assert_eq!(vm.dropped_frames, 0);
    }

    #[cfg(not(feature = "ndi-output"))]
    #[test]
    fn feature_disabled_backend_is_not_available() {
        assert!(!vtuber_ndi::is_sdk_feature_enabled());
    }

    #[test]
    fn backend_error_code_is_not_published_as_app_lifecycle_failure() {
        assert_eq!(
            vtuber_ndi::NdiErrorCode::FeatureDisabled.as_str(),
            "NDI_FEATURE_DISABLED"
        );
    }
}
