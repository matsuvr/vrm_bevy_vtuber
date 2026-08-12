//! `vtuber-tracking`: calibration, pose solving, filtering, and tracking state.
//!
//! This crate must not depend on Bevy or `bevy_vrm1`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Calibration: neutral reference collection and session state.
pub mod calibration;
/// Confidence synthesis and hysteresis gating.
pub mod confidence;
/// MediaPipe blendshape and gaze mapping.
pub mod expressions;
/// Tracking filters: rotation smoothing and expression filtering.
pub mod filter;
/// Loss hold, neutral decay, and recovery blend.
pub mod loss_recovery;
/// Neutral-relative head pose generation and tracking pipeline stages.
pub mod pipeline;
/// Placeholder for tracking subsystem.
pub mod placeholder;
/// Head pose estimation from landmark sets.
pub mod pose;
/// Explicit tracking state machine and transition table.
pub mod state_machine;

pub use calibration::{
    AUTO_NEUTRAL_MIN_SAMPLES, AUTO_NEUTRAL_WINDOW, AutoNeutralCollector, AutoNeutralError,
    AutoNeutralState, AutoNeutralUpdate, CalibrationCollector, CalibrationInput,
    CalibrationSession, CollectorMetrics, NeutralContext, NeutralProfile, NeutralReference,
    NeutralValidationSettings, RejectionReason, SampleDecision,
};
pub use confidence::{
    ConfidenceAssessment, ConfidenceConfigError, ConfidenceError, ConfidenceGate,
    ConfidenceGateParams, ConfidenceInputs, ConfidencePolicies, ConfidenceSignal, ConfidenceSource,
    MissingSourcePolicy, synthesize,
};
pub use expressions::{
    map_mediapipe_expressions, map_mediapipe_gaze, map_mediapipe_raw_expressions,
    parse_mediapipe_blendshapes,
};
pub use filter::{
    ExpressionCalibration, ExpressionCalibrationError, ExpressionChannel, ExpressionFilter,
    ExpressionFilterParams, ExpressionRange, HeadFilterParams, HeadRotationFilter,
    MissingChannelFallback, MissingChannelPolicy,
};
pub use loss_recovery::{
    LossRecovery, LossRecoveryConfigError, LossRecoveryParams, MAX_DECAY_DURATION,
    MAX_HOLD_DURATION, MAX_RECOVERY_DURATION, MIN_DECAY_DURATION, MIN_HOLD_DURATION,
    MIN_RECOVERY_DURATION,
};
pub use pipeline::{
    HeadPoseFailure, HeadPoseFrame, PipelineConfig, PipelineConfigError, PipelineUpdate,
    PoseFailureReason, TrackingPipeline, compute_neutral_relative_pose,
};
pub use pose::mediapipe::{
    MediaPipePoseError, RelativeFaceTransform, mediapipe_to_application_basis, relative_pose,
    relative_transform,
};
pub use pose::planar::{
    CANONICAL_FACE_TEMPLATE, CanonicalFacePoint, PlanarCorrespondence, PlanarLandmark,
    PlanarPoseAlignment, PlanarPoseError, solve_planar_pose,
};
pub use pose::{LandmarkSet, PoseAlignment, PoseError, WeightedPoint, solve_relative_pose};
pub use state_machine::{
    StateMachineConfigError, StateMachineParams, StateTransitionResult, TrackingAction,
    TrackingStateMachine, TransitionInput,
};
