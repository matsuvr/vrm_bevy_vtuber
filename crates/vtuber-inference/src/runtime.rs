//! Face inference runtime traits and implementations.

#[cfg(feature = "onnx")]
use crate::error::InferenceError;
use crate::error::Result;
use vtuber_core::types::{LandmarkSchemaId, RawFaceObservation, VideoFrame};

/// Result of one production frame-level inference attempt.
///
/// `NoFace` is an ordinary observation and must not be counted as a runtime
/// failure. A `Face` observation uses unmirrored source-image normalized
/// coordinates. Implementations own all live model state in the inference
/// worker thread; this contract moves only borrowed frame data across the
/// call boundary.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameInferenceOutcome {
    /// A face was found and decoded into a source-image observation.
    Face(RawFaceObservation),
    /// No face met the detector and landmark validity policy.
    NoFace,
}

/// Frame-level production inference boundary.
///
/// The worker owns the implementing value and its model runtimes. The caller
/// supplies a borrowed [`VideoFrame`]; detector-specific tensors and runtime
/// values remain private to the inference crate.
pub trait FrameFaceInference: Send {
    /// Runs the complete detector-to-landmark pipeline for one frame.
    fn infer_frame(&mut self, frame: &VideoFrame) -> Result<FrameInferenceOutcome>;
}

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

/// Composite production runtime that owns the detector and landmark stages.
///
/// Construction is intended to happen inside the inference worker. The
/// `artifact_root` is the manifest directory; model paths in the plain
/// [`FacePipelineDescriptor`] are relative to that directory. Frame execution
/// is introduced by the later composite-runtime task, so this type currently
/// exposes ownership and descriptor validation without connecting the legacy
/// worker loop.
#[cfg(feature = "onnx")]
#[allow(dead_code)] // The stage fields are consumed by M1-08-013-007 frame execution.
pub struct CompositeFrameInference {
    descriptor: crate::descriptor::FacePipelineDescriptor,
    detector: crate::detector::UltraFaceDetector,
    landmark: OnnxRuntime,
    detector_buffers: crate::detector::UltraFacePreprocessBuffers,
    crop_buffers: crate::crop::FaceCropPreprocessBuffers,
}

#[cfg(feature = "onnx")]
impl CompositeFrameInference {
    /// Constructs both live runtimes and their reusable worker-owned buffers.
    ///
    /// This method must be called by the inference worker, not by the Bevy
    /// main thread. The returned value contains no `World`, entity, or asset
    /// handle and is safe to retain only within that worker's ownership domain.
    pub fn from_pipeline_descriptor(
        descriptor: &crate::descriptor::FacePipelineDescriptor,
        artifact_root: &std::path::Path,
    ) -> Result<Self> {
        if descriptor.detector.role != crate::descriptor::ModelRole::FaceDetector {
            return Err(crate::error::InferenceError::InvalidInput(
                "pipeline detector descriptor has the wrong role".into(),
            ));
        }
        if descriptor.landmarks.role != crate::descriptor::ModelRole::FaceLandmarks {
            return Err(crate::error::InferenceError::InvalidInput(
                "pipeline landmark descriptor has the wrong role".into(),
            ));
        }
        let schema = match descriptor.landmarks.schema.as_deref() {
            Some("peppapig-98") => LandmarkSchemaId("peppapig-98"),
            Some(other) => {
                return Err(crate::error::InferenceError::InvalidInput(format!(
                    "unsupported landmark schema `{other}`"
                )));
            }
            None => {
                return Err(crate::error::InferenceError::InvalidInput(
                    "landmark descriptor has no schema".into(),
                ));
            }
        };

        let detector_path = artifact_root.join(&descriptor.detector.file);
        let landmark_path = artifact_root.join(&descriptor.landmarks.file);
        crate::backend::tract::verify_model_file(&landmark_path, &descriptor.landmarks.sha256)?;
        let detector = crate::detector::UltraFaceDetector::from_path(detector_path)
            .map_err(|error| crate::error::InferenceError::LoadFailed(error.to_string()))?;
        let landmark = OnnxRuntime::new(landmark_path, schema)?;
        let detector_buffers = crate::detector::UltraFacePreprocessBuffers::new();
        let crop_buffers = crate::crop::FaceCropPreprocessBuffers::new(descriptor.crop.output_size)
            .map_err(|error| crate::error::InferenceError::InvalidInput(error.to_string()))?;

        Ok(Self {
            descriptor: descriptor.clone(),
            detector,
            landmark,
            detector_buffers,
            crop_buffers,
        })
    }

    /// Returns the plain descriptor used to construct this composite runtime.
    #[must_use]
    pub fn descriptor(&self) -> &crate::descriptor::FacePipelineDescriptor {
        &self.descriptor
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
