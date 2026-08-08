//! `vtuber-app`: orchestration, UI, settings, model import, and diagnostics.
//!
//! This crate must not contain model-specific inference math or VRM runtime internals.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// UI action commands emitted by the UI layer.
pub mod actions;

/// VRM 1.0 import and preflight inspection.
pub mod import;

/// Placeholder for app subsystem.
pub mod placeholder;

/// UI view models — immutable snapshots for rendering the UI.
pub mod ui_model;
