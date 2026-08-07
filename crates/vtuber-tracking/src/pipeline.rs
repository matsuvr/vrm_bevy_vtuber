//! Neutral-relative head pose generation.
//!
//! Connects a calibrated [`NeutralProfile`] with a live
//! [`RawFaceObservation`] through the weighted Kabsch solver to produce a
//! semantic head pose and a pose confidence. This module does not implement
//! loss recovery, filtering, or expression normalization; those are handled
//! by later stages of the tracking pipeline.

use std::error::Error;
use std::fmt;

use vtuber_core::types::{FrameSeq, HeadPose, Landmark3, MonoTimeNs, RawFaceObservation};

use crate::calibration::NeutralProfile;
use crate::pose::{LandmarkSet, PoseError, solve_relative_pose};

/// Why head pose could not be computed for a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PoseFailureReason {
    /// The observation's landmark schema does not match the neutral profile.
    SchemaMismatch,
    /// The number of landmarks differs between the neutral profile and the
    /// current observation.
    LandmarkCountMismatch {
        /// Number of neutral landmarks.
        neutral: usize,
        /// Number of current landmarks.
        current: usize,
    },
    /// Too few landmarks were available for the Kabsch solver.
    InsufficientLandmarks,
    /// The point cloud is degenerate (collinear, all identical, or a
    /// reflection was detected).
    DegeneratePointCloud,
    /// The observation contained non-finite or out-of-range values.
    InvalidObservation,
}

impl fmt::Display for PoseFailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaMismatch => {
                write!(f, "landmark schema mismatch between neutral and current")
            }
            Self::LandmarkCountMismatch { neutral, current } => {
                write!(
                    f,
                    "landmark count mismatch: neutral={neutral}, current={current}"
                )
            }
            Self::InsufficientLandmarks => write!(f, "insufficient landmarks for pose solving"),
            Self::DegeneratePointCloud => write!(f, "degenerate neutral or current point cloud"),
            Self::InvalidObservation => write!(f, "invalid observation values"),
        }
    }
}

/// A successfully computed neutral-relative head pose for one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadPoseFrame {
    /// Sequence of the source video frame.
    pub source_seq: FrameSeq,
    /// When the source frame was captured.
    pub captured_at: MonoTimeNs,
    /// When this pose frame was produced.
    pub produced_at: MonoTimeNs,
    /// Neutral-relative head pose.
    pub pose: HeadPose,
    /// Aggregated pose confidence in `[0, 1]`.
    pub confidence: f32,
}

/// A head pose computation failure that still preserves source timing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadPoseFailure {
    /// Sequence of the source video frame.
    pub source_seq: FrameSeq,
    /// When the source frame was captured.
    pub captured_at: MonoTimeNs,
    /// Why pose computation failed.
    pub reason: PoseFailureReason,
}

impl fmt::Display for HeadPoseFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "head pose failed for seq {:?} captured at {:?}: {}",
            self.source_seq, self.captured_at, self.reason
        )
    }
}

