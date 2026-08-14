pub mod runtime_descriptor;
pub mod vrmc_node_constraint;
pub mod vrmc_spring_bone;
pub mod vrmc_vrm;

pub use runtime_descriptor::{
    CoordinateBasis, VrmFirstPerson, VrmFirstPersonFlag, VrmGeneration, VrmHumanoid, VrmLookAt,
    VrmLookAtType, VrmMeshAnnotation, VrmMeta, VrmParseError, VrmRangeMap, VrmRuntimeDescriptor,
    parse_runtime_descriptor,
};

use crate::error::AppResult;
use crate::vrm::gltf::extensions::vrmc_spring_bone::{
    Collider, ColliderGroup, Sphere, Spring, SpringJoint, VRMCSpringBone,
};
use crate::vrm::gltf::extensions::vrmc_vrm::{
    Expressions, FirstPerson, FirstPersonFlag, Humanoid, LookAtProperties, LookAtType, Meta,
    MorphTargetBind, RangeMap, VrmPreset, VrmcVrm,
};
use anyhow::Context;
use bevy::gltf::Gltf;
use bevy::platform::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize)]
pub struct VrmExtensions {
    /// Generation-independent core descriptor used by the adapter boundary.
    pub runtime_descriptor: VrmRuntimeDescriptor,
    #[serde(rename = "VRMC_vrm")]
    pub vrmc_vrm: VrmcVrm,

    #[serde(rename = "VRMC_springBone")]
    pub vrmc_spring_bone: Option<VRMCSpringBone>,
}

impl VrmExtensions {
    pub fn new(json: &serde_json::map::Map<String, serde_json::Value>) -> AppResult<Self> {
        let mut root = Value::Object(Map::new());
        if let Value::Object(root_object) = &mut root {
            root_object.insert("extensions".into(), Value::Object(json.clone()));
        }
        let runtime_descriptor = parse_runtime_descriptor(&root)?;
        let vrmc_vrm = match json.get("VRMC_vrm") {
            Some(vrmc) => serde_json::from_value(vrmc.clone())?,
            None => normalized_legacy_vrm(
                &runtime_descriptor,
                json.get("VRM").context("Not found VRM extension")?,
            ),
        };
        let vrmc_spring_bone = match json.get("VRMC_springBone") {
            Some(value) => Some(serde_json::from_value(value.clone())?),
            None => json.get("VRM").and_then(normalized_legacy_spring_bone),
        };
        Ok(Self {
            runtime_descriptor,
            vrmc_vrm,
            vrmc_spring_bone,
        })
    }

    /// Creates a new [`VrmExtensions`] from the glTF asset.
    pub fn from_gltf(gltf: &Gltf) -> AppResult<Self> {
        Self::new(obtain_extensions(gltf)?)
    }

    /// Gets the name of the VRM avatar.
    ///
    /// Returns `None` if the name does not exist in the meta information.
    pub fn name(&self) -> Option<String> {
        self.runtime_descriptor.meta.name.clone()
    }
}

fn normalized_legacy_vrm(
    descriptor: &VrmRuntimeDescriptor,
    legacy: &Value,
) -> VrmcVrm {
    let human_bones = descriptor
        .humanoid
        .human_bones
        .iter()
        .map(|(name, node)| (name.clone(), VrmNode { node: *node }))
        .collect::<HashMap<_, _>>();
    let first_person = descriptor
        .first_person
        .as_ref()
        .map(|first_person| FirstPerson {
            mesh_annotations: first_person
                .mesh_annotations
                .iter()
                .map(
                    |annotation| crate::vrm::gltf::extensions::vrmc_vrm::MeshAnnotation {
                        node: annotation.node,
                        first_person_flag: match annotation.flag {
                            VrmFirstPersonFlag::Auto => FirstPersonFlag::Auto,
                            VrmFirstPersonFlag::Both => FirstPersonFlag::Both,
                            VrmFirstPersonFlag::ThirdPersonOnly => FirstPersonFlag::ThirdPersonOnly,
                            VrmFirstPersonFlag::FirstPersonOnly => FirstPersonFlag::FirstPersonOnly,
                        },
                    },
                )
                .collect(),
        });
    let look_at = descriptor.look_at.as_ref().map(|look_at| LookAtProperties {
        offset_from_head_bone: look_at.offset_from_head_bone,
        range_map_horizontal_inner: RangeMap {
            input_max_value: look_at.range_map_horizontal_inner.input_max_value,
            output_scale: look_at.range_map_horizontal_inner.output_scale,
        },
        range_map_horizontal_outer: RangeMap {
            input_max_value: look_at.range_map_horizontal_outer.input_max_value,
            output_scale: look_at.range_map_horizontal_outer.output_scale,
        },
        range_map_vertical_down: RangeMap {
            input_max_value: look_at.range_map_vertical_down.input_max_value,
            output_scale: look_at.range_map_vertical_down.output_scale,
        },
        range_map_vertical_up: RangeMap {
            input_max_value: look_at.range_map_vertical_up.input_max_value,
            output_scale: look_at.range_map_vertical_up.output_scale,
        },
        r#type: match look_at.r#type {
            VrmLookAtType::Bone => LookAtType::Bone,
            VrmLookAtType::Expression => LookAtType::Expression,
        },
    });

    VrmcVrm {
        expressions: normalized_legacy_expressions(legacy),
        first_person,
        humanoid: Humanoid { human_bones },
        look_at,
        meta: Some(Meta {
            allow_antisocial_or_hate_usage: true,
            allow_excessively_sexual_usage: true,
            allow_excessively_violent_usage: true,
            allow_political_or_religious_usage: true,
            allow_redistribution: true,
            authors: descriptor.meta.authors.clone(),
            avatar_permission: None,
            commercial_usage: None,
            credit_notation: None,
            license_url: descriptor.meta.license_url.clone(),
            modification: None,
            name: descriptor.meta.name.clone(),
            other_license_url: None,
            thumbnail_image: None,
            version: None,
        }),
        spec_version: "1.0".into(),
    }
}

