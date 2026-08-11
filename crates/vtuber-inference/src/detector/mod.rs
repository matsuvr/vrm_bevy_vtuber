//! Detector-specific preprocessing and runtime APIs.

/// UltraFace input preprocessing with reusable worker-owned buffers.
pub mod preprocess;
/// UltraFace ONNX runtime and validated raw detector outputs.
#[cfg(feature = "onnx")]
pub mod runtime;

pub use preprocess::{
    DetectorNormalization, DetectorPreprocessError, ULTRAFACE_INPUT_HEIGHT, ULTRAFACE_INPUT_WIDTH,
    UltraFacePreprocessBuffers,
};
#[cfg(feature = "onnx")]
pub use runtime::{DetectorRawOutputs, DetectorRawTensor, DetectorRuntimeError, UltraFaceDetector};
