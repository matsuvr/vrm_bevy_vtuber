//! App orchestrator — processes UI actions and manages domain state.
//!
//! The orchestrator receives [`UiAction`] commands from the UI layer and
//! translates them into domain service calls (camera, import, tracking, etc.).
//! It updates the [`UiViewModel`] snapshot that the UI reads each frame.

use std::path::PathBuf;

use bevy::prelude::*;

use crate::actions::UiAction;
use crate::import::{self, ImportedModel, ModelImportError};
use crate::ui::UiState;
use crate::ui_model::*;

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
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ImportFailed(msg) => write!(f, "Import failed: {msg}"),
            Self::NoCameraSelected => write!(f, "No camera selected"),
            Self::NoAvatarLoaded => write!(f, "No avatar loaded"),
            Self::PipelineAlreadyRunning => write!(f, "Pipeline already running"),
            Self::PipelineNotRunning => write!(f, "Pipeline not running"),
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
            UiAction::RefreshCameras => {
                self.refresh_cameras();
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
                self.last_error = None;
                // Retry logic depends on the error type.
            }
            _ => {
                // Other actions handled by specific subsystems.
            }
        }
    }

    /// Refresh the list of available cameras.
    fn refresh_cameras(&mut self) {
        // TODO: Call actual camera enumeration when vtuber-camera is integrated.
        // For now, provide a stub.
        self.cameras = vec![
            CameraDescriptor {
                name: "Camera 0 (stub)".to_string(),
                index: 0,
            },
            CameraDescriptor {
                name: "Camera 1 (stub)".to_string(),
                index: 1,
            },
        ];
    }

    /// Select a camera by index.
    fn select_camera(&mut self, index: usize) {
        if index < self.cameras.len() {
            self.selected_camera = Some(index);
        }
    }

    /// Import an avatar from the given path.
    fn import_avatar(&mut self, path: &PathBuf) {
        self.import_state = ImportState::InProgress;
        self.last_error = None;

        match import::import_vrm(path, &self.asset_root, import::DEFAULT_SIZE_LIMIT) {
            Ok(model) => {
                self.imported_model = Some(model);
                self.import_state = ImportState::Success;
            }
            Err(e) => {
                let msg = format_import_error(&e);
                self.import_state = ImportState::Failed(msg.clone());
                self.last_error = Some(OrchestratorError::ImportFailed(msg));
            }
        }
    }

    /// Unload the current avatar.
    fn unload_avatar(&mut self) {
        self.imported_model = None;
        self.import_state = ImportState::Idle;
    }

    /// Start the tracking pipeline.
    fn start_pipeline(&mut self) {
        if self.selected_camera.is_none() {
            self.last_error = Some(OrchestratorError::NoCameraSelected);
            return;
        }
        if self.imported_model.is_none() {
            self.last_error = Some(OrchestratorError::NoAvatarLoaded);
        }
        // TODO: Actually start capture → inference → tracking workers.
    }

    /// Stop the tracking pipeline.
    fn stop_pipeline(&mut self) {
        // TODO: Actually stop workers in reverse order.
    }

    /// Update the UI view model from current orchestrator state.
    pub fn update_view_model(&self, vm: &mut UiViewModel) {
        // Camera.
        vm.camera.available_cameras = self.cameras.clone();
        vm.camera.selected_index = self.selected_camera;

        // Avatar.
        vm.avatar.imported_model = self.imported_model.as_ref().map(|m| ImportedModelSummary {
            id: m.id.clone(),
            name: m.name.clone(),
            original_path: m.original_path.clone(),
            has_required_bones: m.summary.humanoid_nodes.hips < 1000
                && m.summary.humanoid_nodes.head < 1000,
            expression_count: m.summary.expression_presets.len(),
        });
        vm.avatar.is_ready = self.imported_model.is_some();
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
        // Cameras should be populated (stub returns 2).
        assert_eq!(orch.cameras.len(), 2);
    }

    #[test]
    fn orchestrator_select_camera() {
        let mut orch = Orchestrator::default();
        orch.process_action(&UiAction::RefreshCameras);
        orch.process_action(&UiAction::SelectCamera { index: 0 });
        assert_eq!(orch.selected_camera, Some(0));
    }

    #[test]
    fn orchestrator_select_invalid_camera_ignored() {
        let mut orch = Orchestrator::default();
        orch.process_action(&UiAction::RefreshCameras);
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
}
