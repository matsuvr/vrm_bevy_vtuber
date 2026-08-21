//! MediaPipe-to-GNM runtime handoff and decoder retargeting.
//!
//! The app owns mode selection and frame publication. This module owns the
//! model-specific conversion from the canonical 478-point MediaPipe sample to
//! the bounded GNM fitter, then back to the engine-neutral ARKit52 contract.

use std::fmt::{Display, Formatter};

use vtuber_core::{Arkit52Coefficients, FaceTrackingSample};

use crate::{
    DEFAULT_MEDIAPIPE_TO_GNM_MAP, GnmDecoderError, GnmFaceFitter, GnmFaceState, GnmFitError,
    GnmFitterError, GnmModel, GnmSparseObservation, GnmToArkit52Decoder, SparseLandmarkSet,
};

/// One GNM output for a canonical MediaPipe source frame.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmRetargetedFace {
    /// Fitted GNM state, including source sequence and residual diagnostics.
    pub state: GnmFaceState,
    /// Bounded ARKit52 coefficients produced by the frozen decoder.
    pub arkit52: Arkit52Coefficients,
    /// Decoder confidence copied from the frozen training diagnostics.
    pub decoder_confidence: f32,
}

/// Typed failure at the MediaPipe → GNM → ARKit52 handoff.
#[derive(Clone, Debug, PartialEq)]
pub enum GnmRetargetingError {
    /// The canonical MediaPipe sample could not be mapped to 68 points.
    Mapping(GnmFitError),
    /// The bounded GNM fitter rejected the sample or solve.
    Fitting(GnmFitterError),
    /// The frozen decoder rejected the fitted state.
    Decoding(GnmDecoderError),
}

impl Display for GnmRetargetingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mapping(error) => write!(formatter, "GNM correspondence failed: {error}"),
            Self::Fitting(error) => write!(formatter, "GNM fitting failed: {error}"),
            Self::Decoding(error) => write!(formatter, "GNM decoder failed: {error}"),
        }
    }
}

impl std::error::Error for GnmRetargetingError {}

/// Converts one canonical 478-point MediaPipe sample to the GNM sparse input.
pub fn gnm_fitting_sample_from_mediapipe(
    sample: &FaceTrackingSample,
) -> Result<crate::GnmFittingSample, GnmRetargetingError> {
    let landmarks: Vec<[f32; 2]> = sample
        .landmarks
        .iter()
        .map(|landmark| [landmark.x, landmark.y])
        .collect();
    let observation =
        GnmSparseObservation::from_mediapipe(&landmarks, &DEFAULT_MEDIAPIPE_TO_GNM_MAP)
            .map_err(GnmRetargetingError::Mapping)?;
    let confidence = sample
        .quality
        .landmark_presence_median
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    crate::GnmFittingSample::new(
        sample.source_seq.0,
        sample.captured_at.0,
        observation,
        confidence,
    )
    .map_err(GnmRetargetingError::Fitting)
}

/// Runs the bounded GNM fitter and frozen decoder for one MediaPipe sample.
///
/// The caller must have calibrated `fitter` and must provide the decoder that
/// passed its training readiness gate. No MediaPipe teacher values are read
/// during this operation.
pub fn retarget_mediapipe<'model>(
    fitter: &mut GnmFaceFitter<'model>,
    decoder: &GnmToArkit52Decoder,
    sample: &FaceTrackingSample,
) -> Result<GnmRetargetedFace, GnmRetargetingError> {
    let fitting_sample = gnm_fitting_sample_from_mediapipe(sample)?;
    let state = fitter
        .fit_expression(&fitting_sample)
        .map_err(GnmRetargetingError::Fitting)?;
    let arkit52 = decoder
        .decode(&state)
        .map_err(GnmRetargetingError::Decoding)?;
    Ok(GnmRetargetedFace {
        state,
        arkit52,
        decoder_confidence: (1.0 - decoder.diagnostics.train_residual).clamp(0.0, 1.0),
    })
}

/// Validates that the model and sparse landmark asset can be wired into the
/// runtime before a worker or frame is started.
pub fn validate_runtime_assets(
    model: &GnmModel,
    landmarks: &SparseLandmarkSet,
) -> Result<(), GnmFitterError> {
    GnmFaceFitter::new(model, landmarks, crate::GnmFitterConfig::default()).map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use vtuber_core::{
        CameraFaceTransform, FaceBlendshapeSet, FaceLandmark, FaceTrackingQuality, FrameSeq,
        MEDIAPIPE_FACE_LANDMARK_COUNT, MonoTimeNs,
    };

    #[test]
    fn canonical_mediapipe_sample_enters_the_fixed_68_point_contract() {
        let sample = FaceTrackingSample::try_new(
            FrameSeq(7),
            MonoTimeNs(10),
            MonoTimeNs(11),
            MonoTimeNs(12),
            CameraFaceTransform::identity(),
            [0.5, 0.5],
            Arc::from(vec![
                FaceLandmark {
                    x: 0.5,
                    y: 0.5,
                    ..FaceLandmark::default()
                };
                MEDIAPIPE_FACE_LANDMARK_COUNT
            ]),
            FaceBlendshapeSet::default(),
            FaceTrackingQuality {
                landmark_presence_median: Some(0.9),
                matrix_orthogonality_error: 0.0,
                matrix_determinant: 1.0,
            },
        )
        .unwrap();
        let fitting = gnm_fitting_sample_from_mediapipe(&sample).unwrap();
        assert_eq!(fitting.source_seq, 7);
        assert_eq!(fitting.observation.normalized_xy().len(), 68);
        assert_eq!(fitting.confidence, 0.9);
    }

    #[test]
    fn out_of_range_landmark_is_not_silently_clamped() {
        let sample = FaceTrackingSample::try_new(
            FrameSeq(7),
            MonoTimeNs(10),
            MonoTimeNs(11),
            MonoTimeNs(12),
            CameraFaceTransform::identity(),
            [0.5, 0.5],
            Arc::from(vec![
                FaceLandmark {
                    x: 1.5,
                    y: 0.5,
                    ..FaceLandmark::default()
                };
                MEDIAPIPE_FACE_LANDMARK_COUNT
            ]),
            FaceBlendshapeSet::default(),
            FaceTrackingQuality {
                matrix_determinant: 1.0,
                ..FaceTrackingQuality::default()
            },
        )
        .unwrap();
        assert!(matches!(
            gnm_fitting_sample_from_mediapipe(&sample),
            Err(GnmRetargetingError::Mapping(GnmFitError::OutOfRange { .. }))
        ));
    }
}
