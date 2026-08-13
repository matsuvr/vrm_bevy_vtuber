//! `vtuber-avatar`: Bevy and `bevy_vrm1` adapter.
//!
//! This is the only crate that interacts with Bevy entities and `bevy_vrm1` APIs.
//! `bevy_vrm1` types must not leak into `vtuber-core` or `vtuber-tracking`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod arm;
pub mod arm_pose;
pub mod bind;
pub mod binding;
pub mod capabilities;
pub mod compatibility;
pub mod expression;
mod framing;
pub mod gaze;
pub mod lifecycle;
pub mod load;
pub mod mirror;
pub mod placeholder;
pub mod plugin;
pub mod pose;
pub mod unload;

pub use arm::{
    ArmChainBinding, ArmChainCapabilities, ArmChainReferences, ArmIkError, ArmIkInput,
    ArmIkSolution, ArmIkTarget, ArmPoseProfile, ArmRestGeometry, ArmSide, FingerJointReferences,
    FingerJointRestBinding, FingerJointRestReferences, FingerReferences, FingerRestReferences,
    RestSpaceBonePose, default_arm_target, solve_two_bone_arm,
};
pub use arm_pose::{
    DefaultArmPose, ResolvedArmPose, ResolvedBoneDelta, ResolvedFingerJointPose,
    ResolvedFingerPose, apply_default_arm_pose,
};
pub use bind::BindTriggered;
pub use binding::{AvatarBindError, AvatarBinding, bind_humanoid_bones};
pub use capabilities::{
    AvatarCapabilities, BlinkMode, BonePresence, DeclaredLookAtType, EmotionSet,
    ExpressionCapabilities, GazeFallbackReason, LookDirectionSet, MouthMode, SelectedGazeBackend,
    select_gaze_backend,
};
pub use lifecycle::*;
pub use load::{
    AssetPathError, AvatarAssetId, ImportedAvatar, LoadImportedAvatarError,
    LoadImportedAvatarRequest, LoadImportedAvatarResult, PendingAvatarLoad, UserAssetPath,
};
pub use mirror::AvatarMotionMirror;
pub use plugin::{StartupModelPath, VtuberAvatarPlugin};
pub use pose::PoseApplyMetrics;
pub use unload::{
    ActiveControlFrame, ControlFrameError, set_active_control_frame, tag_control_frame,
};
