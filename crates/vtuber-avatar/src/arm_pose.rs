//! Runtime composition for the model-adaptive default arm pose.
//!
//! Binding resolves a typed, rest-relative pose once. This module applies that
//! pose after animation and direct body tracking without changing immutable
//! rest components or accumulating the delta from one frame to the next.

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::arm::{
    ArmChainBinding, ArmIkInput, ArmPoseProfile, FingerJointRestBinding, FingerJointRestReferences,
    FingerRestReferences, default_arm_target, solve_two_bone_arm,
};
use crate::binding::AvatarBinding;
use crate::lifecycle::{ActiveAvatar, AvatarGeneration};

const ROTATION_MATCH_EPSILON: f32 = 1.0e-6;
const SHOULDER_FOLLOW_WEIGHT: f32 = 0.18;
const SHOULDER_FOLLOW_MAX_RADIANS: f32 = 5.0_f32.to_radians();
const FINGER_CURL_RADIANS: f32 = 10.0_f32.to_radians();

/// A resolved default pose for one complete arm chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedArmPose {
    /// Upper-arm entity to receive the local rest-relative delta.
    pub upper_arm: Entity,
    /// Lower-arm entity to receive the local rest-relative delta.
    pub lower_arm: Entity,
    /// Upper-arm local rest-relative rotation.
    pub upper_arm_delta: Quat,
    /// Lower-arm local rest-relative rotation.
    pub lower_arm_delta: Quat,
    /// Optional weak shoulder-follow correction.
    pub shoulder: Option<ResolvedBoneDelta>,
    /// Authored finger curl corrections.
    pub fingers: ResolvedFingerPose,
}

/// One optional bone's local rest-relative correction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedBoneDelta {
    /// Bone entity receiving the correction.
    pub entity: Entity,
    /// Local rest-relative correction.
    pub delta: Quat,
}

/// Resolved weak curl corrections for one arm's finger joints.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ResolvedFingerPose {
    /// Thumb corrections.
    pub thumb: ResolvedFingerJointPose,
    /// Index-finger corrections.
    pub index: ResolvedFingerJointPose,
    /// Middle-finger corrections.
    pub middle: ResolvedFingerJointPose,
    /// Ring-finger corrections.
    pub ring: ResolvedFingerJointPose,
    /// Little-finger corrections.
    pub little: ResolvedFingerJointPose,
}

/// Resolved corrections for the joints of one finger.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ResolvedFingerJointPose {
    /// Metacarpal correction.
    pub metacarpal: Option<ResolvedBoneDelta>,
    /// Proximal correction.
    pub proximal: Option<ResolvedBoneDelta>,
    /// Intermediate correction.
    pub intermediate: Option<ResolvedBoneDelta>,
    /// Distal correction.
    pub distal: Option<ResolvedBoneDelta>,
}

/// The typed default arm pose resolved for one avatar generation.
///
/// Each side is independently optional. An incomplete or degenerate arm chain
/// therefore leaves that side untouched without preventing the avatar from
/// becoming ready.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct DefaultArmPose {
    /// Avatar generation this pose belongs to.
    pub generation: AvatarGeneration,
    /// Resolved left-arm pose, when the complete chain is usable.
    pub left: Option<ResolvedArmPose>,
    /// Resolved right-arm pose, when the complete chain is usable.
    pub right: Option<ResolvedArmPose>,
}

impl DefaultArmPose {
    /// Resolves both default arm poses from immutable binding geometry.
    #[must_use]
    pub fn from_chains(
        generation: AvatarGeneration,
        left: Option<ArmChainBinding>,
        right: Option<ArmChainBinding>,
    ) -> Self {
        let profile = ArmPoseProfile::default();
        Self {
            generation,
            left: left.and_then(|chain| resolve_chain(chain, profile)),
            right: right.and_then(|chain| resolve_chain(chain, profile)),
        }
    }
}

fn resolve_chain(
    chain: ArmChainBinding,
    profile: crate::arm::ArmPoseProfile,
) -> Option<ResolvedArmPose> {
    let target = default_arm_target(&chain, profile).ok()?;
    let input = ArmIkInput::from_geometry(chain.rest, target);
    let solution = solve_two_bone_arm(input).ok()?;
    if !solution.upper_arm_delta.is_finite()
        || !solution.lower_arm_delta.is_finite()
        || solution.upper_arm_delta.length_squared() <= f32::EPSILON
        || solution.lower_arm_delta.length_squared() <= f32::EPSILON
    {
        return None;
    }

    let upper_model_delta = normalized_or_identity(
        solution.upper_arm_global_rotation * chain.rest.upper_arm.global_rotation.inverse(),
    )?;
    let shoulder = chain
        .shoulder
        .zip(chain.rest.shoulder)
        .and_then(|(entity, rest)| {
            weak_follow_delta(upper_model_delta, rest.global_rotation)
                .map(|delta| ResolvedBoneDelta { entity, delta })
        });

    Some(ResolvedArmPose {
        upper_arm: chain.upper_arm,
        lower_arm: chain.lower_arm,
        upper_arm_delta: solution.upper_arm_delta.normalize(),
        lower_arm_delta: solution.lower_arm_delta.normalize(),
        shoulder,
        fingers: resolve_finger_pose(chain.finger_rest),
    })
}

