use crate::prelude::ChildSearcher;
use crate::vrm::VrmNodeIndex;
use crate::vrm::humanoid_bone::RequestInitializeHumanoidBones;
use crate::vrm::spring_bone::registry::{
    SpringColliderRegistry, SpringJointPropsRegistry, SpringNodeRegistry,
};
use crate::vrm::spring_bone::{
    SpringCenterNode, SpringColliders, SpringJointState, SpringJoints, SpringRoot,
};
use bevy::app::{App, Update};
use bevy::prelude::*;

#[derive(EntityEvent)]
pub(crate) struct RequestInitializeSpringBone(pub(crate) Entity);

pub struct SpringBoneInitializePlugin;

impl Plugin for SpringBoneInitializePlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_systems(Update, init_spring_joint_states)
            .add_observer(apply_initialize_joint_props)
            .add_observer(apply_initialize_collider_shapes)
            .add_observer(apply_initialize_spring_roots);
    }
}

fn apply_initialize_joint_props(
    trigger: On<RequestInitializeHumanoidBones>,
    mut commands: Commands,
    child_searcher: ChildSearcher,
    models: Query<&SpringJointPropsRegistry>,
) {
    let root = trigger.event_target();
    let Ok(nodes) = models.get(root) else {
        return;
    };
    for (node_index, props) in nodes.iter() {
        let Some(joint_entity) = child_searcher.find_from_node_index(root, *node_index) else {
            continue;
        };
        commands.entity(joint_entity).insert(*props);
    }
}

fn apply_initialize_collider_shapes(
    trigger: On<RequestInitializeSpringBone>,
    mut commands: Commands,
    child_searcher: ChildSearcher,
    models: Query<&SpringColliderRegistry>,
) {
    let entity = trigger.event_target();
    let Ok(registry) = models.get(entity) else {
        return;
    };
    for (node_index, shape) in registry.iter() {
        let Some(collider_entity) = child_searcher.find_from_node_index(entity, *node_index) else {
            continue;
        };
        commands.entity(collider_entity).insert(*shape);
    }
}

fn apply_initialize_spring_roots(
    trigger: On<RequestInitializeSpringBone>,
    mut commands: Commands,
    child_searcher: ChildSearcher,
    models: Query<&SpringNodeRegistry>,
) {
    let entity = trigger.event_target();
    let Ok(registry) = models.get(entity) else {
        return;
    };
    for spring_root in registry.0.iter().map(|spring| SpringRoot {
        center_node: SpringCenterNode(
            spring
                .center
                .and_then(|center| child_searcher.find_from_node_index(entity, center)),
        ),
        joints: SpringJoints(
            spring
                .joints
                .iter()
                .filter_map(|joint| child_searcher.find_from_node_index(entity, *joint))
                .collect(),
        ),
        colliders: SpringColliders(
            spring
                .colliders
                .iter()
                .filter_map(|(collider, shape)| {
                    let entity = child_searcher.find_from_node_index(entity, *collider)?;
                    Some((entity, *shape))
                })
                .collect(),
        ),
        terminal_length: spring.terminal_length,
    }) {
        let Some(root) = spring_root.joints.first() else {
            continue;
        };
        commands.entity(*root).insert(spring_root);
    }
}

fn init_spring_joint_states(
    par_commands: ParallelCommands,
    spring_roots: Query<&SpringRoot, Added<SpringRoot>>,
    joints: Query<&Transform>,
    global_transforms: Query<&GlobalTransform>,
    children: Query<&Children>,
    node_indices: Query<Option<&VrmNodeIndex>>,
) {
    spring_roots.par_iter().for_each(|root| {
        for w in root.joints.windows(2) {
            let head_entity = w[0];
            let joint_entity = w[1];
            let Ok(head_tf) = joints.get(head_entity) else {
                continue;
            };
            let Ok(tail_tf) = joints.get(joint_entity) else {
                continue;
            };
            let Ok(tail_gtf) = global_transforms.get(joint_entity) else {
                continue;
            };
            let tail_pos = root
                .center_node
                .and_then(|center| global_transforms.get(center).ok())
                .map(|center_gtf| tail_gtf.reparented_to(center_gtf).translation)
                .unwrap_or(tail_gtf.translation());
            let bone_length = tail_tf.translation.length();
            if !bone_length.is_finite() || bone_length <= f32::EPSILON {
                continue;
            }
            let state = SpringJointState {
                prev_tail: tail_pos,
                current_tail: tail_pos,
                bone_axis: tail_tf.translation / bone_length,
                bone_length,
                initial_local_matrix: head_tf.to_matrix(),
                initial_local_rotation: head_tf.rotation,
            };
            par_commands.command_scope(|mut commands| {
                commands.entity(head_entity).insert(state);
            });
        }
        if let (Some(&last_entity), Some(terminal_length)) =
            (root.joints.last(), root.terminal_length)
            && let (Ok(last_tf), Ok(last_gtf)) =
                (joints.get(last_entity), global_transforms.get(last_entity))
            && let Some(bone_axis) =
                terminal_direction(last_entity, &children, &joints, &node_indices, last_tf)
        {
            let terminal_global =
                last_gtf.translation() + last_gtf.rotation().mul_vec3(bone_axis * terminal_length);
            let terminal_tail = root
                .center_node
                .and_then(|center| global_transforms.get(center).ok())
                .map(|center_gtf| {
                    center_gtf
                        .to_matrix()
                        .inverse()
                        .transform_point3(terminal_global)
                })
                .unwrap_or(terminal_global);
            let state = SpringJointState {
                prev_tail: terminal_tail,
                current_tail: terminal_tail,
                bone_axis,
                bone_length: terminal_length,
                initial_local_matrix: last_tf.to_matrix(),
                initial_local_rotation: last_tf.rotation,
            };
            par_commands.command_scope(|mut commands| {
                commands.entity(last_entity).insert(state);
            });
        }
    });
}

fn terminal_direction(
    entity: Entity,
    children: &Query<&Children>,
    transforms: &Query<&Transform>,
    node_indices: &Query<Option<&VrmNodeIndex>>,
    last_transform: &Transform,
) -> Option<Vec3> {
    let child_direction = children
        .get(entity)
        .ok()?
        .iter()
        .filter_map(|child| {
            let transform = transforms.get(child).ok()?;
            node_indices
                .get(child)
                .ok()
                .flatten()
                .map(|_| transform.translation)
        })
        .find(|translation| translation.length_squared() > f32::EPSILON)
        .map(|translation| translation.normalize());
    child_direction.or_else(|| {
        (last_transform.translation.length_squared() > f32::EPSILON)
            .then(|| last_transform.translation.normalize())
    })
}

#[cfg(test)]
mod tests {}
