//! `vtuber-avatar`: Bevy and `bevy_vrm1` adapter.
//!
//! This is the only crate that interacts with Bevy entities and `bevy_vrm1` APIs.
//! `bevy_vrm1` types must not leak into `vtuber-core` or `vtuber-tracking`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bind;
pub mod binding;
pub mod compatibility;
pub mod lifecycle;
pub mod load;
pub mod placeholder;
pub mod plugin;

pub use bind::BindTriggered;
pub use binding::{AvatarBindError, AvatarBinding, bind_humanoid_bones};
pub use lifecycle::*;
pub use load::{
    AssetPathError, AvatarAssetId, ImportedAvatar, LoadImportedAvatarError,
    LoadImportedAvatarRequest, LoadImportedAvatarResult, PendingAvatarLoad, UserAssetPath,
};
pub use plugin::{StartupModelPath, VtuberAvatarPlugin};
