//! Inference worker for the face tracking pipeline.

use std::sync::Arc;
use std::time::{Duration, Instant};

use vtuber_core::types::{MonoTimeNs, RawFaceObservation, VideoFrame};
use vtuber_core::{LatestSlot, ReadResult, StopToken};

use crate::controller::{ControlCommand, InferenceMetrics, InferenceWorkerResult};
use crate::descriptor::ModelDescriptor;
use crate::error::InferenceError;
use crate::runtime::FaceInference;
use crate::state::{FailureStage, InferenceWorkerState, SharedStatus};

/// Runs the inference worker loop.
///
/// The worker owns the model runtime and processes frames from the input slot.
/// It is spawned by [`crate::controller::InferenceController::start_worker`].
pub fn run_inference_worker(
    command_rx: std::sync::mpsc::Receiver<ControlCommand>,
    stop: StopToken,
    status: SharedStatus,
    frame_slot: Arc<LatestSlot<VideoFrame>>,
    output_slot: Arc<LatestSlot<RawFaceObservation>>,
) -> InferenceWorkerResult {
    let mut metrics = InferenceMetrics::default();
    let mut runtime: Option<Box<dyn FaceInference>> = None;
    let mut last_gen = 0u64;
    let mut last_overwritten = 0u64;
    let mut paused = false;

    update_status(&status, |s| {
        s.transition_to(InferenceWorkerState::Idle);
    });

    while !stop.is_stopped() {
        // Drain control commands first so state changes take effect immediately.
        loop {
            match command_rx.try_recv() {
                Ok(ControlCommand::LoadModel {
                    descriptor,
                    settings: _,
                }) => {
                    update_status(&status, |s| {
                        s.transition_to(InferenceWorkerState::LoadingModel);
                    });

                    match load_runtime(&descriptor) {
                        Ok(loaded) => {
                            runtime = Some(loaded);
                            update_status(&status, |s| {
                                s.transition_to(InferenceWorkerState::Running);
                            });
                        }
                        Err(err) => {
                            update_status(&status, |s| {
                                s.record_failure(FailureStage::ModelLoad, err);
                            });
                        }
                    }
                }
                Ok(ControlCommand::Pause) => {
                    paused = true;
                }
                Ok(ControlCommand::Resume) => {
                    paused = false;
                }
                Ok(ControlCommand::Reset) => {
                    runtime = None;
                    last_gen = 0;
                    update_status(&status, |s| {
                        s.transition_to(InferenceWorkerState::Idle);
                    });
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    stop.stop();
                    break;
                }
            }
        }

        if paused || runtime.is_none() {
            // Wait briefly before polling again so the loop remains responsive.
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }

        match frame_slot.wait_read_after(last_gen, Duration::from_millis(100)) {
            Some(ReadResult::New(frame)) => {
                last_gen = frame_slot.generation();
                let overwritten = frame_slot.overwritten_count();
                metrics.frames_overwritten += overwritten.saturating_sub(last_overwritten);
                last_overwritten = overwritten;

                let start = Instant::now();
                let started_at = MonoTimeNs(start.elapsed().as_nanos() as u64);

                match runtime
                    .as_ref()
                    .expect("runtime present when not paused")
                    .infer(&frame.data, frame.width, frame.height)
                {
                    Ok(observation) => {
                        let elapsed = start.elapsed();
                        let finished_at = MonoTimeNs(elapsed.as_nanos() as u64);
                        let observation = RawFaceObservation {
                            source_seq: frame.seq,
                            captured_at: frame.captured_at,
                            inference_started_at: started_at,
                            inference_finished_at: finished_at,
                            ..observation
                        };
                        if !output_slot.publish(observation) {
                            metrics.frames_dropped += 1;
                        }
                        metrics.frames_processed += 1;

                        update_status(&status, |s| {
                            s.record_processed(frame.seq, finished_at, elapsed);
                        });
                    }
                    Err(err) => {
                        update_status(&status, |s| {
                            s.record_failure(FailureStage::FrameInference, err);
                        });
                    }
                }
            }
            Some(ReadResult::Closed) => break,
            None => {}
        }
    }

    update_status(&status, |s| {
        s.transition_to(InferenceWorkerState::Stopping);
    });

    InferenceWorkerResult {
        final_metrics: metrics,
    }
}

fn load_runtime(_descriptor: &ModelDescriptor) -> Result<Box<dyn FaceInference>, InferenceError> {
    // M1-02-002 will construct the real runtime here from the descriptor.
    Err(InferenceError::LoadFailed(
        "runtime construction not yet implemented".into(),
    ))
}

fn update_status<F>(status: &SharedStatus, f: F)
where
    F: FnOnce(&mut crate::state::InferenceWorkerStatus),
{
    let mut s = status
        .lock()
        .expect("InferenceController status mutex poisoned");
    f(&mut s);
}
