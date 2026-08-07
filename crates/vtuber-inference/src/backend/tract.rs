//! Tract-based runtime construction for inference workers.
//!
//! This module performs manifest verification, model loading, optimization,
//! and runnable conversion inside the worker thread. The runnable object is
//! owned by the worker and never moved to the controller thread.

use std::path::Path;
use std::sync::Arc;

use tract_tflite::prelude::{Framework, IntoRunnable, TypedRunnableModel};
use vtuber_core::types::{LandmarkSchemaId, RawFaceObservation};

use crate::descriptor::{ModelDescriptor, ModelFormat};
#[cfg(feature = "onnx")]
use crate::descriptor::Normalization;
use crate::error::{InferenceError, Result};
use crate::runtime::FaceInference;

/// Loads and verifies a face model described by `descriptor` and returns a
/// runnable runtime object.
///
/// The returned object is owned by the worker thread; it is constructed here
/// and never moved to the caller thread.
pub fn load_model_runtime(descriptor: &ModelDescriptor) -> Result<Box<dyn FaceInference>> {
    verify_hash(&descriptor.path, &descriptor.sha256)?;

    match descriptor.format {
        ModelFormat::Tflite => Ok(Box::new(TfliteRuntime::new(descriptor)?)),
        #[cfg(feature = "onnx")]
        ModelFormat::Onnx => Ok(Box::new(load_onnx(descriptor)?)),
    }
}

fn verify_hash(path: &Path, expected_sha256: &str) -> Result<()> {
    let bytes = std::fs::read(path)
        .map_err(|e| InferenceError::LoadFailed(format!("failed to read model file: {e}")))?;
    let actual = sha256_hex(&bytes);
    if actual.eq_ignore_ascii_case(expected_sha256) {
        Ok(())
    } else {
        Err(InferenceError::HashMismatch {
            expected: expected_sha256.to_owned(),
            actual,
        })
    }
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(feature = "onnx")]
fn load_onnx(descriptor: &ModelDescriptor) -> Result<crate::runtime::OnnxRuntime> {
    let (mean, std) = normalization_params(descriptor.normalization);
    crate::runtime::OnnxRuntime::new(&descriptor.path, mean, std, descriptor.schema)
}

#[cfg(feature = "onnx")]
const fn normalization_params(normalization: Normalization) -> ([f32; 3], [f32; 3]) {
    match normalization {
        Normalization::ZeroToOne => ([0.0; 3], [1.0; 3]),
        Normalization::MinusOneToOne => ([0.5; 3], [0.5; 3]),
        Normalization::MeanStd { mean, std } => (mean, std),
    }
}

/// TFLite face inference runtime owned by the worker thread.
struct TfliteRuntime {
    /// The runnable tract model. Kept alive so that resources are dropped in
    /// the worker thread when this runtime is dropped.
    #[allow(dead_code)]
    model: Arc<TypedRunnableModel>,
    schema: LandmarkSchemaId,
}

impl TfliteRuntime {
    fn new(descriptor: &ModelDescriptor) -> Result<Self> {
        let bytes = std::fs::read(&descriptor.path)
            .map_err(|e| InferenceError::LoadFailed(format!("failed to read model file: {e}")))?;
        let model = tract_tflite::tflite()
            .model_for_read(&mut &bytes[..])
            .map_err(|e| InferenceError::LoadFailed(format!("{e:?}")))?
            .into_optimized()
            .map_err(|e| InferenceError::OptimizationFailed(format!("{e:?}")))?
            .into_runnable()
            .map_err(|e| InferenceError::OptimizationFailed(format!("{e:?}")))?;

        Ok(Self {
            model,
            schema: descriptor.schema,
        })
    }
}

impl FaceInference for TfliteRuntime {
    fn infer(&self, _frame: &[u8], _width: u32, _height: u32) -> Result<RawFaceObservation> {
        Err(InferenceError::ExecutionFailed(
            "TFLite inference not yet implemented".into(),
        ))
    }

    fn schema_id(&self) -> LandmarkSchemaId {
        self.schema
    }
}
