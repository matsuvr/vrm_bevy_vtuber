//! Application bridge for the inference worker.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use vtuber_core::metrics::RateCounter;
use vtuber_core::{FrameSeq, LatestSlot, RawFaceObservation, VideoFrame, monotonic_now};
use vtuber_inference::InferenceStage;
use vtuber_inference::{InferenceController, InferenceWorkerState, RuntimeSettings};

use crate::capture_runtime::CaptureRuntime;
use crate::diagnostics::DiagnosticsSnapshot;
use crate::model_catalog::load_production_descriptor;
use crate::orchestrator::{Orchestrator, OrchestratorError, PipelineState};

/// Filesystem root containing the packaged `assets/models/manifest.toml`.
///
/// Desktop applications provide this from their resource locator. Tests and
/// development runs default to the workspace current directory.
#[derive(Resource, Clone, Debug, Default)]
pub struct InferenceProjectRoot(pub PathBuf);

/// Resource containing the inference controller and its latest output slot.
#[derive(Resource)]
pub struct InferenceRuntime {
    controller: InferenceController,
    frame_slot: Arc<LatestSlot<VideoFrame>>,
    project_root: PathBuf,
    worker_started: bool,
    model_requested: bool,
    last_generation: u64,
    /// Latest decoded observation made available to tracking on the main thread.
    pub latest_observation: Option<RawFaceObservation>,
}

impl InferenceRuntime {
    /// Creates an inference runtime sharing the capture frame slot.
    #[must_use]
    pub fn new(frame_slot: Arc<LatestSlot<VideoFrame>>, project_root: PathBuf) -> Self {
        let output_slot = Arc::new(LatestSlot::new());
        Self {
            controller: InferenceController::new(Arc::clone(&frame_slot), output_slot),
            frame_slot,
            project_root,
            worker_started: false,
            model_requested: false,
            last_generation: 0,
            latest_observation: None,
        }
    }

    /// Returns a status snapshot safe for diagnostics.
    #[must_use]
    pub fn status(&self) -> vtuber_inference::InferenceWorkerStatus {
        self.controller.status()
    }

    /// Returns the output slot used by tracking.
    #[must_use]
    pub fn output_slot(&self) -> Arc<LatestSlot<RawFaceObservation>> {
        self.controller.output_slot()
    }

    /// Starts the worker and queues the manifest-defined production model.
    pub fn start_model(&mut self) -> Result<(), String> {
        if !self.worker_started {
            self.controller
                .start_worker()
                .map_err(|error| error.to_string())?;
            self.worker_started = true;
        }
        if !self.model_requested {
            let descriptor = load_production_descriptor(&self.project_root)
                .map_err(|error| error.to_string())?;
            self.controller
                .load_model(descriptor, RuntimeSettings::default())
                .map_err(|error| error.to_string())?;
            self.model_requested = true;
        }
        Ok(())
    }

    /// Resets inference to idle for the next capture session.
    pub fn stop_model(&mut self) {
        if self.worker_started {
            let replacement =
                InferenceController::new(Arc::clone(&self.frame_slot), Arc::new(LatestSlot::new()));
            let controller = std::mem::replace(&mut self.controller, replacement);
            let _ = controller.shutdown_preserving_input();
            self.worker_started = false;
        }
        self.model_requested = false;
        self.last_generation = 0;
        self.latest_observation = None;
    }

    /// Reads one latest-only inference result, suppressing duplicate output.
    pub fn read_latest(&mut self) -> Option<RawFaceObservation> {
        let slot = self.controller.output_slot();
        match slot.try_read_after(self.last_generation) {
            Some(vtuber_core::ReadResult::New(observation)) => {
                self.last_generation = slot.generation();
                self.latest_observation = Some(observation.clone());
                Some(observation)
            }
            Some(vtuber_core::ReadResult::Closed) | None => None,
        }
    }
}

