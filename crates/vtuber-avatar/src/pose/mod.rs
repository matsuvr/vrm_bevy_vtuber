//! Direct-pose body-tracking bridge.
//!
//! Calibrated semantic yaw/pitch/roll values are forwarded unchanged to the
//! dependency-owned `BodyTrackingPoseInput`. All bone distribution, rest-space
//! conversion, filtering, and additive composition lives in `bevy_vrm1`.

pub mod system;
pub use system::{
    PoseApplyMetrics, reset_pose_metrics_on_lifecycle_change, update_body_tracking_pose_input,
};
