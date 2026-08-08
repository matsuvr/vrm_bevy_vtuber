//! Gaze mode selection and application.
//!
//! This module handles gaze direction mapping from tracking data to
//! VRM expression presets or eye bone rotations.

pub mod mode;

pub use mode::{GazeModeSelection, select_gaze_mode};
