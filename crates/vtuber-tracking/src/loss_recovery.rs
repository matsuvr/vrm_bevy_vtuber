//! Loss hold, neutral decay, and recovery blend for face tracking.
//!
//! When a tracked face disappears, the avatar should not snap instantly to
//! neutral. [`LossRecovery`] holds the last valid [`AvatarControlFrame`] for
//! a short time, then smoothly decays head orientation to neutral and
//! expression coefficients to zero. When the face reappears, a short blend
//! from the current recovered pose to the new tracked frame suppresses
//! jumps.
//!
//! All timing uses the caller-supplied [`Duration`] delta and monotonic
//! timestamps, so behaviour is deterministic and testable without a wall
//! clock.

use std::time::Duration;

use nalgebra::{Quaternion, UnitQuaternion};
use thiserror::Error;

use vtuber_core::types::{
    AvatarControlFrame, ExpressionCoefficients, HeadPose, MonoTimeNs, TrackingState,
};

use crate::pose::{quaternion_to_semantic_pose, semantic_pose_to_quaternion};

/// Minimum duration for the lost-face hold phase.
pub const MIN_HOLD_DURATION: Duration = Duration::from_millis(50);
/// Maximum duration for the lost-face hold phase.
pub const MAX_HOLD_DURATION: Duration = Duration::from_millis(1000);
/// Minimum duration for the return-to-neutral decay phase.
pub const MIN_DECAY_DURATION: Duration = Duration::from_millis(100);
/// Maximum duration for the return-to-neutral decay phase.
pub const MAX_DECAY_DURATION: Duration = Duration::from_millis(2000);
/// Minimum duration for the reacquisition recovery blend.
pub const MIN_RECOVERY_DURATION: Duration = Duration::from_millis(20);
/// Maximum duration for the reacquisition recovery blend.
pub const MAX_RECOVERY_DURATION: Duration = Duration::from_millis(500);

/// Parameters governing loss hold, neutral decay, and recovery blend timing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LossRecoveryParams {
    /// How long to keep emitting the last valid frame after the face is lost.
    ///
    /// Must be within [`MIN_HOLD_DURATION`] and [`MAX_HOLD_DURATION`].
    pub hold_duration: Duration,
    /// How long the return-to-neutral motion takes.
    ///
    /// Must be within [`MIN_DECAY_DURATION`] and [`MAX_DECAY_DURATION`].
    pub decay_duration: Duration,
    /// How long to blend from the recovered pose to a newly tracked frame.
    ///
    /// Must be within [`MIN_RECOVERY_DURATION`] and [`MAX_RECOVERY_DURATION`].
    pub recovery_duration: Duration,
}

impl Default for LossRecoveryParams {
    fn default() -> Self {
        Self {
            hold_duration: Duration::from_millis(150),
            decay_duration: Duration::from_millis(500),
            recovery_duration: Duration::from_millis(150),
        }
    }
}

/// Errors that can occur while constructing a [`LossRecovery`] instance.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum LossRecoveryConfigError {
    /// A duration is zero, so timer-driven transitions would be ambiguous.
    #[error("{field} duration must be non-zero")]
    ZeroDuration {
        /// Name of the offending field.
        field: &'static str,
    },
    /// A duration is outside its permitted fixed range.
    #[error("{field} duration {got:?} is outside [{min:?}, {max:?}]")]
    DurationOutOfRange {
        /// Name of the offending field.
        field: &'static str,
        /// Minimum permitted duration.
        min: Duration,
        /// Maximum permitted duration.
        max: Duration,
        /// Supplied duration.
        got: Duration,
    },
}

