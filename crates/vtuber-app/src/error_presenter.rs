//! Error presenter — maps domain errors to user-facing messages.
//!
//! Provides recoverable error display with suggested actions.
//! Technical details are kept for diagnostics; the main UI shows
//! safe summaries only.

use bevy::prelude::*;

use crate::actions::UiAction;

/// A user-facing error presentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorPresentation {
    /// Stable error code for diagnostics.
    pub code: &'static str,
    /// User-safe summary message.
    pub user_message: String,
    /// Suggested recovery actions.
    pub suggested_actions: Vec<UiAction>,
}

/// Map an orchestrator error to a user-facing presentation.
#[must_use]
pub fn present_error(error: &crate::orchestrator::OrchestratorError) -> ErrorPresentation {
    use crate::orchestrator::OrchestratorError;

    match error {
        OrchestratorError::ImportFailed(msg) => ErrorPresentation {
            code: "IMPORT_FAILED",
            user_message: format!("Could not import model: {msg}"),
            suggested_actions: vec![UiAction::DismissError],
        },
        OrchestratorError::NoCameraSelected => ErrorPresentation {
            code: "NO_CAMERA",
            user_message: "Please select a camera before starting.".to_string(),
            suggested_actions: vec![UiAction::RefreshCameras, UiAction::DismissError],
        },
        OrchestratorError::NoAvatarLoaded => ErrorPresentation {
            code: "NO_AVATAR",
            user_message: "Please import an avatar before starting.".to_string(),
            suggested_actions: vec![UiAction::DismissError],
        },
        OrchestratorError::PipelineAlreadyRunning => ErrorPresentation {
            code: "PIPELINE_RUNNING",
            user_message: "The tracking pipeline is already running.".to_string(),
            suggested_actions: vec![UiAction::DismissError],
        },
        OrchestratorError::PipelineNotRunning => ErrorPresentation {
            code: "PIPELINE_NOT_RUNNING",
            user_message: "The tracking pipeline is not running.".to_string(),
            suggested_actions: vec![UiAction::DismissError],
        },
    }
}

/// Resource tracking presented errors to avoid re-displaying the same error.
#[derive(Resource, Debug, Default)]
pub struct ErrorPresenter {
    /// Last presented error code (to avoid duplicates).
    last_presented_code: Option<String>,
    /// Current presentation, if any.
    current: Option<ErrorPresentation>,
}

impl ErrorPresenter {
    /// Update the presenter with a new error. Returns true if the error is new.
    pub fn update(&mut self, error: Option<&crate::orchestrator::OrchestratorError>) -> bool {
        match error {
            Some(err) => {
                let presentation = present_error(err);
                let code = presentation.code.to_string();
                if self.last_presented_code.as_ref() == Some(&code) {
                    return false; // Same error, don't re-present.
                }
                self.last_presented_code = Some(code);
                self.current = Some(presentation);
                true
            }
            None => {
                self.last_presented_code = None;
                self.current = None;
                false
            }
        }
    }

    /// Get the current presentation, if any.
    #[must_use]
    pub fn current(&self) -> Option<&ErrorPresentation> {
        self.current.as_ref()
    }

    /// Dismiss the current error.
    pub fn dismiss(&mut self) {
        self.current = None;
        self.last_presented_code = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::OrchestratorError;

    #[test]
    fn error_presenter_no_error() {
        let mut presenter = ErrorPresenter::default();
        assert!(!presenter.update(None));
        assert!(presenter.current().is_none());
    }

    #[test]
    fn error_presenter_new_error() {
        let mut presenter = ErrorPresenter::default();
        let err = OrchestratorError::NoCameraSelected;
        assert!(presenter.update(Some(&err)));
        assert!(presenter.current().is_some());
        assert_eq!(presenter.current().unwrap().code, "NO_CAMERA");
    }

    #[test]
    fn error_presenter_same_error_not_re_presented() {
        let mut presenter = ErrorPresenter::default();
        let err = OrchestratorError::NoCameraSelected;
        assert!(presenter.update(Some(&err)));
        assert!(!presenter.update(Some(&err))); // Same error.
    }

    #[test]
    fn error_presenter_different_error_re_presented() {
        let mut presenter = ErrorPresenter::default();
        let err1 = OrchestratorError::NoCameraSelected;
        let err2 = OrchestratorError::NoAvatarLoaded;
        assert!(presenter.update(Some(&err1)));
        assert!(presenter.update(Some(&err2))); // Different error.
        assert_eq!(presenter.current().unwrap().code, "NO_AVATAR");
    }

    #[test]
    fn error_presenter_dismiss() {
        let mut presenter = ErrorPresenter::default();
        let err = OrchestratorError::NoCameraSelected;
        presenter.update(Some(&err));
        presenter.dismiss();
        assert!(presenter.current().is_none());
    }

    #[test]
    fn present_error_import_failed_has_dismiss() {
        let err = OrchestratorError::ImportFailed("bad file".to_string());
        let pres = present_error(&err);
        assert_eq!(pres.code, "IMPORT_FAILED");
        assert!(pres.user_message.contains("bad file"));
        assert!(pres.suggested_actions.contains(&UiAction::DismissError));
    }

    #[test]
    fn present_error_no_camera_has_refresh() {
        let err = OrchestratorError::NoCameraSelected;
        let pres = present_error(&err);
        assert!(pres.suggested_actions.contains(&UiAction::RefreshCameras));
    }
}
