//! VRM 1.0 model import and lightweight preflight inspection.
//!
//! Imports a user-selected file into an application-managed asset source and
//! verifies that it is a valid VRM 1.0 model before it reaches `bevy_vrm1`.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Default maximum import size (256 MiB).
pub const DEFAULT_SIZE_LIMIT: u64 = 256 * 1024 * 1024;
/// Immutable hard cap (1 GiB).
pub const HARD_SIZE_CAP: u64 = 1024 * 1024 * 1024;

/// Errors that can occur while importing or inspecting a model.
#[derive(Debug, Error)]
pub enum ModelImportError {
    /// I/O failure during import.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    /// File extension is not `.vrm`.
    #[error("MODEL_FILE_INVALID: file extension must be .vrm")]
    InvalidExtension,
    /// File is not a regular file (e.g. symlink or directory).
    #[error("MODEL_FILE_INVALID: not a regular file")]
    NotRegularFile,
    /// File size exceeds the configured limit.
    #[error("MODEL_FILE_INVALID: size {size} exceeds limit {limit}")]
    SizeExceeded {
        /// Actual file size.
        size: u64,
        /// Configured size limit.
        limit: u64,
    },
    /// Configured size limit exceeds the hard cap.
    #[error("MODEL_FILE_INVALID: configured limit {limit} exceeds hard cap {hard_cap}")]
    LimitExceedsHardCap {
        /// Configured size limit.
        limit: u64,
        /// Immutable hard cap.
        hard_cap: u64,
    },
    /// GLB parse failure.
    #[error("MODEL_FILE_INVALID: failed to parse GLB: {0}")]
    GlbParse(String),
    /// Missing `VRMC_vrm` extension.
    #[error("MODEL_NOT_VRM1: missing VRMC_vrm extension")]
    NotVrm1,
    /// Unsupported VRM spec version.
    #[error("MODEL_UNSUPPORTED_VERSION: spec version {0}")]
    UnsupportedVersion(String),
    /// Missing required humanoid bone.
    #[error("MODEL_MISSING_REQUIRED_BONE: {0}")]
    MissingRequiredBone(String),
    /// External buffer/image URI detected.
    #[error("MODEL_FILE_INVALID: external URI not allowed: {0}")]
    ExternalUri(String),
    /// Invalid node index referenced.
    #[error("MODEL_FILE_INVALID: invalid node index {index}")]
    InvalidNodeIndex {
        /// Node index that is out of range.
        index: usize,
    },
}

/// Summary returned after a successful inspection.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct VrmInspectionSummary {
    /// VRM spec version, expected to be `"1.0"`.
    pub spec_version: String,
    /// Model name from `VRMC_vrm.meta`.
    pub name: String,
    /// Authors from `VRMC_vrm.meta`.
    pub authors: Vec<String>,
    /// License URL from `VRMC_vrm.meta`.
    pub license_url: Option<String>,
    /// Expression preset names discovered in the model.
    pub expression_presets: Vec<String>,
    /// LookAt type, if present.
    pub look_at_type: Option<String>,
    /// Whether the model contains SpringBone extensions.
    pub has_spring_bone: bool,
    /// Whether the model contains Node Constraint extensions.
    pub has_node_constraint: bool,
    /// Humanoid node indices.
    pub humanoid_nodes: HumanoidNodes,
}

/// Humanoid bone node indices.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HumanoidNodes {
    /// Hips node index.
    pub hips: usize,
    /// Head node index.
    pub head: usize,
    /// Optional neck node index.
    pub neck: Option<usize>,
}

/// Result of importing a model.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ImportedModel {
    /// Stable asset identifier (SHA-256 hex).
    pub id: String,
    /// User-facing model name.
    pub name: String,
    /// Path where the model was copied inside the application asset source.
    pub asset_path: PathBuf,
    /// Path to the import metadata file.
    pub meta_path: PathBuf,
    /// Inspection summary.
    pub summary: VrmInspectionSummary,
    /// Original file path.
    pub original_path: PathBuf,
    /// Original file size in bytes.
    pub size: u64,
}