impl LossRecoveryParams {
    /// Validates the timing parameters.
    ///
    /// # Errors
    ///
    /// Returns [`LossRecoveryConfigError::ZeroDuration`] if any duration is
    /// zero, or [`LossRecoveryConfigError::DurationOutOfRange`] if a
    /// duration is outside its fixed range.
    pub fn validate(&self) -> Result<(), LossRecoveryConfigError> {
        let ranges: [(&'static str, Duration, Duration, Duration); 3] = [
            (
                "hold_duration",
                MIN_HOLD_DURATION,
                MAX_HOLD_DURATION,
                self.hold_duration,
            ),
            (
                "decay_duration",
                MIN_DECAY_DURATION,
                MAX_DECAY_DURATION,
                self.decay_duration,
            ),
            (
                "recovery_duration",
                MIN_RECOVERY_DURATION,
                MAX_RECOVERY_DURATION,
                self.recovery_duration,
            ),
        ];

        for (field, min, max, got) in ranges {
            if got.is_zero() {
                return Err(LossRecoveryConfigError::ZeroDuration { field });
            }
            if got < min || got > max {
                return Err(LossRecoveryConfigError::DurationOutOfRange {
                    field,
                    min,
                    max,
                    got,
                });
            }
        }

        Ok(())
    }
}

/// Current phase of the loss-recovery state machine.
#[derive(Clone, Debug, PartialEq)]
enum RecoveryState {
    /// No synthetic motion is in progress; pass tracked frames through.
    Idle,
    /// Holding the last valid frame.
    Holding {
        /// Frame being held.
        frame: AvatarControlFrame,
        /// Time spent in the hold phase.
        elapsed: Duration,
    },
    /// Returning from a held or recovered pose to neutral.
    Returning {
        /// Pose at the start of the return motion.
        from: AvatarControlFrame,
        /// Time spent in the return phase.
        elapsed: Duration,
    },
    /// Blending from a recovered pose to a newly tracked frame.
    Recovering {
        /// Pose at the start of the recovery blend.
        from: AvatarControlFrame,
        /// Latest tracked target.
        to: AvatarControlFrame,
        /// Time spent in the recovery phase.
        elapsed: Duration,
    },
}

/// Holds the last valid frame, decays to neutral when a face is lost, and
/// blends back to tracked frames on reacquire.
#[derive(Clone, Debug, PartialEq)]
pub struct LossRecovery {
    params: LossRecoveryParams,
    state: RecoveryState,
    last_valid: Option<AvatarControlFrame>,
    last_output: Option<AvatarControlFrame>,
}

impl LossRecovery {
    /// Creates a new [`LossRecovery`] with the given parameters.
    ///
    /// # Errors
    ///
    /// Returns [`LossRecoveryConfigError`] if the parameters are invalid.
    pub fn new(params: LossRecoveryParams) -> Result<Self, LossRecoveryConfigError> {
        params.validate()?;
        Ok(Self {
            params,
            state: RecoveryState::Idle,
            last_valid: None,
            last_output: None,
        })
    }

    /// Returns the configured parameters.
    #[must_use]
    pub fn params(&self) -> &LossRecoveryParams {
        &self.params
    }

    /// Returns `true` while the last valid frame is being held.
    #[must_use]
    pub fn is_holding(&self) -> bool {
        matches!(self.state, RecoveryState::Holding { .. })
    }

    /// Returns `true` while returning to neutral.
    #[must_use]
    pub fn is_returning(&self) -> bool {
        matches!(self.state, RecoveryState::Returning { .. })
    }

    /// Returns `true` while blending back to a tracked frame.
    #[must_use]
    pub fn is_recovering(&self) -> bool {
        matches!(self.state, RecoveryState::Recovering { .. })
    }

