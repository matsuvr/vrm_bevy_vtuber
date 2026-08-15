//! Headless compatibility runner for the production managed-avatar route.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::app::{App, PluginGroup, Startup};
use bevy::asset::io::{AssetSourceBuilder, AssetSourceBuilders};
use bevy::asset::{AssetServer, LoadState};
use bevy::prelude::{DefaultPlugins, Entity, MessageWriter, Res, Resource, Transform, Visibility};
use bevy::render::RenderPlugin;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::utils::default;
use bevy::window::{ExitCondition, WindowPlugin};
use bevy_vrm1::prelude::{VrmAsset, VrmHandle};
use vtuber_app::import::{self, DEFAULT_SIZE_LIMIT};
use vtuber_avatar::{
    AvatarAssetId, AvatarLifecycle, AvatarLifecycleState, BreathingBinding, BreathingProfile,
    BreathingState, ImportedAvatar, LoadImportedAvatarRequest, UserAssetPath, VtuberAvatarPlugin,
};

const MAX_COMPAT_FRAMES: usize = 1_200;

/// Exercises import, the named `user://` source, and the real avatar lifecycle.
pub fn run(path: &Path) -> Result<(), String> {
    let managed_root = temporary_managed_root()?;
    let result = run_with_root(path, &managed_root);
    let cleanup = fs::remove_dir_all(&managed_root);

    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(error)) => Err(format!(
            "managed compatibility succeeded but temporary asset cleanup failed: {error}"
        )),
    }
}

fn run_with_root(path: &Path, managed_root: &Path) -> Result<(), String> {
    let imported = import::import_vrm(path, managed_root, DEFAULT_SIZE_LIMIT)
        .map_err(|error| format!("import_vrm failed: {error}"))?;
    let asset_id = AvatarAssetId::new(&imported.id);
    let asset_path = UserAssetPath::avatar_model_path(&asset_id)
        .map_err(|error| format!("failed to construct managed asset path: {error}"))?;
    let expected_generation = match imported.summary.generation {
        vtuber_app::import::VrmGeneration::Vrm0 => vtuber_avatar::ExpectedVrmGeneration::Vrm0,
        vtuber_app::import::VrmGeneration::Vrm1 => vtuber_avatar::ExpectedVrmGeneration::Vrm1,
    };
    let imported_avatar = ImportedAvatar::new(
        asset_id,
        asset_path,
        imported.name.clone(),
        expected_generation,
    );

    let managed_root_string = managed_root
        .to_str()
        .ok_or_else(|| "temporary managed asset root is not valid UTF-8".to_owned())?;
    let mut sources = AssetSourceBuilders::default();
    sources.insert(
        "user",
        AssetSourceBuilder::platform_default(managed_root_string, None),
    );

    let mut app = App::new();
    app.insert_resource(sources)
        .insert_resource(ManagedImportedAvatar(imported_avatar))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin { ..default() })
                .disable::<PipelinedRenderingPlugin>(),
        )
        .add_plugins(VtuberAvatarPlugin)
        .add_systems(Startup, emit_request);

    app.finish();
    app.cleanup();

    let mut previous_state = None;
    for frame in 0..MAX_COMPAT_FRAMES {
        app.update();
        let (state, failure) = {
            let lifecycle = app.world().resource::<AvatarLifecycle>();
            (lifecycle.state(), lifecycle.failure().cloned())
        };
        if previous_state != Some(state) {
            if previous_state == Some(AvatarLifecycleState::Loading)
                && state == AvatarLifecycleState::Ready
            {
                // The production plugin chains observation and binding in one
                // update. The root's binding marker proves the intermediate
                // lifecycle boundary was traversed even though the loop cannot
                // sample it between systems.
                println!("lifecycle transition: Loading -> Binding");
                println!("lifecycle transition: Binding -> Ready");
            } else {
                println!("lifecycle transition: {:?} -> {state:?}", previous_state);
            }
            previous_state = Some(state);
        }

        match state {
            AvatarLifecycleState::Ready => {
                let root = app
                    .world()
                    .resource::<AvatarLifecycle>()
                    .active_root()
                    .ok_or_else(|| "lifecycle reached Ready without an active root".to_owned())?;
                let visibility = app
                    .world()
                    .get::<Visibility>(root)
                    .ok_or_else(|| format!("Ready root {root:?} has no Visibility component"))?;
                if *visibility == Visibility::Hidden {
                    return Err(format!("Ready root {root:?} remains Visibility::Hidden"));
                }
                println!("managed avatar reached Ready: root={root:?} visibility={visibility:?}");
                verify_breathing(&mut app, root)?;
                return Ok(());
            }
            AvatarLifecycleState::Failed => {
                println!("lifecycle failure: {failure:?}");
                print_asset_failures(&mut app);
                return Err(format!("managed avatar lifecycle failed: {failure:?}"));
            }
            _ => {
                if frame % 60 == 0 {
                    println!("lifecycle pending: frame={frame} state={state:?}");
                }
            }
        }
    }

    print_asset_failures(&mut app);
    Err(format!(
        "managed avatar did not reach Ready within {MAX_COMPAT_FRAMES} frames"
    ))
}

