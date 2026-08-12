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
use crate::capabilities::{
    AvatarCapabilities, BonePresence, DeclaredLookAtType, ExpressionCapabilities,
    SelectedGazeBackend,
};
use crate::gaze::fallback_look_at_properties;
use crate::lifecycle::{
    ActiveAvatar, AvatarGeneration, AvatarLifecycle, AvatarLifecycleFailure, AvatarLifecycleState,
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
    /// Generation of the avatar instance this binding belongs to.
    pub generation: AvatarGeneration,
}

impl AvatarBinding {
    /// Creates a head-only binding for the simplest capable avatar.
    #[must_use]
    pub fn head_only(root: Entity, head: Entity, generation: AvatarGeneration) -> Self {
        Self {
            root,
            head,
            neck: None,
            upper_chest: None,
            chest: None,
            spine: None,
            left_eye: None,
            right_eye: None,
            generation,
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
    /// A bone has not received its canonical rest-global transform yet.
    MissingGlobalTransform {
        /// Bone name.
        bone: &'static str,
        /// Entity missing [`RestGlobalTransform`].
        entity: Entity,
    },
    /// A bone rest transform cannot define a stable orientation.
    InvalidRestOrientation {
        /// Bone name.
        bone: &'static str,
        /// Entity with the invalid rest orientation.
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
            | Self::MissingRestTransform { bone, .. }
            | Self::MissingGlobalTransform { bone, .. }
            | Self::InvalidRestOrientation { bone, .. } => Some(*bone),
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
            Self::MissingGlobalTransform { bone, entity } => {
                write!(
                    f,
                    "humanoid bone `{bone}` entity {entity:?} has no RestGlobalTransform"
                )
            }
            Self::InvalidRestOrientation { bone, entity } => {
                write!(
                    f,
                    "humanoid bone `{bone}` entity {entity:?} has an invalid rest orientation"
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
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
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
            Option<&LookAtProperties>,
        ),
        (With<ActiveAvatar>, With<BindTriggered>),
    >,
    bone_query: Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
    deadlines: Query<&BindingDeadline>,
    expression_maps: Query<Option<&ExpressionEntityMap>>,
    spring_roots: Query<Entity, With<SpringRoot>>,
    parents: Query<&ChildOf>,
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

    let (
        root_entity,
        head,
        neck,
        upper_chest,
        chest,
        spine,
        left_eye,
        right_eye,
        look_at_properties,
    ) = root_data;

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
        Ok(mut binding) => {
            // Stamp the binding with the generation assigned when this avatar
            // instance was accepted. Frames targeting any other generation are
            // rejected as stale.
            binding.generation = lifecycle.current_generation();

            let expression_map = expression_maps.get(root_entity).ok().flatten();
            let expression_caps = ExpressionCapabilities::from_map(expression_map);
            let has_spring_bone = spring_roots
                .iter()
                .any(|entity| is_descendant(entity, root_entity, &parents));
            let bones = BonePresence {
                head: true,
                neck: binding.neck.is_some(),
                left_eye: binding.left_eye.is_some(),
                right_eye: binding.right_eye.is_some(),
                upper_chest: binding.upper_chest.is_some(),
                chest: binding.chest.is_some(),
                spine: binding.spine.is_some(),
            };
            let declared_look_at =
                look_at_properties.map_or(DeclaredLookAtType::Missing, |value| {
                    match value.r#type {
                        LookAtType::Bone => DeclaredLookAtType::Bone,
                        LookAtType::Expression => DeclaredLookAtType::Expression,
                    }
                });
            let capabilities = AvatarCapabilities::from_model_capabilities(
                bones,
                &expression_caps,
                has_spring_bone,
                declared_look_at,
            );

            commands.entity(root_entity).insert((
                binding,
                BodyTracking::default(),
                BodyTrackingPoseInput::default(),
                BodyTrackingProfile::default(),
                Visibility::Inherited,
            ));
            if capabilities.gaze_backend != SelectedGazeBackend::None {
                let mut effective_properties = look_at_properties
                    .cloned()
                    .unwrap_or_else(|| fallback_look_at_properties(capabilities.gaze_backend));
                effective_properties.r#type = match capabilities.gaze_backend {
                    SelectedGazeBackend::Bone => LookAtType::Bone,
                    SelectedGazeBackend::Expression => LookAtType::Expression,
                    SelectedGazeBackend::None => effective_properties.r#type,
                };
                commands
                    .entity(root_entity)
                    .insert((effective_properties, DirectLookAtInput::default()));
            }
            commands.entity(root_entity).remove::<BindingDeadline>();
            lifecycle.set_capabilities(Some(capabilities));
            lifecycle.finish_ready();
        }
        Err(error) => {
            // A missing head is permanent, so fail immediately. Everything else
            // is retried until the deadline.
            let permanent = error.is_permanent();
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
        AvatarBindError::Timeout | AvatarBindError::MissingGlobalTransform { .. } => {
            AvatarLifecycleFailure::BindingTimeout
        }
        AvatarBindError::InvalidRestOrientation { bone, .. } => {
            AvatarLifecycleFailure::InvalidRestOrientation { bone }
        }
        _ => AvatarLifecycleFailure::MissingRequiredBone {
            bone: error.bone_name().unwrap_or("head"),
        },
    };
    lifecycle.fail(failure);
}

impl AvatarBindError {
    fn is_permanent(&self) -> bool {
        matches!(
            self,
            Self::MissingRequiredBone { bone: "head" }
                | Self::BoneEntityDespawned { bone: "head", .. }
                | Self::MissingTransform { bone: "head", .. }
                | Self::MissingRestTransform { bone: "head", .. }
                | Self::InvalidRestOrientation { .. }
        )
    }
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
    bone_query: &Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
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
        // Stamped with the lifecycle generation once binding succeeds.
        generation: AvatarGeneration::default(),
    })
}