    /// Updates the recovery logic and returns the synthetic or tracked frame
    /// to publish.
    ///
    /// `state` is the external tracking state (for example from
    /// [`TrackingStateMachine`](crate::TrackingStateMachine)). `dt` is the
    /// elapsed time since the last call. `tracked` is the latest valid
    /// tracked frame, if any. `produced_at` is the monotonic timestamp to
    /// stamp on any produced frame.
    ///
    /// The returned frame reuses the source sequence and capture timestamp of
    /// the last valid frame during hold, decay, and recovery so that a stale
    /// observation is not published as a new frame.
    #[must_use]
    pub fn update(
        &mut self,
        state: TrackingState,
        dt: Duration,
        tracked: Option<AvatarControlFrame>,
        produced_at: MonoTimeNs,
    ) -> Option<AvatarControlFrame> {
        // Keep track of the most recent valid frame for future hold phases.
        if let Some(ref t) = tracked {
            self.last_valid = Some(t.clone());
        }

        let old = std::mem::replace(&mut self.state, RecoveryState::Idle);

        let (next, output) = match (state, old, tracked) {
            // A tracked frame is available while we are actively tracking.
            // Pass it through, or start/continue a recovery blend if we were
            // previously holding or returning.
            (
                TrackingState::Tracking | TrackingState::Acquiring,
                RecoveryState::Idle,
                Some(target),
            ) => (RecoveryState::Idle, Some(target)),
            (
                TrackingState::Tracking | TrackingState::Acquiring,
                RecoveryState::Recovering {
                    from,
                    to: _,
                    elapsed,
                },
                Some(target),
            ) => self.advance_recovery(from, target, elapsed, dt, state, produced_at),
            (
                TrackingState::Tracking | TrackingState::Acquiring,
                RecoveryState::Holding { frame, .. } | RecoveryState::Returning { from: frame, .. },
                Some(target),
            ) => {
                let from = self.last_output.clone().unwrap_or(frame);
                self.advance_recovery(from, target, Duration::ZERO, dt, state, produced_at)
            }

            // Lost hold: keep emitting the last frame until the hold timeout.
            (TrackingState::LostHold, RecoveryState::Holding { frame, elapsed }, None) => {
                let elapsed = elapsed.saturating_add(dt);
                if elapsed >= self.params.hold_duration {
                    let carry = elapsed.saturating_sub(self.params.hold_duration);
                    let blended = blend_to_neutral(
                        &frame,
                        fraction(carry, self.params.decay_duration),
                        TrackingState::ReturningNeutral,
                        produced_at,
                    );
                    (
                        RecoveryState::Returning {
                            from: frame,
                            elapsed: carry,
                        },
                        Some(blended),
                    )
                } else {
                    let mut held = frame.clone();
                    held.produced_at = produced_at;
                    held.state = state;
                    (RecoveryState::Holding { frame, elapsed }, Some(held))
                }
            }
            (TrackingState::LostHold, _, None) => {
                if let Some(frame) = self.last_valid.clone() {
                    let mut held = frame.clone();
                    held.produced_at = produced_at;
                    held.state = state;
                    (
                        RecoveryState::Holding {
                            frame: held.clone(),
                            elapsed: dt,
                        },
                        Some(held),
                    )
                } else {
                    (RecoveryState::Idle, None)
                }
            }

            // Returning to neutral: interpolate head to identity and
            // expressions to zero.
            (TrackingState::ReturningNeutral, RecoveryState::Returning { from, elapsed }, None) => {
                let elapsed = elapsed.saturating_add(dt);
                if elapsed >= self.params.decay_duration {
                    let neutral = neutral_frame(&from, produced_at, state);
                    (RecoveryState::Idle, Some(neutral))
                } else {
                    let blended = blend_to_neutral(
                        &from,
                        fraction(elapsed, self.params.decay_duration),
                        state,
                        produced_at,
                    );
                    (RecoveryState::Returning { from, elapsed }, Some(blended))
                }
            }
            (TrackingState::ReturningNeutral, RecoveryState::Holding { frame, elapsed }, None) => {
                // Account for the time spent in LostHold plus the current
                // frame now that the caller has switched to ReturningNeutral.
                let elapsed = elapsed.saturating_add(dt);
                let carry = elapsed.saturating_sub(self.params.hold_duration);
                let blended = blend_to_neutral(
                    &frame,
                    fraction(carry, self.params.decay_duration),
                    state,
                    produced_at,
                );
                (
                    RecoveryState::Returning {
                        from: frame,
                        elapsed: carry,
                    },
                    Some(blended),
                )
            }
            (TrackingState::ReturningNeutral, _, None) => {
                if let Some(from) = self.last_output.clone() {
                    let blended = blend_to_neutral(
                        &from,
                        fraction(dt, self.params.decay_duration),
                        state,
                        produced_at,
                    );
                    (
                        RecoveryState::Returning { from, elapsed: dt },
                        Some(blended),
                    )
                } else {
                    (RecoveryState::Idle, None)
                }
            }

            // Searching after a complete return: emit neutral frames so the
            // avatar does not stay frozen in the last pose.
            (TrackingState::Searching, RecoveryState::Idle, None) => {
                if let Some(last) = self.last_output.clone() {
                    let neutral = neutral_frame(&last, produced_at, state);
                    (RecoveryState::Idle, Some(neutral))
                } else {
                    (RecoveryState::Idle, None)
                }
            }

            // Any other combination: preserve the previous state and output.
            (_, old, _) => (old, self.last_output.clone()),
        };

        self.state = next;
        if let Some(ref out) = output {
            self.last_output = Some(out.clone());
        }
        output
    }

