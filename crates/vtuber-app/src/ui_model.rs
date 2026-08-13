//! UI view models — immutable snapshots for rendering the UI.
//!
//! These types hide Bevy query details from the UI layer. The UI reads
//! these snapshots and emits [`crate::actions::UiAction`] commands.

use bevy::prelude::Resource;
use std::path::PathBuf;

/// Which screen the UI is currently showing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    /// Initial setup: camera selection, avatar import.
    #[default]
    Setup,
    /// Live preview with tracking active.
    Live,
    /// Performance diagnostics and metrics.
    Diagnostics,
}

/// Overall application lifecycle state for UI display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppLifecycle {
    /// Nothing configured yet.
    #[default]
    Idle,
    /// Workers are starting up.
    Starting,
    /// All workers running, tracking active.
    Running,
    /// Workers are shutting down.
    Stopping,
    /// A recoverable error occurred.
    Failed,
}

/// Camera state for the UI.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CameraViewModel {
    /// Available camera descriptors (display name, index).
    pub available_cameras: Vec<CameraDescriptor>,
    /// Currently selected camera index, if any.
    pub selected_index: Option<usize>,
    /// Whether the camera is currently capturing.
    pub is_capturing: bool,
    /// Camera backend name (e.g. "MSMF", "AVFoundation").
    pub backend: Option<String>,
    /// Active resolution (width x height).
    pub resolution: Option<(u32, u32)>,
}

/// A camera descriptor for display in the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraDescriptor {
    /// Human-readable camera name.
    pub name: String,
    /// Platform-specific index or identifier.
    pub index: usize,
}

/// Avatar state for the UI.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AvatarViewModel {
    /// Imported model summary, if any.
    pub imported_model: Option<ImportedModelSummary>,
    /// Avatar lifecycle state.
    pub lifecycle: AvatarLifecycleState,
    /// Whether the avatar is ready for tracking.
    pub is_ready: bool,
    /// Whether the avatar load/binding failed (recoverable).
    pub load_failed: bool,
}

/// Summary of an imported model for display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedModelSummary {
    /// Stable asset ID (SHA-256 short).
    pub id: String,
    /// Model name from VRM metadata.
    pub name: String,
    /// Original file path (for display only, not used for loading).
    pub original_path: PathBuf,
    /// Whether the model has all required bones.
    pub has_required_bones: bool,
    /// Available expression preset count.
    pub expression_count: usize,
}

/// Avatar lifecycle state for UI display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarLifecycleState {
    /// No avatar loaded.
    #[default]
    None,
    /// Avatar is loading.
    Loading,
    /// Avatar is binding bones.
    Binding,
    /// Avatar is ready.
    Ready,
    /// Avatar is unloading.
    Unloading,
    /// Avatar loading/binding failed.
    Failed,
}

/// Calibration state for the UI.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalibrationViewModel {
    /// Whether calibration is in progress.
    pub is_calibrating: bool,
    /// Number of samples collected so far.
    pub samples_collected: u32,
    /// Target number of samples.
    pub samples_target: u32,
    /// Quality score (0.0 to 1.0), if available.
    pub quality_score: Option<f32>,
    /// Last rejection reason, if any.
    pub last_reject_reason: Option<String>,
    /// Whether calibration completed successfully.
    pub is_complete: bool,
}

/// Tracking state for the UI.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrackingViewModel {
    /// Whether tracking is active.
    pub is_tracking: bool,
    /// Current tracking state (Tracking/Lost/Initializing).
    pub state: TrackingState,
    /// Current confidence (0.0 to 1.0).
    pub confidence: f32,
    /// Face detected.
    pub face_detected: bool,
}

/// Tracking state enum for UI display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackingState {
    /// Not tracking.
    #[default]
    Idle,
    /// Initializing / calibrating.
    Initializing,
    /// Actively tracking.
    Tracking,
    /// Face lost, attempting recovery.
    Lost,
}

