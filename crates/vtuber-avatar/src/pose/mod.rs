//! Semantic pose adapter.
//!
//! Converts [`HeadPose`] (semantic yaw/pitch/roll in radians from tracking)
//! into VRM model-space delta quaternions for head/neck bone application.
//!
//! All math is isolated from Bevy systems so it can be unit-tested without
//! an ECS world. The sign convention and Euler order follow ADR-004.

pub mod binding;
pub mod types;

pub use binding::{RestOrientationCache, RestOrientationError, build_rest_orientation_cache};
pub use types::{
    ClampedHeadPose, ModelSpaceDelta, NonFiniteInputError, RawAxisDeltas, clamp_head_pose,
    raw_axis_deltas, semantic_to_model_delta, semantic_to_model_delta_explicit, validate_head_pose,
};
