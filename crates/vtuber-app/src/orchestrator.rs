//! App orchestrator — processes UI actions and manages domain state.
//!
//! The orchestrator receives [`UiAction`] commands from the UI layer and
//! translates them into domain service calls (camera, import, tracking, etc.).
//! It updates the [`UiViewModel`] snapshot that the UI reads each frame.
//!
//! Avatar loading is bridged to the `vtuber-avatar` lifecycle through a
//! pending-request protocol: after a successful import the orchestrator stores
//! a [`PendingLoadRequest`]; a Bevy system (in the same crate) drains it and
//! emits the corresponding `LoadImportedAvatarRequest` message that the avatar
//! plugin consumes.

use std::path::PathBuf;

use bevy::prelude::*;

use crate::actions::UiAction;
use crate::import::{self, ImportedModel, ModelImportError};
use crate::ui::UiState;
use crate::ui_model::*;
use vtuber_camera::device::CameraDescriptor;

/// A pending avatar load request waiting to be submitted to the lifecycle.
///
/// After a successful `import_vrm()` call the orchestrator stores the
/// [`ImportedModel`] here. A Bevy system drains this value and emits a
/// `LoadImportedAvatarRequest` message that the avatar plugin consumes.
#[derive(Clone, Debug)]
pub struct PendingLoadRequest {
    /// Monotonically increasing correlation identifier.
    pub request_id: u64,
    /// The imported model to load.
    pub model: ImportedModel,
}

/// Resource managing the application orchestration state.
#[derive(Resource, Debug)]
pub struct Orchestrator {
    /// Asset root for imported models.
    asset_root: PathBuf,
    /// Current import state.
    import_state: ImportState,
    /// Imported model, if any.
    imported_model: Option<ImportedModel>,
    /// Camera descriptors.
    cameras: Vec<CameraDescriptor>,
    /// Selected camera index.
    selected_camera: Option<usize>,
    /// Last error, if any.
    last_error: Option<OrchestratorError>,
    /// Pipeline lifecycle state.
    pipeline_state: PipelineState,
    /// Current UI screen.
    current_screen: Screen,
    /// Pending avatar load request not yet submitted to the lifecycle.
    pending_load: Option<PendingLoadRequest>,
    /// Next avatar load request correlation identifier.
    next_load_request_id: u64,
    /// Mirror of the avatar lifecycle state, updated by the sync system.
    lifecycle_state: crate::ui_model::AvatarLifecycleState,
    /// Whether capture should be running (set by Start/Stop actions).
    capture_desired: bool,
    /// Whether the capture system has acknowledged the current intent.
    capture_ack: bool,
    /// Whether a camera enumeration was explicitly requested.
    camera_refresh_requested: bool,
    /// Calibration command waiting for the tracking bridge.
    calibration_request: Option<CalibrationRequest>,
    /// Whether inference should be restarted after a recoverable worker error.
    inference_retry_requested: bool,
}

/// Calibration intent passed from UI orchestration to the tracking domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationRequest {
    /// Start a fresh neutral collection.
    Begin,
    /// Cancel and clear the current collection.
    Cancel,
    /// Retry after resetting the current collection.
    Retry,
}

/// State of the tracking pipeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PipelineState {
    /// Pipeline is idle.
    #[default]
    Idle,
    /// Pipeline is starting up.
    Starting,
    /// Pipeline is running.
    Running,
    /// Pipeline is stopping.
    Stopping,
    /// Pipeline failed to start or crashed.
    Failed,
}

/// State of an in-progress or completed import.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ImportState {
    /// No import in progress.
    #[default]
    Idle,
    /// Import is in progress.
    InProgress,
    /// Import completed successfully.
    Success,
    /// Import failed.
    Failed(String),
}

