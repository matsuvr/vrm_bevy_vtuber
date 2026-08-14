//! Generation-independent VRM core data used at the runtime boundary.
//!
//! This module intentionally contains no Bevy entities, assets, or systems.
//! It parses the core model contract from the glTF JSON and gives the VRM 0.x
//! and VRM 1.0 paths the same descriptor before ECS initialization.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// VRM generation represented by a normalized descriptor.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VrmGeneration {
    /// Legacy VRM 0.x using the root VRM extension.
    Vrm0,
    /// VRM 1.0 using the root `VRMC_vrm` extension.
    Vrm1,
}

/// Single coordinate correction applied to a legacy VRM scene.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CoordinateBasis {
    /// VRM 0.x faces -Z and is placed below a Y=pi basis entity.
    Vrm0Y180,
    /// VRM 1.0 uses the application's canonical glTF basis unchanged.
    Vrm1Identity,
}

/// Data-only descriptor shared by the VRM 0.x and VRM 1.0 runtime paths.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VrmRuntimeDescriptor {
    /// Detected VRM generation.
    pub generation: VrmGeneration,
    /// Stable source version marker (0.x for VRM 0.x).
    pub spec_version: String,
    /// Coordinate basis to apply at the scene boundary.
    pub coordinate_basis: CoordinateBasis,
    /// Model metadata.
    pub meta: VrmMeta,
    /// Humanoid node references by canonical bone name.
    pub humanoid: VrmHumanoid,
    /// Optional first-person metadata.
    pub first_person: Option<VrmFirstPerson>,
    /// Optional gaze metadata.
    pub look_at: Option<VrmLookAt>,
}

/// Common model metadata.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct VrmMeta {
    /// Display name, if supplied by the exporter.
    pub name: Option<String>,
    /// Author names.
    pub authors: Vec<String>,
    /// License URL, if supplied by the exporter.
    pub license_url: Option<String>,
}

/// Common Humanoid node references.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VrmHumanoid {
    /// Node index by canonical VRM Humanoid bone name.
    pub human_bones: BTreeMap<String, usize>,
}

/// First-person mesh annotation data.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct VrmFirstPerson {
    /// Optional first-person reference bone from VRM 0.x.
    pub first_person_bone: Option<usize>,
    /// Mesh/node visibility annotations.
    pub mesh_annotations: Vec<VrmMeshAnnotation>,
}

/// One first-person visibility annotation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VrmMeshAnnotation {
    /// glTF node index for the annotated mesh.
    pub node: usize,
    /// Canonical visibility flag.
    pub flag: VrmFirstPersonFlag,
}

/// Canonical first-person visibility flag.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum VrmFirstPersonFlag {
    /// Let the runtime decide from head weights.
    Auto,
    /// Visible in both views.
    Both,
    /// Visible only in third-person view.
    ThirdPersonOnly,
    /// Visible only in first-person view.
    FirstPersonOnly,
}

/// Common gaze configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VrmLookAt {
    /// Bone or expression gaze backend declared by the model.
    pub r#type: VrmLookAtType,
    /// Offset from the head bone.
    pub offset_from_head_bone: [f32; 3],
    /// Horizontal inner range map.
    pub range_map_horizontal_inner: VrmRangeMap,
    /// Horizontal outer range map.
    pub range_map_horizontal_outer: VrmRangeMap,
    /// Vertical down range map.
    pub range_map_vertical_down: VrmRangeMap,
    /// Vertical up range map.
    pub range_map_vertical_up: VrmRangeMap,
}

/// Canonical gaze backend.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum VrmLookAtType {
    /// Direct eye-bone gaze.
    Bone,
    /// Expression-based gaze.
    Expression,
}

/// Canonical input/output range map.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct VrmRangeMap {
    /// Maximum input magnitude.
    pub input_max_value: f32,
    /// Output magnitude in the runtime's canonical units.
    pub output_scale: f32,
}