impl Error for HeadPoseFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// Computes the neutral-relative head pose for `observation`.
///
/// The current landmarks are aligned to `neutral.landmarks` with the weighted
/// Kabsch solver. The resulting rotation is converted to the semantic
/// yaw/pitch/roll convention from `DESIGN.md` §11.6.
///
/// Pose confidence is derived from the overall face confidence and the mean
/// landmark visibility. It is always in `[0, 1]`.
///
/// # Errors
///
/// Returns [`HeadPoseFailure`] when the observation cannot be solved. The
/// failure carries the original `source_seq` and `captured_at` so callers can
/// correlate it with the input stream. This function does not reuse a
/// previously computed pose on error.
///
/// # Exclusions
///
/// This function does not perform filtering, loss hold, or recovery blending.
/// Callers must apply those separately.
pub fn compute_neutral_relative_pose(
    neutral: &NeutralProfile,
    observation: &RawFaceObservation,
    produced_at: MonoTimeNs,
) -> Result<HeadPoseFrame, HeadPoseFailure> {
    let fail = |reason: PoseFailureReason| {
        Err(HeadPoseFailure {
            source_seq: observation.source_seq,
            captured_at: observation.captured_at,
            reason,
        })
    };

    if !observation_is_valid(observation) {
        return fail(PoseFailureReason::InvalidObservation);
    }

    if neutral.schema != observation.schema {
        return fail(PoseFailureReason::SchemaMismatch);
    }

    if neutral.landmarks.len() != observation.landmarks.len() {
        return fail(PoseFailureReason::LandmarkCountMismatch {
            neutral: neutral.landmarks.len(),
            current: observation.landmarks.len(),
        });
    }

    let neutral_set = landmarks_to_set(&neutral.landmarks);
    let current_set = landmarks_to_set(&observation.landmarks);

    let alignment = match solve_relative_pose(&neutral_set, &current_set) {
        Ok(a) => a,
        Err(err) => return fail(map_pose_error(err)),
    };

    let confidence = pose_confidence(observation, &current_set);

    Ok(HeadPoseFrame {
        source_seq: observation.source_seq,
        captured_at: observation.captured_at,
        produced_at,
        pose: alignment.pose,
        confidence,
    })
}

fn observation_is_valid(observation: &RawFaceObservation) -> bool {
    observation.face_confidence.is_finite()
        && (0.0..=1.0).contains(&observation.face_confidence)
        && !observation.landmarks.is_empty()
        && observation.landmarks.iter().all(|lm| {
            lm.x.is_finite()
                && lm.y.is_finite()
                && lm.z.is_finite()
                && lm.visibility.is_finite()
                && (0.0..=1.0).contains(&lm.visibility)
        })
}

fn landmarks_to_set(landmarks: &[Landmark3]) -> LandmarkSet {
    let mut set = LandmarkSet::new();
    for lm in landmarks {
        set.push([lm.x, lm.y, lm.z], lm.visibility);
    }
    set
}

fn pose_confidence(observation: &RawFaceObservation, current: &LandmarkSet) -> f32 {
    if current.points.is_empty() {
        return 0.0;
    }
    let mean_visibility = current
        .points
        .iter()
        .map(|p| p.weight.clamp(0.0, 1.0))
        .sum::<f32>()
        / current.points.len() as f32;
    (observation.face_confidence * mean_visibility).clamp(0.0, 1.0)
}

fn map_pose_error(err: PoseError) -> PoseFailureReason {
    match err {
        PoseError::InsufficientPoints(_) => PoseFailureReason::InsufficientLandmarks,
        PoseError::DegeneratePointCloud
        | PoseError::ZeroWeight(_)
        | PoseError::ReflectionDetected => PoseFailureReason::DegeneratePointCloud,
    }
}

#[cfg(test)]
mod neutral_relative_pose {
    use super::*;
    use approx::assert_relative_eq;
    use vtuber_core::types::{LandmarkSchemaId, NormalizedRect, RawExpressionObservation};

    fn observation(
        seq: u64,
        landmarks: Vec<Landmark3>,
        face_confidence: f32,
    ) -> RawFaceObservation {
        RawFaceObservation {
            source_seq: FrameSeq(seq),
            captured_at: MonoTimeNs(seq * 33_333_333),
            inference_started_at: MonoTimeNs(seq * 33_333_333 + 5_000_000),
            inference_finished_at: MonoTimeNs(seq * 33_333_333 + 25_000_000),
            face_confidence,
            landmarks,
            blendshapes: None,
            expressions: RawExpressionObservation {
                blink_left: 0.1,
                blink_left_confidence: 0.9,
                blink_right: 0.1,
                blink_right_confidence: 0.9,
                mouth_open: 0.05,
                mouth_open_confidence: 0.9,
            },
            roi: NormalizedRect::default(),
            schema: LandmarkSchemaId("pipeline-test"),
        }
    }

