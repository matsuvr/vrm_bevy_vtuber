//! Tracking pipeline: assembles filtered `AvatarControlFrame` values from
//! raw face observations.
//!
//! The pipeline wires together the subsystems implemented in earlier
//! milestones:
//!
//! 1. Confidence synthesis and hysteresis gating.
//! 2. Tracking state machine.
//! 3. Neutral-relative head pose solving.
//! 4. Head rotation smoothing.
//! 5. Expression normalization and smoothing.
//! 6. Loss hold, neutral decay, and recovery blending.
//!
//! The result is a single [`AvatarControlFrame`] per input observation, or
//! `None` when no frame should be published. All timing uses the caller's
//! monotonic timestamp and a caller-supplied delta-time so that recorded
//! streams replay deterministically.

use std::error::Error;
use std::fmt;
use std::time::Duration;

use vtuber_core::control::{CalibrationSettings, TrackingPipelineSettings};
#[cfg(test)]
use vtuber_core::types::LandmarkSchemaId;
#[cfg(test)]
use vtuber_core::types::NamedCoefficient;
use vtuber_core::types::{
    AvatarControlFrame, FrameSeq, GazePose, HeadPose, Landmark3, MonoTimeNs, RawFaceObservation,
    TrackingState,
};

use crate::calibration::{NeutralProfile, NeutralValidationSettings};
use crate::confidence::{
    ConfidenceAssessment, ConfidenceGate, ConfidenceGateParams, ConfidenceInputs,
    ConfidencePolicies, synthesize,
};
use crate::filter::{
    ExpressionCalibration, ExpressionCalibrationError, ExpressionFilter, ExpressionFilterParams,
    ExpressionRange, HeadFilterParams, HeadRotationFilter,
};
use crate::loss_recovery::{LossRecovery, LossRecoveryParams};
use crate::pose::{
    LandmarkSet, PoseError, quaternion_to_semantic_pose, semantic_pose_to_quaternion,
    solve_relative_pose,
};
use crate::state_machine::{StateMachineParams, TrackingStateMachine, TransitionInput};

// -----------------------------------------------------------------------------
// Existing neutral-relative pose code (M1-03-004)
// -----------------------------------------------------------------------------

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

// -----------------------------------------------------------------------------
// Pipeline assembly (M1-03-010)
// -----------------------------------------------------------------------------

/// Errors that can occur while constructing a [`TrackingPipeline`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PipelineConfigError {
    /// The confidence-gate parameters are invalid.
    ConfidenceGate,
    /// The state-machine parameters are invalid.
    StateMachine,
    /// The loss-recovery parameters are invalid.
    LossRecovery,
    /// The expression calibration derived from the neutral profile is invalid.
    ExpressionCalibration,
}

impl fmt::Display for PipelineConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfidenceGate => write!(f, "invalid confidence-gate parameters"),
            Self::StateMachine => write!(f, "invalid state-machine parameters"),
            Self::LossRecovery => write!(f, "invalid loss-recovery parameters"),
            Self::ExpressionCalibration => write!(f, "invalid expression calibration"),
        }
    }
}

impl Error for PipelineConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// Runtime configuration for the tracking pipeline.
///
/// This is the tracking-side counterpart to
/// [`TrackingPipelineSettings`](vtuber_core::control::TrackingPipelineSettings).
/// It groups every subsystem parameter the pipeline needs to run.
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineConfig {
    /// Calibration collection settings.
    pub calibration: CalibrationSettings,
    /// Neutral-reference validation settings.
    pub validation: NeutralValidationSettings,
    /// Confidence hysteresis gate parameters.
    pub confidence_gate: ConfidenceGateParams,
    /// Tracking state-machine timing.
    pub state_machine: StateMachineParams,
    /// Head rotation filter parameters.
    pub head_filter: HeadFilterParams,
    /// Expression normalization and smoothing parameters.
    pub expression_filter: ExpressionFilterParams,
    /// Loss hold / decay / recovery timing.
    pub loss_recovery: LossRecoveryParams,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            calibration: CalibrationSettings::new(),
            validation: NeutralValidationSettings::default(),
            confidence_gate: ConfidenceGateParams::default(),
            state_machine: StateMachineParams::default(),
            head_filter: HeadFilterParams::default(),
            expression_filter: ExpressionFilterParams::default(),
            loss_recovery: LossRecoveryParams::default(),
        }
    }
}

