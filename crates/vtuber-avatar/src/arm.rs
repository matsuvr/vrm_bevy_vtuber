//! Rest-space arm-chain data used by the model-adaptive default pose.
//!
//! This module contains no pose solving or ECS writes. It only defines the
//! immutable references and measurements produced during avatar binding so
//! later pose systems do not need to rediscover the hierarchy every frame.

use bevy::prelude::*;

/// The side of a humanoid arm chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmSide {
    /// The model's left arm.
    Left,
    /// The model's right arm.
    Right,
}

/// Entity references for one finger's authored joints.
///
/// These are references only. Issue #14 deliberately does not apply any
/// finger pose. The optional metacarpal is retained because VRM 1.0 exposes
/// it for the thumb and later relaxation may need the authored joint chain.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FingerJointReferences {
    /// Optional metacarpal joint (normally present for the thumb).
    pub metacarpal: Option<Entity>,
    /// Proximal joint.
    pub proximal: Option<Entity>,
    /// Intermediate joint.
    pub intermediate: Option<Entity>,
    /// Distal joint.
    pub distal: Option<Entity>,
}

impl FingerJointReferences {
    /// Returns whether at least one joint is available.
    #[must_use]
    pub const fn is_present(self) -> bool {
        self.metacarpal.is_some()
            || self.proximal.is_some()
            || self.intermediate.is_some()
            || self.distal.is_some()
    }
}

/// Optional finger references for one arm.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FingerReferences {
    /// Thumb joints.
    pub thumb: FingerJointReferences,
    /// Index-finger joints.
    pub index: FingerJointReferences,
    /// Middle-finger joints.
    pub middle: FingerJointReferences,
    /// Ring-finger joints.
    pub ring: FingerJointReferences,
    /// Little-finger joints.
    pub little: FingerJointReferences,
}

impl FingerReferences {
    /// Returns whether any authored finger joint was resolved.
    #[must_use]
    pub const fn has_any(self) -> bool {
        self.thumb.is_present()
            || self.index.is_present()
            || self.middle.is_present()
            || self.ring.is_present()
            || self.little.is_present()
    }
}

/// Candidate entity references read from the VRM root during binding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArmChainReferences {
    /// Optional shoulder entity.
    pub shoulder: Option<Entity>,
    /// Upper-arm entity.
    pub upper_arm: Option<Entity>,
    /// Lower-arm entity.
    pub lower_arm: Option<Entity>,
    /// Hand entity, used as the wrist target/origin.
    pub hand: Option<Entity>,
    /// Optional authored finger joints.
    pub fingers: FingerReferences,
}

/// Rest-space pose for one bone.
///
/// `position` and `global_rotation` come from the immutable
/// `RestGlobalTransform`. `local_rotation` comes from `RestTransform` and is
/// retained for converting future model-space rotations back into local
/// rest-relative deltas without assuming identity bone rotations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RestSpaceBonePose {
    /// Bone origin in the model/rest global space.
    pub position: Vec3,
    /// Bone orientation in the model/rest global space.
    pub global_rotation: Quat,
    /// Authored local rest orientation.
    pub local_rotation: Quat,
}

/// Immutable rest-space geometry for a complete arm chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmRestGeometry {
    /// Optional shoulder pose.
    pub shoulder: Option<RestSpaceBonePose>,
    /// Upper-arm origin and rest orientation.
    pub upper_arm: RestSpaceBonePose,
    /// Elbow origin and rest orientation (the lower-arm origin).
    pub elbow: RestSpaceBonePose,
    /// Wrist origin and rest orientation (the hand origin).
    pub wrist: RestSpaceBonePose,
    /// Rest distance from upper-arm origin to elbow.
    pub upper_arm_length: f32,
    /// Rest distance from elbow to wrist.
    pub forearm_length: f32,
    /// Sum of the two rest arm lengths.
    pub total_arm_length: f32,
}

/// Optional-feature capabilities of a resolved arm chain.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArmChainCapabilities {
    /// Whether a valid shoulder reference and rest pose were resolved.
    pub has_shoulder: bool,
    /// Whether at least one valid authored finger reference was resolved.
    pub has_fingers: bool,
}

/// A complete, validated arm chain and its immutable rest-space data.
///
/// The chain is present only when upper arm, lower arm, and hand references
/// all have usable rest data. Optional shoulder and finger data remain
/// explicit capabilities rather than making the avatar binding fail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmChainBinding {
    /// Which side this chain belongs to.
    pub side: ArmSide,
    /// Optional shoulder entity.
    pub shoulder: Option<Entity>,
    /// Upper-arm entity.
    pub upper_arm: Entity,
    /// Lower-arm entity.
    pub lower_arm: Entity,
    /// Hand entity.
    pub hand: Entity,
    /// Optional authored finger entities.
    pub fingers: FingerReferences,
    /// Immutable rest-space positions/orientations and lengths.
    pub rest: ArmRestGeometry,
    /// Optional shoulder/finger capability flags.
    pub capabilities: ArmChainCapabilities,
}