fn resolve_required_bone(
    name: &'static str,
    entity: Option<Entity>,
    bone_query: &Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
) -> Result<Entity, AvatarBindError> {
    let entity = entity.ok_or(AvatarBindError::MissingRequiredBone { bone: name })?;
    let (transform, rest, rest_global) = bone_query
        .get(entity)
        .map_err(|_| AvatarBindError::BoneEntityDespawned { bone: name, entity })?;

    if transform.is_none() {
        return Err(AvatarBindError::MissingTransform { bone: name, entity });
    }
    if rest.is_none() {
        return Err(AvatarBindError::MissingRestTransform { bone: name, entity });
    }
    if rest_global.is_none() {
        return Err(AvatarBindError::MissingGlobalTransform { bone: name, entity });
    }

    Ok(entity)
}

fn resolve_optional_bone(
    name: &'static str,
    entity: Option<Entity>,
    bone_query: &Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
) -> Option<Entity> {
    let entity = entity?;
    let (transform, rest, rest_global) = bone_query.get(entity).ok()?;

    if transform.is_none() || rest.is_none() || rest_global.is_none() {
        warn!(
            "optional humanoid bone `{name}` entity {entity:?} is missing Transform, RestTransform, or RestGlobalTransform; treating as absent"
        );
        return None;
    }

    Some(entity)
}

/// Returns `true` if `entity` is a descendant of `ancestor` by walking parent
/// links. Returns `false` if the entity has no parent or the query fails.
fn is_descendant(entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        let parent_entity = parent.parent();
        if parent_entity == ancestor {
            return true;
        }
        current = parent_entity;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone_entity(world: &mut World) -> Entity {
        world
            .spawn((
                Transform::IDENTITY,
                RestTransform(Transform::IDENTITY),
                RestGlobalTransform(GlobalTransform::IDENTITY),
            ))
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

        let generation = AvatarGeneration(7);
        let binding = AvatarBinding::head_only(root, head, generation);
        assert_eq!(binding.root, root);
        assert_eq!(binding.head, head);
        assert_eq!(binding.generation, generation);
        assert!(binding.neck.is_none());
        assert!(binding.left_eye.is_none());
        assert!(binding.right_eye.is_none());
    }
}
