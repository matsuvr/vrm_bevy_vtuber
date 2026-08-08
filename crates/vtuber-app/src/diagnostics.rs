//! Diagnostics snapshot for the UI.
//!
//! Collects performance metrics, worker state, and model info for display
//! in the Diagnostics screen.

use bevy::prelude::*;

/// Diagnostics snapshot resource.
#[derive(Resource, Debug, Default, Clone)]
pub struct DiagnosticsSnapshot {
    /// Render FPS.
    pub render_fps: f32,
    /// Capture frame rate.
    pub capture_rate: f32,
    /// Inference rate.
    pub inference_rate: f32,
    /// Tracking state description.
    pub tracking_state: String,
    /// Slot overwrite count.
    pub slot_overwrites: u64,
    /// Stage timings (name, duration_ms).
    pub stage_timings: Vec<(String, f32)>,
    /// Model hash (short).
    pub model_hash: Option<String>,
    /// Camera backend name.
    pub camera_backend: Option<String>,
    /// Avatar capability summary.
    pub avatar_capabilities: Option<String>,
    /// Last error message, if any.
    pub last_error: Option<String>,
}

impl DiagnosticsSnapshot {
    /// Check if any workers are currently active.
    #[must_use]
    pub fn has_active_workers(&self) -> bool {
        self.capture_rate > 0.0 || self.inference_rate > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_default_has_no_workers() {
        let snap = DiagnosticsSnapshot::default();
        assert!(!snap.has_active_workers());
    }

    #[test]
    fn diagnostics_with_capture_rate_has_workers() {
        let snap = DiagnosticsSnapshot {
            capture_rate: 30.0,
            ..Default::default()
        };
        assert!(snap.has_active_workers());
    }
}
