//! Gaze mode selection and application.
//!
//! This module handles gaze direction mapping from tracking data to
//! VRM expression presets or eye bone rotations.

pub mod bone;
pub mod expression;
pub mod mode;
pub mod system;

pub use bone::{
    EyeBoneGazeSettings, MissingEyePolicy, compute_eye_bone_local_rotation,
    compute_eye_bone_rotation, has_both_eyes, resolve_eye_targets,
};
pub use expression::{
    GazeExpressionSettings, RawGazeInput, is_gaze_in_dead_zone, map_gaze_to_expressions,
};
pub use mode::{
    GazeModeSelection, select_gaze_mode, supports_expression_gaze, supports_eye_bone_gaze,
};
pub use system::apply_tracked_eye_gaze;
