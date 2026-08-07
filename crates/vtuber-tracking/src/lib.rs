//! `vtuber-tracking`: calibration, pose solving, filtering, and tracking state.
//!
//! This crate must not depend on Bevy or `bevy_vrm1`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Calibration: neutral reference collection and session state.
pub mod calibration;
/// Placeholder for tracking subsystem.
pub mod placeholder;
/// Head pose estimation from landmark sets.
pub mod pose;

pub use calibration::{
    CalibrationCollector, CalibrationInput, CalibrationSession, CollectorMetrics, NeutralContext,
    NeutralProfile, NeutralReference, NeutralValidationSettings, RejectionReason, SampleDecision,
};
pub use pose::{LandmarkSet, PoseAlignment, PoseError, WeightedPoint, solve_relative_pose};