/// Errors that can occur in the orchestrator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrchestratorError {
    /// Model import failed.
    ImportFailed(String),
    /// No camera selected.
    NoCameraSelected,
    /// No avatar loaded.
    NoAvatarLoaded,
    /// Pipeline already running.
    PipelineAlreadyRunning,
    /// Pipeline not running.
    PipelineNotRunning,
    /// The avatar lifecycle rejected an imported model load request.
    AvatarLoadRejected(String),
    /// The avatar lifecycle entered a failed state while loading or binding.
    AvatarLifecycleFailed(String),
    /// Camera enumeration, opening, capture, or reconnect failed.
    CameraFailed(String),
    /// Inference model load, execution, or worker failure.
    InferenceFailed(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ImportFailed(msg) => write!(f, "Import failed: {msg}"),
            Self::NoCameraSelected => write!(f, "No camera selected"),
            Self::NoAvatarLoaded => write!(f, "No avatar loaded"),
            Self::PipelineAlreadyRunning => write!(f, "Pipeline already running"),
            Self::PipelineNotRunning => write!(f, "Pipeline not running"),
            Self::AvatarLoadRejected(msg) => write!(f, "Avatar load rejected: {msg}"),
            Self::AvatarLifecycleFailed(msg) => write!(f, "Avatar lifecycle failed: {msg}"),
            Self::CameraFailed(msg) => write!(f, "Camera failed: {msg}"),
            Self::InferenceFailed(msg) => write!(f, "Inference failed: {msg}"),
        }
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self {
            asset_root: PathBuf::from("assets"),
            import_state: ImportState::Idle,
            imported_model: None,
            cameras: Vec::new(),
            selected_camera: None,
            last_error: None,
            pipeline_state: PipelineState::Idle,
            current_screen: Screen::default(),
            pending_load: None,
            next_load_request_id: 1,
            lifecycle_state: AvatarLifecycleState::None,
            capture_desired: false,
            capture_ack: true,
            camera_refresh_requested: false,
            calibration_request: None,
            inference_retry_requested: false,
        }
    }
}

impl Orchestrator {
    /// Create a new orchestrator with the given asset root.
    #[must_use]
    pub fn new(asset_root: PathBuf) -> Self {
        Self {
            asset_root,
            ..Default::default()
        }
    }

    /// Process a UI action and update internal state.
    pub fn process_action(&mut self, action: &UiAction) {
        match action {
            UiAction::SwitchScreen(screen) => {
                self.current_screen = *screen;
            }
            UiAction::RefreshCameras => {
                // Camera enumeration is handled by the capture bridge system,
                // never by the UI renderer or a fabricated device list.
                self.camera_refresh_requested = true;
            }
            UiAction::SelectCamera { index } => {
                self.select_camera(*index);
            }
            UiAction::ImportAvatar { path } => {
                self.import_avatar(path);
            }
            UiAction::UnloadAvatar => {
                self.unload_avatar();
            }
            UiAction::Start => {
                self.start_pipeline();
            }
            UiAction::Stop => {
                self.stop_pipeline();
            }
            UiAction::DismissError => {
                self.last_error = None;
            }
            UiAction::RetryAfterError => {
                if matches!(self.last_error, Some(OrchestratorError::InferenceFailed(_))) {
                    self.inference_retry_requested = true;
                    self.last_error = None;
                    if self.capture_desired {
                        self.pipeline_state = PipelineState::Starting;
                        // Capture is still owned by the current session; only
                        // the inference worker needs to be restarted.
                        self.capture_ack = true;
                    } else if self.selected_camera.is_some() {
                        // An inference failure stops capture as part of
                        // recovery. The retry must therefore request a fresh
                        // camera start instead of reusing a stopped session.
                        self.pipeline_state = PipelineState::Starting;
                        self.capture_desired = true;
                        self.capture_ack = false;
                    }
                }
                self.retry_avatar_load();
            }
            UiAction::BeginCalibration => {
                if self.pipeline_state == PipelineState::Running {
                    self.calibration_request = Some(CalibrationRequest::Begin);
                }
            }
            UiAction::CancelCalibration => {
                self.calibration_request = Some(CalibrationRequest::Cancel);
            }
            UiAction::RetryCalibration if self.pipeline_state == PipelineState::Running => {
                self.calibration_request = Some(CalibrationRequest::Retry);
            }
            _ => {
                // Other actions handled by specific subsystems.
            }
        }
    }

    /// Refresh the list of available cameras from an external source.
    pub fn set_camera_list(&mut self, cameras: Vec<CameraDescriptor>) {
        self.cameras = cameras;
        // If the previously selected index is now out of range, clear it.
        if let Some(idx) = self.selected_camera
            && idx >= self.cameras.len()
        {
            self.selected_camera = None;
        }
    }

    /// Select a camera by index.
    fn select_camera(&mut self, index: usize) {
        if index < self.cameras.len() {
            self.selected_camera = Some(index);
        }
    }

