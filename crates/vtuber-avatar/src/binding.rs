//! Humanoid bone binding for initialized VRM roots.
//!
//! Once the lifecycle enters [`Binding`](crate::lifecycle::AvatarLifecycleState::Binding),
//! this module resolves the required and optional humanoid bone entities that
//! `bevy_vrm1` inserts on the VRM root, validates their components, and caches
//! the result in an [`AvatarBinding`] component. Systems that drive the avatar
//! read that cache instead of re-querying root components every frame.

use bevy::log::warn;
use bevy::prelude::*;
use bevy_vrm1::prelude::*;
use std::time::{Duration, Instant};

use crate::bind::BindTriggered;
use crate::lifecycle::{
    ActiveAvatar, AvatarLifecycle, AvatarLifecycleFailure, AvatarLifecycleState,
};

/// Maximum time to wait for transient bone components after entering `Binding`.
const BIND_TIMEOUT: Duration = Duration::from_secs(2);

/// Cached humanoid bone bindings for a single active avatar.
///
/// This component is inserted on the avatar root once binding succeeds. Later
/// systems read it instead of performing root-component lookups each frame.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvatarBinding {
    /// Avatar root entity.
    pub root: Entity,
    /// Required head bone entity.
    pub head: Entity,
    /// Optional neck bone entity.
    pub neck: Option<Entity>,
    /// Optional upper-chest bone entity.
    pub upper_chest: Option<Entity>,
    /// Optional chest bone entity.
    pub chest: Option<Entity>,
    /// Optional spine bone entity.
    pub spine: Option<Entity>,
    /// Optional left eye bone entity.
    pub left_eye: Option<Entity>,
    /// Optional right eye bone entity.
    pub right_eye: Option<Entity>,
}

impl AvatarBinding {
    /// Creates a head-only binding for the simplest capable avatar.
    #[must_use]
    pub fn head_only(root: Entity, head: Entity) -> Self {
        Self {
            root,
            head,
            neck: None,
            upper_chest: None,
            chest: None,
            spine: None,
            left_eye: None,
            right_eye: None,
        }
    }
}

/// Errors that can occur while binding humanoid bones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AvatarBindError {
    /// A required bone component is missing from the VRM root.
    MissingRequiredBone {
        /// Human-readable bone name, e.g. `head`.
        bone: &'static str,
    },
    /// A referenced bone entity no longer exists in the world.
    BoneEntityDespawned {
        /// Bone name.
        bone: &'static str,
        /// Entity that was referenced.
        entity: Entity,
    },
    /// A bone entity exists but has no [`Transform`].
    MissingTransform {
        /// Bone name.
        bone: &'static str,
        /// Entity that is missing the component.
        entity: Entity,
    },
    /// A bone entity exists but has no [`RestTransform`].
    MissingRestTransform {
        /// Bone name.
        bone: &'static str,
        /// Entity that is missing the component.
        entity: Entity,
    },
    /// Binding did not complete within the deadline.
    Timeout,
}

impl AvatarBindError {
    /// Returns the bone name for errors that refer to a specific bone.
    #[must_use]
    pub const fn bone_name(&self) -> Option<&'static str> {
        match self {
            Self::MissingRequiredBone { bone }
            | Self::BoneEntityDespawned { bone, .. }
            | Self::MissingTransform { bone, .. }
            | Self::MissingRestTransform { bone, .. } => Some(*bone),
            Self::Timeout => None,
        }
    }
}

impl std::fmt::Display for AvatarBindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequiredBone { bone } => {
                write!(f, "required humanoid bone `{bone}` is missing")
            }
            Self::BoneEntityDespawned { bone, entity } => {
                write!(f, "humanoid bone `{bone}` entity {entity:?} was despawned")
            }
            Self::MissingTransform { bone, entity } => {
                write!(
                    f,
                    "humanoid bone `{bone}` entity {entity:?} has no Transform"
                )
            }
            Self::MissingRestTransform { bone, entity } => {
                write!(
                    f,
                    "humanoid bone `{bone}` entity {entity:?} has no RestTransform"
                )
            }
            Self::Timeout => write!(f, "humanoid bone binding timed out"),
        }
    }
}

impl std::error::Error for AvatarBindError {}

/// Internal deadline used to bound retry time while waiting for bone
/// components.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingDeadline(Instant);

