//! Humanoid bone binding for initialized VRM roots.
//!
//! Once the lifecycle enters [`Binding`](crate::lifecycle::AvatarLifecycleState::Binding),
//! this module resolves the required and optional humanoid bone entities that
//! `bevy_vrm1` inserts on the VRM root, validates their components, and caches
//! the result in an [`AvatarBinding`] component. Systems that drive the avatar
//! read that cache instead of re-querying root components every frame.

use bevy::ecs::world::EntityRef;
use bevy::log::warn;
use bevy::prelude::*;
use bevy_vrm1::prelude::*;
use std::time::{Duration, Instant};

use crate::arm::{
    ArmChainBinding, ArmChainReferences, ArmRestGeometry, ArmSide, FingerJointReferences,
    FingerJointRestBinding, FingerJointRestReferences, FingerReferences, FingerRestReferences,
    RestSpaceBonePose,
};
use crate::arm_pose::DefaultArmPose;
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
#[derive(Component, Debug, Clone, Copy, PartialEq)]
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
    /// Optional left upper-arm bone entity.
    pub left_upper_arm: Option<Entity>,
    /// Optional right upper-arm bone entity.
    pub right_upper_arm: Option<Entity>,
    /// Validated left arm chain and immutable rest-space geometry.
    pub left_arm: Option<ArmChainBinding>,
    /// Validated right arm chain and immutable rest-space geometry.
    pub right_arm: Option<ArmChainBinding>,
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
            left_upper_arm: None,
            right_upper_arm: None,
            left_arm: None,
            right_arm: None,
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
        EntityRef,
        (
            With<ActiveAvatar>,
            With<BindTriggered>,
            Without<AvatarLifecycle>,
        ),
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

    let root_ref = match roots.get(root) {
        Ok(root_ref) => root_ref,
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

    let root_entity = root_ref.id();
    let head = entity_reference::<HeadBoneEntity>(&root_ref);
    let neck = entity_reference::<NeckBoneEntity>(&root_ref);
    let upper_chest = entity_reference::<UpperChestBoneEntity>(&root_ref);
    let chest = entity_reference::<ChestBoneEntity>(&root_ref);
    let spine = entity_reference::<SpineBoneEntity>(&root_ref);
    let left_arm = ArmChainReferences {
        shoulder: entity_reference::<LeftShoulderBoneEntity>(&root_ref),
        upper_arm: entity_reference::<LeftUpperArmBoneEntity>(&root_ref),
        lower_arm: entity_reference::<LeftLowerArmBoneEntity>(&root_ref),
        hand: entity_reference::<LeftHandBoneEntity>(&root_ref),
        fingers: left_finger_references(&root_ref),
    };
    let right_arm = ArmChainReferences {
        shoulder: entity_reference::<RightShoulderBoneEntity>(&root_ref),
        upper_arm: entity_reference::<RightUpperArmBoneEntity>(&root_ref),
        lower_arm: entity_reference::<RightLowerArmBoneEntity>(&root_ref),
        hand: entity_reference::<RightHandBoneEntity>(&root_ref),
        fingers: right_finger_references(&root_ref),
    };
    let left_eye = entity_reference::<LeftEyeBoneEntity>(&root_ref);
    let right_eye = entity_reference::<RightEyeBoneEntity>(&root_ref);
    let look_at_properties = root_ref.get::<LookAtProperties>();

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
        left_arm,
        right_arm,
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
            let default_arm_pose = DefaultArmPose::from_chains(
                binding.generation,
                binding.left_arm,
                binding.right_arm,
            );

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
            if let Some(reason) = capabilities.gaze_fallback {
                warn!(
                    "VRM gaze backend fallback selected: declared={:?}, selected={:?}, reason={reason:?}",
                    capabilities.declared_look_at, capabilities.gaze_backend
                );
            }

            commands.entity(root_entity).insert((
                binding,
                default_arm_pose,
                BodyTracking::default(),
                BodyTrackingPoseInput::default(),
                BodyTrackingProfile::default(),
                Visibility::Inherited,
            ));
            if capabilities.gaze_backend != SelectedGazeBackend::None {
                let effective_properties =
                    effective_look_at_properties(look_at_properties, capabilities.gaze_backend);
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

fn effective_look_at_properties(
    source: Option<&LookAtProperties>,
    selected: SelectedGazeBackend,
) -> LookAtProperties {
    let fallback = fallback_look_at_properties(selected);
    let Some(source) = source else {
        return fallback;
    };
    let selected_type = match selected {
        SelectedGazeBackend::Bone => LookAtType::Bone,
        SelectedGazeBackend::Expression => LookAtType::Expression,
        SelectedGazeBackend::None => source.r#type,
    };
    if source.r#type == selected_type {
        return source.clone();
    }

    let mut converted = source.clone();
    converted.r#type = selected_type;
    converted.range_map_horizontal_inner.output_scale =
        fallback.range_map_horizontal_inner.output_scale;
    converted.range_map_horizontal_outer.output_scale =
        fallback.range_map_horizontal_outer.output_scale;
    converted.range_map_vertical_down.output_scale = fallback.range_map_vertical_down.output_scale;
    converted.range_map_vertical_up.output_scale = fallback.range_map_vertical_up.output_scale;
    converted
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
    head: Option<Entity>,
    neck: Option<Entity>,
    upper_chest: Option<Entity>,
    chest: Option<Entity>,
    spine: Option<Entity>,
    left_arm: ArmChainReferences,
    right_arm: ArmChainReferences,
    left_eye: Option<Entity>,
    right_eye: Option<Entity>,
    bone_query: &Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
) -> Result<AvatarBinding, AvatarBindError> {
    let head = resolve_required_bone("head", head, bone_query)?;
    let neck = resolve_optional_bone("neck", neck, bone_query);
    let upper_chest = resolve_optional_bone("upperChest", upper_chest, bone_query);
    let chest = resolve_optional_bone("chest", chest, bone_query);
    let spine = resolve_optional_bone("spine", spine, bone_query);
    let left_upper_arm = resolve_optional_bone("leftUpperArm", left_arm.upper_arm, bone_query);
    let right_upper_arm = resolve_optional_bone("rightUpperArm", right_arm.upper_arm, bone_query);
    let left_arm = resolve_arm_chain(ArmSide::Left, left_arm, bone_query);
    let right_arm = resolve_arm_chain(ArmSide::Right, right_arm, bone_query);
    let left_eye = resolve_optional_bone("leftEye", left_eye, bone_query);
    let right_eye = resolve_optional_bone("rightEye", right_eye, bone_query);

    Ok(AvatarBinding {
        root,
        head,
        neck,
        upper_chest,
        chest,
        spine,
        left_upper_arm,
        right_upper_arm,
        left_arm,
        right_arm,
        left_eye,
        right_eye,
        // Stamped with the lifecycle generation once binding succeeds.
        generation: AvatarGeneration::default(),
    })
}

fn entity_reference<T>(root: &EntityRef<'_>) -> Option<Entity>
where
    T: Component + std::ops::Deref<Target = Entity>,
{
    root.get::<T>().map(|reference| **reference)
}

fn left_finger_references(root: &EntityRef<'_>) -> FingerReferences {
    FingerReferences {
        thumb: FingerJointReferences {
            metacarpal: entity_reference::<LeftThumbMetacarpalBoneEntity>(root),
            proximal: entity_reference::<LeftThumbProximalBoneEntity>(root),
            intermediate: None,
            distal: entity_reference::<LeftThumbDistalBoneEntity>(root),
        },
        index: FingerJointReferences {
            proximal: entity_reference::<LeftIndexProximalBoneEntity>(root),
            intermediate: entity_reference::<LeftIndexIntermediateBoneEntity>(root),
            distal: entity_reference::<LeftIndexDistalBoneEntity>(root),
            ..default()
        },
        middle: FingerJointReferences {
            proximal: entity_reference::<LeftMiddleProximalBoneEntity>(root),
            intermediate: entity_reference::<LeftMiddleIntermediateBoneEntity>(root),
            distal: entity_reference::<LeftMiddleDistalBoneEntity>(root),
            ..default()
        },
        ring: FingerJointReferences {
            proximal: entity_reference::<LeftRingProximalBoneEntity>(root),
            intermediate: entity_reference::<LeftRingIntermediateBoneEntity>(root),
            distal: entity_reference::<LeftRingDistalBoneEntity>(root),
            ..default()
        },
        little: FingerJointReferences {
            proximal: entity_reference::<LeftLittleProximalBoneEntity>(root),
            intermediate: entity_reference::<LeftLittleIntermediateBoneEntity>(root),
            distal: entity_reference::<LeftLittleDistalBoneEntity>(root),
            ..default()
        },
    }
}

fn right_finger_references(root: &EntityRef<'_>) -> FingerReferences {
    FingerReferences {
        thumb: FingerJointReferences {
            metacarpal: entity_reference::<RightThumbMetacarpalBoneEntity>(root),
            proximal: entity_reference::<RightThumbProximalBoneEntity>(root),
            intermediate: None,
            distal: entity_reference::<RightThumbDistalBoneEntity>(root),
        },
        index: FingerJointReferences {
            proximal: entity_reference::<RightIndexProximalBoneEntity>(root),
            intermediate: entity_reference::<RightIndexIntermediateBoneEntity>(root),
            distal: entity_reference::<RightIndexDistalBoneEntity>(root),
            ..default()
        },
        middle: FingerJointReferences {
            proximal: entity_reference::<RightMiddleProximalBoneEntity>(root),
            intermediate: entity_reference::<RightMiddleIntermediateBoneEntity>(root),
            distal: entity_reference::<RightMiddleDistalBoneEntity>(root),
            ..default()
        },
        ring: FingerJointReferences {
            proximal: entity_reference::<RightRingProximalBoneEntity>(root),
            intermediate: entity_reference::<RightRingIntermediateBoneEntity>(root),
            distal: entity_reference::<RightRingDistalBoneEntity>(root),
            ..default()
        },
        little: FingerJointReferences {
            proximal: entity_reference::<RightLittleProximalBoneEntity>(root),
            intermediate: entity_reference::<RightLittleIntermediateBoneEntity>(root),
            distal: entity_reference::<RightLittleDistalBoneEntity>(root),
            ..default()
        },
    }
}

fn resolve_arm_chain(
    side: ArmSide,
    references: ArmChainReferences,
    bone_query: &Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
) -> Option<ArmChainBinding> {
    let upper_arm = resolve_optional_bone("upperArm", references.upper_arm, bone_query);
    let lower_arm = resolve_optional_bone("lowerArm", references.lower_arm, bone_query);
    let hand = resolve_optional_bone("hand", references.hand, bone_query);
    let (Some(upper_arm), Some(lower_arm), Some(hand)) = (upper_arm, lower_arm, hand) else {
        return None;
    };

    let shoulder = resolve_optional_bone("shoulder", references.shoulder, bone_query)
        .and_then(|entity| rest_space_pose(entity, bone_query).map(|_| entity));
    let rest = ArmRestGeometry {
        shoulder: shoulder.and_then(|entity| rest_space_pose(entity, bone_query)),
        upper_arm: rest_space_pose(upper_arm, bone_query)?,
        elbow: rest_space_pose(lower_arm, bone_query)?,
        wrist: rest_space_pose(hand, bone_query)?,
        upper_arm_length: 0.0,
        forearm_length: 0.0,
        total_arm_length: 0.0,
    };
    let upper_arm_length = rest.upper_arm.position.distance(rest.elbow.position);
    let forearm_length = rest.elbow.position.distance(rest.wrist.position);
    let total_arm_length = upper_arm_length + forearm_length;
    if !valid_length(upper_arm_length)
        || !valid_length(forearm_length)
        || !valid_length(total_arm_length)
    {
        warn!("{side:?} arm rest geometry is degenerate; enhanced default arm pose is unavailable");
        return None;
    }

    let (fingers, finger_rest) = resolve_finger_references(references.fingers, bone_query);
    let capabilities = crate::arm::ArmChainCapabilities {
        has_shoulder: rest.shoulder.is_some(),
        has_fingers: finger_rest.has_any(),
    };

    Some(ArmChainBinding {
        side,
        shoulder,
        upper_arm,
        lower_arm,
        hand,
        fingers,
        finger_rest,
        rest: ArmRestGeometry {
            upper_arm_length,
            forearm_length,
            total_arm_length,
            ..rest
        },
        capabilities,
    })
}

fn resolve_finger_references(
    references: FingerReferences,
    bone_query: &Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
) -> (FingerReferences, FingerRestReferences) {
    let fingers = FingerReferences {
        thumb: resolve_finger_joints("thumb", references.thumb, bone_query),
        index: resolve_finger_joints("index", references.index, bone_query),
        middle: resolve_finger_joints("middle", references.middle, bone_query),
        ring: resolve_finger_joints("ring", references.ring, bone_query),
        little: resolve_finger_joints("little", references.little, bone_query),
    };
    let rest = FingerRestReferences {
        thumb: resolve_finger_rest_joints(fingers.thumb, bone_query),
        index: resolve_finger_rest_joints(fingers.index, bone_query),
        middle: resolve_finger_rest_joints(fingers.middle, bone_query),
        ring: resolve_finger_rest_joints(fingers.ring, bone_query),
        little: resolve_finger_rest_joints(fingers.little, bone_query),
    };
    (fingers, rest)
}

fn resolve_finger_joints(
    name: &'static str,
    references: FingerJointReferences,
    bone_query: &Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
) -> FingerJointReferences {
    FingerJointReferences {
        metacarpal: resolve_optional_bone(name, references.metacarpal, bone_query),
        proximal: resolve_optional_bone(name, references.proximal, bone_query),
        intermediate: resolve_optional_bone(name, references.intermediate, bone_query),
        distal: resolve_optional_bone(name, references.distal, bone_query),
    }
}

fn resolve_finger_rest_joints(
    references: FingerJointReferences,
    bone_query: &Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
) -> FingerJointRestReferences {
    let rest = |entity: Option<Entity>| {
        entity.and_then(|entity| {
            rest_space_pose(entity, bone_query)
                .map(|pose| FingerJointRestBinding { entity, rest: pose })
        })
    };
    FingerJointRestReferences {
        metacarpal: rest(references.metacarpal),
        proximal: rest(references.proximal),
        intermediate: rest(references.intermediate),
        distal: rest(references.distal),
    }
}

fn rest_space_pose(
    entity: Entity,
    bone_query: &Query<(
        Option<&Transform>,
        Option<&RestTransform>,
        Option<&RestGlobalTransform>,
    )>,
) -> Option<RestSpaceBonePose> {
    let Ok((_, Some(rest), Some(rest_global))) = bone_query.get(entity) else {
        return None;
    };
    let (scale, global_rotation, position) = rest_global.0.to_scale_rotation_translation();
    let local_rotation = rest.0.rotation;
    if !position.is_finite()
        || !scale.is_finite()
        || !local_rotation.is_finite()
        || !global_rotation.is_finite()
        || scale.x.abs() <= f32::EPSILON
        || scale.y.abs() <= f32::EPSILON
        || scale.z.abs() <= f32::EPSILON
        || local_rotation.length_squared() <= f32::EPSILON
        || global_rotation.length_squared() <= f32::EPSILON
    {
        return None;
    }

    Some(RestSpaceBonePose {
        position,
        global_rotation: global_rotation.normalize(),
        local_rotation: local_rotation.normalize(),
    })
}

fn valid_length(length: f32) -> bool {
    length.is_finite() && length > 1.0e-5
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

    fn look_at_properties(r#type: LookAtType, output_scale: f32) -> LookAtProperties {
        LookAtProperties {
            offset_from_head_bone: [0.1, 0.2, 0.3],
            range_map_horizontal_inner: RangeMap {
                input_max_value: 11.0,
                output_scale,
            },
            range_map_horizontal_outer: RangeMap {
                input_max_value: 22.0,
                output_scale,
            },
            range_map_vertical_down: RangeMap {
                input_max_value: 33.0,
                output_scale,
            },
            range_map_vertical_up: RangeMap {
                input_max_value: 44.0,
                output_scale,
            },
            r#type,
        }
    }

    fn assert_all_output_scales(properties: &LookAtProperties, expected: f32) {
        assert_eq!(properties.range_map_horizontal_inner.output_scale, expected);
        assert_eq!(properties.range_map_horizontal_outer.output_scale, expected);
        assert_eq!(properties.range_map_vertical_down.output_scale, expected);
        assert_eq!(properties.range_map_vertical_up.output_scale, expected);
    }

    fn assert_fallback_profile(
        actual: &LookAtProperties,
        expected_type: LookAtType,
        expected_output: f32,
    ) {
        assert_eq!(actual.r#type, expected_type);
        assert_eq!(actual.offset_from_head_bone, [0.0; 3]);
        assert_eq!(actual.range_map_horizontal_inner.input_max_value, 30.0);
        assert_eq!(actual.range_map_horizontal_outer.input_max_value, 30.0);
        assert_eq!(actual.range_map_vertical_down.input_max_value, 30.0);
        assert_eq!(actual.range_map_vertical_up.input_max_value, 30.0);
        assert_all_output_scales(actual, expected_output);
    }

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

    #[test]
    fn expression_metadata_converts_output_units_when_bone_is_selected() {
        let source = look_at_properties(LookAtType::Expression, 1.0);
        let effective = effective_look_at_properties(Some(&source), SelectedGazeBackend::Bone);

        assert_eq!(effective.r#type, LookAtType::Bone);
        assert_all_output_scales(&effective, 10.0);
        assert_eq!(effective.range_map_horizontal_inner.input_max_value, 11.0);
        assert_eq!(effective.range_map_vertical_up.input_max_value, 44.0);
    }

    #[test]
    fn bone_metadata_converts_output_units_when_expression_is_selected() {
        let source = look_at_properties(LookAtType::Bone, 10.0);
        let effective =
            effective_look_at_properties(Some(&source), SelectedGazeBackend::Expression);

        assert_eq!(effective.r#type, LookAtType::Expression);
        assert_all_output_scales(&effective, 1.0);
        assert_eq!(effective.range_map_horizontal_outer.input_max_value, 22.0);
        assert_eq!(effective.range_map_vertical_down.input_max_value, 33.0);
    }

    #[test]
    fn missing_metadata_uses_complete_bone_fallback_profile() {
        let effective = effective_look_at_properties(None, SelectedGazeBackend::Bone);
        assert_fallback_profile(&effective, LookAtType::Bone, 10.0);
    }

    #[test]
    fn missing_metadata_uses_complete_expression_fallback_profile() {
        let effective = effective_look_at_properties(None, SelectedGazeBackend::Expression);
        assert_fallback_profile(&effective, LookAtType::Expression, 1.0);
    }
}