    fn profile_from_landmarks(landmarks: Vec<Landmark3>) -> NeutralProfile {
        NeutralProfile {
            version: 1,
            schema: LandmarkSchemaId("pipeline-test"),
            landmarks,
            head_pose: HeadPose::default(),
            blink_left_baseline: 0.1,
            blink_right_baseline: 0.1,
            mouth_open_baseline: 0.05,
            face_scale: 1.0,
            confidence_baseline: 0.9,
            collected_at: MonoTimeNs(0),
            model_hash: None,
            camera_fingerprint: None,
        }
    }

    fn face_landmarks() -> Vec<Landmark3> {
        crate::pose::synthetic_face_points()
            .into_iter()
            .map(|p| Landmark3 {
                x: p[0],
                y: p[1],
                z: p[2],
                visibility: 1.0,
            })
            .collect()
    }

    fn rotate_landmarks(landmarks: &[Landmark3], pose: HeadPose) -> Vec<Landmark3> {
        use nalgebra::OVector;
        let q = crate::pose::semantic_pose_to_quaternion(pose);
        landmarks
            .iter()
            .map(|lm| {
                let v = OVector::<f32, nalgebra::U3>::new(lm.x, lm.y, lm.z);
                let r = q * v;
                Landmark3 {
                    x: r.x,
                    y: r.y,
                    z: r.z,
                    visibility: lm.visibility,
                }
            })
            .collect()
    }

    #[test]
    fn neutral_observation_yields_identity() {
        let neutral = profile_from_landmarks(face_landmarks());
        let current = observation(1, face_landmarks(), 0.9);
        let produced = MonoTimeNs(1_000_000_000);

        let frame = compute_neutral_relative_pose(&neutral, &current, produced)
            .expect("neutral observation should solve");

        assert_eq!(frame.source_seq, FrameSeq(1));
        assert_eq!(frame.captured_at, MonoTimeNs(33_333_333));
        assert_eq!(frame.produced_at, produced);
        assert_relative_eq!(frame.pose.yaw_rad, 0.0, epsilon = 1e-4);
        assert_relative_eq!(frame.pose.pitch_rad, 0.0, epsilon = 1e-4);
        assert_relative_eq!(frame.pose.roll_rad, 0.0, epsilon = 1e-4);
        assert_relative_eq!(frame.confidence, 0.9, epsilon = 1e-6);
    }

    #[test]
    fn known_synthetic_rotations_return_expected_pose() {
        let neutral = profile_from_landmarks(face_landmarks());
        let cases = [
            HeadPose {
                yaw_rad: 15.0f32.to_radians(),
                pitch_rad: 0.0,
                roll_rad: 0.0,
            },
            HeadPose {
                yaw_rad: 0.0,
                pitch_rad: 10.0f32.to_radians(),
                roll_rad: 0.0,
            },
            HeadPose {
                yaw_rad: 0.0,
                pitch_rad: 0.0,
                roll_rad: -12.0f32.to_radians(),
            },
            HeadPose {
                yaw_rad: 15.0f32.to_radians(),
                pitch_rad: 10.0f32.to_radians(),
                roll_rad: -12.0f32.to_radians(),
            },
        ];

        for expected in cases {
            let rotated = rotate_landmarks(&face_landmarks(), expected);
            let obs = observation(10, rotated, 0.85);
            let frame = compute_neutral_relative_pose(&neutral, &obs, MonoTimeNs(0))
                .unwrap_or_else(|e| panic!("failed for {expected:?}: {e}"));

            assert_relative_eq!(frame.pose.yaw_rad, expected.yaw_rad, epsilon = 1e-3);
            assert_relative_eq!(frame.pose.pitch_rad, expected.pitch_rad, epsilon = 1e-3);
            assert_relative_eq!(frame.pose.roll_rad, expected.roll_rad, epsilon = 1e-3);
            assert_relative_eq!(frame.confidence, 0.85, epsilon = 1e-6);
        }
    }