    /// Import an avatar from the given path.
    ///
    /// On success the imported model is stored and a [`PendingLoadRequest`] is
    /// queued. The sync system drains the pending request and emits a
    /// `LoadImportedAvatarRequest` that the avatar lifecycle consumes.
    fn import_avatar(&mut self, path: &PathBuf) {
        self.import_state = ImportState::InProgress;
        self.last_error = None;

        match import::import_vrm(path, &self.asset_root, import::DEFAULT_SIZE_LIMIT) {
            Ok(model) => {
                let request_id = self.next_load_request_id;
                self.next_load_request_id += 1;
                self.pending_load = Some(PendingLoadRequest {
                    request_id,
                    model: model.clone(),
                });
                self.imported_model = Some(model);
                self.import_state = ImportState::Success;
                // Reset lifecycle from any previous Failed state so the new
                // load can proceed.
                if self.lifecycle_state == AvatarLifecycleState::Failed {
                    self.lifecycle_state = AvatarLifecycleState::None;
                }
            }
            Err(e) => {
                let msg = format_import_error(&e);
                self.import_state = ImportState::Failed(msg.clone());
                self.last_error = Some(OrchestratorError::ImportFailed(msg));
            }
        }
    }

    /// Unload the current avatar.
    ///
    /// Clears the imported model and any pending load request. The sync system
    /// detects the removal and emits an `UnloadAvatarRequest`.
    fn unload_avatar(&mut self) {
        self.imported_model = None;
        self.import_state = ImportState::Idle;
        self.pending_load = None;
    }

    /// Retry a failed avatar load by re-submitting the current imported model.
    fn retry_avatar_load(&mut self) {
        if self.lifecycle_state != AvatarLifecycleState::Failed {
            return;
        }
        if let Some(model) = self.imported_model.clone() {
            let request_id = self.next_load_request_id;
            self.next_load_request_id += 1;
            self.pending_load = Some(PendingLoadRequest { request_id, model });
            self.last_error = None;
            self.lifecycle_state = AvatarLifecycleState::None;
        }
    }

    /// Start the tracking pipeline.
    fn start_pipeline(&mut self) {
        if self.pipeline_state == PipelineState::Running
            || self.pipeline_state == PipelineState::Starting
        {
            self.last_error = Some(OrchestratorError::PipelineAlreadyRunning);
            return;
        }
        if self.selected_camera.is_none() {
            self.last_error = Some(OrchestratorError::NoCameraSelected);
            return;
        }
        if self.imported_model.is_none() {
            self.last_error = Some(OrchestratorError::NoAvatarLoaded);
            return;
        }
        self.pipeline_state = PipelineState::Starting;
        self.capture_desired = true;
        self.capture_ack = false;
        self.inference_retry_requested = false;
    }

    /// Stop the tracking pipeline.
    fn stop_pipeline(&mut self) {
        if self.pipeline_state == PipelineState::Idle
            || self.pipeline_state == PipelineState::Stopping
        {
            return;
        }
        self.pipeline_state = PipelineState::Stopping;
        self.capture_desired = false;
        self.capture_ack = false;
        self.inference_retry_requested = false;
    }

    /// Get the current pipeline state.
    #[must_use]
    pub fn pipeline_state(&self) -> PipelineState {
        self.pipeline_state
    }

    /// Update the UI view model from current orchestrator state.
    ///
    /// The avatar `lifecycle` and `is_ready` fields are driven by the
    /// lifecycle mirror maintained by the sync system, not by the presence of
    /// an imported model. A model becomes ready only after the `bevy_vrm1`
    /// asset has initialized and humanoid binding has completed.
    pub fn update_view_model(&self, vm: &mut UiViewModel) {
        // Screen.
        vm.screen = self.current_screen;

        // Lifecycle.
        vm.lifecycle = match self.pipeline_state {
            PipelineState::Idle => AppLifecycle::Idle,
            PipelineState::Starting => AppLifecycle::Starting,
            PipelineState::Running => AppLifecycle::Running,
            PipelineState::Stopping => AppLifecycle::Stopping,
            PipelineState::Failed => AppLifecycle::Failed,
        };

        // Camera — convert from vtuber_camera descriptors to UI model descriptors.
        vm.camera.available_cameras = self
            .cameras
            .iter()
            .enumerate()
            .map(|(i, c)| crate::ui_model::CameraDescriptor {
                name: c.label.clone(),
                index: i,
            })
            .collect();
        vm.camera.selected_index = self.selected_camera;

        // Avatar — imported model summary for display.
        vm.avatar.imported_model = self.imported_model.as_ref().map(|m| ImportedModelSummary {
            id: m.id.clone(),
            name: m.name.clone(),
            original_path: m.original_path.clone(),
            has_required_bones: m.summary.humanoid_nodes.hips < 1000
                && m.summary.humanoid_nodes.head < 1000,
            expression_count: m.summary.expression_presets.len(),
        });

        // Avatar lifecycle — driven by the sync system, not by import state.
        vm.avatar.lifecycle = self.lifecycle_state;
        vm.avatar.is_ready = self.lifecycle_state == AvatarLifecycleState::Ready;
        vm.avatar.load_failed = self.lifecycle_state == AvatarLifecycleState::Failed;
    }