fn normalized_or_identity(value: Quat) -> Option<Quat> {
    if value.is_finite() && value.length_squared() > f32::EPSILON {
        Some(value.normalize())
    } else {
        None
    }
}

fn weak_follow_delta(model_delta: Quat, rest_global: Quat) -> Option<Quat> {
    let model_delta = normalized_or_identity(model_delta)?;
    let (axis, angle) = model_delta.to_axis_angle();
    let angle = (angle * SHOULDER_FOLLOW_WEIGHT).min(SHOULDER_FOLLOW_MAX_RADIANS);
    let weak_model_delta = Quat::from_axis_angle(axis, angle);
    normalized_or_identity(rest_global.inverse() * weak_model_delta * rest_global)
}

fn resolve_finger_pose(fingers: FingerRestReferences) -> ResolvedFingerPose {
    ResolvedFingerPose {
        thumb: resolve_finger_joints(fingers.thumb),
        index: resolve_finger_joints(fingers.index),
        middle: resolve_finger_joints(fingers.middle),
        ring: resolve_finger_joints(fingers.ring),
        little: resolve_finger_joints(fingers.little),
    }
}

fn resolve_finger_joints(finger: FingerJointRestReferences) -> ResolvedFingerJointPose {
    ResolvedFingerJointPose {
        metacarpal: resolve_finger_joint(finger.metacarpal, finger.proximal, None),
        proximal: resolve_finger_joint(finger.proximal, finger.intermediate, finger.metacarpal),
        intermediate: resolve_finger_joint(finger.intermediate, finger.distal, finger.proximal),
        distal: resolve_finger_joint(finger.distal, None, finger.intermediate),
    }
}

fn resolve_finger_joint(
    joint: Option<FingerJointRestBinding>,
    next: Option<FingerJointRestBinding>,
    previous: Option<FingerJointRestBinding>,
) -> Option<ResolvedBoneDelta> {
    let joint = joint?;
    let segment = next
        .map(|next| next.rest.position - joint.rest.position)
        .or_else(|| previous.map(|previous| joint.rest.position - previous.rest.position))?;
    let segment_direction = finite_normalized(segment)?;
    // Prefer authored local Z/Y axes, projected off the finger segment. This
    // gives a bend axis for the common straight +X finger without assuming a
    // universal world Euler axis, while still handling unusual authored axes.
    let axis = [
        joint.rest.global_rotation * Vec3::Z,
        joint.rest.global_rotation * Vec3::Y,
        joint.rest.global_rotation * Vec3::X,
    ]
    .into_iter()
    .map(|candidate| candidate - segment_direction * candidate.dot(segment_direction))
    .find_map(finite_normalized)?;
    let model_delta = Quat::from_axis_angle(axis, FINGER_CURL_RADIANS);
    let delta = normalized_or_identity(
        joint.rest.global_rotation.inverse() * model_delta * joint.rest.global_rotation,
    )?;
    Some(ResolvedBoneDelta {
        entity: joint.entity,
        delta,
    })
}