fn normalized_legacy_expressions(legacy: &Value) -> Option<Expressions> {
    let groups = legacy
        .get("blendShapeMaster")
        .and_then(|master| master.get("blendShapeGroups"))
        .and_then(Value::as_array)?;
    let mut preset = HashMap::default();
    for group in groups {
        let Some(name) = normalized_legacy_expression_name(group) else {
            continue;
        };
        if preset.contains_key(&name) {
            continue;
        }
        let morph_target_binds = group.get("binds").and_then(Value::as_array).map(|binds| {
            binds
                .iter()
                .filter_map(|bind| {
                    let node = bind.get("mesh")?.as_u64()?.try_into().ok()?;
                    let index = bind.get("index")?.as_u64()?.try_into().ok()?;
                    let weight = bind.get("weight")?.as_f64()? as f32 / 100.0;
                    Some(MorphTargetBind {
                        index,
                        node,
                        weight: if weight.is_finite() {
                            weight.clamp(0.0, 1.0)
                        } else {
                            0.0
                        },
                    })
                })
                .collect()
        });
        preset.insert(
            name,
            VrmPreset {
                is_binary: group
                    .get("isBinary")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                morph_target_binds,
                override_blink: "none".into(),
                override_look_at: "none".into(),
                override_mouth: "none".into(),
            },
        );
    }
    Some(Expressions { preset })
}

fn normalized_legacy_expression_name(group: &Value) -> Option<String> {
    let preset = group.get("presetName").and_then(Value::as_str);
    let name = group.get("name").and_then(Value::as_str);
    let source = preset
        .filter(|value| !value.is_empty() && *value != "unknown")
        .or(name)
        .filter(|value| !value.is_empty())?;
    Some(
        match source {
            "A" => "aa",
            "I" => "ih",
            "U" => "ou",
            "E" => "ee",
            "O" => "oh",
            "Blink" => "blink",
            "Blink_L" => "blinkLeft",
            "Blink_R" => "blinkRight",
            "LookUp" => "lookUp",
            "LookDown" => "lookDown",
            "LookLeft" => "lookLeft",
            "LookRight" => "lookRight",
            "Joy" => "joy",
            "Angry" => "angry",
            "Sorrow" => "sorrow",
            "Fun" => "fun",
            other => other,
        }
        .into(),
    )
}

