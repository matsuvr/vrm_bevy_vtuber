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

fn load_runtime(descriptor: &ModelDescriptor) -> Result<Box<dyn FaceInference>, InferenceError> {
    crate::backend::tract::load_model_runtime(descriptor)
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use vtuber_core::types::{
        FrameSeq, LandmarkSchemaId, MonoTimeNs, PixelFormat, RawFaceObservation, VideoFrame,
    };
    use vtuber_core::{LatestSlot, ReadResult};

    use crate::controller::InferenceController;
    use crate::descriptor::{
        ChannelOrder, ModelDescriptor, ModelFormat, Normalization, RuntimeSettings,
    };
    use crate::error::InferenceError;
    use crate::state::{FailureStage, InferenceWorkerState};

    fn sha256_hex(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    fn dummy_video_frame(seq: u64) -> VideoFrame {
        VideoFrame {
            seq: FrameSeq(seq),
            captured_at: MonoTimeNs(seq * 1_000_000),
            width: 64,
            height: 64,
            stride_bytes: 64 * 3,
            format: PixelFormat::Rgb8,
            data: vec![0u8; 64 * 64 * 3].into(),
        }
    }

    fn test_descriptor(
        path: PathBuf,
        sha256: &str,
        format: ModelFormat,
        normalization: Normalization,
    ) -> ModelDescriptor {
        ModelDescriptor {
            id: "test-model".into(),
            format,
            path,
            sha256: sha256.into(),
            input_name: "input".into(),
            input_shape: vec![1, 256, 256, 3],
            input_dtype: "f32".into(),
            channel_order: ChannelOrder::Rgb,
            normalization,
            schema: LandmarkSchemaId("test-schema"),
        }
    }

    fn new_controller() -> (
        InferenceController,
        Arc<LatestSlot<VideoFrame>>,
        Arc<LatestSlot<RawFaceObservation>>,
    ) {
        let frame_slot: Arc<LatestSlot<VideoFrame>> = Arc::new(LatestSlot::new());
        let output_slot: Arc<LatestSlot<RawFaceObservation>> = Arc::new(LatestSlot::new());
        let controller = InferenceController::new(
            Arc::clone(&frame_slot),
            Arc::clone(&output_slot),
        );
        (controller, frame_slot, output_slot)
    }

    #[test]
    fn worker_model_startup_hash_mismatch() {
        let path = std::env::temp_dir().join("vtuber_inference_hash_mismatch_test.bin");
        let contents = b"not a real model";
        std::fs::write(&path, contents).expect("write temp model file");
        let actual_sha256 = sha256_hex(contents);

        let descriptor = test_descriptor(
            path.clone(),
            "0000000000000000000000000000000000000000000000000000000000000000",
            ModelFormat::Tflite,
            Normalization::ZeroToOne,
        );

        let (mut controller, frame_slot, _output_slot) = new_controller();
        controller.start_worker().expect("start worker");

        // Publish a frame before asking the worker to load the model. A startup
        // failure must not consume it.
        frame_slot.publish(dummy_video_frame(1));

        controller
            .load_model(descriptor, RuntimeSettings::default())
            .expect("send load model");

        std::thread::sleep(Duration::from_millis(200));
        let status = controller.status();
        assert_eq!(status.state, InferenceWorkerState::Failed);
        assert_eq!(
            status.last_failure.as_ref().map(|f| f.stage),
            Some(FailureStage::ModelLoad)
        );

        let failure = status.last_failure.expect("failure recorded");
        match failure.error {
            InferenceError::HashMismatch { expected, actual } => {
                assert_eq!(expected, "0000000000000000000000000000000000000000000000000000000000000000");
                assert_eq!(actual, actual_sha256);
            }
            other => panic!("expected HashMismatch, got {other:?}"),
        }

        // The frame must still be unread because the worker failed before
        // entering the inference loop.
        assert_eq!(status.frames_processed, 0);
        assert_eq!(frame_slot.generation(), 1);
        assert!(
            matches!(frame_slot.try_read_after(0), Some(ReadResult::New(_))),
            "frame slot should still contain the published frame"
        );

        controller.shutdown();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn worker_model_startup_load_error() {
        let path = std::env::temp_dir().join("vtuber_inference_load_error_test.bin");
        std::fs::write(&path, b"").expect("write empty temp model file");
        let actual_sha256 = sha256_hex(b"");

        let descriptor = test_descriptor(
            path.clone(),
            &actual_sha256,
            ModelFormat::Tflite,
            Normalization::ZeroToOne,
        );

        let (mut controller, frame_slot, _output_slot) = new_controller();
        controller.start_worker().expect("start worker");

        frame_slot.publish(dummy_video_frame(1));

        controller
            .load_model(descriptor, RuntimeSettings::default())
            .expect("send load model");

        std::thread::sleep(Duration::from_millis(200));
        let status = controller.status();
        assert_eq!(status.state, InferenceWorkerState::Failed);
        assert_eq!(
            status.last_failure.as_ref().map(|f| f.stage),
            Some(FailureStage::ModelLoad)
        );

        let failure = status.last_failure.expect("failure recorded");
        assert!(
            matches!(
                failure.error,
                InferenceError::LoadFailed(_) | InferenceError::OptimizationFailed(_)
            ),
            "expected load/optimization error, got {:?}",
            failure.error
        );

        assert_eq!(status.frames_processed, 0);
        assert_eq!(frame_slot.generation(), 1);
        assert!(
            matches!(frame_slot.try_read_after(0), Some(ReadResult::New(_))),
            "frame slot should still contain the published frame"
        );

        controller.shutdown();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    #[cfg(feature = "onnx")]
    fn worker_model_startup_success_onnx() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.pop();
        path.push("assets");
        path.push("models");
        path.push("peppapig_student_1x3x256x256.onnx");

        let descriptor = ModelDescriptor {
            id: "peppapig-onnx".into(),
            format: ModelFormat::Onnx,
            path,
            sha256: "73EDF90954F05EBEF4639E7FA8620C5F83CCA09D2476DE66AB100F26C2B25E7A".into(),
            input_name: "input".into(),
            input_shape: vec![1, 3, 256, 256],
            input_dtype: "f32".into(),
            channel_order: ChannelOrder::Rgb,
            normalization: Normalization::MeanStd {
                mean: [0.485, 0.456, 0.406],
                std: [0.229, 0.224, 0.225],
            },
            schema: LandmarkSchemaId("peppapig-98"),
        };

        let (mut controller, _frame_slot, _output_slot) = new_controller();
        controller.start_worker().expect("start worker");
        controller
            .load_model(descriptor, RuntimeSettings::default())
            .expect("send load model");

        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        let mut last_state = InferenceWorkerState::LoadingModel;
        while std::time::Instant::now() < deadline {
            let status = controller.status();
            last_state = status.state;
            if last_state == InferenceWorkerState::Running
                || last_state == InferenceWorkerState::Failed
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        assert_eq!(
            last_state,
            InferenceWorkerState::Running,
            "worker should reach Running after loading a valid ONNX model; last_failure={:?}",
            controller.status().last_failure
        );

        controller.shutdown();
    }
}