    #[test]
    fn pose_error_preserves_current_timestamps() {
        let neutral = profile_from_landmarks(face_landmarks());
        let valid = observation(1, face_landmarks(), 0.9);
        let produced = MonoTimeNs(1_000_000_000);
        let _ = compute_neutral_relative_pose(&neutral, &valid, produced)
            .expect("valid observation should solve");

        let mut bad_landmarks = face_landmarks();
        // Make all landmarks collinear on the X axis.
        for (i, lm) in bad_landmarks.iter_mut().enumerate() {
            lm.x = i as f32 * 0.1;
            lm.y = 0.0;
            lm.z = 0.0;
        }
        let bad = observation(2, bad_landmarks, 0.9);
        let failure = compute_neutral_relative_pose(&neutral, &bad, produced)
            .expect_err("degenerate observation should fail");

        assert_eq!(failure.source_seq, FrameSeq(2));
        assert_eq!(failure.captured_at, MonoTimeNs(66_666_666));
        assert_eq!(failure.reason, PoseFailureReason::DegeneratePointCloud);
    }

    #[test]
    fn schema_mismatch_returns_dedicated_reason() {
        let mut neutral = profile_from_landmarks(face_landmarks());
        neutral.schema = LandmarkSchemaId("neutral-schema");
        let current = observation(1, face_landmarks(), 0.9);

        let failure = compute_neutral_relative_pose(&neutral, &current, MonoTimeNs(0))
            .expect_err("schema mismatch should fail");

        assert_eq!(failure.reason, PoseFailureReason::SchemaMismatch);
    }

    #[test]
    fn landmark_count_mismatch_returns_dedicated_reason() {
        let neutral = profile_from_landmarks(face_landmarks());
        let mut current = face_landmarks();
        current.pop();
        let obs = observation(1, current, 0.9);

        let failure = compute_neutral_relative_pose(&neutral, &obs, MonoTimeNs(0))
            .expect_err("count mismatch should fail");

        assert_eq!(
            failure.reason,
            PoseFailureReason::LandmarkCountMismatch {
                neutral: face_landmarks().len(),
                current: face_landmarks().len() - 1,
            }
        );
    }

    #[test]
    fn invalid_observation_values_return_dedicated_reason() {
        let neutral = profile_from_landmarks(face_landmarks());
        let mut landmarks = face_landmarks();
        landmarks[0].x = f32::NAN;
        let obs = observation(1, landmarks, 0.9);

        let failure = compute_neutral_relative_pose(&neutral, &obs, MonoTimeNs(0))
            .expect_err("invalid observation should fail");

        assert_eq!(failure.reason, PoseFailureReason::InvalidObservation);
    }

    #[test]
    fn confidence_reflects_mean_visibility() {
        let neutral = profile_from_landmarks(face_landmarks());
        let mut landmarks = face_landmarks();
        for lm in &mut landmarks {
            lm.visibility = 0.5;
        }
        let obs = observation(1, landmarks, 0.8);

        let frame = compute_neutral_relative_pose(&neutral, &obs, MonoTimeNs(0))
            .expect("observation should solve");

        assert_relative_eq!(frame.confidence, 0.4, epsilon = 1e-6);
    }

    #[test]
    fn no_previous_value_is_reused_after_error() {
        let neutral = profile_from_landmarks(face_landmarks());
        let valid = compute_neutral_relative_pose(
            &neutral,
            &observation(1, face_landmarks(), 0.9),
            MonoTimeNs(0),
        )
        .expect("valid");

        // Subsequent failure must not borrow or return the earlier pose.
        let mut bad = face_landmarks();
        bad.clear();
        let bad_obs = observation(2, bad, 0.9);
        let failure = compute_neutral_relative_pose(&neutral, &bad_obs, MonoTimeNs(0))
            .expect_err("empty landmarks should fail");

        // The successful frame is independent of the later failure.
        assert_eq!(failure.source_seq, FrameSeq(2));
        assert!(valid.source_seq != failure.source_seq);
    }
}