    fn advance_recovery(
        &self,
        from: AvatarControlFrame,
        to: AvatarControlFrame,
        elapsed: Duration,
        dt: Duration,
        state: TrackingState,
        produced_at: MonoTimeNs,
    ) -> (RecoveryState, Option<AvatarControlFrame>) {
        let elapsed = elapsed.saturating_add(dt);
        if elapsed >= self.params.recovery_duration {
            (RecoveryState::Idle, Some(to))
        } else {
            let t = fraction(elapsed, self.params.recovery_duration);
            let blended = blend_frames(&from, &to, t, state, produced_at);
            (
                RecoveryState::Recovering { from, to, elapsed },
                Some(blended),
            )
        }
    }
}

/// Converts a progress fraction from `0` to `total` duration.
fn fraction(elapsed: Duration, total: Duration) -> f32 {
    if total.is_zero() {
        return 1.0;
    }
    (elapsed.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0)
}

/// Linearly interpolates two scalar values.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Returns `target` or `-target`, whichever is closer to `current`.
fn choose_shortest_arc(
    current: UnitQuaternion<f32>,
    target: UnitQuaternion<f32>,
) -> UnitQuaternion<f32> {
    let c = current.quaternion();
    let t = target.quaternion();
    let dot = c.w * t.w + c.i * t.i + c.j * t.j + c.k * t.k;
    if dot < 0.0 { negate(target) } else { target }
}

/// Explicitly negates a unit quaternion, preserving unit norm.
fn negate(q: UnitQuaternion<f32>) -> UnitQuaternion<f32> {
    let inner = q.quaternion();
    UnitQuaternion::from_quaternion(Quaternion::new(-inner.w, -inner.i, -inner.j, -inner.k))
}

/// Builds a frame that is fully neutral in head pose and expressions.
fn neutral_frame(
    base: &AvatarControlFrame,
    produced_at: MonoTimeNs,
    state: TrackingState,
) -> AvatarControlFrame {
    AvatarControlFrame {
        source_seq: base.source_seq,
        captured_at: base.captured_at,
        produced_at,
        confidence: 0.0,
        state,
        head: HeadPose::default(),
        gaze: None,
        expressions: ExpressionCoefficients::default(),
    }
}

/// Blends two frames, keeping the source sequence from `from`.
fn blend_frames(
    from: &AvatarControlFrame,
    to: &AvatarControlFrame,
    t: f32,
    state: TrackingState,
    produced_at: MonoTimeNs,
) -> AvatarControlFrame {
    let q_from = semantic_pose_to_quaternion(from.head);
    let q_to = semantic_pose_to_quaternion(to.head);
    let q_to = choose_shortest_arc(q_from, q_to);
    let q = q_from.slerp(&q_to, t);

    AvatarControlFrame {
        source_seq: from.source_seq,
        captured_at: from.captured_at,
        produced_at,
        confidence: lerp(from.confidence, to.confidence, t),
        state,
        head: quaternion_to_semantic_pose(q),
        gaze: to.gaze,
        expressions: blend_expressions(&from.expressions, &to.expressions, t),
    }
}

/// Blends a frame toward the neutral pose and zero expressions.
fn blend_to_neutral(
    from: &AvatarControlFrame,
    t: f32,
    state: TrackingState,
    produced_at: MonoTimeNs,
) -> AvatarControlFrame {
    let q_from = semantic_pose_to_quaternion(from.head);
    let q_to = UnitQuaternion::identity();
    let q_to = choose_shortest_arc(q_from, q_to);
    let q = q_from.slerp(&q_to, t);

    AvatarControlFrame {
        source_seq: from.source_seq,
        captured_at: from.captured_at,
        produced_at,
        confidence: lerp(from.confidence, 0.0, t),
        state,
        head: quaternion_to_semantic_pose(q),
        gaze: None,
        expressions: blend_expressions(&from.expressions, &ExpressionCoefficients::default(), t),
    }
}

/// Linearly interpolates every expression coefficient.
fn blend_expressions(
    a: &ExpressionCoefficients,
    b: &ExpressionCoefficients,
    t: f32,
) -> ExpressionCoefficients {
    ExpressionCoefficients {
        blink_left: lerp(a.blink_left, b.blink_left, t),
        blink_right: lerp(a.blink_right, b.blink_right, t),
        aa: lerp(a.aa, b.aa, t),
        ih: lerp(a.ih, b.ih, t),
        ou: lerp(a.ou, b.ou, t),
        ee: lerp(a.ee, b.ee, t),
        oh: lerp(a.oh, b.oh, t),
        look_left: lerp(a.look_left, b.look_left, t),
        look_right: lerp(a.look_right, b.look_right, t),
        look_up: lerp(a.look_up, b.look_up, t),
        look_down: lerp(a.look_down, b.look_down, t),
        happy: lerp(a.happy, b.happy, t),
        angry: lerp(a.angry, b.angry, t),
        sad: lerp(a.sad, b.sad, t),
        relaxed: lerp(a.relaxed, b.relaxed, t),
        surprised: lerp(a.surprised, b.surprised, t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use vtuber_core::types::FrameSeq;

    fn test_params() -> LossRecoveryParams {
        LossRecoveryParams {
            hold_duration: Duration::from_millis(100),
            decay_duration: Duration::from_millis(200),
            recovery_duration: Duration::from_millis(100),
        }
    }

    fn frame(
        seq: u64,
        yaw: f32,
        pitch: f32,
        roll: f32,
        expression_value: f32,
    ) -> AvatarControlFrame {
        AvatarControlFrame {
            source_seq: FrameSeq(seq),
            captured_at: MonoTimeNs(seq * 33_333_333),
            produced_at: MonoTimeNs(seq * 33_333_333),
            confidence: 0.9,
            state: TrackingState::Tracking,
            head: HeadPose {
                yaw_rad: yaw,
                pitch_rad: pitch,
                roll_rad: roll,
            },
            gaze: None,
            expressions: ExpressionCoefficients {
                aa: expression_value,
                ..ExpressionCoefficients::default()
            },
        }
    }

    #[test]
    fn loss_recovery_default_params_are_valid() {
        assert!(LossRecoveryParams::default().validate().is_ok());
    }

    #[test]
    fn loss_recovery_rejects_zero_duration() {
        let err = LossRecoveryParams {
            hold_duration: Duration::ZERO,
            ..LossRecoveryParams::default()
        }
        .validate()
        .unwrap_err();
        assert_eq!(
            err,
            LossRecoveryConfigError::ZeroDuration {
                field: "hold_duration"
            }
        );
    }

    #[test]
    fn loss_recovery_rejects_out_of_range_duration() {
        let err = LossRecoveryParams {
            hold_duration: Duration::from_secs(5),
            ..LossRecoveryParams::default()
        }
        .validate()
        .unwrap_err();
        assert!(matches!(
            err,
            LossRecoveryConfigError::DurationOutOfRange {
                field: "hold_duration",
                ..
            }
        ));
    }

    #[test]
    fn loss_recovery_holds_last_pose_during_hold_phase() {
        let mut lr = LossRecovery::new(test_params()).unwrap();
        let tracked = frame(1, 0.5, 0.2, -0.3, 0.8);

        let _ = lr.update(
            TrackingState::Tracking,
            Duration::from_millis(16),
            Some(tracked.clone()),
            MonoTimeNs(33_333_333),
        );
        let held = lr
            .update(
                TrackingState::LostHold,
                Duration::from_millis(50),
                None,
                MonoTimeNs(50_000_000),
            )
            .expect("should emit held frame");

        assert_eq!(held.source_seq, tracked.source_seq);
        assert_relative_eq!(held.head.yaw_rad, tracked.head.yaw_rad, epsilon = 1e-5);
        assert_relative_eq!(held.expressions.aa, tracked.expressions.aa, epsilon = 1e-5);
        assert!(lr.is_holding());
    }

    #[test]
    fn loss_recovery_does_not_stick_after_hold_timeout() {
        let mut lr = LossRecovery::new(test_params()).unwrap();
        let tracked = frame(1, 0.5, 0.0, 0.0, 0.8);

        let _ = lr.update(
            TrackingState::Tracking,
            Duration::from_millis(16),
            Some(tracked.clone()),
            MonoTimeNs(33_333_333),
        );
        let _ = lr.update(
            TrackingState::LostHold,
            Duration::from_millis(100),
            None,
            MonoTimeNs(100_000_000),
        );
        let returning = lr
            .update(
                TrackingState::LostHold,
                Duration::from_millis(50),
                None,
                MonoTimeNs(150_000_000),
            )
            .expect("should emit frame");

        // After the hold timeout (100 ms) plus a bit of decay, the pose should
        // be on its way to neutral, not still equal to the tracked pose.
        assert!(
            returning.head.yaw_rad.abs() < tracked.head.yaw_rad.abs(),
            "yaw should decay toward zero, got {}",
            returning.head.yaw_rad
        );
        assert!(
            returning.expressions.aa < tracked.expressions.aa,
            "expression should decay toward zero, got {}",
            returning.expressions.aa
        );
        assert!(lr.is_returning());
    }

    #[test]
    fn loss_recovery_returns_to_neutral_over_decay_duration() {
        let mut lr = LossRecovery::new(test_params()).unwrap();
        let tracked = frame(1, 0.5, 0.25, -0.4, 0.8);

        let _ = lr.update(
            TrackingState::Tracking,
            Duration::from_millis(16),
            Some(tracked.clone()),
            MonoTimeNs(33_333_333),
        );
        let _ = lr.update(
            TrackingState::LostHold,
            Duration::from_millis(100),
            None,
            MonoTimeNs(100_000_000),
        );
        let final_frame = lr
            .update(
                TrackingState::ReturningNeutral,
                Duration::from_millis(250),
                None,
                MonoTimeNs(350_000_000),
            )
            .expect("should emit neutral frame");

        assert_relative_eq!(final_frame.head.yaw_rad, 0.0, epsilon = 1e-4);
        assert_relative_eq!(final_frame.head.pitch_rad, 0.0, epsilon = 1e-4);
        assert_relative_eq!(final_frame.head.roll_rad, 0.0, epsilon = 1e-4);
        assert_relative_eq!(final_frame.expressions.aa, 0.0, epsilon = 1e-4);
        assert_eq!(final_frame.state, TrackingState::ReturningNeutral);
    }

    #[test]
    fn loss_recovery_uses_shortest_arc_to_neutral() {
        let mut lr = LossRecovery::new(test_params()).unwrap();
        // A large positive yaw close to +pi. The shortest arc to identity is
        // to decrease the magnitude, not to wrap through -pi.
        let tracked = frame(1, 3.0, 0.0, 0.0, 0.0);

        let _ = lr.update(
            TrackingState::Tracking,
            Duration::from_millis(16),
            Some(tracked.clone()),
            MonoTimeNs(33_333_333),
        );
        let _ = lr.update(
            TrackingState::LostHold,
            Duration::from_millis(100),
            None,
            MonoTimeNs(100_000_000),
        );
        let returning = lr
            .update(
                TrackingState::ReturningNeutral,
                Duration::from_millis(100),
                None,
                MonoTimeNs(200_000_000),
            )
            .expect("should be returning");

        // Halfway through the 200 ms decay, yaw should still be positive and
        // roughly half the original magnitude.
        assert!(
            returning.head.yaw_rad > 0.0 && returning.head.yaw_rad < tracked.head.yaw_rad,
            "shortest arc should stay on the positive side, got {}",
            returning.head.yaw_rad
        );
    }

    #[test]
    fn loss_recovery_reacquire_blends_smoothly() {
        let mut lr = LossRecovery::new(test_params()).unwrap();
        let first = frame(1, 0.0, 0.0, 0.0, 0.0);
        let _lost = frame(2, 1.0, 0.0, 0.0, 0.0);
        let reacquired = frame(3, -1.0, 0.0, 0.0, 0.0);

        let _ = lr.update(
            TrackingState::Tracking,
            Duration::from_millis(16),
            Some(first.clone()),
            MonoTimeNs(33_333_333),
        );
        // Lose the face and let it return partly to neutral.
        let _ = lr.update(
            TrackingState::LostHold,
            Duration::from_millis(100),
            None,
            MonoTimeNs(133_333_333),
        );
        let before_reacquire = lr
            .update(
                TrackingState::ReturningNeutral,
                Duration::from_millis(100),
                None,
                MonoTimeNs(233_333_333),
            )
            .unwrap();

        // Reacquire with a large opposing pose.
        let during_recovery = lr
            .update(
                TrackingState::Tracking,
                Duration::from_millis(50),
                Some(reacquired.clone()),
                MonoTimeNs(283_333_333),
            )
            .unwrap();

        // The recovery output should sit between the pre-reacquire pose and
        // the target, not jump all the way to the target.
        assert!(
            during_recovery.head.yaw_rad.abs() < reacquired.head.yaw_rad.abs(),
            "recovery should not jump to target immediately, got {}",
            during_recovery.head.yaw_rad
        );
        assert!(
            during_recovery.head.yaw_rad.signum() != before_reacquire.head.yaw_rad.signum()
                || during_recovery.head.yaw_rad.abs() < before_reacquire.head.yaw_rad.abs(),
            "recovery should move toward target, got {} from {}",
            during_recovery.head.yaw_rad,
            before_reacquire.head.yaw_rad
        );
        assert!(lr.is_recovering());
    }
}
