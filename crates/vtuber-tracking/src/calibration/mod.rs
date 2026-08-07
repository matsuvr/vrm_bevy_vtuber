//! Calibration: neutral reference collection and session state.
//!
//! This module is responsible for turning raw face observations into a
//! validated neutral profile, and for tracking the lifecycle of a calibration
//! session. It must not depend on Bevy or camera backends.

pub mod collector;
pub mod neutral;
pub mod types;

pub use collector::{CalibrationCollector, CollectorMetrics, RejectionReason, SampleDecision};
pub use neutral::{NeutralContext, NeutralReference, NeutralValidationSettings};
pub use types::{CalibrationInput, CalibrationSession, NeutralProfile};
