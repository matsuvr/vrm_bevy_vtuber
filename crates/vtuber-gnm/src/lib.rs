//! Rust-side GNM Head v3 model boundary for sparse and selected-surface face-state evaluation.
//!
//! This crate deliberately stops at validated, engine-neutral GNM geometry,
//! observation, calibration, dynamic-state lifecycle, and temporal-energy
//! contracts. It does not contain a renderer, a Bevy system, or an avatar
//! retargeting policy; those belong to later Issue #50 leaves.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod dense;
mod error;
mod identity_calibration;
mod landmarks;
mod lifecycle;
mod model;
mod npz;
mod temporal_regularization;

pub use dense::{
    AnatomicalSide, CorrespondenceProvenance, CorrespondenceReliability, DenseCorrespondenceSet,
    DenseCoveragePolicy, DenseCoverageSummary, DenseMappingVersion, DenseObservationStatus,
    FaceRegion, GnmDenseError, GnmDenseObservation, GnmDenseObservationPoint, GnmSurfacePointRef,
    MEDIAPIPE_FACE_LANDMARK_COUNT, MediaPipeGnmDenseCorrespondence, SPARSE_BOOTSTRAP_POINT_COUNT,
    canonicalize_mediapipe_xy,
};
pub use error::GnmModelError;
pub use identity_calibration::{
    FixedGnmIdentity, GnmIdentityCalibration, GnmIdentityCalibrationError, IdentityFitDiagnostics,
    NeutralCalibrationCandidate, NeutralCalibrationReadiness, NeutralCalibrationRejection,
    NeutralCalibrationRejectionReason, NeutralCalibrationSelection,
    NeutralCalibrationSelectionConfig, NeutralCalibrationWindowDiagnostics,
    NeutralNormalizationScales, NeutralPoseDiversity, select_neutral_calibration_candidates,
};
pub use landmarks::{SparseLandmark, SparseLandmarkSet, head_sparse_68};
pub use lifecycle::{
    GnmFitInitialization, GnmFitOutcome, GnmFrameStamp, PersistentGnmAction, PersistentGnmEvent,
    PersistentGnmLifecycleConfig, PersistentGnmLifecycleDecision, PersistentGnmLifecycleError,
    PersistentGnmLifecycleState, PersistentGnmPhase, advance_persistent_gnm_lifecycle,
};
pub use model::{
    DenseArray, GNM_HEAD_V3_EXPRESSION_DIM, GNM_HEAD_V3_IDENTITY_DIM, GNM_HEAD_V3_VERSION,
    GnmExpressionState, GnmIdentityState, GnmJointState, GnmModel, GnmModelData, GnmSparseVertices,
    GnmVariant, GnmVersion,
};
pub use npz::{GNM_DATA_SCHEMA_KEYS, load_gnm_head_v3};
pub use temporal_regularization::{
    GnmTemporalNormalization, GnmTemporalStateView, TemporalGroupPenaltyMetrics,
    TemporalGroupPenaltyWeights, TemporalHistoryTiming, TemporalRegularizationConfig,
    TemporalRegularizationError, TemporalRegularizationInput, TemporalRegularizationMetrics,
    evaluate_temporal_regularization,
};
