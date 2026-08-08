//! UI rendering module using `bevy_egui`.
//!
//! The UI reads [`crate::ui_model::UiViewModel`] snapshots and emits
//! [`crate::actions::UiAction`] commands. It never accesses domain
//! services directly.

pub mod shell;

pub use shell::{UiShellPlugin, UiState};
