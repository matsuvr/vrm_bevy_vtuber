//! Face inference runtime traits and implementations.

#[cfg(feature = "onnx")]
use crate::error::InferenceError;
use crate::error::Result;
use vtuber_core::types::{LandmarkSchemaId, RawFaceObservation};

/// Trait for a face inference runtime.
pub trait FaceInference: Send + Sync {
    /// Performs inference on a preprocessed input tensor.
    ///
    /// `tensor` contains normalized float values in the layout described by
    /// `input_shape`. The shape is passed explicitly because the trait object
    /// may be created from descriptors with varying input layouts.
    fn infer(&self, tensor: &[f32], input_shape: &[usize; 4]) -> Result<RawFaceObservation>;
    /// Returns the schema ID used by this runtime.
    fn schema_id(&self) -> LandmarkSchemaId;
}

/// ONNX implementation of the face inference runtime.
#[cfg(feature = "onnx")]
pub struct OnnxRuntime {
    model: std::sync::Arc<tract_core::prelude::TypedRunnableModel>,
    schema: LandmarkSchemaId,
}

#[cfg(feature = "onnx")]
impl OnnxRuntime {
    /// Constructs a new OnnxRuntime from a model file.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be read, optimized, or made runnable.
    pub fn new(
        path: impl AsRef<std::path::Path>,
        schema: LandmarkSchemaId,
    ) -> crate::error::Result<Self> {
        use tract_onnx::prelude::*;

        let mut file = std::fs::File::open(path.as_ref())
            .map_err(|e| InferenceError::LoadFailed(e.to_string()))?;
        let model = tract_onnx::onnx()
            .model_for_read(&mut file)
            .map_err(|e| InferenceError::LoadFailed(format!("{e:?}")))?
            .into_optimized()
            .map_err(|e| InferenceError::OptimizationFailed(format!("{e:?}")))?
            .into_runnable()
            .map_err(|e| InferenceError::OptimizationFailed(format!("{e:?}")))?;

        Ok(Self { model, schema })
    }
}

#[cfg(feature = "onnx")]
impl FaceInference for OnnxRuntime {
    fn infer(&self, tensor: &[f32], input_shape: &[usize; 4]) -> Result<RawFaceObservation> {
        use tract_onnx::prelude::*;
        use vtuber_core::types::{FrameSeq, MonoTimeNs, NormalizedRect};

        let input_array = tensors_to_tract(tensor, input_shape)
            .map_err(|e| InferenceError::ExecutionFailed(format!("invalid tensor shape: {e:?}")))?;

        let result = self
            .model
            .run(tvec!(input_array.into_tvalue()))
            .map_err(|e| InferenceError::ExecutionFailed(format!("{e:?}")))?;

        let output = result[0]
            .to_plain_array_view::<f32>()
            .map_err(|e| InferenceError::ExecutionFailed(format!("{e:?}")))?;

        let landmarks = decode_landmarks(output);
        let expressions = crate::decode::expressions::decode_expressions(
            None,
            None,
            Some(&landmarks),
            self.schema,
            1.0,
        )
        .ok()
        .flatten()
        .unwrap_or_default();

        Ok(RawFaceObservation {
            source_seq: FrameSeq(0),
            captured_at: MonoTimeNs(0),
            inference_started_at: MonoTimeNs(0),
            inference_finished_at: MonoTimeNs(0),
            face_confidence: 1.0,
            landmarks,
            blendshapes: None,
            expressions,
            roi: NormalizedRect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
                rotation_rad: 0.0,
            },
            schema: self.schema,
        })
    }

    fn schema_id(&self) -> LandmarkSchemaId {
        self.schema
    }
}

#[cfg(feature = "onnx")]
fn tensors_to_tract(
    data: &[f32],
    input_shape: &[usize; 4],
) -> std::result::Result<tract_core::ndarray::Array4<f32>, tract_core::ndarray::ShapeError> {
    tract_core::ndarray::Array::from_shape_vec(
        (
            input_shape[0],
            input_shape[1],
            input_shape[2],
            input_shape[3],
        ),
        data.to_vec(),
    )
}

#[cfg(feature = "onnx")]
fn decode_landmarks(
    output: tract_core::ndarray::ArrayViewD<f32>,
) -> Vec<vtuber_core::types::Landmark3> {
    use vtuber_core::types::Landmark3;

    let mut landmarks = Vec::with_capacity(98);
    for i in 0..98 {
        let x = output[[0, i, 0]];
        let y = output[[0, i, 1]];
        let conf = output[[0, i, 2]];
        landmarks.push(Landmark3 {
            x,
            y,
            z: 0.0,
            visibility: conf,
        });
    }
    landmarks
}