fn normalized_legacy_spring_bone(legacy: &Value) -> Option<VRMCSpringBone> {
    let secondary = legacy.get("secondaryAnimation")?;
    let mut colliders = Vec::new();
    let mut collider_groups = Vec::new();

    for group in secondary
        .get("colliderGroups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(node) = group.get("node").and_then(node_index) else {
            collider_groups.push(ColliderGroup {
                name: None,
                colliders: Vec::new(),
            });
            continue;
        };
        let group_colliders = group
            .get("colliders")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|collider| {
                let offset = collider
                    .get("offset")
                    .and_then(vector3)
                    .unwrap_or([0.0, 0.0, 0.0]);
                let radius = collider
                    .get("radius")
                    .and_then(finite_f32)
                    .map(|radius| radius.max(0.0))?;
                let index = colliders.len() as u64;
                colliders.push(Collider {
                    node,
                    shape: crate::vrm::gltf::extensions::vrmc_spring_bone::ColliderShape::Sphere(
                        Sphere { offset, radius },
                    ),
                });
                Some(index)
            })
            .collect();
        collider_groups.push(ColliderGroup {
            name: None,
            colliders: group_colliders,
        });
    }

    let springs = secondary
        .get("boneGroups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, group)| {
            let joints = group
                .get("bones")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(node_index)
                .map(|node| SpringJoint {
                    node,
                    drag_force: Some(
                        group
                            .get("dragForce")
                            .and_then(finite_f32)
                            .unwrap_or(0.0)
                            .clamp(0.0, 1.0),
                    ),
                    gravity_dir: Some(
                        group
                            .get("gravityDir")
                            .and_then(vector3)
                            .map(legacy_gravity_direction)
                            .unwrap_or([0.0, -1.0, 0.0]),
                    ),
                    gravity_power: Some(
                        group
                            .get("gravityPower")
                            .and_then(finite_f32)
                            .unwrap_or(0.0)
                            .max(0.0),
                    ),
                    hit_radius: Some(
                        group
                            .get("hitRadius")
                            .and_then(finite_f32)
                            .unwrap_or(0.0)
                            .max(0.0),
                    ),
                    stiffness: Some(
                        group
                            .get("stiffiness")
                            .or_else(|| group.get("stiffness"))
                            .and_then(finite_f32)
                            .unwrap_or(0.0)
                            .clamp(0.0, 1.0),
                    ),
                })
                .collect::<Vec<_>>();
            if joints.is_empty() {
                return None;
            }
            let collider_groups =
                group
                    .get("colliderGroups")
                    .and_then(Value::as_array)
                    .map(|groups| {
                        groups
                            .iter()
                            .filter_map(|group| {
                                group.as_u64().and_then(|index| index.try_into().ok())
                            })
                            .filter(|index: &usize| *index < collider_groups.len())
                            .collect()
                    });
            Some(Spring {
                name: format!("legacy-spring-{index}"),
                joints,
                collider_groups,
                center: group.get("center").and_then(node_index),
                terminal_length: Some(0.07),
            })
        })
        .collect::<Vec<_>>();

    let mut claimed_nodes = HashSet::<usize>::default();
    let springs = springs
        .into_iter()
        .filter_map(|mut spring| {
            spring
                .joints
                .retain(|joint| claimed_nodes.insert(joint.node));
            (!spring.joints.is_empty()).then_some(spring)
        })
        .collect();
    Some(VRMCSpringBone {
        spec_version: "1.0".into(),
        colliders,
        collider_groups,
        springs,
    })
}

fn node_index(value: &Value) -> Option<usize> {
    value.as_u64()?.try_into().ok()
}

fn finite_f32(value: &Value) -> Option<f32> {
    value
        .as_f64()
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
}

fn vector3(value: &Value) -> Option<[f32; 3]> {
    let object = value.as_object()?;
    Some([
        finite_f32(object.get("x")?)?,
        finite_f32(object.get("y")?)?,
        finite_f32(object.get("z")?)?,
    ])
}

fn legacy_gravity_direction([x, y, z]: [f32; 3]) -> [f32; 3] {
    // VRM 0.x faces -Z while the normalized runtime basis faces +Z. The
    // scene basis rotates node transforms; gravity is a world-space vector,
    // so it receives the same Y=pi conversion exactly once here.
    [-x, y, -z]
}

/// Represents a node in the glTF file.
#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub struct VrmNode {
    /// The index of the node in the glTF file.
    pub node: usize,
}

pub(crate) fn obtain_extensions(
    gltf: &Gltf
) -> AppResult<&serde_json::map::Map<String, serde_json::Value>> {
    gltf.source
        .as_ref()
        .and_then(|source| source.extensions())
        .context("Not found gltf extensions")
}

