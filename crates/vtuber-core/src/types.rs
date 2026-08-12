//! Engine-independent domain types.

use std::sync::Arc;

/// Monotonic timestamp in nanoseconds from a process-local epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonoTimeNs(pub u64);

/// Monotonically increasing frame sequence number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameSeq(pub u64);

/// Pixel formats supported for decoded camera frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// 24-bit RGB, 8 bits per channel.
    Rgb8,
    /// 24-bit BGR, 8 bits per channel.
    Bgr8,
    /// 32-bit RGBA, 8 bits per channel.
    Rgba8,
    /// 8-bit luminance.
    Gray8,
}

/// A decoded camera frame owned by the application.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoFrame {
    /// Sequence number assigned by the capture pipeline.
    pub seq: FrameSeq,
    /// When the frame was captured, in monotonic nanoseconds.
    pub captured_at: MonoTimeNs,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row stride in bytes.
    pub stride_bytes: usize,
    /// Pixel format of `data`.
    pub format: PixelFormat,
    /// Owned frame bytes.
    pub data: Arc<[u8]>,
}

/// 3D facial landmark with normalized image coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Landmark3 {
    /// Normalized image X, left = 0, right = 1.
    pub x: f32,
    /// Normalized image Y, top = 0, bottom = 1.
    pub y: f32,
    /// Model-defined relative depth.
    pub z: f32,
    /// Visibility or presence confidence in `[0, 1]`.
    pub visibility: f32,
}

/// Named blendshape or expression coefficient.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedCoefficient {
    /// Expression name, e.g. `blinkLeft` or `aa`.
    pub name: String,
    /// Coefficient in `[0, 1]`.
    pub value: f32,
}

/// Identifies which landmark schema an observation uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LandmarkSchemaId(pub &'static str);

/// Normalized region-of-interest rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NormalizedRect {
    /// Top-left X in `[0, 1]`.
    pub x: f32,
    /// Top-left Y in `[0, 1]`.
    pub y: f32,
    /// Width in `[0, 1]`.
    pub width: f32,
    /// Height in `[0, 1]`.
    pub height: f32,
    /// Rotation in radians.
    pub rotation_rad: f32,
}

/// Raw output from the inference worker.
#[derive(Clone, Debug, PartialEq)]
pub struct InferenceOutput {
    /// Sequence of the source video frame.
    pub source_seq: FrameSeq,
    /// When the source frame was captured.
    pub captured_at: MonoTimeNs,
    /// When inference started.
    pub inference_started_at: MonoTimeNs,
    /// When inference finished.
    pub inference_finished_at: MonoTimeNs,
    /// Observed face, or `None` if no face was detected.
    pub observation: Option<RawFaceObservation>,
}

pub use crate::observation::RawExpressionObservation;

/// A single face observation produced by inference.
#[derive(Clone, Debug, PartialEq)]
pub struct RawFaceObservation {
    /// Sequence of the source video frame.
    pub source_seq: FrameSeq,
    /// When the source frame was captured.
    pub captured_at: MonoTimeNs,
    /// When inference started.
    pub inference_started_at: MonoTimeNs,
    /// When inference finished.
    pub inference_finished_at: MonoTimeNs,
    /// Overall face confidence in `[0, 1]`.
    pub face_confidence: f32,
    /// Facial landmarks.
    pub landmarks: Vec<Landmark3>,
    /// Optional blendshape coefficients.
    pub blendshapes: Option<Vec<NamedCoefficient>>,
    /// Raw expression coefficients before calibration.
    pub expressions: RawExpressionObservation,
    /// Face region of interest.
    pub roi: NormalizedRect,
    /// Landmark schema used by `landmarks`.
    pub schema: LandmarkSchemaId,
}

/// Semantic head pose in radians.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeadPose {
    /// Positive when turning right in the unmirrored image.
    pub yaw_rad: f32,
    /// Positive when the chin goes up.
    pub pitch_rad: f32,
    /// Positive when the head tilts clockwise as viewed in the unmirrored image.
    pub roll_rad: f32,
}

/// Availability and reliability of an eye-in-head gaze observation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GazeTrackingState {
    /// Both eyes provide a reliable common gaze estimate.
    Tracked,
    /// A usable estimate exists, but binocular agreement or visibility is reduced.
    Degraded,
    /// No new eye-in-head observation is available.
    #[default]
    Unavailable,
}

/// Engine-neutral, normalized eye-in-head gaze signal.
///
/// This is not a physical angle. Model-specific conversion to VRM LookAt
/// degrees belongs to the avatar adapter.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GazeSignal {
    /// Horizontal eye-in-head signal in `[-1, 1]`; image right is positive.
    pub horizontal: f32,
    /// Vertical eye-in-head signal in `[-1, 1]`; up is positive.
    pub vertical: f32,
    /// Reliability in `[0, 1]`.
    pub confidence: f32,
    /// Whether this value is tracked, degraded, or unavailable.
    pub state: GazeTrackingState,
}

