//! Rust-side GNM Head v3 model boundary for sparse face-state evaluation.
//!
//! This crate deliberately stops at a validated, engine-neutral sparse point
//! evaluator. It does not contain a renderer, a Bevy system, or a retargeting
//! policy; those belong to later Issue #50 leaves.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod ab;
mod correspondence;
mod decoder;
mod error;
mod fitting;
mod landmarks;
mod model;
mod npz;

pub use ab::{GnmAbError, GnmAbEvaluator, GnmAbReport, GnmAbSample};
pub use correspondence::{
    DEFAULT_MEDIAPIPE_TO_GNM_MAP, GnmFitError, GnmProjectionFit, GnmProjectionModel,
    GnmSparseObservation, MEDIAPIPE_LANDMARK_COUNT, MediaPipeToGnmSparseMap,
    SPARSE_FACE_LANDMARK_COUNT, SparseFaceLandmarkSemantic, fit_weak_perspective,
    project_weak_perspective, validate_map,
};
pub use decoder::{
    ARKIT52_TEACHER_CHANNEL_COUNT, DEFAULT_DECODER_MAX_CONDITION_NUMBER,
    DEFAULT_DECODER_REGULARIZATION, DEFAULT_MAX_TRAINING_RESIDUAL, DEFAULT_MIN_CHANNEL_VARIANCE,
    DEFAULT_MIN_DECODER_SAMPLES, DEFAULT_MIN_TRAINING_CONFIDENCE, GnmDecoderConfig,
    GnmDecoderDiagnostics, GnmDecoderError, GnmDecoderTrainer, GnmDecoderTrainingResult,
    GnmDecoderTrainingSample, GnmToArkit52Decoder,
};
pub use error::GnmModelError;
pub use fitting::{
    DEFAULT_ACTIVE_EXPRESSION_DIMENSION, DEFAULT_ACTIVE_IDENTITY_DIMENSION,
    DEFAULT_EXPRESSION_REGULARIZATION, DEFAULT_IDENTITY_COEFFICIENT_BOUND,
    DEFAULT_IDENTITY_REGULARIZATION, DEFAULT_MAX_CONDITION_NUMBER, DEFAULT_MAX_ITERATIONS,
    DEFAULT_MIN_CALIBRATION_SAMPLES, DEFAULT_RESIDUAL_THRESHOLD, DEFAULT_TEMPORAL_REGULARIZATION,
    GnmFaceFitter, GnmFaceState, GnmFaceStatus, GnmFitterConfig, GnmFitterError, GnmFittingSample,
    GnmIdentityCalibration,
};
pub use landmarks::{SparseLandmark, SparseLandmarkSet, head_sparse_68};
pub use model::{
    DenseArray, GNM_HEAD_V3_EXPRESSION_DIM, GNM_HEAD_V3_IDENTITY_DIM, GnmExpressionState,
    GnmIdentityState, GnmJointState, GnmModel, GnmModelData, GnmSparseVertices, GnmVariant,
    GnmVersion,
};
pub use npz::{GNM_DATA_SCHEMA_KEYS, load_gnm_head_v3};
