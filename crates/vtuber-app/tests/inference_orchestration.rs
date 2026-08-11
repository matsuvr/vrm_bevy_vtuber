//! Application-level checks for the production composite inference handoff.

use std::sync::Arc;
use std::time::Duration;

use vtuber_app::inference_runtime::InferenceRuntime;
use vtuber_core::{LatestSlot, VideoFrame};
use vtuber_inference::{FailureStage, InferenceWorkerState};

#[test]
fn production_start_queues_composite_pipeline_and_classifies_load_failure() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate is nested beneath workspace root");
    let project = tempfile::tempdir().expect("temporary project root");
    let model_root = project.path().join("assets").join("models");
    std::fs::create_dir_all(&model_root).expect("model directory");
    let manifest = std::fs::read(workspace_root.join("assets/models/manifest.toml"))
        .expect("repository manifest");
    std::fs::write(model_root.join("manifest.toml"), manifest).expect("temporary manifest");

    let frame_slot: Arc<LatestSlot<VideoFrame>> = Arc::new(LatestSlot::new());
    let mut runtime = InferenceRuntime::new(frame_slot, project.path().to_path_buf());
    runtime
        .start_model()
        .expect("pipeline load command should be queued");

    for _ in 0..100 {
        if runtime.status().state == InferenceWorkerState::Failed {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let status = runtime.status();
    assert_eq!(status.state, InferenceWorkerState::Failed);
    assert_eq!(
        status.last_failure.as_ref().map(|failure| failure.stage),
        Some(FailureStage::ModelLoad)
    );

    runtime.stop_model();
}
