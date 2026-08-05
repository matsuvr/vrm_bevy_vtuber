//! `vtuber-app`: orchestration, UI, settings, model import, and diagnostics.
//!
//! This crate must not contain model-specific inference math or VRM runtime internals.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// VRM 1.0 import and preflight inspection.
pub mod import;

/// Placeholder for app subsystem.
pub mod placeholder;