/// Errors returned by the pure core descriptor parser.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VrmParseError {
    /// No supported root extension was found.
    MissingGeneration,
    /// Both generation roots were supplied.
    AmbiguousGeneration,
    /// VRM 1.0 declared an unsupported version.
    UnsupportedVersion(String),
    /// A required field was not present.
    MissingField(String),
    /// A field had an invalid JSON type or value.
    InvalidField { path: String, reason: String },
    /// A legacy human bone was declared more than once.
    DuplicateBone(String),
}

impl fmt::Display for VrmParseError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::MissingGeneration => write!(f, "missing VRM or VRMC_vrm extension"),
            Self::AmbiguousGeneration => {
                write!(f, "both VRM and VRMC_vrm extensions are present")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported VRMC_vrm specVersion {version}")
            }
            Self::MissingField(path) => write!(f, "missing required field {path}"),
            Self::InvalidField { path, reason } => write!(f, "invalid field {path}: {reason}"),
            Self::DuplicateBone(name) => write!(f, "duplicate Humanoid bone {name}"),
        }
    }
}

impl std::error::Error for VrmParseError {}

/// Parses either the glTF root JSON or a root JSON object containing the
/// extensions member into a common core descriptor.
pub fn parse_runtime_descriptor(root: &Value) -> Result<VrmRuntimeDescriptor, VrmParseError> {
    let extensions = root
        .get("extensions")
        .and_then(Value::as_object)
        .ok_or(VrmParseError::MissingGeneration)?;
    let legacy = extensions.get("VRM");
    let modern = extensions.get("VRMC_vrm");

    match (legacy, modern) {
        (Some(_), Some(_)) => Err(VrmParseError::AmbiguousGeneration),
        (Some(vrm), None) => parse_vrm0(vrm),
        (None, Some(vrmc)) => parse_vrm1(vrmc),
        (None, None) => Err(VrmParseError::MissingGeneration),
    }
}

fn parse_vrm0(vrm: &Value) -> Result<VrmRuntimeDescriptor, VrmParseError> {
    let meta = parse_vrm0_meta(vrm.get("meta"));
    let humanoid = parse_vrm0_humanoid(vrm.get("humanoid"))?;
    let first_person = vrm
        .get("firstPerson")
        .map(parse_vrm0_first_person)
        .transpose()?;
    let look_at = vrm
        .get("lookAtMaster")
        .map(parse_vrm0_look_at)
        .transpose()?;

    Ok(VrmRuntimeDescriptor {
        generation: VrmGeneration::Vrm0,
        spec_version: "0.x".into(),
        coordinate_basis: CoordinateBasis::Vrm0Y180,
        meta,
        humanoid,
        first_person,
        look_at,
    })
}

fn parse_vrm1(vrmc: &Value) -> Result<VrmRuntimeDescriptor, VrmParseError> {
    let spec_version = required_string(vrmc, "specVersion")?;
    if spec_version != "1.0" {
        return Err(VrmParseError::UnsupportedVersion(spec_version));
    }
    let meta = parse_vrm1_meta(vrmc.get("meta"));
    let humanoid = parse_vrm1_humanoid(vrmc.get("humanoid"))?;
    let first_person = vrmc
        .get("firstPerson")
        .map(parse_vrm1_first_person)
        .transpose()?;
    let look_at = vrmc.get("lookAt").map(parse_vrm1_look_at).transpose()?;

    Ok(VrmRuntimeDescriptor {
        generation: VrmGeneration::Vrm1,
        spec_version,
        coordinate_basis: CoordinateBasis::Vrm1Identity,
        meta,
        humanoid,
        first_person,
        look_at,
    })
}

fn parse_vrm0_meta(meta: Option<&Value>) -> VrmMeta {
    let Some(meta) = meta.and_then(Value::as_object) else {
        return VrmMeta::default();
    };
    VrmMeta {
        name: string_field(meta, "title").or_else(|| string_field(meta, "name")),
        authors: string_field(meta, "author").into_iter().collect(),
        license_url: string_field(meta, "otherLicenseUrl")
            .or_else(|| string_field(meta, "licenseUrl")),
    }
}

