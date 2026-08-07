//! `vtuber-camera`: OS camera backends and capture worker.
//!
//! Native camera objects are constructed, opened, used, stopped, and dropped
//! inside the capture worker. Backend buffers and OS handles are never exposed.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Production capture service.
pub mod capture;
/// Camera device and format domain types.
pub mod device;
/// Format negotiation logic.
pub mod format;
/// Mock backend for tests.
pub mod mock;
/// Placeholder for camera subsystem.
pub mod placeholder;

pub use capture::{CaptureController, CaptureMetrics, CaptureServiceState};
pub use device::{CameraDescriptor, CameraError, CameraFormat, CameraRequest};
pub use format::select_format;