/// Starts/stops inference in response to the capture-backed application state.
pub fn inference_bridge_system(
    capture: Res<CaptureRuntime>,
    mut inference: ResMut<InferenceRuntime>,
    mut orchestrator: ResMut<Orchestrator>,
) {
    if orchestrator.take_inference_retry_request() {
        // Retain the capture session and replace only the inference worker.
        // `stop_model` joins the old worker before the new one is started.
        inference.stop_model();
        if orchestrator.capture_desired() {
            orchestrator.set_pipeline_state(PipelineState::Starting);
            orchestrator.set_last_error(None);
        }
    }

    if orchestrator.capture_desired()
        && matches!(
            capture.state(),
            vtuber_camera::CaptureServiceState::Starting
                | vtuber_camera::CaptureServiceState::Running
        )
        && matches!(
            orchestrator.pipeline_state(),
            PipelineState::Starting | PipelineState::Running
        )
        && !matches!(inference.status().state, InferenceWorkerState::Running)
        && let Err(error) = inference.start_model()
    {
        orchestrator.set_pipeline_state(PipelineState::Failed);
        orchestrator.set_last_error(Some(OrchestratorError::InferenceFailed(error)));
    }

    if !orchestrator.capture_desired()
        && matches!(
            orchestrator.pipeline_state(),
            PipelineState::Stopping | PipelineState::Failed
        )
    {
        inference.stop_model();
    }

    if orchestrator.capture_desired() {
        let status = inference.status();
        match status.state {
            InferenceWorkerState::Failed => {
                let message = status
                    .last_failure
                    .map(|failure| failure.error.to_string())
                    .unwrap_or_else(|| "inference worker failed".into());
                orchestrator.set_pipeline_state(PipelineState::Failed);
                orchestrator.set_last_error(Some(OrchestratorError::InferenceFailed(message)));
            }
            InferenceWorkerState::Running
                if capture.state() == vtuber_camera::CaptureServiceState::Running
                    && orchestrator.pipeline_state() == PipelineState::Starting =>
            {
                orchestrator.set_pipeline_state(PipelineState::Running);
            }
            _ => {}
        }
    }
}

/// Reads the latest observation and updates worker diagnostics.
pub fn read_inference_output_system(
    mut inference: ResMut<InferenceRuntime>,
    mut diagnostics: ResMut<DiagnosticsSnapshot>,
    mut last_seq: Local<Option<FrameSeq>>,
    mut rate: Local<Option<RateCounter>>,
) {
    let _ = inference.read_latest();
    let status = inference.status();
    let rate_counter = rate.get_or_insert_with(|| RateCounter::new(1_000_000_000));
    if let (Some(seq), Some(finished_at)) = (status.last_source_seq, status.last_finished_at)
        && *last_seq != Some(seq)
    {
        rate_counter.record(finished_at.0);
        *last_seq = Some(seq);
    }
    diagnostics.inference_rate = rate_counter.rate_hz(monotonic_now().0) as f32;
    diagnostics.inference_state = format!("{:?}", status.state);
    diagnostics.inference_last_source_seq = status.last_source_seq.map(|seq| seq.0);
    diagnostics.inference_frames_processed = status.frames_processed;
    diagnostics.inference_duplicates_suppressed = status.duplicate_frames_suppressed;
    diagnostics.inference_input_overwrites = status.frames_overwritten;
    diagnostics.last_inference_ms = status
        .last_inference_duration
        .map(|duration| duration.as_secs_f32() * 1_000.0);

    let metrics = status.metrics();
    diagnostics.stage_timings = InferenceStage::ALL
        .into_iter()
        .filter_map(|stage| {
            let timing = metrics.stage(stage);
            (timing.count > 0).then(|| {
                (
                    format!("inference_{stage:?}_mean"),
                    timing.mean_ns as f32 / 1_000_000.0,
                )
            })
        })
        .collect();
    if let Some(failure) = status.last_failure {
        diagnostics.last_error = Some(failure.error.to_string());
    }
}
