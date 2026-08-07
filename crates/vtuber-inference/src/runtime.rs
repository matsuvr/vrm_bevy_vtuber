//! Face inference runtime traits and implementations.

use crate::error::Result;
use vtuber_core::types::{LandmarkSchemaId, RawFaceObservation};

/// Trait for a face inference runtime.
pub trait FaceInference: Send + Sync {
    /// Performs inference on the given RGB frame.
    fn infer(&self, frame: &[u8], width: u32, height: u32) -> Result<RawFaceObservation>;
    /// Returns the schema ID used by this runtime.
    fn schema_id(&self) -> LandmarkSchemaId;
}

/// ONNX implementation of the face inference runtime.
#[cfg(feature = "onnx")]
pub struct OnnxRuntime {
    model: std::sync::Arc<tract_core::prelude::TypedRunnableModel>,
    input_shape: [usize; 4],
    mean: [f32; 3],
    std: [f32; 3],
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
        mean: [f32; 3],
        std: [f32; 3],
        schema: LandmarkSchemaId,
    ) -> crate::error::Result<Self> {
        use crate::error::InferenceError;
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

        let input_shape = [1, 3, 256, 256];

        Ok(Self {
            model,
            input_shape,
            mean,
            std,
            schema,
        })
    }
}

#[cfg(feature = "onnx")]
impl FaceInference for OnnxRuntime {
    fn infer(&self, frame: &[u8], width: u32, height: u32) -> Result<RawFaceObservation> {
        use crate::error::InferenceError;
        use crate::schema::BasicObservation;
        use tract_onnx::prelude::*;
        use vtuber_core::types::{FrameSeq, MonoTimeNs, NamedCoefficient, NormalizedRect};

        let input_tensor = preprocess(
            frame,
            width,
            height,
            &self.input_shape,
            &self.mean,
            &self.std,
        );

        let result = self
            .model
            .run(tvec!(tensors_to_tract(&input_tensor).into_tvalue()))
            .map_err(|e| InferenceError::ExecutionFailed(format!("{e:?}")))?;

        let output = result[0]
            .to_plain_array_view::<f32>()
            .map_err(|e| InferenceError::ExecutionFailed(format!("{e:?}")))?;

        let landmarks = decode_landmarks(output);
        let obs = BasicObservation::from_landmarks(&landmarks);

        Ok(RawFaceObservation {
            source_seq: FrameSeq(0),
            captured_at: MonoTimeNs(0),
            inference_started_at: MonoTimeNs(0),
            inference_finished_at: MonoTimeNs(0),
            face_confidence: 1.0,
            landmarks,
            blendshapes: Some(vec![
                NamedCoefficient {
                    name: "blinkLeft".into(),
                    value: obs.blink_left,
                },
                NamedCoefficient {
                    name: "blinkRight".into(),
                    value: obs.blink_right,
                },
                NamedCoefficient {
                    name: "aa".into(),
                    value: obs.mouth_open,
                },
            ]),
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
fn preprocess(
    frame: &[u8],
    width: u32,
    height: u32,
    shape: &[usize; 4],
    mean: &[f32; 3],
    std: &[f32; 3],
) -> Vec<f32> {
    let target_w = shape[2];
    let target_h = shape[3];
    let mut output = vec![0.0f32; 3 * target_w * target_h];

    let crop_size = width.min(height);
    let offset_x = (width - crop_size) / 2;
    let offset_y = (height - crop_size) / 2;

    for y in 0..target_h {
        for x in 0..target_w {
            let src_x = offset_x + (x as u32 * crop_size / target_w as u32);
            let src_y = offset_y + (y as u32 * crop_size / target_h as u32);
            let src_idx = (src_y * width + src_x) as usize * 3;

            if src_idx + 2 < frame.len() {
                for c in 0..3 {
                    let val = frame[src_idx + c] as f32 / 255.0;
                    output[c * target_w * target_h + y * target_w + x] = (val - mean[c]) / std[c];
                }
            }
        }
    }
    output
}

#[cfg(feature = "onnx")]
fn tensors_to_tract(data: &[f32]) -> tract_core::ndarray::Array4<f32> {
    tract_core::ndarray::Array::from_shape_vec((1, 3, 256, 256), data.to_vec())
        .expect("shape matches the fixed ONNX input")
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