impl PipelineConfig {
    /// Creates a configuration from persisted pipeline settings.
    ///
    /// Runtime parameters use their documented defaults; only the persisted
    /// calibration settings are taken from `settings`.
    #[must_use]
    pub fn from_settings(settings: &TrackingPipelineSettings) -> Self {
        Self {
            calibration: settings.calibration().clone(),
            ..Self::default()
        }
    }
}

/// Result of a single pipeline update.
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineUpdate {
    /// The emitted control frame, if any.
    pub frame: Option<AvatarControlFrame>,
    /// Confidence assessment for this frame.
    pub confidence: ConfidenceAssessment,
    /// Tracking state after this update.
    pub state: TrackingState,
}

/// Owns the end-to-end tracking pipeline.
///
/// `TrackingPipeline` is single-threaded and contains no worker handles,
/// channels, or rendering state. It is suitable for use on the Bevy main
/// thread or in deterministic replay tests.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackingPipeline {
    config: PipelineConfig,
    profile: Option<NeutralProfile>,
    expression_filter: ExpressionFilter,
    head_filter: HeadRotationFilter,
    confidence_gate: ConfidenceGate,
    state_machine: TrackingStateMachine,
    loss_recovery: LossRecovery,
}

impl TrackingPipeline {
    /// Creates a new pipeline from a validated configuration.
    ///
    /// # Errors
    ///
    /// Returns [`PipelineConfigError`] if any subsystem parameter is invalid.
    pub fn new(config: PipelineConfig) -> Result<Self, PipelineConfigError> {
        let expression_filter =
            ExpressionFilter::new(default_expression_calibration(), config.expression_filter);
        let head_filter = HeadRotationFilter::new(config.head_filter);
        let confidence_gate = ConfidenceGate::new(config.confidence_gate)
            .map_err(|_| PipelineConfigError::ConfidenceGate)?;
        let state_machine = TrackingStateMachine::new(config.state_machine)
            .map_err(|_| PipelineConfigError::StateMachine)?;
        let loss_recovery = LossRecovery::new(config.loss_recovery)
            .map_err(|_| PipelineConfigError::LossRecovery)?;

        Ok(Self {
            config,
            profile: None,
            expression_filter,
            head_filter,
            confidence_gate,
            state_machine,
            loss_recovery,
        })
    }

    /// Returns the active neutral profile, if any.
    #[must_use]
    pub fn profile(&self) -> Option<&NeutralProfile> {
        self.profile.as_ref()
    }

    /// Returns `true` if a calibrated neutral profile is available.
    #[must_use]
    pub fn is_calibrated(&self) -> bool {
        self.profile.is_some()
    }

    /// Applies a new neutral profile and resets filters so that smoothing
    /// state does not blend data from a previous calibration.
    ///
    /// # Errors
    ///
    /// Returns [`PipelineConfigError::ExpressionCalibration`] if the profile
    /// cannot be turned into expression calibration ranges.
    pub fn apply_calibration(
        &mut self,
        profile: NeutralProfile,
    ) -> Result<(), PipelineConfigError> {
        let calibration = expression_calibration_from_profile(&profile)
            .map_err(|_| PipelineConfigError::ExpressionCalibration)?;
        self.expression_filter = ExpressionFilter::new(calibration, self.config.expression_filter);
        self.head_filter.reset();
        self.profile = Some(profile);
        Ok(())
    }

    /// Clears calibration and resets all smoothing state.
    pub fn reset_calibration(&mut self) {
        self.profile = None;
        self.head_filter.reset();
        self.expression_filter = ExpressionFilter::new(
            default_expression_calibration(),
            self.config.expression_filter,
        );
    }

    /// Resets filters and tracking state without changing calibration.
    pub fn reset(&mut self) {
        self.head_filter.reset();
        self.expression_filter.reset();
        self.confidence_gate.reset();
        self.state_machine = TrackingStateMachine::new(self.config.state_machine)
            .expect("config was validated in constructor");
        self.loss_recovery = LossRecovery::new(self.config.loss_recovery)
            .expect("config was validated in constructor");
    }

