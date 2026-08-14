use crate::vrm::gltf::extensions::vrmc_spring_bone::{
    Collider, ColliderShape, Spring, SpringJoint, VRMCSpringBone,
};
use crate::vrm::spring_bone::SpringJointProps;
use bevy::app::App;
use bevy::asset::{Assets, Handle};
use bevy::gltf::GltfNode;
use bevy::math::Vec3;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

pub(super) struct SpringBoneRegistryPlugin;

impl Plugin for SpringBoneRegistryPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<SpringColliderRegistry>()
            .register_type::<SpringJointPropsRegistry>()
            .register_type::<SpringNodeRegistry>();
    }
}

#[derive(Component, Deref, Debug, Default, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub(crate) struct SpringColliderRegistry(pub(crate) HashMap<Name, ColliderShape>);

impl SpringColliderRegistry {
    pub fn new(
        colliders: &[Collider],
        node_assets: &Assets<GltfNode>,
        nodes: &[Handle<GltfNode>],
    ) -> Self {
        let unique_names = unique_node_names(node_assets, nodes);
        let mut claimed_names = HashSet::<String>::default();
        Self(
            colliders
                .iter()
                .filter_map(|collider| {
                    let node_handle = nodes.get(collider.node)?;
                    let node = node_assets.get(node_handle)?;
                    if !unique_names.contains(&node.name)
                        || !claimed_names.insert(node.name.clone())
                    {
                        return None;
                    }
                    Some((Name::new(node.name.clone()), collider.shape))
                })
                .collect(),
        )
    }
}

#[derive(Component, Deref, Debug, Default, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub(crate) struct SpringJointPropsRegistry(pub(crate) HashMap<Name, SpringJointProps>);

impl SpringJointPropsRegistry {
    pub fn new(
        joints: &[SpringJoint],
        node_assets: &Assets<GltfNode>,
        nodes: &[Handle<GltfNode>],
    ) -> Self {
        let unique_names = unique_node_names(node_assets, nodes);
        let mut claimed_names = HashSet::<String>::default();
        Self(
            joints
                .iter()
                .filter_map(|joint| {
                    let node_handle = nodes.get(joint.node)?;
                    let node = node_assets.get(node_handle)?;
                    if !unique_names.contains(&node.name)
                        || !claimed_names.insert(node.name.clone())
                    {
                        return None;
                    }
                    let dir = joint.gravity_dir?;
                    Some((
                        Name::new(node.name.clone()),
                        SpringJointProps {
                            drag_force: joint.drag_force?,
                            gravity_power: joint.gravity_power?,
                            hit_radius: joint.hit_radius?,
                            stiffness: joint.stiffness?,
                            gravity_dir: Vec3::new(dir[0], dir[1], dir[2]),
                        },
                    ))
                })
                .collect(),
        )
    }
}

#[derive(Component, Debug, Default, Clone, PartialEq, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub(crate) struct SpringNode {
    pub center: Option<Name>,
    pub joints: Vec<Name>,
    pub colliders: Vec<(Name, ColliderShape)>,
    pub terminal_length: Option<f32>,
}

#[derive(Component, Deref, Default, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub(crate) struct SpringNodeRegistry(pub Vec<SpringNode>);

impl SpringNodeRegistry {
    pub fn new(
        spring_bone: &VRMCSpringBone,
        node_assets: &Assets<GltfNode>,
        nodes: &[Handle<GltfNode>],
    ) -> Self {
        let unique_names = unique_node_names(node_assets, nodes);
        let mut claimed_joints = HashSet::<usize>::default();
        Self(
            spring_bone
                .springs
                .iter()
                .filter_map(|spring| {
                    let joints = spring
                        .joints
                        .iter()
                        .filter_map(|joint| {
                            let name = get_node_name(joint.node, node_assets, nodes)?;
                            unique_names
                                .contains(name.as_str())
                                .then_some((joint.node, name))
                        })
                        .filter(|(node, _)| claimed_joints.insert(*node))
                        .map(|(_, name)| name)
                        .collect::<Vec<_>>();
                    (!joints.is_empty()).then_some(SpringNode {
                        joints,
                        colliders: obtain_colliders(
                            spring_bone,
                            spring,
                            node_assets,
                            nodes,
                            &unique_names,
                        ),
                        center: spring
                            .center
                            .and_then(|index| get_node_name(index, node_assets, nodes))
                            .filter(|name| unique_names.contains(name.as_str())),
                        terminal_length: spring.terminal_length,
                    })
                })
                .collect(),
        )
    }
}

fn obtain_colliders(
    spring_bone: &VRMCSpringBone,
    spring: &Spring,
    node_assets: &Assets<GltfNode>,
    nodes: &[Handle<GltfNode>],
    unique_names: &HashSet<String>,
) -> Vec<(Name, ColliderShape)> {
    let Some(collider_groups) = spring.collider_groups.as_ref() else {
        return vec![];
    };
    spring_bone
        .spring_colliders(collider_groups)
        .iter()
        .flat_map(|collider| {
            let name = get_node_name(collider.node, node_assets, nodes)?;
            if !unique_names.contains(name.as_str()) {
                return None;
            }
            Some((name, collider.shape))
        })
        .collect()
}

fn unique_node_names(
    node_assets: &Assets<GltfNode>,
    nodes: &[Handle<GltfNode>],
) -> HashSet<String> {
    let mut counts = HashMap::<String, usize>::new();
    for handle in nodes {
        if let Some(node) = node_assets.get(handle) {
            *counts.entry(node.name.clone()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(name, count)| (count == 1).then_some(name))
        .collect()
}

fn get_node_name(
    node_index: usize,
    node_assets: &Assets<GltfNode>,
    nodes: &[Handle<GltfNode>],
) -> Option<Name> {
    let node_handle = nodes.get(node_index)?;
    let node = node_assets.get(node_handle)?;
    Some(Name::new(node.name.clone()))
}
