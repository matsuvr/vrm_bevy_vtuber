//! Model descriptor and runtime settings for inference workers.
//!
//! A [`ModelDescriptor`] is a plain, sendable description of a face model that
//! can cross thread boundaries. The actual runtime object is constructed from
//! the descriptor inside the inference worker, so controller code never owns
//! a live runtime instance.

use std::path::PathBuf;

use vtuber_core::types::LandmarkSchemaId;

/// Manifest mapping from backend blendshape names to canonical expressions.
///
/// A model may export coefficients under different naming conventions.  Each
/// canonical expression stores a list of candidate names; the decoder uses
/// the first match it finds in the runtime output.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExpressionMapping {
    /// Candidate names for the left eye blink coefficient.
    pub blink_left: Vec<String>,
    /// Candidate names for the right eye blink coefficient.
    pub blink_right: Vec<String>,
    /// Candidate names for the mouth openness coefficient.
    pub mouth_open: Vec<String>,
}

/// Supported model file formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelFormat {
    /// TensorFlow Lite flatbuffer.
    Tflite,
    /// ONNX.
    #[cfg(feature = "onnx")]
    Onnx,
}

/// Describes a face model to load inside the inference worker.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelDescriptor {
    /// Human-readable model identifier.
    pub id: String,
    /// Model format.
    pub format: ModelFormat,
    /// Absolute path to the model file.
    pub path: PathBuf,
    /// Expected SHA-256 hex digest of the model file.
    pub sha256: String,
    /// Input tensor name.
    pub input_name: String,
    /// Input tensor shape in the model's native order.
    pub input_shape: Vec<usize>,
    /// Input data type, e.g. `f32` or `u8`.
    pub input_dtype: String,
    /// Channel order of the input image.
    pub channel_order: ChannelOrder,
    /// Normalization applied after converting to float.
    pub normalization: Normalization,
    /// Landmark schema produced by this model.
    pub schema: LandmarkSchemaId,
    /// Optional blendshape-to-expression mapping for models that export
    /// expression coefficients as named blendshape output.
    pub expression_mapping: Option<ExpressionMapping>,
}

/// Channel order of the input image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChannelOrder {
    /// Red, green, blue.
    Rgb,
    /// Red, green, blue, alpha.
    Rgba,
    /// Blue, green, red.
    Bgr,
    /// Blue, green, red, alpha.
    Bgra,
    /// Single luminance channel.
    Gray,
}

/// Normalization applied to input pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Normalization {
    /// Scale `[0, 255]` to `[0, 1]`.
    ZeroToOne,
    /// Scale `[0, 255]` to `[-1, 1]`.
    MinusOneToOne,
    /// Subtract mean and divide by standard deviation per channel.
    MeanStd {
        /// Per-channel mean.
        mean: [f32; 3],
        /// Per-channel standard deviation.
        std: [f32; 3],
    },
}

/// Runtime settings that control worker behavior without owning runtime state.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSettings {
    /// Maximum time to wait for a new frame before polling the stop token.
    pub frame_wait_timeout_ms: u64,
    /// Interval between face detector runs while tracking is active.
    pub detector_interval_frames: u32,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            frame_wait_timeout_ms: 100,
            detector_interval_frames: 5,
        }
    }
}

impl ModelDescriptor {
    /// Returns a short display name for diagnostics.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_settings_default() {
        let settings = RuntimeSettings::default();
        assert_eq!(settings.frame_wait_timeout_ms, 100);
        assert_eq!(settings.detector_interval_frames, 5);
    }

    #[test]
    fn descriptor_display_name() {
        let desc = ModelDescriptor {
            id: "test-model".into(),
            format: ModelFormat::Tflite,
            path: PathBuf::from("/tmp/model.tflite"),
            sha256: "abcd".into(),
            input_name: "input".into(),
            input_shape: vec![1, 256, 256, 3],
            input_dtype: "f32".into(),
            channel_order: ChannelOrder::Rgb,
            normalization: Normalization::ZeroToOne,
            schema: LandmarkSchemaId("test-schema"),
            expression_mapping: None,
        };
        assert_eq!(desc.display_name(), "test-model");
    }
}