fn parse_vrm1_meta(meta: Option<&Value>) -> VrmMeta {
    let Some(meta) = meta.and_then(Value::as_object) else {
        return VrmMeta::default();
    };
    VrmMeta {
        name: string_field(meta, "name"),
        authors: meta
            .get("authors")
            .and_then(Value::as_array)
            .map(|authors| {
                authors
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default(),
        license_url: string_field(meta, "licenseUrl"),
    }
}

fn parse_vrm0_humanoid(humanoid: Option<&Value>) -> Result<VrmHumanoid, VrmParseError> {
    let bones = humanoid
        .and_then(|value| value.get("humanBones"))
        .and_then(Value::as_array)
        .ok_or_else(|| VrmParseError::MissingField("VRM.humanoid.humanBones".into()))?;
    let mut human_bones = BTreeMap::new();
    for (index, bone) in bones.iter().enumerate() {
        let path = format!("VRM.humanoid.humanBones[{index}]");
        let name = bone
            .get("bone")
            .and_then(Value::as_str)
            .ok_or_else(|| VrmParseError::MissingField(format!("{path}.bone")))?;
        let node = required_usize(bone, "node", &format!("{path}.node"))?;
        if human_bones.insert(name.to_string(), node).is_some() {
            return Err(VrmParseError::DuplicateBone(name.into()));
        }
    }
    require_humanoid_bones(&human_bones)?;
    Ok(VrmHumanoid { human_bones })
}

fn parse_vrm1_humanoid(humanoid: Option<&Value>) -> Result<VrmHumanoid, VrmParseError> {
    let bones = humanoid
        .and_then(|value| value.get("humanBones"))
        .and_then(Value::as_object)
        .ok_or_else(|| VrmParseError::MissingField("VRMC_vrm.humanoid.humanBones".into()))?;
    let mut human_bones = BTreeMap::new();
    for (name, bone) in bones {
        let node = required_usize(
            bone,
            "node",
            &format!("VRMC_vrm.humanoid.humanBones.{name}.node"),
        )?;
        human_bones.insert(name.clone(), node);
    }
    require_humanoid_bones(&human_bones)?;
    Ok(VrmHumanoid { human_bones })
}

fn require_humanoid_bones(bones: &BTreeMap<String, usize>) -> Result<(), VrmParseError> {
    for required in ["hips", "head"] {
        if !bones.contains_key(required) {
            return Err(VrmParseError::MissingField(format!(
                "humanoid.humanBones.{required}"
            )));
        }
    }
    Ok(())
}

fn parse_vrm0_first_person(value: &Value) -> Result<VrmFirstPerson, VrmParseError> {
    let first_person_bone = value
        .get("firstPersonBone")
        .map(|_| required_usize(value, "firstPersonBone", "VRM.firstPerson.firstPersonBone"))
        .transpose()?;
    let mut mesh_annotations = Vec::new();
    if let Some(annotations) = value.get("meshAnnotations").and_then(Value::as_array) {
        for (index, annotation) in annotations.iter().enumerate() {
            let path = format!("VRM.firstPerson.meshAnnotations[{index}]");
            let node = annotation
                .get("mesh")
                .or_else(|| annotation.get("node"))
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| VrmParseError::MissingField(format!("{path}.mesh")))?;
            let flag = parse_first_person_flag(
                annotation.get("firstPersonFlag"),
                &format!("{path}.firstPersonFlag"),
            )?;
            mesh_annotations.push(VrmMeshAnnotation { node, flag });
        }
    }
    Ok(VrmFirstPerson {
        first_person_bone,
        mesh_annotations,
    })
}

fn parse_vrm1_first_person(value: &Value) -> Result<VrmFirstPerson, VrmParseError> {
    let mut mesh_annotations = Vec::new();
    if let Some(annotations) = value.get("meshAnnotations").and_then(Value::as_array) {
        for (index, annotation) in annotations.iter().enumerate() {
            let path = format!("VRMC_vrm.firstPerson.meshAnnotations[{index}]");
            let node = required_usize(annotation, "node", &format!("{path}.node"))?;
            let flag = parse_first_person_flag(
                annotation.get("firstPersonFlag"),
                &format!("{path}.firstPersonFlag"),
            )?;
            mesh_annotations.push(VrmMeshAnnotation { node, flag });
        }
    }
    Ok(VrmFirstPerson {
        first_person_bone: None,
        mesh_annotations,
    })
}

fn parse_first_person_flag(
    value: Option<&Value>,
    path: &str,
) -> Result<VrmFirstPersonFlag, VrmParseError> {
    let value = value
        .and_then(Value::as_str)
        .ok_or_else(|| VrmParseError::MissingField(path.into()))?;
    match value {
        "Auto" | "auto" => Ok(VrmFirstPersonFlag::Auto),
        "Both" | "both" => Ok(VrmFirstPersonFlag::Both),
        "ThirdPersonOnly" | "thirdPersonOnly" | "third_person_only" => {
            Ok(VrmFirstPersonFlag::ThirdPersonOnly)
        }
        "FirstPersonOnly" | "firstPersonOnly" | "first_person_only" => {
            Ok(VrmFirstPersonFlag::FirstPersonOnly)
        }
        _ => Err(VrmParseError::InvalidField {
            path: path.into(),
            reason: format!("unknown first-person flag {value}"),
        }),
    }
}

fn parse_vrm0_look_at(value: &Value) -> Result<VrmLookAt, VrmParseError> {
    let r#type = match required_string(value, "type")?.as_str() {
        "Bone" | "bone" => VrmLookAtType::Bone,
        "BlendShape" | "blendShape" | "Expression" | "expression" => VrmLookAtType::Expression,
        other => {
            return Err(VrmParseError::InvalidField {
                path: "VRM.lookAtMaster.type".into(),
                reason: format!("unknown look-at type {other}"),
            });
        }
    };
    Ok(VrmLookAt {
        r#type,
        offset_from_head_bone: array3(value, "offsetFromHeadBone", "VRM.lookAtMaster")?,
        range_map_horizontal_inner: parse_vrm0_range_map(
            value,
            "lookAtHorizontalInner",
            "VRM.lookAtMaster.lookAtHorizontalInner",
        )?,
        range_map_horizontal_outer: parse_vrm0_range_map(
            value,
            "lookAtHorizontalOuter",
            "VRM.lookAtMaster.lookAtHorizontalOuter",
        )?,
        range_map_vertical_down: parse_vrm0_range_map(
            value,
            "lookAtVerticalDown",
            "VRM.lookAtMaster.lookAtVerticalDown",
        )?,
        range_map_vertical_up: parse_vrm0_range_map(
            value,
            "lookAtVerticalUp",
            "VRM.lookAtMaster.lookAtVerticalUp",
        )?,
    })
}

