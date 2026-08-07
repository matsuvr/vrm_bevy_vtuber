//! `vtuber-inference`: face model loading, preprocessing, and pure-Rust inference.
//!
//! The inference runtime is constructed and owned inside the inference worker.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Tract-based inference backends.
pub mod backend;
/// Inference worker controller and command protocol.
pub mod controller;
/// Output decoding from runtime tensors to engine-independent observations.
pub mod decode;
/// Model descriptor and runtime settings.
pub mod descriptor;
/// Typed errors for the inference subsystem.
pub mod error;
/// Face inference pipeline orchestration.
pub mod pipeline;
/// Placeholder for inference subsystem.
pub mod placeholder;
/// Video frame preprocessing for model input.
pub mod preprocess;
/// Model provenance probing.
pub mod probe;
/// Typed region-of-interest state for face inference.
pub mod roi;
/// Face inference runtime traits and implementations.
pub mod runtime;
/// Landmark schema and basic expression heuristics.
pub mod schema;
/// Inference worker state and status.
pub mod state;
/// Inference worker loop.
pub mod worker;

pub use controller::{
    ControlCommand, InferenceController, InferenceMetrics, InferenceWorkerResult,
};
pub use descriptor::{ChannelOrder, ModelDescriptor, ModelFormat, Normalization, RuntimeSettings};
pub use error::{InferenceError, Result};
pub use runtime::FaceInference;
#[cfg(feature = "onnx")]
pub use runtime::OnnxRuntime;
pub use state::{FailureStage, InferenceWorkerState, InferenceWorkerStatus, WorkerFailure};
