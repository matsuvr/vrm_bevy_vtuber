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
pub mod breathing;
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
    ARM_POSE_PROFILE_OVERRIDE_VERSION, ArmChainBinding, ArmChainCapabilities, ArmChainReferences,
    ArmIkError, ArmIkInput, ArmIkSolution, ArmIkTarget, ArmPoseProfile, ArmPoseProfileOverride,
    ArmPoseProfileOverrideError, ArmRestGeometry, ArmSide, FingerJointReferences,
    FingerJointRestBinding, FingerJointRestReferences, FingerReferences, FingerRestReferences,
    RestSpaceBonePose, default_arm_target, solve_two_bone_arm,
};
pub use arm_pose::{
    ArmPoseBlendSide, ArmPoseBlendState, ArmPoseOverrideStore, ArmPoseOverrideStoreError,
    ArmPoseProfileChange, DEFAULT_ARM_RETURN_SECONDS, DEFAULT_ARM_TRANSITION_SECONDS,
    DefaultArmPose, ResolvedArmPose, ResolvedBoneDelta, ResolvedFingerJointPose,
    ResolvedFingerPose, apply_arm_pose_profile_changes, apply_default_arm_pose,
};
pub use bevy_vrm1::prelude::{
    LegacyShaderKind, Vrm0MetaDiagnostics, VrmCompatibilityWarning, VrmCompatibilityWarningCode,
    VrmRuntimeDescriptor, classify_legacy_shader, collect_legacy_compatibility_warnings,
};
pub use bind::BindTriggered;
pub use binding::{AvatarBindError, AvatarBinding, bind_humanoid_bones};
pub use breathing::{
    BreathingBinding, BreathingProfile, BreathingProfileError, BreathingState,
    DEFAULT_BREATHING_PERIOD_SECONDS, DEFAULT_FORWARD_HEIGHT_FACTOR,
    DEFAULT_VERTICAL_HEIGHT_FACTOR, FORWARD_AMPLITUDE_MAX_METERS, FORWARD_AMPLITUDE_MIN_METERS,
    VERTICAL_AMPLITUDE_MAX_METERS, VERTICAL_AMPLITUDE_MIN_METERS, apply_breathing_hips_translation,
    breathing_envelope, breathing_phase, resolve_breathing_amplitudes, resolve_breathing_binding,
};
pub use capabilities::{
    AvatarCapabilities, BlinkMode, BonePresence, DeclaredLookAtType, EmotionSet,
    ExpressionCapabilities, GazeFallbackReason, LookDirectionSet, MouthMode, SelectedGazeBackend,
    select_gaze_backend,
};
pub use framing::camera_control::geometry as camera_control_geometry;
pub use framing::camera_control::{
    AvatarCameraControl, AvatarCameraControlState, CameraControlConfig, CameraControlGeometryError,
    CameraControlPose, CameraDistanceLimits, CameraPointerInputGate, FIXED_VERTICAL_FOV,
};
pub use framing::camera_input::{CameraInputSet, CameraPointerGesture, normalized_vertical_scroll};
pub use framing::camera_reset::ResetCameraRequest;
pub use lifecycle::*;
pub use load::{
    AssetPathError, AvatarAssetId, ExpectedVrmGeneration, ImportedAvatar, LoadImportedAvatarError,
    LoadImportedAvatarRequest, LoadImportedAvatarResult, PendingAvatarLoad, UserAssetPath,
};
pub use mirror::AvatarMotionMirror;
pub use plugin::{StartupModelPath, VtuberAvatarPlugin};
pub use pose::PoseApplyMetrics;
pub use unload::{
    ActiveControlFrame, ControlFrameError, set_active_control_frame, tag_control_frame,
};