fn parse_vrm0_range_map(
    value: &Value,
    field: &str,
    path: &str,
) -> Result<VrmRangeMap, VrmParseError> {
    let curve = value
        .get(field)
        .and_then(|range| range.get("curve"))
        .ok_or_else(|| VrmParseError::MissingField(format!("{path}.curve")))?;
    Ok(VrmRangeMap {
        input_max_value: required_f32(curve, "xRange", &format!("{path}.curve.xRange"))?,
        output_scale: required_f32(curve, "yRange", &format!("{path}.curve.yRange"))?,
    })
}

fn parse_vrm1_look_at(value: &Value) -> Result<VrmLookAt, VrmParseError> {
    let r#type = match required_string(value, "type")?.as_str() {
        "bone" | "Bone" => VrmLookAtType::Bone,
        "expression" | "Expression" => VrmLookAtType::Expression,
        other => {
            return Err(VrmParseError::InvalidField {
                path: "VRMC_vrm.lookAt.type".into(),
                reason: format!("unknown look-at type {other}"),
            });
        }
    };
    Ok(VrmLookAt {
        r#type,
        offset_from_head_bone: array3(value, "offsetFromHeadBone", "VRMC_vrm.lookAt")?,
        range_map_horizontal_inner: parse_vrm1_range_map(
            value,
            "rangeMapHorizontalInner",
            "VRMC_vrm.lookAt.rangeMapHorizontalInner",
        )?,
        range_map_horizontal_outer: parse_vrm1_range_map(
            value,
            "rangeMapHorizontalOuter",
            "VRMC_vrm.lookAt.rangeMapHorizontalOuter",
        )?,
        range_map_vertical_down: parse_vrm1_range_map(
            value,
            "rangeMapVerticalDown",
            "VRMC_vrm.lookAt.rangeMapVerticalDown",
        )?,
        range_map_vertical_up: parse_vrm1_range_map(
            value,
            "rangeMapVerticalUp",
            "VRMC_vrm.lookAt.rangeMapVerticalUp",
        )?,
    })
}

