//! Semantic pose adapter.
//!
//! Converts [`HeadPose`] (semantic yaw/pitch/roll in radians from tracking)
//! into VRM model-space delta quaternions for head/neck bone application.
//!
//! All math is isolated from Bevy systems so it can be unit-tested without
//! an ECS world. The sign convention and Euler order follow ADR-004.

pub mod binding;
pub mod distribution;
pub mod math;
pub mod system;
pub mod types;

pub use binding::{RestOrientationCache, RestOrientationError, build_rest_orientation_cache};
pub use distribution::{
    DistributedPose, DistributionDiagnostic, HeadNeckWeights, PoseClampSettings,
    PoseDistributionSettings, apply_distributed_pose, distribute_pose,
};
pub use math::{
    apply_model_delta_to_bone, bone_rest_model_rotation, compute_output_rotation,
    model_delta_to_local_delta,
};
pub use system::{
    PoseApplyMetrics, apply_tracked_head_pose, reset_pose_metrics_on_lifecycle_change,
};
pub use types::{
    ClampedHeadPose, ModelSpaceDelta, NonFiniteInputError, RawAxisDeltas, clamp_head_pose,
    raw_axis_deltas, semantic_to_model_delta, semantic_to_model_delta_explicit, validate_head_pose,
};
