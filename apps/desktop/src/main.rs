//! Desktop entry point for vrm-bevy-vtuber.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use bevy::prelude::*;
use vtuber_avatar::{StartupModelPath, VtuberAvatarPlugin};

fn main() {
    let model_path = parse_model_arg();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VtuberAvatarPlugin)
        .insert_resource(StartupModelPath(model_path))
        .run();
}

/// Parses the optional `--model <path>` command-line argument.
///
/// Accepts both absolute paths and paths relative to the workspace root.
/// When omitted, no default model is loaded.
fn parse_model_arg() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--model" {
            return args.next().map(canonicalize_asset_path);
        }
    }
    None
}

/// Converts an arbitrary filesystem path into a path usable by Bevy's
/// `AssetServer` while keeping it inside the approved asset root.
///
/// Bevy 0.19 rejects absolute paths by default and resolves relative paths
/// against `apps/desktop/assets`. To load models outside that directory
/// (e.g. `tests/fixtures/vrm/`), we copy the file into the approved asset
/// root on startup and return the asset-relative path.
fn canonicalize_asset_path(path: String) -> String {
    // If the path is already inside the approved asset root, use it as-is.
    if !std::path::Path::new(&path).is_absolute() {
        return path;
    }

    let workspace_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let assets_dir = workspace_root.join("apps/desktop/assets/models");
    let _ = std::fs::create_dir_all(&assets_dir);

    let source = std::path::Path::new(&path);
    let file_name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "model.vrm".to_string());
    let dest = assets_dir.join(&file_name);

    if let Err(e) = std::fs::copy(source, &dest) {
        eprintln!("Failed to stage model into asset root: {e}");
        return path;
    }

    format!("models/{file_name}")
}