/// Resolves humanoid bones for the active avatar root and caches them in
/// [`AvatarBinding`].
///
/// Runs while the lifecycle is in `Binding`. On success it inserts
/// [`AvatarBinding`] on the root and transitions the lifecycle to `Ready`. On
/// failure it transitions to `Failed` with a typed error.
#[allow(clippy::type_complexity)]
pub fn bind_humanoid_bones(
    mut commands: Commands,
    mut lifecycle: ResMut<AvatarLifecycle>,
    roots: Query<
        (
            Entity,
            Option<&HeadBoneEntity>,
            Option<&NeckBoneEntity>,
            Option<&UpperChestBoneEntity>,
            Option<&ChestBoneEntity>,
            Option<&SpineBoneEntity>,
            Option<&LeftEyeBoneEntity>,
            Option<&RightEyeBoneEntity>,
        ),
        (With<ActiveAvatar>, With<BindTriggered>),
    >,
    bone_query: Query<(Option<&Transform>, Option<&RestTransform>)>,
    deadlines: Query<&BindingDeadline>,
) {
    if lifecycle.state() != AvatarLifecycleState::Binding {
        return;
    }

    let Some(root) = lifecycle.active_root() else {
        return;
    };

    let root_data = match roots.get(root) {
        Ok(data) => data,
        Err(_) => {
            // The tracked root is not a valid active bind target. Treat this as
            // a missing required head so the lifecycle does not stall.
            fail_binding(
                &mut commands,
                &mut lifecycle,
                root,
                AvatarBindError::MissingRequiredBone { bone: "head" },
            );
            return;
        }
    };

    let (root_entity, head, neck, upper_chest, chest, spine, left_eye, right_eye) = root_data;

    let deadline = match deadlines.get(root_entity) {
        Ok(BindingDeadline(deadline)) => *deadline,
        Err(_) => {
            let deadline = Instant::now() + BIND_TIMEOUT;
            commands
                .entity(root_entity)
                .insert(BindingDeadline(deadline));
            deadline
        }
    };

    let result = resolve_binding(
        root_entity,
        head,
        neck,
        upper_chest,
        chest,
        spine,
        left_eye,
        right_eye,
        &bone_query,
    );

    match result {
        Ok(binding) => {
            commands.entity(root_entity).insert(binding);
            commands.entity(root_entity).remove::<BindingDeadline>();
            lifecycle.finish_ready();
        }
        Err(error) => {
            // A missing head is permanent, so fail immediately. Everything else
            // is retried until the deadline.
            let permanent = error.bone_name() == Some("head");
            if permanent || Instant::now() >= deadline {
                fail_binding(&mut commands, &mut lifecycle, root_entity, error);
            }
        }
    }
}

fn fail_binding(
    commands: &mut Commands,
    lifecycle: &mut AvatarLifecycle,
    root: Entity,
    error: AvatarBindError,
) {
    if let Ok(mut entity_commands) = commands.get_entity(root) {
        entity_commands.remove::<ActiveAvatar>();
        entity_commands.remove::<BindingDeadline>();
    }

    let failure = match &error {
        AvatarBindError::Timeout => AvatarLifecycleFailure::BindingTimeout,
        _ => AvatarLifecycleFailure::MissingRequiredBone {
            bone: error.bone_name().unwrap_or("head"),
        },
    };
    lifecycle.fail(failure);
}

#[allow(clippy::too_many_arguments)]
fn resolve_binding(
    root: Entity,
    head: Option<&HeadBoneEntity>,
    neck: Option<&NeckBoneEntity>,
    upper_chest: Option<&UpperChestBoneEntity>,
    chest: Option<&ChestBoneEntity>,
    spine: Option<&SpineBoneEntity>,
    left_eye: Option<&LeftEyeBoneEntity>,
    right_eye: Option<&RightEyeBoneEntity>,
    bone_query: &Query<(Option<&Transform>, Option<&RestTransform>)>,
) -> Result<AvatarBinding, AvatarBindError> {
    let head = resolve_required_bone("head", head.map(|h| **h), bone_query)?;
    let neck = resolve_optional_bone("neck", neck.map(|n| **n), bone_query);
    let upper_chest = resolve_optional_bone("upperChest", upper_chest.map(|b| **b), bone_query);
    let chest = resolve_optional_bone("chest", chest.map(|b| **b), bone_query);
    let spine = resolve_optional_bone("spine", spine.map(|b| **b), bone_query);
    let left_eye = resolve_optional_bone("leftEye", left_eye.map(|e| **e), bone_query);
    let right_eye = resolve_optional_bone("rightEye", right_eye.map(|e| **e), bone_query);

    Ok(AvatarBinding {
        root,
        head,
        neck,
        upper_chest,
        chest,
        spine,
        left_eye,
        right_eye,
    })
}

fn resolve_required_bone(
    name: &'static str,
    entity: Option<Entity>,
    bone_query: &Query<(Option<&Transform>, Option<&RestTransform>)>,
) -> Result<Entity, AvatarBindError> {
    let entity = entity.ok_or(AvatarBindError::MissingRequiredBone { bone: name })?;
    let (transform, rest) = bone_query
        .get(entity)
        .map_err(|_| AvatarBindError::BoneEntityDespawned { bone: name, entity })?;

    if transform.is_none() {
        return Err(AvatarBindError::MissingTransform { bone: name, entity });
    }
    if rest.is_none() {
        return Err(AvatarBindError::MissingRestTransform { bone: name, entity });
    }

    Ok(entity)
}

fn resolve_optional_bone(
    name: &'static str,
    entity: Option<Entity>,
    bone_query: &Query<(Option<&Transform>, Option<&RestTransform>)>,
) -> Option<Entity> {
    let entity = entity?;
    let (transform, rest) = bone_query.get(entity).ok()?;

    if transform.is_none() || rest.is_none() {
        warn!(
            "optional humanoid bone `{name}` entity {entity:?} is missing Transform or RestTransform; treating as absent"
        );
        return None;
    }

    Some(entity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone_entity(world: &mut World) -> Entity {
        world
            .spawn((Transform::IDENTITY, RestTransform(Transform::IDENTITY)))
            .id()
    }

    #[test]
    fn avatar_bind_error_reports_bone_name() {
        let err = AvatarBindError::MissingRequiredBone { bone: "head" };
        assert_eq!(err.bone_name(), Some("head"));
        assert!(err.to_string().contains("head"));

        assert_eq!(AvatarBindError::Timeout.bone_name(), None);
    }

    #[test]
    fn avatar_binding_head_only_constructor() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let head = bone_entity(&mut world);

        let binding = AvatarBinding::head_only(root, head);
        assert_eq!(binding.root, root);
        assert_eq!(binding.head, head);
        assert!(binding.neck.is_none());
        assert!(binding.left_eye.is_none());
        assert!(binding.right_eye.is_none());
    }
}