    /// Get the last error, if any.
    #[must_use]
    pub fn last_error(&self) -> Option<&OrchestratorError> {
        self.last_error.as_ref()
    }

    /// Get the import state.
    #[must_use]
    pub fn import_state(&self) -> &ImportState {
        &self.import_state
    }

    /// Take the pending load request, if any.
    ///
    /// The sync system calls this once per request to obtain the data needed
    /// to construct a `LoadImportedAvatarRequest` message.
    pub fn take_pending_load_request(&mut self) -> Option<PendingLoadRequest> {
        self.pending_load.take()
    }

    /// Update the lifecycle state mirror.
    ///
    /// The sync system calls this after reading the `AvatarLifecycle` resource
    /// so that `update_view_model` can report the true lifecycle state.
    pub fn set_lifecycle_state(&mut self, state: AvatarLifecycleState) {
        self.lifecycle_state = state;
    }

    /// Whether the orchestrator has an imported model.
    #[must_use]
    pub fn has_imported_model(&self) -> bool {
        self.imported_model.is_some()
    }

    /// The asset root used for model imports.
    #[must_use]
    pub fn asset_root(&self) -> &PathBuf {
        &self.asset_root
    }

    /// Whether capture should be running.
    #[must_use]
    pub fn capture_desired(&self) -> bool {
        self.capture_desired
    }

    /// Whether the capture system has acknowledged the current intent.
    #[must_use]
    pub fn capture_ack(&self) -> bool {
        self.capture_ack
    }

    /// Whether a fresh camera enumeration is pending.
    #[must_use]
    pub fn camera_refresh_requested(&self) -> bool {
        self.camera_refresh_requested
    }

    /// Marks a camera enumeration request as handled.
    pub fn clear_camera_refresh_request(&mut self) {
        self.camera_refresh_requested = false;
    }

    /// Acknowledge the current capture state.
    pub fn set_capture_ack(&mut self, ack: bool) {
        self.capture_ack = ack;
    }

    /// Set the pipeline state (called by the capture bridge system).
    pub fn set_pipeline_state(&mut self, state: PipelineState) {
        self.pipeline_state = state;
    }

    /// Set the last error (called by the capture bridge system).
    pub fn set_last_error(&mut self, error: Option<OrchestratorError>) {
        self.last_error = error;
    }

    /// Moves an inference failure through the normal reverse-order shutdown.
    ///
    /// The inference bridge calls this after observing a failed worker. The
    /// capture bridge then stops the camera, while the inference bridge joins
    /// the failed inference worker first.
    pub fn fail_inference(&mut self, message: String) {
        self.last_error = Some(OrchestratorError::InferenceFailed(message));
        self.capture_desired = false;
        self.capture_ack = false;
        self.pipeline_state = PipelineState::Stopping;
    }

    /// Moves a capture-worker failure through the same recoverable shutdown.
    pub fn fail_camera(&mut self, message: String) {
        self.last_error = Some(OrchestratorError::CameraFailed(message));
        self.capture_desired = false;
        self.capture_ack = false;
        self.pipeline_state = PipelineState::Stopping;
    }

    /// Completes a capture stop while preserving a recoverable failure state.
    pub fn complete_capture_stop(&mut self) {
        if self.last_error.is_some() {
            self.pipeline_state = PipelineState::Failed;
        } else {
            self.pipeline_state = PipelineState::Idle;
        }
    }

    /// Get the selected camera descriptor, if any.
    #[must_use]
    pub fn selected_camera_descriptor(&self) -> Option<CameraDescriptor> {
        self.selected_camera
            .and_then(|idx| self.cameras.get(idx).cloned())
    }

    /// Takes the pending calibration intent, if any.
    pub fn take_calibration_request(&mut self) -> Option<CalibrationRequest> {
        self.calibration_request.take()
    }

