//! Runtime composition for the model-adaptive default arm pose.
//!
//! Binding resolves a typed, rest-relative pose once. This module applies that
//! pose after animation and direct body tracking without changing immutable
//! rest components or accumulating the delta from one frame to the next.

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::arm::{ArmChainBinding, ArmIkInput, default_arm_target, solve_two_bone_arm};
use crate::binding::AvatarBinding;
use crate::lifecycle::{ActiveAvatar, AvatarGeneration};

const ROTATION_MATCH_EPSILON: f32 = 1.0e-6;

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
        let profile = crate::arm::ArmPoseProfile::default();
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

    Some(ResolvedArmPose {
        upper_arm: chain.upper_arm,
        lower_arm: chain.lower_arm,
        upper_arm_delta: solution.upper_arm_delta.normalize(),
        lower_arm_delta: solution.lower_arm_delta.normalize(),
    })
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
            let upper_changed = apply_delta(
                resolved.upper_arm,
                resolved.upper_arm_delta,
                &mut transforms,
                &mut bone_states,
            );
            let lower_changed = apply_delta(
                resolved.lower_arm,
                resolved.lower_arm_delta,
                &mut transforms,
                &mut bone_states,
            );
            if !(upper_changed || lower_changed) {
                continue;
            }

            let mut computed = HashMap::new();
            let mut visiting = HashSet::new();
            if refresh_parent_global(
                resolved.upper_arm,
                &mut transforms,
                &child_ofs,
                &mut computed,
                &mut visiting,
            )
            .is_none()
            {
                continue;
            }
            let upper_parent_global = child_ofs
                .get(resolved.upper_arm)
                .ok()
                .and_then(|child_of| computed.get(&child_of.parent()).copied())
                .unwrap_or(GlobalTransform::IDENTITY);
            refresh_subtree(
                resolved.upper_arm,
                upper_parent_global,
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
