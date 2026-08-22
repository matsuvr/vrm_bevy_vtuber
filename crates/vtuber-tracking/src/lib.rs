//! `vtuber-tracking`: calibration, pose solving, filtering, and tracking state.
//!
//! This crate must not depend on Bevy or `bevy_vrm1`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Source-aligned A/B backend contract and fallback arbitration.
pub mod ab_backend;
/// Pure motion/confidence-adaptive temporal-weight policy.
pub mod adaptive_temporal;
/// Optional MediaPipe semantic observations used only as an auxiliary fitting term.
pub mod auxiliary_expression;
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
/// Timestamp-aware pure metrics for temporal tracking quality.
pub mod temporal_metrics;

pub use ab_backend::{
    AbBackendError, AlignedBackendOutputs, AlignedLatencyComparison, BackendLatencyMetrics,
    BackendOutputTiming, BackendSelectionConfig, BackendSelectionDecision, BackendSelectionState,
    FaceTrackingBackend, FaceTrackingMode, GnmFallbackReason, GnmRuntimeHealth, GnmTransientIssue,
    GnmUnavailableReason, SourceFrameStamp, StampedBackendOutput, advance_backend_selection,
    backend_latency_metrics,
};
pub use adaptive_temporal::{
    AdaptiveTemporalConfig, AdaptiveTemporalError, AdaptiveTemporalInput, AdaptiveTemporalRegime,
    AdaptiveTemporalState, TemporalGroupWeights, TemporalObservationHealth,
    advance_adaptive_temporal_policy,
};
pub use auxiliary_expression::{
    AuxChannelReliability, AuxiliaryChannelConfig, AuxiliaryExpressionChannel,
    AuxiliaryExpressionError, AuxiliaryExpressionGroup, AuxiliaryExpressionObservation,
    AuxiliaryExpressionSemantic, AuxiliaryExpressionStatus, AuxiliaryGroupResiduals,
    AuxiliaryLossConfig, AuxiliaryLossDiagnostics, AuxiliaryNeutralCalibration,
    PredictedAuxiliaryFeature, evaluate_auxiliary_expression_loss,
    validate_auxiliary_source_alignment,
};
pub use calibration::{
    AUTO_NEUTRAL_MIN_SAMPLES, AUTO_NEUTRAL_WINDOW, AutoNeutralCollector, AutoNeutralError,
    AutoNeutralState, AutoNeutralUpdate, CalibrationCollector, CalibrationInput,
    CalibrationSession, CollectorMetrics, GazeNeutralBaseline, NeutralContext, NeutralProfile,
    NeutralReference, NeutralValidationSettings, RejectionReason, SampleDecision,
};
pub use confidence::{
    ConfidenceAssessment, ConfidenceConfigError, ConfidenceError, ConfidenceGate,
    ConfidenceGateParams, ConfidenceInputs, ConfidencePolicies, ConfidenceSignal, ConfidenceSource,
    MissingSourcePolicy, synthesize,
};
pub use expressions::{
    BinocularGazeObservation, PerEyeGazeObservation, fuse_binocular_gaze,
    map_mediapipe_expressions, map_mediapipe_gaze, map_mediapipe_raw_expressions,
    observe_mediapipe_gaze, parse_mediapipe_blendshapes,
};
pub use filter::{
    ExpressionCalibration, ExpressionCalibrationError, ExpressionChannel, ExpressionFilter,
    ExpressionFilterParams, ExpressionRange, GazeFilter, GazeFilterParams, HeadFilterParams,
    HeadRotationFilter, MissingChannelFallback, MissingChannelPolicy,
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
pub use temporal_metrics::{
    PulseResponseMetrics, PulseResponseSpec, StepResponseMetrics, StepResponseSpec,
    TemporalMetricError, TemporalNoiseMetrics, TemporalSample, TemporalTrace,
    pulse_response_metrics, step_response_metrics, temporal_noise_metrics,
};
