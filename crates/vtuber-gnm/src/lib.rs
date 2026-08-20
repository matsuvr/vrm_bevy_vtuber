//! Rust-side GNM Head v3 model boundary for sparse face-state evaluation.
//!
//! This crate deliberately stops at a validated, engine-neutral sparse point
//! evaluator. It does not contain a renderer, a Bevy system, or a retargeting
//! policy; those belong to later Issue #50 leaves.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod correspondence;
mod error;
mod landmarks;
mod model;
mod npz;

pub use correspondence::{
    DEFAULT_MEDIAPIPE_TO_GNM_MAP, GnmFitError, GnmProjectionFit, GnmProjectionModel,
    GnmSparseObservation, MEDIAPIPE_LANDMARK_COUNT, MediaPipeToGnmSparseMap,
    SPARSE_FACE_LANDMARK_COUNT, SparseFaceLandmarkSemantic, fit_weak_perspective, validate_map,
};
pub use error::GnmModelError;
pub use landmarks::{SparseLandmark, SparseLandmarkSet, head_sparse_68};
pub use model::{
    DenseArray, GNM_HEAD_V3_EXPRESSION_DIM, GNM_HEAD_V3_IDENTITY_DIM, GnmExpressionState,
    GnmIdentityState, GnmJointState, GnmModel, GnmModelData, GnmSparseVertices, GnmVariant,
    GnmVersion,
};
pub use npz::{GNM_DATA_SCHEMA_KEYS, load_gnm_head_v3};
