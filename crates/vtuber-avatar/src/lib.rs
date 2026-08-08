//! `vtuber-avatar`: Bevy and `bevy_vrm1` adapter.
//!
//! This is the only crate that interacts with Bevy entities and `bevy_vrm1` APIs.
//! `bevy_vrm1` types must not leak into `vtuber-core` or `vtuber-tracking`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod compatibility;
pub mod lifecycle;
pub mod placeholder;
pub mod plugin;

pub use lifecycle::*;
pub use plugin::{StartupModelPath, VtuberAvatarPlugin};
