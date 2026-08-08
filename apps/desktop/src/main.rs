//! Desktop entry point for vrm-bevy-vtuber.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::path::PathBuf;

use bevy::asset::io::{AssetSourceBuilder, AssetSourceBuilders};
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use vtuber_app::import;
use vtuber_app::orchestrator::Orchestrator;
use vtuber_app::ui::UiShellPlugin;
use vtuber_avatar::{
    AvatarAssetId, ImportedAvatar, LoadImportedAvatarRequest, StartupModelPath, UserAssetPath,
    VtuberAvatarPlugin,
};

fn main() {
    let managed_root = managed_asset_root();
    std::fs::create_dir_all(&managed_root).ok();

    // Import CLI model through the managed asset source so that the same
    // `user://avatars/<sha256>/model.vrm` path invariant is used.
    let startup_model = parse_model_arg().and_then(|path| {
        match import::import_vrm(&path, &managed_root, import::DEFAULT_SIZE_LIMIT) {
            Ok(model) => {
                let id = AvatarAssetId::new(&model.id);
                match UserAssetPath::avatar_model_path(&id) {
                    Ok(asset_path) => Some(ImportedAvatar::new(id, asset_path, model.name.clone())),
                    Err(e) => {
                        eprintln!("Failed to construct user asset path for CLI model: {e}");
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to import CLI model: {e}");
                None
            }
        }
    });

    // Register the `user` asset source BEFORE DefaultPlugins so that
    // `user://avatars/<sha256>/model.vrm` resolves to
    // `<managed_root>/avatars/<sha256>/model.vrm`.
    let mut sources = AssetSourceBuilders::default();
    sources.insert(
        "user",
        AssetSourceBuilder::platform_default(
            managed_root.to_str().expect("managed root is valid UTF-8"),
            None,
        ),
    );

    let mut app = App::new();
    app.insert_resource(sources)
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(VtuberAvatarPlugin)
        .add_plugins(UiShellPlugin)
        .insert_resource(Orchestrator::new(managed_root));

    if let Some(imported) = startup_model {
        app.insert_resource(StartupModelPath(Some(imported.id.0.clone())));
        app.insert_resource(StartupImportedAvatar(imported));
    }

    app.add_systems(Startup, submit_startup_model_request).run();
}

/// Returns the application-managed asset root directory.
///
/// On Windows this is `%APPDATA%\vrm-bevy-vtuber`. Falls back to
/// `.vtuber` in the current directory when the platform directories
/// crate cannot determine a suitable location.
fn managed_asset_root() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "vrm-bevy-vtuber") {
        proj_dirs.data_dir().to_path_buf()
    } else {
        PathBuf::from(".vtuber")
    }
}

/// Parses the optional `--model <path>` command-line argument.
///
/// Accepts both absolute paths and paths relative to the workspace root.
/// When omitted, no default model is loaded.
fn parse_model_arg() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--model" {
            return args.next();
        }
    }
    None
}

/// Resource holding the imported avatar from the CLI `--model` argument.
///
/// The startup system reads this and emits a [`LoadImportedAvatarRequest`]
/// so the model is loaded through the avatar lifecycle.
#[derive(Resource, Debug)]
struct StartupImportedAvatar(ImportedAvatar);

/// Startup system that submits a CLI-imported model to the avatar lifecycle.
///
/// Reads the [`StartupImportedAvatar`] resource (if present) and writes a
/// [`LoadImportedAvatarRequest`] message that the avatar plugin consumes.
fn submit_startup_model_request(
    startup: Option<Res<StartupImportedAvatar>>,
    mut load_requests: MessageWriter<LoadImportedAvatarRequest>,
) {
    let Some(imported) = startup else { return };
    load_requests.write(LoadImportedAvatarRequest {
        request_id: 0,
        imported: imported.0.clone(),
    });
}
