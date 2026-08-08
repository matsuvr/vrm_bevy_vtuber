//! `vtuber-core`: platform- and engine-independent data and synchronization contracts.
//!
//! This crate must not depend on Bevy, `bevy_vrm1`, `nokhwa`, tract, or OS-specific APIs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Re-export core types used across worker boundaries.
pub mod types;

/// Control settings and commands for the tracking pipeline.
pub mod control;

/// Fixed-size metrics collection for acceptance testing.
pub mod metrics;

/// Raw observation contract between inference and tracking.
pub mod observation;
/// Latest-value slot for single-producer / single-consumer communication.
pub mod slot;
/// Worker stop token.
pub mod stop;
/// Deterministic worker supervision helpers.
pub mod worker;

pub use control::{CalibrationError, CalibrationSettings};
pub use observation::RawExpressionObservation;
pub use slot::{LatestSlot, ReadResult};
pub use stop::StopToken;
pub use types::*;
pub use worker::{WorkerHandle, WorkerResult};