    /// Takes a pending inference restart request.
    pub fn take_inference_retry_request(&mut self) -> bool {
        let requested = self.inference_retry_requested;
        self.inference_retry_requested = false;
        requested
    }
}

/// Format an import error for user display.
fn format_import_error(error: &ModelImportError) -> String {
    match error {
        ModelImportError::InvalidExtension => "File must have .vrm extension".to_string(),
        ModelImportError::NotRegularFile => "Not a regular file".to_string(),
        ModelImportError::SizeExceeded { size, limit } => {
            format!("File size ({size} bytes) exceeds limit ({limit} bytes)")
        }
        ModelImportError::NotVrm1 => "File is not a VRM 1.0 model".to_string(),
        ModelImportError::UnsupportedVersion(v) => format!("Unsupported VRM version: {v}"),
        ModelImportError::MissingRequiredBone(bone) => format!("Missing required bone: {bone}"),
        ModelImportError::GlbParse(msg) => format!("Failed to parse model: {msg}"),
        ModelImportError::ExternalUri(uri) => format!("External URI not allowed: {uri}"),
        ModelImportError::InvalidNodeIndex { index } => {
            format!("Invalid node index: {index}")
        }
        ModelImportError::Io(e) => format!("I/O error: {e}"),
        ModelImportError::LimitExceedsHardCap { .. } => "Configuration error".to_string(),
    }
}

/// System that processes pending UI actions through the orchestrator.
pub fn process_ui_actions_system(
    mut orchestrator: ResMut<Orchestrator>,
    mut ui_state: ResMut<UiState>,
    mut view_model: ResMut<UiViewModel>,
) {
    let actions = ui_state.take_actions();
    for action in &actions {
        orchestrator.process_action(action);
    }
    orchestrator.update_view_model(&mut view_model);
}

/// Converts the avatar lifecycle's internal state to the UI model's state.
fn map_avatar_lifecycle_state(
    state: vtuber_avatar::lifecycle::AvatarLifecycleState,
) -> AvatarLifecycleState {
    use vtuber_avatar::lifecycle::AvatarLifecycleState as Engine;
    match state {
        Engine::NoAvatar => AvatarLifecycleState::None,
        Engine::Loading => AvatarLifecycleState::Loading,
        Engine::Binding => AvatarLifecycleState::Binding,
        Engine::Ready => AvatarLifecycleState::Ready,
        Engine::Unloading => AvatarLifecycleState::Unloading,
        Engine::Failed => AvatarLifecycleState::Failed,
    }
}