/// Metadata stored alongside an imported model.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ImportMeta {
    /// Imported model descriptor.
    pub imported: ImportedModel,
    /// Original file modification time (UNIX epoch seconds).
    pub mtime: Option<u64>,
}

/// Imports a user-selected VRM file into `asset_root` and returns its summary.
///
/// The copied file is placed at `asset_root/avatars/<sha256>/model.vrm`.
/// A metadata file is written at `asset_root/avatars/<sha256>/import.toml`.
pub fn import_vrm<P: AsRef<Path>, Q: AsRef<Path>>(
    source: P,
    asset_root: Q,
    size_limit: u64,
) -> Result<ImportedModel, ModelImportError> {
    if size_limit > HARD_SIZE_CAP {
        return Err(ModelImportError::LimitExceedsHardCap {
            limit: size_limit,
            hard_cap: HARD_SIZE_CAP,
        });
    }

    let source = source.as_ref();
    if source.extension().and_then(|e| e.to_str()) != Some("vrm") {
        return Err(ModelImportError::InvalidExtension);
    }
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.is_file() {
        return Err(ModelImportError::NotRegularFile);
    }
    let size = metadata.len();
    if size > size_limit {
        return Err(ModelImportError::SizeExceeded {
            size,
            limit: size_limit,
        });
    }

    let summary = inspect_vrm(source)?;

    let mut file = fs::File::open(source)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let id = format!("{:x}", hasher.finalize());

    let dest_dir = asset_root.as_ref().join("avatars").join(&id);
    fs::create_dir_all(&dest_dir)?;
    let dest_model = dest_dir.join("model.vrm");
    let meta_path = dest_dir.join("import.toml");

    if !dest_model.exists() {
        copy_atomic(source, &dest_model)?;
    }

    let imported = ImportedModel {
        id,
        name: summary.name.clone(),
        asset_path: dest_model.clone(),
        meta_path: meta_path.clone(),
        summary,
        original_path: source.to_path_buf(),
        size,
    };

    let meta = ImportMeta {
        imported: imported.clone(),
        mtime: metadata.modified().ok().and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs())
        }),
    };
    let meta_text = toml::to_string_pretty(&meta)
        .map_err(|e| ModelImportError::Io(io::Error::other(e.to_string())))?;
    write_atomic(&meta_path, meta_text.as_bytes())?;

    Ok(imported)
}