    /// Runs one frame through the pipeline.
    ///
    /// `observation` is the latest inference output, or `None` when no face
    /// was detected. `now` is the monotonic timestamp to stamp on the
    /// produced frame. `dt` is the elapsed wall time since the previous
    /// update and is used by the state machine and loss recovery.
    ///
    /// The returned frame is `None` only when the pipeline has no prior
    /// state to hold or return to neutral from. Once a face has been
    /// tracked at least once, lost-face periods still emit held or decaying
    /// frames.
    #[must_use]
    pub fn update(
        &mut self,
        observation: Option<&RawFaceObservation>,
        now: MonoTimeNs,
        dt: Duration,
    ) -> PipelineUpdate {
        let calibration_available = self.profile.is_some();

        // 1. Pose solve (only meaningful when calibrated).
        let pose_result = self.profile.as_ref().and_then(|profile| {
            observation.map(|obs| compute_neutral_relative_pose(profile, obs, now))
        });

        // 2. Frame confidence from all available sources.
        let frame_confidence = self.synthesize_confidence(observation, &pose_result);

        // 3. Hysteresis gate.
        let confidence_assessment = self.confidence_gate.update(frame_confidence);

        // 4. Tracking state machine.
        let transition = self.state_machine.update(TransitionInput {
            signal: confidence_assessment.signal,
            dt,
            observation,
            calibration_available,
        });

        // 5. Apply state-machine actions.
        for action in &transition.actions {
            match action {
                crate::state_machine::TrackingAction::ResetFilters => {
                    self.head_filter.reset();
                    self.expression_filter.reset();
                }
                crate::state_machine::TrackingAction::StartHold
                | crate::state_machine::TrackingAction::StartReturnToNeutral => {}
            }
        }

        // 6. Update filters and build a tracked frame when a face is present.
        let tracked = observation.map(|obs| {
            let head = self.update_head_filter(&pose_result, now);
            let expressions = self.expression_filter.update(&obs.expressions, now);
            let gaze = extract_gaze(obs);

            AvatarControlFrame {
                source_seq: obs.source_seq,
                captured_at: obs.captured_at,
                produced_at: now,
                confidence: frame_confidence,
                state: transition.current,
                head,
                gaze,
                expressions,
            }
        });

        // 7. Loss recovery blends held/decay/recovery frames.
        let frame = self
            .loss_recovery
            .update(transition.current, dt, tracked, now);

        PipelineUpdate {
            frame,
            confidence: confidence_assessment,
            state: transition.current,
        }
    }

    fn synthesize_confidence(
        &self,
        observation: Option<&RawFaceObservation>,
        pose_result: &Option<Result<HeadPoseFrame, HeadPoseFailure>>,
    ) -> f32 {
        let Some(obs) = observation else {
            return 0.0;
        };

        let landmark = mean_landmark_confidence(&obs.landmarks);
        let pose = pose_result
            .as_ref()
            .and_then(|r| r.as_ref().map(|p| p.confidence).ok());
        let expression = if obs.expressions.is_valid() {
            Some(
                (obs.expressions.blink_left_confidence
                    + obs.expressions.blink_right_confidence
                    + obs.expressions.mouth_open_confidence)
                    / 3.0,
            )
        } else {
            None
        };

        let inputs = ConfidenceInputs {
            detector: Some(obs.face_confidence),
            landmark,
            pose,
            expression,
        };

        synthesize(&inputs, &ConfidencePolicies::default()).unwrap_or(0.0)
    }

    fn update_head_filter(
        &mut self,
        pose_result: &Option<Result<HeadPoseFrame, HeadPoseFailure>>,
        now: MonoTimeNs,
    ) -> HeadPose {
        if let Some(Ok(pose_frame)) = pose_result {
            let q = semantic_pose_to_quaternion(pose_frame.pose);
            let filtered_q = self.head_filter.update(q, now);
            quaternion_to_semantic_pose(filtered_q)
        } else {
            self.head_filter
                .current()
                .map(quaternion_to_semantic_pose)
                .unwrap_or_default()
        }
    }
}

fn mean_landmark_confidence(landmarks: &[Landmark3]) -> Option<f32> {
    if landmarks.is_empty() {
        return None;
    }
    let sum = landmarks
        .iter()
        .map(|lm| lm.visibility.clamp(0.0, 1.0))
        .sum::<f32>();
    Some(sum / landmarks.len() as f32)
}

