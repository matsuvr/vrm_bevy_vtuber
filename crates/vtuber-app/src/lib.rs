//! `vtuber-app`: orchestration, UI, settings, model import, and diagnostics.
//!
//! This crate must not contain model-specific inference math or VRM runtime internals.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// UI action commands emitted by the UI layer.
pub mod actions;

/// Diagnostics snapshot for the UI.
pub mod diagnostics;

/// Error presenter — maps domain errors to user-facing messages.
pub mod error_presenter;

/// VRM 1.0 import and preflight inspection.
pub mod import;

/// App orchestrator — processes UI actions and manages domain state.
pub mod orchestrator;

/// Placeholder for app subsystem.
pub mod placeholder;

/// Camera preview texture pipeline.
pub mod preview;

/// UI rendering module using bevy_egui.
pub mod ui;

/// UI view models — immutable snapshots for rendering the UI.
pub mod ui_model;