/// System that bridges the orchestrator to the avatar lifecycle.
///
/// 1. Reads the [`AvatarLifecycle`] state and mirrors it into the orchestrator
///    so that `update_view_model` reports the true engine state.
/// 2. Drains any pending load request from the orchestrator and emits a
///    [`LoadImportedAvatarRequest`] message.
/// 3. Detects when the user has cleared the imported model while the lifecycle
///    still has an active avatar, and emits an [`UnloadAvatarRequest`].
///
/// This system must run after [`process_ui_actions_system`] so that it sees
/// the latest orchestrator mutations.
pub fn sync_avatar_lifecycle_system(
    mut orchestrator: ResMut<Orchestrator>,
    lifecycle: Res<vtuber_avatar::lifecycle::AvatarLifecycle>,
    mut load_requests: MessageWriter<vtuber_avatar::LoadImportedAvatarRequest>,
    mut load_results: MessageReader<vtuber_avatar::LoadImportedAvatarResult>,
    mut unload_requests: MessageWriter<vtuber_avatar::lifecycle::UnloadAvatarRequest>,
) {
    // 1. Mirror the lifecycle state into the orchestrator.
    let engine_state = lifecycle.state();
    let ui_state = map_avatar_lifecycle_state(engine_state);
    orchestrator.set_lifecycle_state(ui_state);

    // Consume the request result so a malformed path or lifecycle rejection
    // becomes a recoverable UI error instead of remaining invisible. Accepted
    // requests are intentionally not treated as ready: readiness is driven by
    // bevy_vrm1 initialization and humanoid binding below.
    for result in load_results.read() {
        if let vtuber_avatar::LoadImportedAvatarResult::Rejected { error, .. } = result {
            let message = error.to_string();
            orchestrator.set_last_error(Some(OrchestratorError::AvatarLoadRejected(message)));
            orchestrator.set_lifecycle_state(AvatarLifecycleState::Failed);
        }
    }

    // A load can also fail after acceptance, for example when the asset
    // handle never initializes or binding times out. Preserve that typed
    // reason for the UI instead of reducing every failure to `Failed`.
    if engine_state == vtuber_avatar::lifecycle::AvatarLifecycleState::Failed
        && !matches!(
            orchestrator.last_error(),
            Some(OrchestratorError::AvatarLoadRejected(_))
        )
    {
        let message = lifecycle
            .failure()
            .map(ToString::to_string)
            .unwrap_or_else(|| "avatar lifecycle failed without a recorded reason".to_string());
        orchestrator.set_last_error(Some(OrchestratorError::AvatarLifecycleFailed(message)));
    }

    // 2. Drain pending load requests.
    if let Some(pending) = orchestrator.take_pending_load_request() {
        let id = vtuber_avatar::AvatarAssetId::new(&pending.model.id);
        let asset_path = vtuber_avatar::UserAssetPath::avatar_model_path(&id);

        match asset_path {
            Ok(path) => {
                let imported =
                    vtuber_avatar::ImportedAvatar::new(id, path, pending.model.name.clone());
                load_requests.write(vtuber_avatar::LoadImportedAvatarRequest {
                    request_id: pending.request_id,
                    imported,
                });
            }
            Err(e) => {
                // Should never happen for a well-formed SHA-256 id, but handle
                // gracefully rather than panicking.
                bevy::log::error!(
                    "failed to construct user asset path for import {}: {e}",
                    pending.model.id
                );
                orchestrator.set_lifecycle_state(AvatarLifecycleState::Failed);
                orchestrator
                    .set_last_error(Some(OrchestratorError::AvatarLoadRejected(e.to_string())));
            }
        }
    }

    // 3. Detect unload: model cleared while lifecycle is still active.
    if !orchestrator.has_imported_model() {
        use vtuber_avatar::lifecycle::AvatarLifecycleState as Engine;
        match engine_state {
            Engine::Ready | Engine::Loading | Engine::Binding => {
                unload_requests.write(vtuber_avatar::lifecycle::UnloadAvatarRequest);
            }
            Engine::NoAvatar | Engine::Unloading | Engine::Failed => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orchestrator_default_state() {
        let orch = Orchestrator::default();
        assert_eq!(orch.import_state(), &ImportState::Idle);
        assert!(orch.last_error().is_none());
    }

    #[test]
    fn orchestrator_refresh_cameras() {
        let mut orch = Orchestrator::default();
        orch.process_action(&UiAction::RefreshCameras);
        // Refresh only signals enumeration; it must not manufacture devices.
        assert!(orch.cameras.is_empty());
        orch.set_camera_list(vec![
            CameraDescriptor {
                id: "test:0".into(),
                label: "Test camera 0".into(),
            },
            CameraDescriptor {
                id: "test:1".into(),
                label: "Test camera 1".into(),
            },
        ]);
        assert_eq!(orch.cameras.len(), 2);
    }

    #[test]
    fn orchestrator_select_camera() {
        let mut orch = Orchestrator::default();
        orch.process_action(&UiAction::RefreshCameras);
        orch.set_camera_list(vec![CameraDescriptor {
            id: "test:0".into(),
            label: "Test camera 0".into(),
        }]);
        orch.process_action(&UiAction::SelectCamera { index: 0 });
        assert_eq!(orch.selected_camera, Some(0));
    }

    #[test]
    fn orchestrator_select_invalid_camera_ignored() {
        let mut orch = Orchestrator::default();
        orch.process_action(&UiAction::RefreshCameras);
        orch.set_camera_list(vec![CameraDescriptor {
            id: "test:0".into(),
            label: "Test camera 0".into(),
        }]);
        orch.process_action(&UiAction::SelectCamera { index: 99 });
        assert_eq!(orch.selected_camera, None);
    }

    #[test]
    fn orchestrator_unload_avatar() {
        let mut orch = Orchestrator {
            imported_model: Some(ImportedModel {
                id: "test".into(),
                name: "test".into(),
                asset_path: PathBuf::new(),
                meta_path: PathBuf::new(),
                summary: Default::default(),
                original_path: PathBuf::new(),
                size: 0,
            }),
            ..Default::default()
        };
        orch.process_action(&UiAction::UnloadAvatar);
        assert!(orch.imported_model.is_none());
    }

    #[test]
    fn orchestrator_start_without_camera_sets_error() {
        let mut orch = Orchestrator {
            imported_model: Some(ImportedModel {
                id: "test".into(),
                name: "test".into(),
                asset_path: PathBuf::new(),
                meta_path: PathBuf::new(),
                summary: Default::default(),
                original_path: PathBuf::new(),
                size: 0,
            }),
            ..Default::default()
        };
        orch.process_action(&UiAction::Start);
        assert_eq!(
            orch.last_error(),
            Some(&OrchestratorError::NoCameraSelected)
        );
    }

    #[test]
    fn orchestrator_start_without_avatar_sets_error() {
        let mut orch = Orchestrator {
            selected_camera: Some(0),
            ..Default::default()
        };
        orch.process_action(&UiAction::Start);
        assert_eq!(orch.last_error(), Some(&OrchestratorError::NoAvatarLoaded));
    }

    #[test]
    fn orchestrator_dismiss_error() {
        let mut orch = Orchestrator {
            last_error: Some(OrchestratorError::NoCameraSelected),
            ..Default::default()
        };
        orch.process_action(&UiAction::DismissError);
        assert!(orch.last_error().is_none());
    }

    #[test]
    fn orchestrator_update_view_model() {
        let mut orch = Orchestrator::default();
        orch.process_action(&UiAction::RefreshCameras);
        orch.set_camera_list(vec![
            CameraDescriptor {
                id: "test:0".into(),
                label: "Test camera 0".into(),
            },
            CameraDescriptor {
                id: "test:1".into(),
                label: "Test camera 1".into(),
            },
        ]);
        orch.process_action(&UiAction::SelectCamera { index: 0 });

        let mut vm = UiViewModel::default();
        orch.update_view_model(&mut vm);

        assert_eq!(vm.camera.available_cameras.len(), 2);
        assert_eq!(vm.camera.selected_index, Some(0));
    }

    #[test]
    fn format_import_error_not_vrm1() {
        let err = ModelImportError::NotVrm1;
        let msg = format_import_error(&err);
        assert!(msg.contains("VRM 1.0"));
    }

    #[test]
    fn format_import_error_missing_bone() {
        let err = ModelImportError::MissingRequiredBone("hips".to_string());
        let msg = format_import_error(&err);
        assert!(msg.contains("hips"));
    }

    fn stub_imported_model() -> ImportedModel {
        ImportedModel {
            id: "abc123".into(),
            name: "Test Model".into(),
            asset_path: PathBuf::new(),
            meta_path: PathBuf::new(),
            summary: Default::default(),
            original_path: PathBuf::new(),
            size: 0,
        }
    }

    #[test]
    fn orchestrator_unload_clears_pending_load() {
        let mut orch = Orchestrator {
            imported_model: Some(stub_imported_model()),
            pending_load: Some(PendingLoadRequest {
                request_id: 1,
                model: stub_imported_model(),
            }),
            ..Default::default()
        };
        orch.process_action(&UiAction::UnloadAvatar);
        assert!(orch.imported_model.is_none());
        assert!(orch.take_pending_load_request().is_none());
    }

    #[test]
    fn orchestrator_retry_after_failure_creates_pending_load() {
        let model = stub_imported_model();
        let mut orch = Orchestrator {
            imported_model: Some(model.clone()),
            lifecycle_state: AvatarLifecycleState::Failed,
            ..Default::default()
        };
        orch.process_action(&UiAction::RetryAfterError);
        let pending = orch
            .take_pending_load_request()
            .expect("should have pending load");
        assert_eq!(pending.request_id, 1);
        assert_eq!(pending.model.id, model.id);
        assert_eq!(orch.lifecycle_state, AvatarLifecycleState::None);
    }

    #[test]
    fn orchestrator_retry_ignored_when_not_failed() {
        let mut orch = Orchestrator {
            imported_model: Some(stub_imported_model()),
            lifecycle_state: AvatarLifecycleState::Ready,
            ..Default::default()
        };
        orch.process_action(&UiAction::RetryAfterError);
        assert!(orch.take_pending_load_request().is_none());
    }

    #[test]
    fn orchestrator_retry_after_inference_failure_restarts_only_inference() {
        let mut orch = Orchestrator {
            imported_model: Some(stub_imported_model()),
            capture_desired: true,
            capture_ack: true,
            pipeline_state: PipelineState::Failed,
            last_error: Some(OrchestratorError::InferenceFailed("model failed".into())),
            ..Default::default()
        };

        orch.process_action(&UiAction::RetryAfterError);

        assert_eq!(orch.pipeline_state, PipelineState::Starting);
        assert!(orch.capture_desired);
        assert!(orch.capture_ack);
        assert!(orch.last_error.is_none());
        assert!(orch.take_inference_retry_request());
        assert!(!orch.take_inference_retry_request());
        assert!(orch.take_pending_load_request().is_none());
    }

    #[test]
    fn inference_failure_requests_reverse_order_capture_shutdown() {
        let mut orch = Orchestrator {
            capture_desired: true,
            capture_ack: true,
            pipeline_state: PipelineState::Running,
            ..Default::default()
        };

        orch.fail_inference("landmark failed".into());

        assert_eq!(orch.pipeline_state, PipelineState::Stopping);
        assert!(!orch.capture_desired);
        assert!(!orch.capture_ack);
        assert!(matches!(
            orch.last_error(),
            Some(OrchestratorError::InferenceFailed(message)) if message == "landmark failed"
        ));

        orch.complete_capture_stop();
        assert_eq!(orch.pipeline_state, PipelineState::Failed);
    }

    #[test]
    fn retry_after_failed_inference_restarts_capture_when_it_was_stopped() {
        let mut orch = Orchestrator {
            imported_model: Some(stub_imported_model()),
            selected_camera: Some(0),
            capture_desired: false,
            capture_ack: true,
            pipeline_state: PipelineState::Failed,
            last_error: Some(OrchestratorError::InferenceFailed("model failed".into())),
            ..Default::default()
        };

        orch.process_action(&UiAction::RetryAfterError);

        assert_eq!(orch.pipeline_state, PipelineState::Starting);
        assert!(orch.capture_desired);
        assert!(!orch.capture_ack);
        assert!(orch.take_inference_retry_request());
    }

    #[test]
    fn orchestrator_view_model_reflects_lifecycle_not_import() {
        let orch = Orchestrator {
            imported_model: Some(stub_imported_model()),
            lifecycle_state: AvatarLifecycleState::Loading,
            ..Default::default()
        };
        let mut vm = UiViewModel::default();
        orch.update_view_model(&mut vm);

        // Model is imported but lifecycle is Loading, so is_ready must be false.
        assert!(vm.avatar.imported_model.is_some());
        assert!(!vm.avatar.is_ready);
        assert_eq!(vm.avatar.lifecycle, AvatarLifecycleState::Loading);
        assert!(!vm.avatar.load_failed);
    }

    #[test]
    fn orchestrator_view_model_ready_only_when_lifecycle_ready() {
        let orch = Orchestrator {
            imported_model: Some(stub_imported_model()),
            lifecycle_state: AvatarLifecycleState::Ready,
            ..Default::default()
        };
        let mut vm = UiViewModel::default();
        orch.update_view_model(&mut vm);

        assert!(vm.avatar.is_ready);
        assert_eq!(vm.avatar.lifecycle, AvatarLifecycleState::Ready);
        assert!(!vm.avatar.load_failed);
    }

    #[test]
    fn orchestrator_view_model_failed_sets_load_failed() {
        let orch = Orchestrator {
            imported_model: Some(stub_imported_model()),
            lifecycle_state: AvatarLifecycleState::Failed,
            ..Default::default()
        };
        let mut vm = UiViewModel::default();
        orch.update_view_model(&mut vm);

        assert!(!vm.avatar.is_ready);
        assert!(vm.avatar.load_failed);
        assert_eq!(vm.avatar.lifecycle, AvatarLifecycleState::Failed);
    }

    #[test]
    fn map_lifecycle_state_round_trip() {
        use vtuber_avatar::lifecycle::AvatarLifecycleState as Engine;

        assert_eq!(
            map_avatar_lifecycle_state(Engine::NoAvatar),
            AvatarLifecycleState::None
        );
        assert_eq!(
            map_avatar_lifecycle_state(Engine::Loading),
            AvatarLifecycleState::Loading
        );
        assert_eq!(
            map_avatar_lifecycle_state(Engine::Binding),
            AvatarLifecycleState::Binding
        );
        assert_eq!(
            map_avatar_lifecycle_state(Engine::Ready),
            AvatarLifecycleState::Ready
        );
        assert_eq!(
            map_avatar_lifecycle_state(Engine::Unloading),
            AvatarLifecycleState::Unloading
        );
        assert_eq!(
            map_avatar_lifecycle_state(Engine::Failed),
            AvatarLifecycleState::Failed
        );
    }
}
