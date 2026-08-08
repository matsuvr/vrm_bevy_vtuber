//! Tracking state machine.
//!
//! Implements the explicit states and transitions used by the tracking
//! pipeline: [`Searching`](TrackingState::Searching),
//! [`Acquiring`](TrackingState::Acquiring),
//! [`Tracking`](TrackingState::Tracking),
//! [`Degraded`](TrackingState::Degraded),
//! [`LostHold`](TrackingState::LostHold), and
//! [`ReturningNeutral`](TrackingState::ReturningNeutral).
//!
//! The transition logic is expressed as an exhaustive table so that only
//! legal transitions are representable. Inputs are intentionally limited
//! to:
//!
//! - a [`ConfidenceSignal`] from the confidence hysteresis gate,
//! - elapsed time since entering the current state,
//! - a new face observation (or `None`), and
//! - whether calibration is currently available.

use std::time::Duration;

use vtuber_core::RawFaceObservation;

pub use vtuber_core::TrackingState;

use crate::confidence::ConfidenceSignal;

/// Parameters that govern the timing of the tracking state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateMachineParams {
    /// How long to hold the last pose after the face is lost before
    /// returning to neutral.
    pub hold_duration: Duration,
    /// How long the return-to-neutral motion is allowed to take.
    pub return_duration: Duration,
}

impl Default for StateMachineParams {
    fn default() -> Self {
        Self {
            hold_duration: Duration::from_millis(150),
            return_duration: Duration::from_millis(500),
        }
    }
}

/// Errors that can occur while constructing a [`TrackingStateMachine`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StateMachineConfigError {
    /// A timeout duration must be non-zero so that timer-driven transitions
    /// are unambiguous.
    #[error("timeout durations must be non-zero")]
    ZeroDuration,
}

impl StateMachineParams {
    /// Validates the state-machine parameters.
    ///
    /// # Errors
    ///
    /// Returns [`StateMachineConfigError::ZeroDuration`] if either timeout
    /// is zero.
    pub fn validate(&self) -> Result<(), StateMachineConfigError> {
        if self.hold_duration == Duration::ZERO || self.return_duration == Duration::ZERO {
            return Err(StateMachineConfigError::ZeroDuration);
        }
        Ok(())
    }
}

/// Action emitted by a state transition.
///
/// Actions are owned values produced alongside the new state. Callers use
/// them to drive side effects such as resetting filters or starting timed
/// blend/hold motions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrackingAction {
    /// Reset pose and expression filters to avoid stale smoothing.
    ResetFilters,
    /// Begin holding the last valid pose.
    StartHold,
    /// Begin returning smoothly to the calibrated neutral pose.
    StartReturnToNeutral,
}

/// Inputs required for one state-machine update.
///
/// The fields are intentionally restricted to the four sources of
/// information the state machine is allowed to consider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitionInput<'a> {
    /// Confidence signal from the hysteresis gate.
    pub signal: ConfidenceSignal,
    /// Elapsed time since the last update.
    pub dt: Duration,
    /// New observation from inference, or `None` if no face was found.
    pub observation: Option<&'a RawFaceObservation>,
    /// Whether a calibrated neutral reference is available.
    pub calibration_available: bool,
}

/// Result of a single state-machine transition.
#[derive(Clone, Debug, PartialEq)]
pub struct StateTransitionResult {
    /// State before the transition.
    pub previous: TrackingState,
    /// State after the transition.
    pub current: TrackingState,
    /// Actions produced by the transition.
    pub actions: Vec<TrackingAction>,
    /// Elapsed time accumulated in the new state after the transition.
    pub elapsed_in_state: Duration,
}

/// Tracking state machine.
///
/// The machine starts in [`TrackingState::Searching`] and transitions
/// through the table implemented by [`transition_table`]. It owns no
/// thread handles, clock resources, or rendering state.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackingStateMachine {
    params: StateMachineParams,
    state: TrackingState,
    elapsed_in_state: Duration,
}

impl TrackingStateMachine {
    /// Creates a new state machine starting in [`TrackingState::Searching`].
    ///
    /// # Errors
    ///
    /// Returns [`StateMachineConfigError`] if the parameters are invalid.
    pub fn new(params: StateMachineParams) -> Result<Self, StateMachineConfigError> {
        params.validate()?;
        Ok(Self {
            params,
            state: TrackingState::Searching,
            elapsed_in_state: Duration::ZERO,
        })
    }