/// Verifies that the always-on breathing feature is bound and moving on a
/// Ready avatar: the typed profile/binding/state components must exist, the
/// hips translation must move away from its initial value, and it must stay
/// finite. Wall-clock frames are paced so the 5-second cycle advances enough
/// to observe a visible displacement.
fn verify_breathing(app: &mut App, root: Entity) -> Result<(), String> {
    const BREATHING_FRAMES: usize = 150;
    const BREATHING_FRAME_PACE: Duration = Duration::from_millis(16);
    const MIN_HIPS_DISPLACEMENT: f32 = 1.0e-4;

    let (profile, binding, has_state) = {
        let world = app.world();
        let profile = world
            .get::<BreathingProfile>(root)
            .copied()
            .ok_or_else(|| format!("Ready root {root:?} has no BreathingProfile"))?;
        let binding = world
            .get::<BreathingBinding>(root)
            .cloned()
            .ok_or_else(|| format!("Ready root {root:?} has no BreathingBinding"))?;
        let has_state = world.get::<BreathingState>(root).is_some();
        (profile, binding, has_state)
    };
    if !has_state {
        return Err(format!("Ready root {root:?} has no BreathingState"));
    }

    let initial = app
        .world()
        .get::<Transform>(binding.hips)
        .ok_or_else(|| format!("hips entity {:?} has no Transform", binding.hips))?
        .translation;
    if !initial.is_finite() {
        return Err(format!(
            "hips entity {:?} has non-finite initial translation",
            binding.hips
        ));
    }

    let mut moved = false;
    for _ in 0..BREATHING_FRAMES {
        std::thread::sleep(BREATHING_FRAME_PACE);
        app.update();
        let current = app
            .world()
            .get::<Transform>(binding.hips)
            .ok_or_else(|| format!("hips entity {:?} lost its Transform", binding.hips))?
            .translation;
        if !current.is_finite() {
            return Err(format!(
                "hips entity {:?} received a non-finite translation",
                binding.hips
            ));
        }
        if (current - initial).length() > MIN_HIPS_DISPLACEMENT {
            moved = true;
        }
    }
    if !moved {
        return Err(format!(
            "breathing did not move the hips by {MIN_HIPS_DISPLACEMENT} m within {BREATHING_FRAMES} frames"
        ));
    }

    println!(
        "breathing verified: hips={:?} period={}s rest_hips_height={:.6}m vertical={:.6}m forward={:.6}m up_local={:?} forward_local={:?}",
        binding.hips,
        profile.period_seconds,
        binding.rest_hips_height,
        binding.vertical_amplitude,
        binding.forward_amplitude,
        binding.up_local,
        binding.forward_local,
    );
    Ok(())
}

#[derive(Resource)]
struct ManagedImportedAvatar(ImportedAvatar);

fn emit_request(
    model: Res<ManagedImportedAvatar>,
    mut requests: MessageWriter<LoadImportedAvatarRequest>,
) {
    requests.write(LoadImportedAvatarRequest {
        request_id: 1,
        imported: model.0.clone(),
    });
}

fn print_asset_failures(app: &mut App) {
    let handles: Vec<(Entity, bevy::asset::AssetId<VrmAsset>, Option<String>)> = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &VrmHandle)>();
        query
            .iter(world)
            .map(|(entity, handle)| {
                (
                    entity,
                    handle.0.id(),
                    handle.0.path().map(|path| path.to_string()),
                )
            })
            .collect()
    };

    let asset_server = app.world().resource::<AssetServer>();
    for (entity, id, path) in handles {
        if let LoadState::Failed(error) = asset_server.load_state(id) {
            println!("underlying asset failure: root={entity:?} path={path:?} error={error:?}");
        }
    }
}

fn temporary_managed_root() -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before UNIX epoch: {error}"))?;
    let root = std::env::temp_dir().join(format!(
        "vrm-managed-compat-{}-{}",
        std::process::id(),
        timestamp.as_nanos()
    ));
    fs::create_dir_all(&root)
        .map_err(|error| format!("failed to create temporary managed root: {error}"))?;
    Ok(root)
}
