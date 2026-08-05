//! Desktop entry point for vrm-bevy-vtuber.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use bevy::prelude::*;
use vtuber_avatar::VtuberAvatarPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VtuberAvatarPlugin)
        .run();
}
