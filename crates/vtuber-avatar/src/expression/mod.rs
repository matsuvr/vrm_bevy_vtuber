//! Expression command building and application.
//!
//! This module handles the conversion from tracking data (blink, mouth, gaze)
//! into VRM expression weight commands. The builder is pure and testable
//! without Bevy ECS.

pub mod blink;
pub mod command;
pub mod mouth;

pub use blink::{RawBlinkInput, map_blink_to_expressions, map_blink_with_fallback};
pub use command::{ExpressionCommand, ExpressionCommandBuilder, ExpressionCommandMetrics};
pub use mouth::{
    RawMouthInput, is_valid_mouth_preset, map_mouth_to_expressions, map_mouth_with_fallback,
};
