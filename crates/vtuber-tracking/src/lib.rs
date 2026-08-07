//! `vtuber-tracking`: calibration, pose solving, filtering, and tracking state.
//!
//! This crate must not depend on Bevy or `bevy_vrm1`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Calibration: neutral reference collection and session state.
pub mod calibration;
/// Confidence synthesis and hysteresis gating.
pub mod confidence;
/// Tracking filters: rotation smoothing and expression filtering.
pub mod filter;
/// Neutral-relative head pose generation and tracking pipeline stages.
pub mod pipeline;
/// Placeholder for tracking subsystem.
pub mod placeholder;
/// Head pose estimation from landmark sets.
pub mod pose;

pub use calibration::{
    CalibrationCollector, CalibrationInput, CalibrationSession, CollectorMetrics, NeutralContext,
    NeutralProfile, NeutralReference, NeutralValidationSettings, RejectionReason, SampleDecision,
};
pub use confidence::{
    ConfidenceAssessment, ConfidenceConfigError, ConfidenceError, ConfidenceGate,
    ConfidenceGateParams, ConfidenceInputs, ConfidencePolicies, ConfidenceSignal, ConfidenceSource,
    MissingSourcePolicy, synthesize,
};
pub use filter::{
    ExpressionCalibration, ExpressionCalibrationError, ExpressionChannel, ExpressionFilter,
    ExpressionFilterParams, ExpressionRange, HeadFilterParams, HeadRotationFilter,
    MissingChannelFallback, MissingChannelPolicy,
};
pub use pipeline::{
    HeadPoseFailure, HeadPoseFrame, PoseFailureReason, compute_neutral_relative_pose,
};
pub use pose::{LandmarkSet, PoseAlignment, PoseError, WeightedPoint, solve_relative_pose};
