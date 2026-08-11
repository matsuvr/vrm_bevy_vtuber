#![cfg(feature = "onnx")]
#![doc = "Exact UltraFace RFB-320 tract-onnx probe acceptance test."]

use std::path::PathBuf;

use vtuber_inference::probe::{
    OnnxProbeStage, ULTRAFACE_RFB_320_MODEL_ID, ULTRAFACE_RFB_320_SHA256, probe_ultraface_model,
};

fn model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/version-RFB-320.onnx")
}

#[test]
fn ultraface_probe_exact_artifact_or_reproducible_blocker() {
    match probe_ultraface_model(model_path()) {
        Ok(report) => {
            assert_eq!(report.model_id, ULTRAFACE_RFB_320_MODEL_ID);
            assert_eq!(report.sha256, ULTRAFACE_RFB_320_SHA256);
            assert_eq!(report.inputs[0].dtype, "F32");
            assert_eq!(report.inputs[0].shape, [1, 3, 240, 320]);
            assert_eq!(report.outputs.len(), 2);
            assert_eq!(report.outputs[0].shape, [1, 4420, 2]);
            assert_eq!(report.outputs[1].shape, [1, 4420, 4]);
            assert_eq!(report.runs.len(), 2);
            assert!(
                report
                    .runs
                    .iter()
                    .flat_map(|run| &run.outputs)
                    .all(|output| output.all_finite)
            );
            println!("{report:#?}");
        }
        Err(error) => {
            println!("EXACT_ULTRAFACE_PROBE_BLOCKED: {error}");
            assert_eq!(error.model_id, ULTRAFACE_RFB_320_MODEL_ID);
            assert_eq!(error.stage, OnnxProbeStage::ArtifactRead);
            assert!(
                error.sha256.as_ref() == "unknown"
                    || error.sha256.as_ref() == ULTRAFACE_RFB_320_SHA256,
                "unexpected blocker SHA: {}",
                error.sha256
            );
        }
    }
}