fn finite_normalized(value: Vec3) -> Option<Vec3> {
    let length_squared = value.length_squared();
    if value.is_finite() && length_squared.is_finite() && length_squared > f32::EPSILON {
        Some(value.normalize())
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct DefaultArmPoseBoneState {
    base: Quat,
    last_delta: Quat,
    initialized: bool,
}

impl Default for DefaultArmPoseBoneState {
    fn default() -> Self {
        Self {
            base: Quat::IDENTITY,
            last_delta: Quat::IDENTITY,
            initialized: false,
        }
    }
}

/// Applies the resolved default arm pose after animation and direct tracking.
///
/// The state detects an animation change by comparing the current transform to
/// the previous composed output. This makes the operation stable across
/// frames while still allowing an animation system to provide a new base pose.
/// The affected subtree is then propagated through its actual `ChildOf` path,
/// including intermediate nodes, before VRM gaze and constraints execute.
#[allow(clippy::type_complexity)]
pub fn apply_default_arm_pose(
    roots: Query<(&AvatarBinding, &DefaultArmPose), With<ActiveAvatar>>,
    mut transforms: Query<(&mut Transform, &mut GlobalTransform)>,
    child_ofs: Query<&ChildOf>,
    children: Query<&Children>,
    mut bone_states: Local<HashMap<Entity, DefaultArmPoseBoneState>>,
) {
    bone_states.retain(|entity, _| transforms.contains(*entity));

    for (binding, pose) in roots.iter() {
        if pose.generation != binding.generation {
            continue;
        }

        for resolved in [pose.left, pose.right].into_iter().flatten() {
            let mut any_changed = false;
            if let Some(shoulder) = resolved.shoulder {
                any_changed |= apply_delta(
                    shoulder.entity,
                    shoulder.delta,
                    &mut transforms,
                    &mut bone_states,
                );
            }
            any_changed |= apply_delta(
                resolved.upper_arm,
                resolved.upper_arm_delta,
                &mut transforms,
                &mut bone_states,
            );
            any_changed |= apply_delta(
                resolved.lower_arm,
                resolved.lower_arm_delta,
                &mut transforms,
                &mut bone_states,
            );
            for finger in [
                resolved.fingers.thumb.metacarpal,
                resolved.fingers.thumb.proximal,
                resolved.fingers.thumb.intermediate,
                resolved.fingers.thumb.distal,
                resolved.fingers.index.metacarpal,
                resolved.fingers.index.proximal,
                resolved.fingers.index.intermediate,
                resolved.fingers.index.distal,
                resolved.fingers.middle.metacarpal,
                resolved.fingers.middle.proximal,
                resolved.fingers.middle.intermediate,
                resolved.fingers.middle.distal,
                resolved.fingers.ring.metacarpal,
                resolved.fingers.ring.proximal,
                resolved.fingers.ring.intermediate,
                resolved.fingers.ring.distal,
                resolved.fingers.little.metacarpal,
                resolved.fingers.little.proximal,
                resolved.fingers.little.intermediate,
                resolved.fingers.little.distal,
            ]
            .into_iter()
            .flatten()
            {
                any_changed |= apply_delta(
                    finger.entity,
                    finger.delta,
                    &mut transforms,
                    &mut bone_states,
                );
            }
            if !any_changed {
                continue;
            }

            let mut computed = HashMap::new();
            let mut visiting = HashSet::new();
            if refresh_parent_global(
                resolved
                    .shoulder
                    .map(|bone| bone.entity)
                    .unwrap_or(resolved.upper_arm),
                &mut transforms,
                &child_ofs,
                &mut computed,
                &mut visiting,
            )
            .is_none()
            {
                continue;
            }
            let refresh_root = resolved
                .shoulder
                .map(|bone| bone.entity)
                .unwrap_or(resolved.upper_arm);
            let root_parent_global = child_ofs
                .get(refresh_root)
                .ok()
                .and_then(|child_of| computed.get(&child_of.parent()).copied())
                .unwrap_or(GlobalTransform::IDENTITY);
            refresh_subtree(
                refresh_root,
                root_parent_global,
                &mut transforms,
                &children,
                &mut HashSet::new(),
            );
        }
    }
}

fn apply_delta(
    entity: Entity,
    delta: Quat,
    transforms: &mut Query<(&mut Transform, &mut GlobalTransform)>,
    bone_states: &mut HashMap<Entity, DefaultArmPoseBoneState>,
) -> bool {
    if !delta.is_finite() || delta.length_squared() <= f32::EPSILON {
        return false;
    }
    let Ok((mut transform, _global)) = transforms.get_mut(entity) else {
        return false;
    };

    let state = bone_states.entry(entity).or_default();
    let expected_previous = state.base * state.last_delta;
    let animation_changed = !state.initialized
        || !transform.rotation.is_finite()
        || transform.rotation.dot(expected_previous).abs() < 1.0 - ROTATION_MATCH_EPSILON;
    let base = if animation_changed {
        finite_normalized_or(transform.rotation, Quat::IDENTITY)
    } else {
        state.base
    };
    let delta = delta.normalize();
    let output = finite_normalized_or(base * delta, base);
    transform.rotation = output;
    state.base = base;
    state.last_delta = delta;
    state.initialized = true;
    true
}

fn finite_normalized_or(value: Quat, fallback: Quat) -> Quat {
    if value.is_finite() && value.length_squared() > f32::EPSILON {
        value.normalize()
    } else {
        fallback
    }
}

fn refresh_parent_global(
    entity: Entity,
    transforms: &mut Query<(&mut Transform, &mut GlobalTransform)>,
    child_ofs: &Query<&ChildOf>,
    computed: &mut HashMap<Entity, GlobalTransform>,
    visiting: &mut HashSet<Entity>,
) -> Option<GlobalTransform> {
    if let Some(global) = computed.get(&entity) {
        return Some(*global);
    }
    if !visiting.insert(entity) {
        return None;
    }

    let parent_global = match child_ofs.get(entity) {
        Ok(child_of) => {
            refresh_parent_global(child_of.parent(), transforms, child_ofs, computed, visiting)?
        }
        Err(_) => GlobalTransform::IDENTITY,
    };
    let Ok((transform, mut global)) = transforms.get_mut(entity) else {
        visiting.remove(&entity);
        return None;
    };
    *global = parent_global.mul_transform(*transform);
    let result = *global;
    computed.insert(entity, result);
    visiting.remove(&entity);
    Some(result)
}

fn refresh_subtree(
    entity: Entity,
    parent_global: GlobalTransform,
    transforms: &mut Query<(&mut Transform, &mut GlobalTransform)>,
    children: &Query<&Children>,
    visited: &mut HashSet<Entity>,
) {
    if !visited.insert(entity) {
        return;
    }
    let current_global = {
        let Ok((transform, mut global)) = transforms.get_mut(entity) else {
            return;
        };
        *global = parent_global.mul_transform(*transform);
        *global
    };

    if let Ok(child_entities) = children.get(entity) {
        for child in child_entities.iter() {
            refresh_subtree(child, current_global, transforms, children, visited);
        }
    }
}