    /// Creates a state machine with a specific initial state.
    ///
    /// [`TrackingState::Starting`] is normalized to
    /// [`TrackingState::Searching`] because this state machine does not
    /// model the application startup phase.
    ///
    /// # Errors
    ///
    /// Returns [`StateMachineConfigError`] if the parameters are invalid.
    pub fn with_initial_state(
        params: StateMachineParams,
        initial: TrackingState,
    ) -> Result<Self, StateMachineConfigError> {
        params.validate()?;
        Ok(Self {
            params,
            state: normalize_state(initial),
            elapsed_in_state: Duration::ZERO,
        })
    }

    /// Returns the current state.
    #[must_use]
    pub fn state(&self) -> TrackingState {
        self.state
    }

    /// Returns the configured parameters.
    #[must_use]
    pub fn params(&self) -> &StateMachineParams {
        &self.params
    }

    /// Returns the time elapsed since entering the current state.
    #[must_use]
    pub fn elapsed_in_state(&self) -> Duration {
        self.elapsed_in_state
    }

    /// Updates the state machine with one frame of input.
    ///
    /// The update is deterministic and uses only the supplied inputs. Time
    /// accumulation is capped to avoid overflow during long streams.
    pub fn update(&mut self, input: TransitionInput<'_>) -> StateTransitionResult {
        let previous = self.state;
        let elapsed_after = self.elapsed_in_state.saturating_add(input.dt);
        let has_observation = input.observation.is_some();

        let (next, actions, elapsed) = transition_table(
            self.state,
            input.signal,
            has_observation,
            input.calibration_available,
            elapsed_after,
            self.params,
        );

        self.state = next;
        self.elapsed_in_state = elapsed;

        StateTransitionResult {
            previous,
            current: next,
            actions,
            elapsed_in_state: elapsed,
        }
    }
}

fn normalize_state(state: TrackingState) -> TrackingState {
    match state {
        TrackingState::Starting => TrackingState::Searching,
        other => other,
    }
}

