//! Expression command building and application.
//!
//! This module handles the conversion from tracking data (blink, mouth, gaze)
//! into VRM expression weight commands. The builder is pure and testable
//! without Bevy ECS.

pub mod command;

pub use command::{ExpressionCommand, ExpressionCommandBuilder, ExpressionCommandMetrics};