/// Builds expression calibration ranges from a validated neutral profile.
///
/// The profile stores open-eye and closed-mouth baselines. Fully-closed
/// eye and fully-open mouth values are inferred by adding a fixed margin to
/// the baseline. This is a deliberate MVP simplification; future tasks may
/// store explicit open/closed calibration targets in the profile.
fn expression_calibration_from_profile(
    profile: &NeutralProfile,
) -> Result<ExpressionCalibration, ExpressionCalibrationError> {
    let blink_closed_left = (profile.blink_left_baseline + 0.85).min(1.0);
    let blink_closed_right = (profile.blink_right_baseline + 0.85).min(1.0);
    let mouth_open = (profile.mouth_open_baseline + 0.75).min(1.0);

    Ok(ExpressionCalibration::new(
        ExpressionRange::for_blink(
            profile.blink_left_baseline,
            profile.blink_left_baseline,
            blink_closed_left,
        )?,
        ExpressionRange::for_blink(
            profile.blink_right_baseline,
            profile.blink_right_baseline,
            blink_closed_right,
        )?,
        ExpressionRange::for_mouth(profile.mouth_open_baseline, mouth_open)?,
    ))
}

/// A default identity mapping used when no calibration is available.
///
/// Raw expression coefficients pass through unscaled until the user
/// completes calibration. This keeps the avatar responsive during the
/// initial setup without requiring a persisted profile.
fn default_expression_calibration() -> ExpressionCalibration {
    // These constructors cannot fail for the fixed [0,1] ranges.
    ExpressionCalibration::new(
        ExpressionRange::for_blink(0.0, 0.0, 1.0).unwrap_or_else(|_| unreachable!()),
        ExpressionRange::for_blink(0.0, 0.0, 1.0).unwrap_or_else(|_| unreachable!()),
        ExpressionRange::for_mouth(0.0, 1.0).unwrap_or_else(|_| unreachable!()),
    )
}