/// Transition table for the tracking state machine.
///
/// This function is the single source of truth for legal transitions. The
/// `match` is exhaustive over the six tracked states; any combination not
/// explicitly listed stays in the current state with no action.
#[allow(clippy::too_many_arguments)]
fn transition_table(
    state: TrackingState,
    signal: ConfidenceSignal,
    has_observation: bool,
    calibration_available: bool,
    elapsed: Duration,
    params: StateMachineParams,
) -> (TrackingState, Vec<TrackingAction>, Duration) {
    use ConfidenceSignal::{Acquire, Degrade};
    use TrackingAction::{ResetFilters, StartHold, StartReturnToNeutral};
    use TrackingState::{Acquiring, Degraded, LostHold, ReturningNeutral, Searching, Tracking};

    let none: Vec<TrackingAction> = Vec::new();

    match state {
        Searching => match (signal, has_observation) {
            (Acquire, true) => (Acquiring, vec![ResetFilters], Duration::ZERO),
            _ => (Searching, none, elapsed),
        },

        Acquiring => match (signal, has_observation, calibration_available) {
            (Acquire, true, true) => (Tracking, vec![ResetFilters], Duration::ZERO),
            // Once the gate has become confident (the only way to enter
            // Acquiring), a face with calibration is enough to start tracking
            // even if the gate does not emit another Acquire signal.
            (ConfidenceSignal::None, true, true) => (Tracking, none, Duration::ZERO),
            (Degrade, false, _) => (LostHold, vec![StartHold], Duration::ZERO),
            _ => (Acquiring, none, elapsed),
        },

        Tracking => match (signal, has_observation) {
            (Degrade, true) => (Degraded, none, Duration::ZERO),
            (Degrade, false) => (LostHold, vec![StartHold], Duration::ZERO),
            _ => (Tracking, none, elapsed),
        },

        Degraded => match (signal, has_observation) {
            (Acquire, true) => (Tracking, none, Duration::ZERO),
            (Degrade, false) => (LostHold, vec![StartHold], Duration::ZERO),
            _ => (Degraded, none, elapsed),
        },

        LostHold => {
            if elapsed >= params.hold_duration {
                let carry = elapsed.saturating_sub(params.hold_duration);
                (ReturningNeutral, vec![StartReturnToNeutral], carry)
            } else {
                match (signal, has_observation) {
                    (Acquire, true) => (Acquiring, vec![ResetFilters], Duration::ZERO),
                    _ => (LostHold, none, elapsed),
                }
            }
        }

        ReturningNeutral => {
            if elapsed >= params.return_duration {
                let carry = elapsed.saturating_sub(params.return_duration);
                (Searching, none, carry)
            } else {
                match (signal, has_observation) {
                    (Acquire, true) => (Acquiring, vec![ResetFilters], Duration::ZERO),
                    _ => (ReturningNeutral, none, elapsed),
                }
            }
        }

        TrackingState::Starting => (Searching, none, Duration::ZERO),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> StateMachineParams {
        StateMachineParams {
            hold_duration: Duration::from_millis(100),
            return_duration: Duration::from_millis(200),
        }
    }

    #[test]
    fn invalid_zero_duration_is_rejected() {
        let err = StateMachineParams {
            hold_duration: Duration::ZERO,
            return_duration: Duration::from_millis(100),
        }
        .validate()
        .unwrap_err();
        assert_eq!(err, StateMachineConfigError::ZeroDuration);
    }

    #[test]
    fn starting_state_is_normalized_to_searching() {
        let sm =
            TrackingStateMachine::with_initial_state(params(), TrackingState::Starting).unwrap();
        assert_eq!(sm.state(), TrackingState::Searching);
    }

    fn dummy_obs() -> RawFaceObservation {
        use vtuber_core::{FrameSeq, Landmark3, LandmarkSchemaId, MonoTimeNs, NormalizedRect};
        RawFaceObservation {
            source_seq: FrameSeq(1),
            captured_at: MonoTimeNs(0),
            inference_started_at: MonoTimeNs(0),
            inference_finished_at: MonoTimeNs(0),
            face_confidence: 0.9,
            landmarks: vec![Landmark3 {
                x: 0.5,
                y: 0.5,
                z: 0.0,
                visibility: 1.0,
            }],
            blendshapes: None,
            expressions: vtuber_core::RawExpressionObservation::default(),
            roi: NormalizedRect::default(),
            schema: LandmarkSchemaId("unit-test"),
        }
    }

    #[test]
    fn searching_acquires_face_to_acquiring() {
        let obs = dummy_obs();
        let mut sm = TrackingStateMachine::new(params()).unwrap();
        let result = sm.update(TransitionInput {
            signal: ConfidenceSignal::Acquire,
            dt: Duration::from_millis(16),
            observation: Some(&obs),
            calibration_available: true,
        });
        assert_eq!(result.previous, TrackingState::Searching);
        assert_eq!(result.current, TrackingState::Acquiring);
        assert_eq!(result.actions, vec![TrackingAction::ResetFilters]);
    }

    #[test]
    fn searching_stays_searching_without_observation() {
        let mut sm = TrackingStateMachine::new(params()).unwrap();
        let result = sm.update(TransitionInput {
            signal: ConfidenceSignal::Acquire,
            dt: Duration::from_millis(16),
            observation: None,
            calibration_available: true,
        });
        assert_eq!(result.current, TrackingState::Searching);
        assert!(result.actions.is_empty());
    }

    #[test]
    fn lost_hold_advances_to_returning_neutral_on_timeout() {
        let mut sm =
            TrackingStateMachine::with_initial_state(params(), TrackingState::LostHold).unwrap();
        let result = sm.update(TransitionInput {
            signal: ConfidenceSignal::None,
            dt: params().hold_duration,
            observation: None,
            calibration_available: true,
        });
        assert_eq!(result.previous, TrackingState::LostHold);
        assert_eq!(result.current, TrackingState::ReturningNeutral);
        assert_eq!(result.actions, vec![TrackingAction::StartReturnToNeutral]);
    }

    #[test]
    fn returning_neutral_advances_to_searching_on_timeout() {
        let mut sm =
            TrackingStateMachine::with_initial_state(params(), TrackingState::ReturningNeutral)
                .unwrap();
        let result = sm.update(TransitionInput {
            signal: ConfidenceSignal::None,
            dt: params().return_duration,
            observation: None,
            calibration_available: true,
        });
        assert_eq!(result.previous, TrackingState::ReturningNeutral);
        assert_eq!(result.current, TrackingState::Searching);
        assert!(result.actions.is_empty());
    }
}
