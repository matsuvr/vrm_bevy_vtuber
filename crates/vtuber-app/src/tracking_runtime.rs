//! Main-thread bridge from inference observations to the pure tracking core.

use std::sync::Arc;
use std::time::Duration;

use bevy::prelude::*;
use vtuber_core::metrics::RateCounter;
use vtuber_core::{
    AvatarControlFrame, FrameSeq, LatestSlot, MonoTimeNs, RawFaceObservation, TrackingState,
};
use vtuber_tracking::{
    CalibrationCollector, CalibrationInput, CalibrationSession, NeutralContext, NeutralReference,
    PipelineConfig, TrackingPipeline,
};

use crate::diagnostics::DiagnosticsSnapshot;
use crate::inference_runtime::InferenceRuntime;
use crate::orchestrator::{CalibrationRequest, Orchestrator};
use crate::ui_model::{CalibrationViewModel, TrackingState as UiTrackingState, UiViewModel};

/// Maximum age of an inference result before it is treated as face-lost.
///
/// The current inference slot carries successful face observations only. If a
/// detector reports no face without publishing an explicit `InferenceOutput`,
/// this watchdog converts the absence of a fresh result into the tracking
/// pipeline's normal `None` input. It prevents a last face observation from
/// being replayed forever while keeping normal 15 Hz inference output from
/// being mistaken for a loss.
const INFERENCE_SILENCE_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, PartialEq)]
enum ObservationDispatch {
    /// No new inference result and the last result is still fresh.
    NoUpdate,
    /// A new, fresh face observation is ready for tracking.
    Face(RawFaceObservation),
    /// The current inference result is absent or stale.
    NoFace,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ObservationGate {
    last_source_seq: Option<FrameSeq>,
}

impl ObservationGate {
    fn dispatch(
        &mut self,
        latest: Option<&RawFaceObservation>,
        now: MonoTimeNs,
    ) -> ObservationDispatch {
        let Some(observation) = latest else {
            return ObservationDispatch::NoFace;
        };

        let age = Duration::from_nanos(now.0.saturating_sub(observation.inference_finished_at.0));
        let fresh = age <= INFERENCE_SILENCE_TIMEOUT;
        let is_new = self.last_source_seq != Some(observation.source_seq);
        if is_new {
            self.last_source_seq = Some(observation.source_seq);
            return if fresh {
                ObservationDispatch::Face(observation.clone())
            } else {
                ObservationDispatch::NoFace
            };
        }

        if fresh {
            ObservationDispatch::NoUpdate
        } else {
            ObservationDispatch::NoFace
        }
    }
}

/// Tracking domain state owned by the Bevy main thread.
#[derive(Resource)]
pub struct TrackingRuntime {
    pipeline: TrackingPipeline,
    collector: CalibrationCollector,
    session: CalibrationSession,
    last_update: Option<MonoTimeNs>,
    last_calibration_source_seq: Option<FrameSeq>,
    last_avatar_generation: vtuber_avatar::AvatarGeneration,
    last_calibration_reject_reason: Option<String>,
    observation_gate: ObservationGate,
    /// Whether the avatar bridge may retain the most recently published
    /// control frame for the current capture session.
    pub control_active: bool,
    /// Latest-only control frame for the avatar bridge.
    pub control_slot: Arc<LatestSlot<AvatarControlFrame>>,
    /// Most recent frame consumed by the avatar bridge.
    pub latest_control: Option<AvatarControlFrame>,
}

impl Default for TrackingRuntime {
    fn default() -> Self {
        let config = PipelineConfig::default();
        let collector = CalibrationCollector::new(config.calibration.clone());
        let pipeline = TrackingPipeline::new(config)
            .expect("default tracking configuration is an internal invariant");
        Self {
            pipeline,
            collector,
            session: CalibrationSession::default(),
            last_update: None,
            last_calibration_source_seq: None,
            last_avatar_generation: vtuber_avatar::AvatarGeneration::default(),
            last_calibration_reject_reason: None,
            observation_gate: ObservationGate::default(),
            control_active: false,
            control_slot: Arc::new(LatestSlot::new()),
            latest_control: None,
        }
    }
}

impl TrackingRuntime {
    /// Returns the latest-only control frame slot.
    #[must_use]
    pub fn control_slot(&self) -> Arc<LatestSlot<AvatarControlFrame>> {
        Arc::clone(&self.control_slot)
    }