impl GazeSignal {
    /// Explicit unavailable signal. Unlike centered tracked gaze, it carries no observation.
    pub const UNAVAILABLE: Self = Self {
        horizontal: 0.0,
        vertical: 0.0,
        confidence: 0.0,
        state: GazeTrackingState::Unavailable,
    };

    /// Builds a bounded tracked signal, safely degrading non-finite input to unavailable.
    #[must_use]
    pub fn tracked(horizontal: f32, vertical: f32, confidence: f32) -> Self {
        Self::available(horizontal, vertical, confidence, GazeTrackingState::Tracked)
    }

    /// Builds a bounded degraded signal, safely degrading non-finite input to unavailable.
    #[must_use]
    pub fn degraded(horizontal: f32, vertical: f32, confidence: f32) -> Self {
        Self::available(
            horizontal,
            vertical,
            confidence,
            GazeTrackingState::Degraded,
        )
    }

    fn available(
        horizontal: f32,
        vertical: f32,
        confidence: f32,
        state: GazeTrackingState,
    ) -> Self {
        if !horizontal.is_finite() || !vertical.is_finite() || !confidence.is_finite() {
            return Self::UNAVAILABLE;
        }
        Self {
            horizontal: horizontal.clamp(-1.0, 1.0),
            vertical: vertical.clamp(-1.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            state,
        }
    }

    /// Returns whether this signal contains a current or degraded observation.
    #[must_use]
    pub const fn is_available(self) -> bool {
        !matches!(self.state, GazeTrackingState::Unavailable)
    }
}

/// Expression coefficients applied to the avatar.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExpressionCoefficients {
    /// Left eye blink.
    pub blink_left: f32,
    /// Right eye blink.
    pub blink_right: f32,
    /// `aa` mouth shape.
    pub aa: f32,
    /// `ih` mouth shape.
    pub ih: f32,
    /// `ou` mouth shape.
    pub ou: f32,
    /// `ee` mouth shape.
    pub ee: f32,
    /// `oh` mouth shape.
    pub oh: f32,
    /// Look left expression.
    pub look_left: f32,
    /// Look right expression.
    pub look_right: f32,
    /// Look up expression.
    pub look_up: f32,
    /// Look down expression.
    pub look_down: f32,
    /// Happy expression.
    pub happy: f32,
    /// Angry expression.
    pub angry: f32,
    /// Sad expression.
    pub sad: f32,
    /// Relaxed expression.
    pub relaxed: f32,
    /// Surprised expression.
    pub surprised: f32,
}

/// Tracking state machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrackingState {
    /// Pipeline is starting.
    #[default]
    Starting,
    /// Searching for a face.
    Searching,
    /// Face detected but not yet stable.
    Acquiring,
    /// Face is being tracked normally.
    Tracking,
    /// Tracking confidence is degraded.
    Degraded,
    /// Face was lost; holding last pose briefly.
    LostHold,
    /// Returning to neutral after lost hold expires.
    ReturningNeutral,
}

/// Control frame produced by the tracking filter and consumed by the avatar adapter.
#[derive(Clone, Debug, PartialEq)]
pub struct AvatarControlFrame {
    /// Sequence of the source video frame.
    pub source_seq: FrameSeq,
    /// When the source frame was captured.
    pub captured_at: MonoTimeNs,
    /// When this control frame was produced.
    pub produced_at: MonoTimeNs,
    /// Aggregated tracking confidence in `[0, 1]`.
    pub confidence: f32,
    /// Current tracking state.
    pub state: TrackingState,
    /// Head pose relative to calibrated neutral.
    pub head: HeadPose,
    /// Explicit normalized eye-in-head gaze signal.
    pub gaze: GazeSignal,
    /// Expression coefficients.
    pub expressions: ExpressionCoefficients,
}

#[cfg(test)]
mod gaze_contract_tests {
    use super::*;

    #[test]
    fn centered_tracked_gaze_is_distinct_from_unavailable() {
        let centered = GazeSignal::tracked(0.0, 0.0, 1.0);
        assert!(centered.is_available());
        assert_eq!(centered.horizontal, 0.0);
        assert_eq!(centered.vertical, 0.0);
        assert_ne!(centered, GazeSignal::UNAVAILABLE);
    }

    #[test]
    fn gaze_contract_clamps_ranges_and_rejects_non_finite_values() {
        let bounded = GazeSignal::degraded(2.0, -2.0, 4.0);
        assert_eq!(bounded.horizontal, 1.0);
        assert_eq!(bounded.vertical, -1.0);
        assert_eq!(bounded.confidence, 1.0);
        assert_eq!(
            GazeSignal::tracked(f32::NAN, 0.0, 1.0),
            GazeSignal::UNAVAILABLE
        );
    }
}