/// Extracts a semantic gaze direction from inference blendshape coefficients.
///
/// Looks for the standard MediaPipe-style names `eyeLookLeft`,
/// `eyeLookRight`, `eyeLookUp`, and `eyeLookDown` (case-sensitive). When
/// all four are present, yaw = right - left and pitch = up - down. The
/// result is clamped to a physiologically plausible range.
fn extract_gaze(observation: &RawFaceObservation) -> Option<GazePose> {
    let coefficients = observation.blendshapes.as_ref()?;
    let find = |name: &str| {
        coefficients
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.value.clamp(0.0, 1.0))
            .unwrap_or(0.0)
    };

    let left = find("eyeLookLeft");
    let right = find("eyeLookRight");
    let up = find("eyeLookUp");
    let down = find("eyeLookDown");

    if left == 0.0 && right == 0.0 && up == 0.0 && down == 0.0 {
        return None;
    }

    // Maximum expected eye movement: 35 degrees in either direction.
    const MAX_GAZE_RAD: f32 = 35.0f32.to_radians();
    let yaw = ((right - left) * MAX_GAZE_RAD).clamp(-MAX_GAZE_RAD, MAX_GAZE_RAD);
    let pitch = ((up - down) * MAX_GAZE_RAD).clamp(-MAX_GAZE_RAD, MAX_GAZE_RAD);

    Some(GazePose {
        yaw_rad: yaw,
        pitch_rad: pitch,
    })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod neutral_relative_pose {
    use super::*;
    use approx::assert_relative_eq;
    use vtuber_core::types::{NormalizedRect, RawExpressionObservation};

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

#[cfg(test)]
mod assembly {
    use super::*;
    use approx::assert_relative_eq;
    use vtuber_core::types::{NormalizedRect, RawExpressionObservation};

    fn observation(
        seq: u64,
        landmarks: Vec<Landmark3>,
        expressions: RawExpressionObservation,
    ) -> RawFaceObservation {
        RawFaceObservation {
            source_seq: FrameSeq(seq),
            captured_at: MonoTimeNs(seq * 33_333_333),
            inference_started_at: MonoTimeNs(seq * 33_333_333 + 5_000_000),
            inference_finished_at: MonoTimeNs(seq * 33_333_333 + 25_000_000),
            face_confidence: 0.9,
            landmarks,
            blendshapes: None,
            expressions,
            roi: NormalizedRect::default(),
            schema: LandmarkSchemaId("pipeline-test"),
        }
    }

    fn neutral_profile() -> NeutralProfile {
        NeutralProfile {
            version: 1,
            schema: LandmarkSchemaId("pipeline-test"),
            landmarks: crate::pose::synthetic_face_points()
                .into_iter()
                .map(|p| Landmark3 {
                    x: p[0],
                    y: p[1],
                    z: p[2],
                    visibility: 1.0,
                })
                .collect(),
            head_pose: HeadPose::default(),
            blink_left_baseline: 0.05,
            blink_right_baseline: 0.05,
            mouth_open_baseline: 0.05,
            face_scale: 1.0,
            confidence_baseline: 0.9,
            collected_at: MonoTimeNs(0),
            model_hash: None,
            camera_fingerprint: None,
        }
    }

    fn config() -> PipelineConfig {
        PipelineConfig {
            calibration: CalibrationSettings::try_new(5, 5.0, 0.5, 5.0f32.to_radians(), 0.15)
                .unwrap(),
            validation: NeutralValidationSettings::try_new(5.0f32.to_radians(), 1.0).unwrap(),
            confidence_gate: ConfidenceGateParams {
                enter_threshold: 0.6,
                exit_threshold: 0.3,
                required_consecutive_good: 1,
                required_consecutive_bad: 1,
                max_count: 100,
            },
            state_machine: StateMachineParams {
                hold_duration: Duration::from_millis(100),
                return_duration: Duration::from_millis(200),
            },
            head_filter: HeadFilterParams::with_time_constant(0.05),
            expression_filter: ExpressionFilterParams::with_time_constants(0.03, 0.10),
            loss_recovery: LossRecoveryParams {
                hold_duration: Duration::from_millis(100),
                decay_duration: Duration::from_millis(200),
                recovery_duration: Duration::from_millis(100),
            },
        }
    }

    fn relaxed_expression() -> RawExpressionObservation {
        RawExpressionObservation {
            blink_left: 0.05,
            blink_left_confidence: 0.9,
            blink_right: 0.05,
            blink_right_confidence: 0.9,
            mouth_open: 0.05,
            mouth_open_confidence: 0.9,
        }
    }

    #[test]
    fn pipeline_starts_uncalibrated_and_searching() {
        let pipeline = TrackingPipeline::new(config()).unwrap();
        assert!(!pipeline.is_calibrated());
    }

    #[test]
    fn uncalibrated_observation_does_not_crash() {
        let mut pipeline = TrackingPipeline::new(config()).unwrap();
        let obs = observation(1, vec![], relaxed_expression());
        let update = pipeline.update(
            Some(&obs),
            MonoTimeNs(33_333_333),
            Duration::from_millis(33),
        );
        assert_eq!(update.state, TrackingState::Searching);
        assert!(update.frame.is_none() || update.frame.unwrap().confidence < 0.1);
    }

    #[test]
    fn calibrated_neutral_observation_yields_identity() {
        let mut pipeline = TrackingPipeline::new(config()).unwrap();
        pipeline.apply_calibration(neutral_profile()).unwrap();

        let obs1 = observation(1, neutral_profile().landmarks.clone(), relaxed_expression());
        let _ = pipeline.update(
            Some(&obs1),
            MonoTimeNs(33_333_333),
            Duration::from_millis(33),
        );

        let obs2 = observation(2, neutral_profile().landmarks.clone(), relaxed_expression());
        let update = pipeline.update(
            Some(&obs2),
            MonoTimeNs(66_666_666),
            Duration::from_millis(33),
        );

        let frame = update.frame.expect("should emit frame");
        assert_relative_eq!(frame.head.yaw_rad, 0.0, epsilon = 1e-3);
        assert_relative_eq!(frame.head.pitch_rad, 0.0, epsilon = 1e-3);
        assert_relative_eq!(frame.head.roll_rad, 0.0, epsilon = 1e-3);
        assert!(frame.expressions.blink_left.abs() < 1e-3);
        assert!(frame.expressions.aa.abs() < 1e-3);
        assert_eq!(frame.state, TrackingState::Tracking);
        assert_eq!(frame.source_seq, FrameSeq(2));
    }

    #[test]
    fn output_timestamps_trace_back_to_input() {
        let mut pipeline = TrackingPipeline::new(config()).unwrap();
        pipeline.apply_calibration(neutral_profile()).unwrap();

        let obs = observation(7, neutral_profile().landmarks.clone(), relaxed_expression());
        let update = pipeline.update(
            Some(&obs),
            MonoTimeNs(1_000_000_000),
            Duration::from_millis(33),
        );

        let frame = update.frame.expect("should emit frame");
        assert_eq!(frame.source_seq, FrameSeq(7));
        assert_eq!(frame.captured_at, MonoTimeNs(7 * 33_333_333));
        assert_eq!(frame.produced_at, MonoTimeNs(1_000_000_000));
    }

    #[test]
    fn reset_clears_filter_state_but_keeps_calibration() {
        let mut pipeline = TrackingPipeline::new(config()).unwrap();
        pipeline.apply_calibration(neutral_profile()).unwrap();

        // Move to a non-neutral pose.
        let mut rotated_landmarks = neutral_profile().landmarks.clone();
        rotated_landmarks = rotated_landmarks
            .iter()
            .map(|lm| {
                let q = semantic_pose_to_quaternion(HeadPose {
                    yaw_rad: 0.5,
                    pitch_rad: 0.0,
                    roll_rad: 0.0,
                });
                let v = nalgebra::OVector::<f32, nalgebra::U3>::new(lm.x, lm.y, lm.z);
                let r = q * v;
                Landmark3 {
                    x: r.x,
                    y: r.y,
                    z: r.z,
                    visibility: lm.visibility,
                }
            })
            .collect();
        let obs = observation(1, rotated_landmarks, relaxed_expression());
        let _ = pipeline.update(
            Some(&obs),
            MonoTimeNs(33_333_333),
            Duration::from_millis(33),
        );

        pipeline.reset();

        // After reset, a neutral observation should again be near identity.
        let obs2 = observation(2, neutral_profile().landmarks.clone(), relaxed_expression());
        let update = pipeline.update(
            Some(&obs2),
            MonoTimeNs(66_666_666),
            Duration::from_millis(33),
        );
        let frame = update.frame.expect("should emit frame");
        assert_relative_eq!(frame.head.yaw_rad, 0.0, epsilon = 1e-3);
    }

    #[test]
    fn applying_new_calibration_resets_filter_state() {
        let mut pipeline = TrackingPipeline::new(config()).unwrap();
        pipeline.apply_calibration(neutral_profile()).unwrap();

        let mut rotated_landmarks = neutral_profile().landmarks.clone();
        rotated_landmarks = rotated_landmarks
            .iter()
            .map(|lm| {
                let q = semantic_pose_to_quaternion(HeadPose {
                    yaw_rad: 0.5,
                    pitch_rad: 0.0,
                    roll_rad: 0.0,
                });
                let v = nalgebra::OVector::<f32, nalgebra::U3>::new(lm.x, lm.y, lm.z);
                let r = q * v;
                Landmark3 {
                    x: r.x,
                    y: r.y,
                    z: r.z,
                    visibility: lm.visibility,
                }
            })
            .collect();
        let obs = observation(1, rotated_landmarks, relaxed_expression());
        let _ = pipeline.update(
            Some(&obs),
            MonoTimeNs(33_333_333),
            Duration::from_millis(33),
        );

        // Re-apply the same calibration: filter state must reset.
        pipeline.apply_calibration(neutral_profile()).unwrap();
        let obs2 = observation(2, neutral_profile().landmarks.clone(), relaxed_expression());
        let update = pipeline.update(
            Some(&obs2),
            MonoTimeNs(66_666_666),
            Duration::from_millis(33),
        );
        let frame = update.frame.expect("should emit frame");
        assert_relative_eq!(frame.head.yaw_rad, 0.0, epsilon = 1e-3);
    }

    #[test]
    fn gaze_is_extracted_from_blendshapes() {
        let mut pipeline = TrackingPipeline::new(config()).unwrap();
        pipeline.apply_calibration(neutral_profile()).unwrap();

        let mut obs = observation(1, neutral_profile().landmarks.clone(), relaxed_expression());
        obs.blendshapes = Some(vec![
            NamedCoefficient {
                name: "eyeLookRight".into(),
                value: 0.5,
            },
            NamedCoefficient {
                name: "eyeLookUp".into(),
                value: 0.25,
            },
        ]);

        let update = pipeline.update(
            Some(&obs),
            MonoTimeNs(33_333_333),
            Duration::from_millis(33),
        );
        let frame = update.frame.expect("should emit frame");
        let gaze = frame.gaze.expect("gaze should be present");
        assert!(
            gaze.yaw_rad > 0.0,
            "looking right should yield positive yaw"
        );
        assert!(
            gaze.pitch_rad > 0.0,
            "looking up should yield positive pitch"
        );
    }

    #[test]
    fn no_blendshapes_means_no_gaze() {
        let mut pipeline = TrackingPipeline::new(config()).unwrap();
        pipeline.apply_calibration(neutral_profile()).unwrap();

        let obs = observation(1, neutral_profile().landmarks.clone(), relaxed_expression());
        let update = pipeline.update(
            Some(&obs),
            MonoTimeNs(33_333_333),
            Duration::from_millis(33),
        );
        let frame = update.frame.expect("should emit frame");
        assert!(frame.gaze.is_none());
    }
}