    fn reset_calibration(&mut self) {
        self.collector.reset();
        self.session = CalibrationSession::default();
        self.pipeline.reset_calibration();
        self.last_calibration_source_seq = None;
        self.last_calibration_reject_reason = None;
    }
}

/// Applies calibration intents and processes one latest inference result.
pub fn tracking_bridge_system(
    mut tracking: ResMut<TrackingRuntime>,
    inference: Res<InferenceRuntime>,
    lifecycle: Res<vtuber_avatar::AvatarLifecycle>,
    mut orchestrator: ResMut<Orchestrator>,
    mut view_model: ResMut<UiViewModel>,
    mut diagnostics: ResMut<DiagnosticsSnapshot>,
    mut tracking_rate: Local<Option<RateCounter>>,
) {
    if lifecycle.current_generation() != tracking.last_avatar_generation {
        tracking.last_avatar_generation = lifecycle.current_generation();
        // A completed neutral profile is avatar-independent. Preserve it so
        // a model replacement can resume tracking without forcing a second
        // calibration, while an in-progress or failed calibration is reset.
        if tracking.session.profile().is_some() {
            tracking.pipeline.reset();
        } else {
            tracking.reset_calibration();
        }
        tracking.latest_control = None;
        tracking.control_active = false;
        tracking.last_update = None;
    }

    let pipeline_state = orchestrator.pipeline_state();
    let capture_inactive = !orchestrator.capture_desired()
        && matches!(
            pipeline_state,
            crate::orchestrator::PipelineState::Idle | crate::orchestrator::PipelineState::Stopping
        );
    let pipeline_failed = pipeline_state == crate::orchestrator::PipelineState::Failed;
    if capture_inactive || pipeline_failed {
        tracking.pipeline.reset();
        tracking.latest_control = None;
        tracking.control_active = false;
        tracking.last_update = None;
        view_model.tracking = tracking_view(TrackingState::Starting, 0.0);
        view_model.calibration = calibration_view(&tracking);
        diagnostics.tracking_state = format!("{:?}", TrackingState::Starting);
        return;
    }

    if let Some(request) = orchestrator.take_calibration_request() {
        match request {
            CalibrationRequest::Begin | CalibrationRequest::Retry => {
                tracking.reset_calibration();
                let now = vtuber_core::monotonic_now();
                tracking.session =
                    tracking
                        .session
                        .start(now)
                        .unwrap_or(CalibrationSession::Collecting {
                            started_at: now,
                            samples_collected: 0,
                        });
            }
            CalibrationRequest::Cancel => tracking.reset_calibration(),
        }
    }

    let now = vtuber_core::monotonic_now();
    let dt = tracking
        .last_update
        .map(|last| Duration::from_nanos(now.0.saturating_sub(last.0).min(250_000_000)))
        .unwrap_or_else(|| Duration::from_nanos(33_333_333));
    tracking.last_update = Some(now);

    let observation = match tracking
        .observation_gate
        .dispatch(inference.latest_observation.as_ref(), now)
    {
        ObservationDispatch::NoUpdate => {
            // Do not feed the same inference result into the filters again.
            // The previous control frame remains active until a newer face or
            // a face-loss update replaces it.
            view_model.calibration = calibration_view(&tracking);
            return;
        }
        ObservationDispatch::Face(observation) => Some(observation),
        ObservationDispatch::NoFace => None,
    };

    if let Some(observation) = observation.as_ref()
        && matches!(tracking.session, CalibrationSession::Collecting { .. })
    {
        let input = CalibrationInput {
            source_seq: observation.source_seq,
            captured_at: observation.captured_at,
            face_confidence: observation.face_confidence,
            landmarks: observation.landmarks.clone(),
            expressions: observation.expressions,
            schema: observation.schema,
        };
        let should_offer = tracking.last_calibration_source_seq != Some(observation.source_seq);
        let decision = should_offer.then(|| tracking.collector.offer(input));
        if should_offer {
            tracking.last_calibration_source_seq = Some(observation.source_seq);
        }
        if let Some(vtuber_tracking::SampleDecision::Rejected(reason)) = decision {
            tracking.last_calibration_reject_reason = Some(format!("{reason:?}"));
        }
        if tracking.collector.is_ready() {
            let context = NeutralContext::new(now, None, None);
            match NeutralReference::aggregate(
                &tracking.collector,
                &PipelineConfig::default().validation,
                &context,
            ) {
                Ok(profile) => {
                    if tracking.pipeline.apply_calibration(profile.clone()).is_ok() {
                        tracking.session = tracking
                            .session
                            .ready(profile)
                            .and_then(|session| session.complete())
                            .unwrap_or_default();
                    }
                }
                Err(error) => {
                    tracking.last_calibration_reject_reason = Some(format!("neutral: {error:?}"));
                }
            }
        }
    }

    let update = tracking.pipeline.update(observation.as_ref(), now, dt);
    if let Some(frame) = update.frame {
        let _ = tracking.control_slot.publish(frame.clone());
        tracking.latest_control = Some(frame);
        tracking.control_active = true;
        let rate = tracking_rate.get_or_insert_with(|| RateCounter::new(1_000_000_000));
        rate.record(now.0);
    }
    if let Some(rate) = tracking_rate.as_mut() {
        diagnostics.tracking_rate = rate.rate_hz(now.0) as f32;
    } else {
        diagnostics.tracking_rate = 0.0;
    }

    view_model.calibration = calibration_view(&tracking);
    view_model.tracking = tracking_view(update.state, update.confidence.frame_confidence);
    diagnostics.tracking_state = format!("{:?}", update.state);
}

fn calibration_view(runtime: &TrackingRuntime) -> CalibrationViewModel {
    let settings = runtime.collector.settings();
    let metrics = runtime.collector.metrics();
    CalibrationViewModel {
        is_calibrating: matches!(runtime.session, CalibrationSession::Collecting { .. }),
        samples_collected: metrics.accepted.min(u64::from(u32::MAX)) as u32,
        samples_target: settings.required_sample_count().min(u32::MAX as usize) as u32,
        quality_score: (metrics.accepted > 0).then(|| {
            (metrics.accepted as f32 / settings.required_sample_count() as f32).clamp(0.0, 1.0)
        }),
        last_reject_reason: runtime.last_calibration_reject_reason.clone(),
        is_complete: matches!(runtime.session, CalibrationSession::Completed { .. }),
    }
}

fn tracking_view(state: TrackingState, confidence: f32) -> crate::ui_model::TrackingViewModel {
    let state = match state {
        TrackingState::Tracking | TrackingState::Acquiring | TrackingState::Degraded => {
            UiTrackingState::Tracking
        }
        TrackingState::LostHold | TrackingState::ReturningNeutral => UiTrackingState::Lost,
        TrackingState::Starting | TrackingState::Searching => UiTrackingState::Initializing,
    };
    crate::ui_model::TrackingViewModel {
        is_tracking: matches!(state, UiTrackingState::Tracking),
        state,
        confidence: confidence.clamp(0.0, 1.0),
        face_detected: !matches!(state, UiTrackingState::Lost),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtuber_core::types::{
        Landmark3, LandmarkSchemaId, NamedCoefficient, NormalizedRect, RawExpressionObservation,
    };

    fn observation(seq: u64, finished_at: u64) -> RawFaceObservation {
        RawFaceObservation {
            source_seq: FrameSeq(seq),
            captured_at: MonoTimeNs(finished_at.saturating_sub(1_000_000)),
            inference_started_at: MonoTimeNs(finished_at.saturating_sub(500_000)),
            inference_finished_at: MonoTimeNs(finished_at),
            face_confidence: 1.0,
            landmarks: vec![Landmark3::default()],
            blendshapes: Some(vec![NamedCoefficient {
                name: "blinkLeft".into(),
                value: 0.0,
            }]),
            expressions: RawExpressionObservation::default(),
            roi: NormalizedRect {
                x: 0.25,
                y: 0.25,
                width: 0.5,
                height: 0.5,
                rotation_rad: 0.0,
            },
            schema: LandmarkSchemaId("tracking-runtime-test"),
        }
    }

    #[test]
    fn observation_gate_does_not_replay_a_fresh_face() {
        let mut gate = ObservationGate::default();
        let face = observation(7, 1_000_000_000);

        assert!(matches!(
            gate.dispatch(Some(&face), MonoTimeNs(1_050_000_000)),
            ObservationDispatch::Face(_)
        ));
        assert_eq!(
            gate.dispatch(Some(&face), MonoTimeNs(1_100_000_000)),
            ObservationDispatch::NoUpdate
        );
    }

    #[test]
    fn observation_gate_turns_a_stale_face_into_face_loss() {
        let mut gate = ObservationGate::default();
        let face = observation(7, 1_000_000_000);

        let _ = gate.dispatch(Some(&face), MonoTimeNs(1_050_000_000));
        assert_eq!(
            gate.dispatch(Some(&face), MonoTimeNs(1_251_000_000)),
            ObservationDispatch::NoFace
        );
    }

    #[test]
    fn observation_gate_accepts_a_new_face_after_loss() {
        let mut gate = ObservationGate::default();
        let first = observation(7, 1_000_000_000);
        let second = observation(8, 1_300_000_000);

        let _ = gate.dispatch(Some(&first), MonoTimeNs(1_050_000_000));
        let _ = gate.dispatch(Some(&first), MonoTimeNs(1_251_000_000));
        assert!(matches!(
            gate.dispatch(Some(&second), MonoTimeNs(1_350_000_000)),
            ObservationDispatch::Face(_)
        ));
    }

    #[test]
    fn observation_gate_reports_missing_output_as_face_loss() {
        let mut gate = ObservationGate::default();
        assert_eq!(
            gate.dispatch(None, MonoTimeNs(1)),
            ObservationDispatch::NoFace
        );
    }
}
