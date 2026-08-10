//! Main-thread bridge from inference observations to the pure tracking core.

use std::sync::Arc;
use std::time::Duration;

use bevy::prelude::*;
use vtuber_core::metrics::RateCounter;
use vtuber_core::{AvatarControlFrame, FrameSeq, LatestSlot, MonoTimeNs, TrackingState};
use vtuber_tracking::{
    CalibrationCollector, CalibrationInput, CalibrationSession, NeutralContext, NeutralReference,
    PipelineConfig, TrackingPipeline,
};

use crate::diagnostics::DiagnosticsSnapshot;
use crate::inference_runtime::InferenceRuntime;
use crate::orchestrator::{CalibrationRequest, Orchestrator};
use crate::ui_model::{CalibrationViewModel, TrackingState as UiTrackingState, UiViewModel};

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
        tracking.reset_calibration();
        tracking.pipeline.reset();
        tracking.latest_control = None;
        tracking.last_update = None;
    }

    if !orchestrator.capture_desired()
        && matches!(
            orchestrator.pipeline_state(),
            crate::orchestrator::PipelineState::Idle
                | crate::orchestrator::PipelineState::Stopping
                | crate::orchestrator::PipelineState::Failed
        )
    {
        tracking.pipeline.reset();
        tracking.latest_control = None;
        tracking.last_update = None;
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

    let observation = inference.latest_observation.clone();
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