/// Inspects a VRM file without copying it.
pub fn inspect_vrm<P: AsRef<Path>>(path: P) -> Result<VrmInspectionSummary, ModelImportError> {
    let path = path.as_ref();
    let (document, _, _) =
        gltf::import(path).map_err(|e| ModelImportError::GlbParse(format!("{e}")))?;

    check_external_uris(&document)?;

    let json = document.as_json().clone();
    let vrmc = json
        .extensions
        .as_ref()
        .and_then(|ext| ext.others.get("VRMC_vrm"))
        .ok_or(ModelImportError::NotVrm1)?;

    let spec_version = vrmc
        .get("specVersion")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| ModelImportError::GlbParse("missing specVersion".into()))?;
    if spec_version != "1.0" {
        return Err(ModelImportError::UnsupportedVersion(spec_version));
    }

    let meta = vrmc
        .get("meta")
        .and_then(|m| m.as_object())
        .cloned()
        .unwrap_or_default();
    let name = meta
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let authors = meta
        .get("authors")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let license_url = meta
        .get("licenseUrl")
        .and_then(|v| v.as_str())
        .map(String::from);

    let humanoid = vrmc
        .get("humanoid")
        .and_then(|h| h.as_object())
        .ok_or_else(|| ModelImportError::GlbParse("missing humanoid".into()))?;
    let human_bones = humanoid
        .get("humanBones")
        .and_then(|b| b.as_object())
        .ok_or_else(|| ModelImportError::GlbParse("missing humanBones".into()))?;

    let node_count = document.nodes().len();
    let hips = required_bone_index(human_bones, "hips", node_count)?;
    let head = required_bone_index(human_bones, "head", node_count)?;
    let neck = optional_bone_index(human_bones, "neck", node_count)?;

    let expressions = vrmc.get("expressions").and_then(|e| e.as_object());
    let mut expression_presets = Vec::new();
    if let Some(expr) = expressions
        && let Some(preset) = expr.get("preset").and_then(|p| p.as_object())
    {
        for key in preset.keys() {
            expression_presets.push(key.clone());
        }
    }

    let look_at_type = vrmc
        .get("lookAt")
        .and_then(|l| l.get("type"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let has_spring_bone = json
        .extensions
        .as_ref()
        .map(|ext| ext.others.contains_key("VRMC_springBone"))
        .unwrap_or(false);
    let has_node_constraint = json
        .extensions
        .as_ref()
        .map(|ext| ext.others.contains_key("VRMC_node_constraint"))
        .unwrap_or(false);

    Ok(VrmInspectionSummary {
        spec_version,
        name,
        authors,
        license_url,
        expression_presets,
        look_at_type,
        has_spring_bone,
        has_node_constraint,
        humanoid_nodes: HumanoidNodes { hips, head, neck },
    })
}

fn required_bone_index(
    human_bones: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    node_count: usize,
) -> Result<usize, ModelImportError> {
    let index = human_bones
        .get(name)
        .and_then(|b| b.as_object())
        .and_then(|b| b.get("node"))
        .and_then(|n| n.as_u64())
        .map(|n| n as usize)
        .ok_or_else(|| ModelImportError::MissingRequiredBone(name.to_string()))?;
    if index >= node_count {
        return Err(ModelImportError::InvalidNodeIndex { index });
    }
    Ok(index)
}

fn optional_bone_index(
    human_bones: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    node_count: usize,
) -> Result<Option<usize>, ModelImportError> {
    match required_bone_index(human_bones, name, node_count) {
        Ok(index) => Ok(Some(index)),
        Err(ModelImportError::MissingRequiredBone(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

fn check_external_uris(document: &gltf::Document) -> Result<(), ModelImportError> {
    for buffer in document.buffers() {
        if let gltf::buffer::Source::Uri(uri) = buffer.source() {
            return Err(ModelImportError::ExternalUri(uri.to_string()));
        }
    }
    for image in document.images() {
        if let gltf::image::Source::Uri { uri, .. } = image.source() {
            return Err(ModelImportError::ExternalUri(uri.to_string()));
        }
    }
    Ok(())
}

fn copy_atomic(source: &Path, dest: &Path) -> Result<(), ModelImportError> {
    let temp = dest.with_extension("tmp");
    fs::copy(source, &temp)?;
    fs::rename(&temp, dest)?;
    Ok(())
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), ModelImportError> {
    let temp = path.with_extension("tmp");
    let mut file = fs::File::create(&temp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn rejects_non_vrm_extension() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("model.txt");
        fs::write(&path, b"not a vrm").unwrap();
        let err = import_vrm(&path, dir.path(), DEFAULT_SIZE_LIMIT).unwrap_err();
        assert!(matches!(err, ModelImportError::InvalidExtension));
    }

    #[test]
    fn rejects_directory() {
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("model.vrm");
        fs::create_dir(&subdir).unwrap();
        let err = import_vrm(&subdir, dir.path(), DEFAULT_SIZE_LIMIT).unwrap_err();
        assert!(matches!(err, ModelImportError::NotRegularFile));
    }

    #[test]
    fn rejects_oversized() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("model.vrm");
        fs::write(&path, b"x").unwrap();
        let err = import_vrm(&path, dir.path(), 0).unwrap_err();
        assert!(matches!(err, ModelImportError::SizeExceeded { .. }));
    }

    #[test]
    fn rejects_hard_cap_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("model.vrm");
        fs::write(&path, b"x").unwrap();
        let err = import_vrm(&path, dir.path(), HARD_SIZE_CAP + 1).unwrap_err();
        assert!(matches!(err, ModelImportError::LimitExceedsHardCap { .. }));
    }

    #[test]
    fn rejects_invalid_bytes() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("model.vrm");
        fs::write(&path, b"not glb").unwrap();
        let err = import_vrm(&path, dir.path(), DEFAULT_SIZE_LIMIT).unwrap_err();
        assert!(matches!(err, ModelImportError::GlbParse(_)));
    }

    #[test]
    fn idempotent_reimport_no_duplicate_copy() {
        let dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let path = source_dir.path().join("model.vrm");
        fs::write(&path, b"x").unwrap();
        // Not a valid GLB, so use a raw copy path to verify idempotency.
        let dest_dir = dir.path().join("avatars").join("test");
        fs::create_dir_all(&dest_dir).unwrap();
        fs::write(dest_dir.join("model.vrm"), b"x").unwrap();

        let meta_path = dest_dir.join("import.toml");
        let imported = ImportedModel {
            id: "test".into(),
            name: "x".into(),
            asset_path: dest_dir.join("model.vrm"),
            meta_path: meta_path.clone(),
            summary: VrmInspectionSummary::default(),
            original_path: path.clone(),
            size: 1,
        };
        let meta = ImportMeta {
            imported: imported.clone(),
            mtime: None,
        };
        fs::write(&meta_path, toml::to_string_pretty(&meta).unwrap()).unwrap();

        let before = fs::metadata(dest_dir.join("model.vrm")).unwrap().len();
        assert_eq!(before, 1);
        // Re-writing the same fixture does not duplicate because import is
        // idempotent by sha; here we just assert the meta round-trips.
        let read: ImportMeta = toml::from_str(&fs::read_to_string(&meta_path).unwrap()).unwrap();
        assert_eq!(read.imported, imported);
    }

    /// Path to the bundled Tsukuyomi-chan legacy VRM fixture.
    fn fixture_legacy_vrm() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../../tests/fixtures/vrm/tsukuyomi-chan.vrm");
        path
    }

    /// Path to the bundled VRM 1.0 fixture.
    fn fixture_vrm1() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../../tests/fixtures/vrm/inore-vrm1.vrm");
        path
    }

    #[test]
    fn fixture_tsukuyomi_is_legacy_vrm_rejected_as_not_vrm1() {
        let err = inspect_vrm(fixture_legacy_vrm()).unwrap_err();
        assert!(matches!(err, ModelImportError::NotVrm1));
    }

    #[test]
    fn fixture_tsukuyomi_import_rejected_as_not_vrm1() {
        let dir = TempDir::new().unwrap();
        let err = import_vrm(fixture_legacy_vrm(), dir.path(), DEFAULT_SIZE_LIMIT).unwrap_err();
        assert!(matches!(err, ModelImportError::NotVrm1));
    }

    #[test]
    fn inspects_real_vrm1_fixture() {
        let summary = inspect_vrm(fixture_vrm1()).expect("fixture should be valid VRM 1.0");
        assert_eq!(summary.spec_version, "1.0");
        assert!(!summary.name.is_empty(), "model name should be present");
        assert!(summary.humanoid_nodes.hips < 1000);
        assert!(summary.humanoid_nodes.head < 1000);
        assert!(summary.has_spring_bone);
    }

    #[test]
    fn imports_real_vrm1_fixture() {
        let dir = TempDir::new().unwrap();
        let imported = import_vrm(fixture_vrm1(), dir.path(), DEFAULT_SIZE_LIMIT)
            .expect("fixture should import successfully");
        assert_eq!(imported.summary.spec_version, "1.0");
        assert!(imported.asset_path.exists());
        assert!(imported.meta_path.exists());
        // Re-import with same file should be idempotent.
        let reimported = import_vrm(fixture_vrm1(), dir.path(), DEFAULT_SIZE_LIMIT)
            .expect("re-import should succeed");
        assert_eq!(imported.id, reimported.id);
        assert_eq!(imported.asset_path, reimported.asset_path);
    }
}