pub(crate) fn obtain_vrmc_vrm(
    json: &serde_json::map::Map<String, serde_json::Value>
) -> AppResult<serde_json::Value> {
    Ok(json
        .get("VRMC_vrm")
        .or_else(|| json.get("VRMC_vrm_animation"))
        .context("Not found VRMC_vrm or VRMC_vrm_animation")?
        .clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_legacy_core_into_existing_vrm1_contract() {
        let root = json!({
            "extensions": {"VRM": {
                "meta": {"title": "legacy", "author": "author"},
                "humanoid": {"humanBones": [
                    {"bone": "hips", "node": 0}, {"bone": "head", "node": 1}
                ]},
                "blendShapeMaster": {"blendShapeGroups": [
                    {"name": "vowel-a", "presetName": "A", "binds": [
                        {"mesh": 3, "index": 4, "weight": 50.0}
                    ]},
                    {"name": "left-blink", "presetName": "Blink_L", "isBinary": true}
                ]},
                "firstPerson": {"meshAnnotations": [
                    {"mesh": 2, "firstPersonFlag": "Both"}
                ]},
                "lookAtMaster": {
                    "type": "Bone",
                    "offsetFromHeadBone": [0.0, 0.0, 0.0],
                    "lookAtHorizontalInner": {"curve": {"xRange": 90.0, "yRange": 10.0}},
                    "lookAtHorizontalOuter": {"curve": {"xRange": 90.0, "yRange": 10.0}},
                    "lookAtVerticalDown": {"curve": {"xRange": 90.0, "yRange": 10.0}},
                    "lookAtVerticalUp": {"curve": {"xRange": 90.0, "yRange": 10.0}}
                }
            }}
        });
        let extensions = root["extensions"].as_object().unwrap().clone();
        let normalized = VrmExtensions::new(&extensions).expect("legacy core should normalize");

        assert_eq!(
            normalized.runtime_descriptor.generation,
            VrmGeneration::Vrm0
        );
        assert_eq!(normalized.name().as_deref(), Some("legacy"));
        assert_eq!(normalized.vrmc_vrm.spec_version, "1.0");
        assert_eq!(normalized.vrmc_vrm.humanoid.human_bones["head"].node, 1);
        let expressions = normalized.vrmc_vrm.expressions.as_ref().unwrap();
        assert!(expressions.preset.contains_key("aa"));
        assert!(expressions.preset.contains_key("blinkLeft"));
        let bind = &expressions.preset["aa"]
            .morph_target_binds
            .as_ref()
            .unwrap()[0];
        assert_eq!(bind.node, 3);
        assert_eq!(bind.index, 4);
        assert_eq!(bind.weight, 0.5);
        assert!(expressions.preset["blinkLeft"].is_binary);
        assert_eq!(
            normalized
                .vrmc_vrm
                .first_person
                .as_ref()
                .unwrap()
                .mesh_annotations[0]
                .first_person_flag,
            FirstPersonFlag::Both
        );
        assert_eq!(
            normalized.vrmc_vrm.look_at.as_ref().unwrap().r#type,
            LookAtType::Bone
        );
    }

    #[test]
    fn canonicalizes_legacy_expression_preset_names() {
        let groups = json!([
            {"name": "a", "presetName": "A"},
            {"name": "i", "presetName": "I"},
            {"name": "u", "presetName": "U"},
            {"name": "e", "presetName": "E"},
            {"name": "o", "presetName": "O"},
            {"name": "blink", "presetName": "Blink"},
            {"name": "joy", "presetName": "Joy"},
            {"name": "look-up", "presetName": "LookUp"},
            {"name": "custom", "presetName": "unknown"}
        ]);
        let legacy = json!({"blendShapeMaster": {"blendShapeGroups": groups}});
        let expressions = normalized_legacy_expressions(&legacy).expect("groups should parse");
        for name in [
            "aa", "ih", "ou", "ee", "oh", "blink", "joy", "lookUp", "custom",
        ] {
            assert!(expressions.preset.contains_key(name), "missing {name}");
        }
    }

    #[test]
    fn normalizes_legacy_secondary_animation_with_terminal_and_gravity_basis() {
        let legacy = json!({
            "secondaryAnimation": {
                "colliderGroups": [{
                    "node": 2,
                    "colliders": [{
                        "offset": {"x": 0.1, "y": 0.2, "z": 0.3},
                        "radius": 0.04
                    }]
                }],
                "boneGroups": [{
                    "stiffiness": 0.8,
                    "gravityPower": 0.5,
                    "gravityDir": {"x": 1.0, "y": -1.0, "z": 0.25},
                    "dragForce": 0.2,
                    "center": 0,
                    "hitRadius": 0.02,
                    "bones": [4, 5],
                    "colliderGroups": [0]
                }]
            }
        });
        let spring = normalized_legacy_spring_bone(&legacy).expect("spring should parse");
        assert_eq!(spring.springs.len(), 1);
        assert_eq!(spring.springs[0].joints.len(), 2);
        assert_eq!(spring.springs[0].terminal_length, Some(0.07));
        assert_eq!(
            spring.springs[0].joints[0].gravity_dir,
            Some([-1.0, -1.0, -0.25])
        );
        assert_eq!(spring.colliders.len(), 1);
        assert_eq!(spring.spring_colliders(&[0, 9]).len(), 1);
    }

    #[test]
    fn drops_duplicate_legacy_joint_writers() {
        let legacy = json!({
            "secondaryAnimation": {
                "boneGroups": [
                    {"bones": [4, 5]},
                    {"bones": [5, 6]}
                ]
            }
        });
        let spring = normalized_legacy_spring_bone(&legacy).expect("spring should parse");
        assert_eq!(
            spring.springs[0]
                .joints
                .iter()
                .map(|joint| joint.node)
                .collect::<Vec<_>>(),
            [4, 5]
        );
        assert_eq!(
            spring.springs[1]
                .joints
                .iter()
                .map(|joint| joint.node)
                .collect::<Vec<_>>(),
            [6]
        );
    }
}
