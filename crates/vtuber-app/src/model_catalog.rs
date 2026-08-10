//! Manifest-driven production model descriptor construction.

use std::path::{Path, PathBuf};

use vtuber_core::types::LandmarkSchemaId;
use vtuber_inference::{ChannelOrder, ModelDescriptor, ModelFormat, Normalization};

/// Errors while reading the model manifest.
#[derive(Debug, thiserror::Error)]
pub enum ModelCatalogError {
    /// The manifest file could not be read.
    #[error("cannot read model manifest: {0}")]
    Io(#[from] std::io::Error),
    /// The manifest is syntactically invalid.
    #[error("cannot parse model manifest: {0}")]
    Parse(#[from] toml::de::Error),
    /// A required manifest field is absent or malformed.
    #[error("invalid model manifest: {0}")]
    Invalid(String),
}

/// Loads the approved production model descriptor from the repository
/// manifest without duplicating its path, hash, or tensor contract in code.
pub fn load_production_descriptor(
    project_root: &Path,
) -> Result<ModelDescriptor, ModelCatalogError> {
    let manifest_path = project_root
        .join("assets")
        .join("models")
        .join("manifest.toml");
    let text = std::fs::read_to_string(&manifest_path)?;
    let value: toml::Value = toml::from_str(&text)?;
    let models = value
        .get("models")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| ModelCatalogError::Invalid("models array is missing".into()))?;
    let model = models
        .first()
        .ok_or_else(|| ModelCatalogError::Invalid("production model list is empty".into()))?;

    let string = |name: &str| {
        model
            .get(name)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ModelCatalogError::Invalid(format!("models[0].{name} is missing")))
    };
    let file = string("file")?;
    let runtime = string("runtime")?;
    let sha256 = string("sha256")?;
    let input_name = string("input_name")?;
    let schema = string("schema")?;
    let input = model
        .get("input")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ModelCatalogError::Invalid("models[0].input is missing".into()))?;
    let shape = input
        .get("shape")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| ModelCatalogError::Invalid("input.shape is missing".into()))?
        .iter()
        .map(|value| {
            value
                .as_integer()
                .and_then(|v| usize::try_from(v).ok())
                .ok_or_else(|| {
                    ModelCatalogError::Invalid("input.shape must contain positive integers".into())
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if shape.len() != 4 || shape.contains(&0) {
        return Err(ModelCatalogError::Invalid(
            "input.shape must contain four non-zero dimensions".into(),
        ));
    }

    let pose = model
        .get("pose")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ModelCatalogError::Invalid("models[0].pose is missing".into()))?;
    let pose_method = pose
        .get("method")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| ModelCatalogError::Invalid("pose.method is missing".into()))?;
    if pose_method != "canonical_orthographic_2d" {
        return Err(ModelCatalogError::Invalid(format!(
            "unsupported pose method `{pose_method}`"
        )));
    }
    let representative_indices = pose
        .get("representative_indices")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            ModelCatalogError::Invalid("pose.representative_indices is missing".into())
        })?;
    if representative_indices.len() < 6
        || representative_indices
            .iter()
            .any(|value| !value.as_integer().is_some_and(|index| index >= 0))
    {
        return Err(ModelCatalogError::Invalid(
            "pose.representative_indices must contain non-negative integers".into(),
        ));
    }

    let normalization = if let (Some(mean), Some(std)) = (
        input.get("mean").and_then(toml::Value::as_array),
        input.get("std").and_then(toml::Value::as_array),
    ) {
        Normalization::MeanStd {
            mean: read_three_floats(mean, "input.mean")?,
            std: read_three_floats(std, "input.std")?,
        }
    } else {
        Normalization::ZeroToOne
    };

    let format = match runtime.as_str() {
        value if value.starts_with("tract-onnx") => ModelFormat::Onnx,
        other => {
            return Err(ModelCatalogError::Invalid(format!(
                "unsupported production runtime `{other}`"
            )));
        }
    };

    let schema = match schema.as_str() {
        "peppapig-98" => LandmarkSchemaId("peppapig-98"),
        other => {
            return Err(ModelCatalogError::Invalid(format!(
                "unsupported landmark schema `{other}`"
            )));
        }
    };

    Ok(ModelDescriptor {
        id: string("name")?,
        format,
        path: project_root
            .join("assets")
            .join("models")
            .join(PathBuf::from(file)),
        sha256,
        input_name,
        input_shape: shape,
        input_dtype: input
            .get("dtype")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| ModelCatalogError::Invalid("input.dtype is missing".into()))?
            .to_owned(),
        channel_order: ChannelOrder::Rgb,
        normalization,
        schema,
        expression_mapping: None,
    })
}

fn read_three_floats(values: &[toml::Value], field: &str) -> Result<[f32; 3], ModelCatalogError> {
    if values.len() != 3 {
        return Err(ModelCatalogError::Invalid(format!(
            "{field} must have 3 values"
        )));
    }
    let mut result = [0.0; 3];
    for (index, value) in values.iter().enumerate() {
        result[index] = value.as_float().ok_or_else(|| {
            ModelCatalogError::Invalid(format!("{field}[{index}] must be a float"))
        })? as f32;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_manifest_is_the_descriptor_source() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate is nested beneath workspace root");
        let descriptor = load_production_descriptor(root).expect("manifest should parse");
        assert_eq!(descriptor.format, ModelFormat::Onnx);
        assert_eq!(descriptor.input_shape, vec![1, 3, 256, 256]);
        assert_eq!(descriptor.schema, LandmarkSchemaId("peppapig-98"));
        assert!(
            descriptor
                .path
                .ends_with("peppapig_student_1x3x256x256.onnx")
        );
    }
}