/// Complete UI view model snapshot.
#[derive(Clone, Debug, Default, Resource)]
pub struct UiViewModel {
    /// Current screen.
    pub screen: Screen,
    /// Application lifecycle.
    pub lifecycle: AppLifecycle,
    /// Camera state.
    pub camera: CameraViewModel,
    /// Avatar state.
    pub avatar: AvatarViewModel,
    /// Calibration state.
    pub calibration: CalibrationViewModel,
    /// Tracking state.
    pub tracking: TrackingViewModel,
    /// Whether preview mirroring is enabled.
    pub mirror_preview: bool,
    /// Whether avatar motion is reflected for the operator.
    pub mirror_avatar_motion: bool,
    /// Whether preview is visible.
    pub preview_visible: bool,
}

impl UiViewModel {
    /// Check if the Start action is available.
    #[must_use]
    pub fn can_start(&self) -> bool {
        self.lifecycle == AppLifecycle::Idle
            && self.avatar.is_ready
            && self.camera.selected_index.is_some()
    }

    /// Check if the Stop action is available.
    #[must_use]
    pub fn can_stop(&self) -> bool {
        self.lifecycle == AppLifecycle::Running
    }

    /// Check if calibration can begin.
    #[must_use]
    pub fn can_calibrate(&self) -> bool {
        self.lifecycle == AppLifecycle::Running
            && !self.calibration.is_calibrating
            && !self.calibration.is_complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_model_default_is_idle() {
        let vm = UiViewModel::default();
        assert_eq!(vm.lifecycle, AppLifecycle::Idle);
        assert_eq!(vm.screen, Screen::Setup);
        assert!(!vm.can_start());
        assert!(!vm.can_stop());
    }

    #[test]
    fn ui_model_can_start_when_ready() {
        let mut vm = UiViewModel::default();
        vm.avatar.is_ready = true;
        vm.camera.selected_index = Some(0);
        assert!(vm.can_start());
    }

    #[test]
    fn ui_model_cannot_start_without_camera() {
        let mut vm = UiViewModel::default();
        vm.avatar.is_ready = true;
        // No camera selected.
        assert!(!vm.can_start());
    }

    #[test]
    fn ui_model_cannot_start_without_avatar() {
        let mut vm = UiViewModel::default();
        vm.camera.selected_index = Some(0);
        // Avatar not ready.
        assert!(!vm.can_start());
    }

    #[test]
    fn ui_model_can_stop_when_running() {
        let vm = UiViewModel {
            lifecycle: AppLifecycle::Running,
            ..Default::default()
        };
        assert!(vm.can_stop());
    }

    #[test]
    fn ui_model_cannot_stop_when_idle() {
        let vm = UiViewModel::default();
        assert!(!vm.can_stop());
    }

    #[test]
    fn ui_model_can_calibrate_when_running() {
        let vm = UiViewModel {
            lifecycle: AppLifecycle::Running,
            ..Default::default()
        };
        assert!(vm.can_calibrate());
    }

    #[test]
    fn ui_model_cannot_calibrate_when_calibrating() {
        let vm = UiViewModel {
            lifecycle: AppLifecycle::Running,
            calibration: CalibrationViewModel {
                is_calibrating: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!vm.can_calibrate());
    }

    #[test]
    fn ui_model_cannot_calibrate_when_complete() {
        let vm = UiViewModel {
            lifecycle: AppLifecycle::Running,
            calibration: CalibrationViewModel {
                is_complete: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!vm.can_calibrate());
    }

    #[test]
    fn ui_model_screen_transitions() {
        let mut vm = UiViewModel::default();
        assert_eq!(vm.screen, Screen::Setup);
        vm.screen = Screen::Live;
        assert_eq!(vm.screen, Screen::Live);
        vm.screen = Screen::Diagnostics;
        assert_eq!(vm.screen, Screen::Diagnostics);
    }
}
