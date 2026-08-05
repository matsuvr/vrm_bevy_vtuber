//! Model provenance probing for G0-05.

use std::path::Path;

use tract_tflite::prelude::*;

/// Result of loading a TFLite model and inspecting its inputs/outputs.
#[derive(Debug, Clone)]
pub struct ModelProbe {
    /// Model file path.
    pub path: String,
    /// SHA-256 hex of the model file.
    pub sha256: String,
    /// Input fact descriptors.
    pub inputs: Vec<IOFact>,
    /// Output fact descriptors.
    pub outputs: Vec<IOFact>,
}

/// Input or output tensor descriptor.
#[derive(Debug, Clone)]
pub struct IOFact {
    /// Tensor name from the model.
    pub name: String,
    /// Datum type string.
    pub dtype: String,
    /// Shape dimensions.
    pub shape: Vec<i64>,
}

/// Probe a TFLite model at `path` and return its input/output facts.
pub fn probe_tflite_model(path: impl AsRef<Path>) -> anyhow::Result<ModelProbe> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    let sha256 = sha256_hex(&bytes);

    let model = tract_tflite::tflite()
        .model_for_read(&mut &bytes[..])
        .map_err(|e| anyhow::anyhow!("failed to load model: {e:?}"))?;

    let inputs = model
        .input_outlets()
        .map_err(|e| anyhow::anyhow!("failed to get inputs: {e}"))?
        .iter()
        .map(|outlet| {
            let fact = model
                .outlet_fact(*outlet)
                .map_err(|e| anyhow::anyhow!("failed to get input fact: {e}"))?;
            Ok(IOFact {
                name: model
                    .node_names()
                    .nth(outlet.node)
                    .unwrap_or("")
                    .to_string(),
                dtype: format!("{:?}", fact.datum_type),
                shape: fact
                    .shape
                    .as_concrete()
                    .map(|dims| dims.iter().map(|&d| d as i64).collect())
                    .unwrap_or_default(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let outputs = model
        .output_outlets()
        .map_err(|e| anyhow::anyhow!("failed to get outputs: {e}"))?
        .iter()
        .map(|outlet| {
            let fact = model
                .outlet_fact(*outlet)
                .map_err(|e| anyhow::anyhow!("failed to get output fact: {e}"))?;
            Ok(IOFact {
                name: model
                    .node_names()
                    .nth(outlet.node)
                    .unwrap_or("")
                    .to_string(),
                dtype: format!("{:?}", fact.datum_type),
                shape: fact
                    .shape
                    .as_concrete()
                    .map(|dims| dims.iter().map(|&d| d as i64).collect())
                    .unwrap_or_default(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(ModelProbe {
        path: path.to_string_lossy().to_string(),
        sha256,
        inputs,
        outputs,
    })
}

/// Probe an ONNX model at `path` and return its input/output facts.
#[cfg(feature = "onnx")]
pub fn probe_onnx_model(path: impl AsRef<Path>) -> anyhow::Result<ModelProbe> {
    use tract_onnx::prelude::*;

    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    let sha256 = sha256_hex(&bytes);

    let model = tract_onnx::onnx()
        .model_for_read(&mut &bytes[..])
        .map_err(|e| anyhow::anyhow!("failed to load onnx model: {e:?}"))?
        .into_optimized()
        .map_err(|e| anyhow::anyhow!("failed to optimize onnx model: {e:?}"))?;

    let inputs = model
        .input_outlets()
        .map_err(|e| anyhow::anyhow!("failed to get inputs: {e}"))?
        .iter()
        .map(|outlet| {
            let fact = model
                .outlet_fact(*outlet)
                .map_err(|e| anyhow::anyhow!("failed to get input fact: {e}"))?;
            Ok(IOFact {
                name: model
                    .node_names()
                    .nth(outlet.node)
                    .unwrap_or("")
                    .to_string(),
                dtype: format!("{:?}", fact.datum_type),
                shape: fact
                    .shape
                    .as_concrete()
                    .map(|dims| dims.iter().map(|&d| d as i64).collect())
                    .unwrap_or_default(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let outputs = model
        .output_outlets()
        .map_err(|e| anyhow::anyhow!("failed to get outputs: {e}"))?
        .iter()
        .map(|outlet| {
            let fact = model
                .outlet_fact(*outlet)
                .map_err(|e| anyhow::anyhow!("failed to get output fact: {e}"))?;
            Ok(IOFact {
                name: model
                    .node_names()
                    .nth(outlet.node)
                    .unwrap_or("")
                    .to_string(),
                dtype: format!("{:?}", fact.datum_type),
                shape: fact
                    .shape
                    .as_concrete()
                    .map(|dims| dims.iter().map(|&d| d as i64).collect())
                    .unwrap_or_default(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(ModelProbe {
        path: path.to_string_lossy().to_string(),
        sha256,
        inputs,
        outputs,
    })
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode_upper(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_dir() -> std::path::PathBuf {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.pop();
        path.join("assets").join("models")
    }

    #[test]
    #[ignore = "requires downloaded model artifacts"]
    fn probe_face_detector() {
        let probe = probe_tflite_model(model_dir().join("face_detector.tflite")).unwrap();
        assert!(!probe.inputs.is_empty());
        assert!(!probe.outputs.is_empty());
    }

    #[test]
    #[ignore = "requires downloaded model artifacts"]
    fn probe_face_landmarks_detector() {
        let probe = probe_tflite_model(model_dir().join("face_landmarks_detector.tflite")).unwrap();
        assert!(!probe.inputs.is_empty());
        assert!(!probe.outputs.is_empty());
    }

    #[test]
    #[ignore = "requires downloaded model artifacts"]
    fn probe_face_blendshapes() {
        let probe = probe_tflite_model(model_dir().join("face_blendshapes.tflite")).unwrap();
        assert!(!probe.inputs.is_empty());
        assert!(!probe.outputs.is_empty());
    }

    #[test]
    #[ignore = "requires downloaded model artifacts"]
    fn probe_legacy_face_landmark() {
        let probe = probe_tflite_model(model_dir().join("face_landmark.tflite")).unwrap();
        assert!(!probe.inputs.is_empty());
        assert!(!probe.outputs.is_empty());
        println!("{probe:#?}");
    }

    #[test]
    #[cfg(feature = "onnx")]
    #[ignore = "requires downloaded model artifacts"]
    fn probe_peppapig_student_256() {
        let probe =
            probe_onnx_model(model_dir().join("peppapig_student_1x3x256x256.onnx")).unwrap();
        assert!(!probe.inputs.is_empty());
        assert!(!probe.outputs.is_empty());
        println!("{probe:#?}");
    }
}
