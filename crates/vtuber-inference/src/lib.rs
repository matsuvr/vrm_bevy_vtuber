//! `vtuber-inference`: face model loading, preprocessing, and pure-Rust inference.
//!
//! The inference runtime is constructed and owned inside the inference worker.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Tract-based inference backends.
pub mod backend;
/// Inference worker controller and command protocol.
pub mod controller;
/// Detector-box to landmark-crop transforms and crop preprocessing.
pub mod crop;
/// Output decoding from runtime tensors to engine-independent observations.
pub mod decode;
/// Model descriptor and runtime settings.
pub mod descriptor;
/// Detector-specific preprocessing and raw-output runtime.
pub mod detector;
/// Typed errors for the inference subsystem.
pub mod error;
/// Fixed-size inference timing and drop metrics.
pub mod metrics;
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

pub use controller::{ControlCommand, InferenceController, InferenceWorkerResult};
pub use crop::{
    CropError, FaceCropPreprocessBuffers, FaceCropTransform, LandmarkCoordinateEncoding,
};
pub use descriptor::{
    ChannelOrder, CropInterpolation, CropOutsideFill, DetectorPostprocessConfig, FaceCropConfig,
    FacePipelineDescriptor, InputValueDomain, ModelArtifactDescriptor, ModelDescriptor,
    ModelFormat, ModelRole, Normalization, NormalizationContract, OutputTensorContract,
    RuntimeSettings, TensorContract, TensorLayout,
};
pub use error::{InferenceError, Result};
pub use metrics::{DropCounters, InferenceMetrics, InferenceStage, StageTimingSnapshot};
pub use runtime::FaceInference;
#[cfg(feature = "onnx")]
pub use runtime::OnnxRuntime;
pub use state::{FailureStage, InferenceWorkerState, InferenceWorkerStatus, WorkerFailure};