fn parse_vrm1_range_map(
    value: &Value,
    field: &str,
    path: &str,
) -> Result<VrmRangeMap, VrmParseError> {
    let range = value
        .get(field)
        .ok_or_else(|| VrmParseError::MissingField(path.into()))?;
    Ok(VrmRangeMap {
        input_max_value: required_f32(range, "inputMaxValue", &format!("{path}.inputMaxValue"))?,
        output_scale: required_f32(range, "outputScale", &format!("{path}.outputScale"))?,
    })
}

fn string_field(
    values: &Map<String, Value>,
    field: &str,
) -> Option<String> {
    values.get(field).and_then(Value::as_str).map(String::from)
}

fn required_string(
    value: &Value,
    field: &str,
) -> Result<String, VrmParseError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| VrmParseError::MissingField(field.into()))
}

fn required_usize(
    value: &Value,
    field: &str,
    path: &str,
) -> Result<usize, VrmParseError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| VrmParseError::MissingField(path.into()))
}

fn required_f32(
    value: &Value,
    field: &str,
    path: &str,
) -> Result<f32, VrmParseError> {
    value
        .get(field)
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
        .ok_or_else(|| VrmParseError::InvalidField {
            path: path.into(),
            reason: "expected a finite number".into(),
        })
}

fn array3(
    value: &Value,
    field: &str,
    parent_path: &str,
) -> Result<[f32; 3], VrmParseError> {
    let values = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| VrmParseError::MissingField(format!("{parent_path}.{field}")))?;
    if values.len() != 3 {
        return Err(VrmParseError::InvalidField {
            path: format!("{parent_path}.{field}"),
            reason: "expected three numbers".into(),
        });
    }
    let mut result = [0.0; 3];
    for (index, value) in values.iter().enumerate() {
        result[index] = value
            .as_f64()
            .map(|value| value as f32)
            .filter(|value| value.is_finite())
            .ok_or_else(|| VrmParseError::InvalidField {
                path: format!("{parent_path}.{field}[{index}]"),
                reason: "expected a finite number".into(),
            })?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vrm0_root() -> Value {
        json!({
            "extensions": {"VRM": {
                "meta": {"title": "legacy", "author": "author", "otherLicenseUrl": "https://example.test/license"},
                "humanoid": {"humanBones": [
                    {"bone": "hips", "node": 0},
                    {"bone": "head", "node": 1}
                ]},
                "firstPerson": {"firstPersonBone": 1, "meshAnnotations": [
                    {"mesh": 2, "firstPersonFlag": "ThirdPersonOnly"}
                ]},
                "lookAtMaster": {
                    "type": "BlendShape",
                    "offsetFromHeadBone": [0.0, 0.1, 0.2],
                    "lookAtHorizontalInner": {"curve": {"xRange": 90.0, "yRange": 10.0}},
                    "lookAtHorizontalOuter": {"curve": {"xRange": 90.0, "yRange": 10.0}},
                    "lookAtVerticalDown": {"curve": {"xRange": 90.0, "yRange": 10.0}},
                    "lookAtVerticalUp": {"curve": {"xRange": 90.0, "yRange": 10.0}}
                }
            }}
        })
    }

    fn vrm1_root() -> Value {
        json!({
            "extensions": {"VRMC_vrm": {
                "specVersion": "1.0",
                "meta": {"name": "modern", "authors": ["author"]},
                "humanoid": {"humanBones": {
                    "hips": {"node": 0}, "head": {"node": 1}
                }},
                "firstPerson": {"meshAnnotations": [
                    {"node": 2, "firstPersonFlag": "thirdPersonOnly"}
                ]},
                "lookAt": {
                    "type": "bone",
                    "offsetFromHeadBone": [0.0, 0.1, 0.2],
                    "rangeMapHorizontalInner": {"inputMaxValue": 90.0, "outputScale": 10.0},
                    "rangeMapHorizontalOuter": {"inputMaxValue": 90.0, "outputScale": 10.0},
                    "rangeMapVerticalDown": {"inputMaxValue": 90.0, "outputScale": 10.0},
                    "rangeMapVerticalUp": {"inputMaxValue": 90.0, "outputScale": 10.0}
                }
            }}
        })
    }

    #[test]
    fn parses_legacy_core_into_common_descriptor() {
        let descriptor = parse_runtime_descriptor(&vrm0_root()).expect("VRM 0.x should parse");
        assert_eq!(descriptor.generation, VrmGeneration::Vrm0);
        assert_eq!(descriptor.coordinate_basis, CoordinateBasis::Vrm0Y180);
        assert_eq!(descriptor.meta.name.as_deref(), Some("legacy"));
        assert_eq!(descriptor.humanoid.human_bones["head"], 1);
        assert_eq!(
            descriptor.first_person.as_ref().unwrap().first_person_bone,
            Some(1)
        );
        assert_eq!(
            descriptor.first_person.as_ref().unwrap().mesh_annotations[0].flag,
            VrmFirstPersonFlag::ThirdPersonOnly
        );
        assert_eq!(
            descriptor.look_at.as_ref().unwrap().r#type,
            VrmLookAtType::Expression
        );
        assert_eq!(
            descriptor
                .look_at
                .as_ref()
                .unwrap()
                .range_map_horizontal_inner
                .output_scale,
            10.0
        );
    }

    #[test]
    fn parses_modern_core_into_the_same_descriptor_shape() {
        let descriptor = parse_runtime_descriptor(&vrm1_root()).expect("VRM 1.0 should parse");
        assert_eq!(descriptor.generation, VrmGeneration::Vrm1);
        assert_eq!(descriptor.coordinate_basis, CoordinateBasis::Vrm1Identity);
        assert_eq!(descriptor.spec_version, "1.0");
        assert_eq!(descriptor.meta.authors, vec!["author"]);
        assert_eq!(
            descriptor.first_person.as_ref().unwrap().mesh_annotations[0].node,
            2
        );
        assert_eq!(
            descriptor.look_at.as_ref().unwrap().r#type,
            VrmLookAtType::Bone
        );
    }

    #[test]
    fn rejects_ambiguous_and_unsupported_roots() {
        let mut ambiguous = vrm1_root();
        ambiguous["extensions"]["VRM"] = json!({});
        assert_eq!(
            parse_runtime_descriptor(&ambiguous),
            Err(VrmParseError::AmbiguousGeneration)
        );

        let mut unsupported = vrm1_root();
        unsupported["extensions"]["VRMC_vrm"]["specVersion"] = json!("1.1");
        assert_eq!(
            parse_runtime_descriptor(&unsupported),
            Err(VrmParseError::UnsupportedVersion("1.1".into()))
        );
    }

    #[test]
    fn rejects_legacy_duplicate_required_bones() {
        let mut root = vrm0_root();
        root["extensions"]["VRM"]["humanoid"]["humanBones"]
            .as_array_mut()
            .unwrap()
            .push(json!({"bone": "head", "node": 3}));
        assert_eq!(
            parse_runtime_descriptor(&root),
            Err(VrmParseError::DuplicateBone("head".into()))
        );
    }
}
