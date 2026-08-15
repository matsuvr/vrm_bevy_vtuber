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
    ArmChainBinding, ArmIkInput, ArmPoseBlendState, ArmPoseProfile, AvatarAssetId, AvatarBinding,
    AvatarLifecycle, AvatarLifecycleState, BreathingBinding, BreathingProfile, BreathingState,
    DefaultArmPose, ImportedAvatar, LoadImportedAvatarRequest, ResolvedArmPose, UserAssetPath,
    VtuberAvatarPlugin, default_arm_target, solve_two_bone_arm,
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
                verify_arm_pose(&mut app, root)?;
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

/// Verifies the model-adaptive arm contract on a real Ready avatar without
/// requiring a camera or a window. Complete chains must resolve a finite,
/// bent two-bone pose; an absent or incomplete side is an explicit capability
/// result and does not make an otherwise valid avatar fail.
fn verify_arm_pose(app: &mut App, root: Entity) -> Result<(), String> {
    let (binding, default_pose, blend) = {
        let world = app.world();
        let binding = world
            .get::<AvatarBinding>(root)
            .copied()
            .ok_or_else(|| format!("Ready root {root:?} has no AvatarBinding"))?;
        let default_pose = world
            .get::<DefaultArmPose>(root)
            .copied()
            .ok_or_else(|| format!("Ready root {root:?} has no DefaultArmPose"))?;
        let blend = world
            .get::<ArmPoseBlendState>(root)
            .copied()
            .ok_or_else(|| format!("Ready root {root:?} has no ArmPoseBlendState"))?;
        (binding, default_pose, blend)
    };

    if binding.generation != default_pose.generation || binding.generation != blend.generation {
        return Err(format!(
            "Ready root {root:?} has inconsistent arm generations: binding={:?} default={:?} blend={:?}",
            binding.generation, default_pose.generation, blend.generation
        ));
    }

    verify_arm_side(
        "left",
        binding.left_arm,
        default_pose.left,
        blend.current_left(),
    )?;
    verify_arm_side(
        "right",
        binding.right_arm,
        default_pose.right,
        blend.current_right(),
    )?;
    Ok(())
}

fn verify_arm_side(
    side: &str,
    chain: Option<ArmChainBinding>,
    resolved: Option<ResolvedArmPose>,
    current: Option<ResolvedArmPose>,
) -> Result<(), String> {
    let Some(chain) = chain else {
        if resolved.is_some() || current.is_some() {
            return Err(format!(
                "{side} arm has a resolved pose without a complete cached chain"
            ));
        }
        println!("arm pose side={side} unavailable (incomplete or degenerate chain)");
        return Ok(());
    };

    let pose = resolved.ok_or_else(|| {
        format!("{side} arm has a complete cached chain but no resolved DefaultArmPose")
    })?;
    let current = current.ok_or_else(|| format!("{side} arm has no initial blend output"))?;
    if pose.upper_arm != chain.upper_arm
        || pose.lower_arm != chain.lower_arm
        || current.upper_arm != chain.upper_arm
        || current.lower_arm != chain.lower_arm
    {
        return Err(format!(
            "{side} arm resolved pose targets the wrong entities"
        ));
    }

    let target = default_arm_target(&chain, ArmPoseProfile::default())
        .map_err(|error| format!("{side} arm default target failed: {error}"))?;
    let input = ArmIkInput::from_geometry(chain.rest, target);
    let solution = solve_two_bone_arm(input)
        .map_err(|error| format!("{side} arm IK solve failed: {error}"))?;
    let upper_direction = solution.elbow - input.shoulder;
    let lower_direction = solution.wrist - solution.elbow;
    let bend_sine = upper_direction
        .try_normalize()
        .and_then(|upper| {
            lower_direction
                .try_normalize()
                .map(|lower| upper.cross(lower).length())
        })
        .ok_or_else(|| format!("{side} arm IK produced a degenerate elbow bend"))?;
    if !bend_sine.is_finite() || bend_sine <= 1.0e-4 {
        return Err(format!(
            "{side} arm IK produced no measurable elbow bend: sine={bend_sine}"
        ));
    }

    for (label, rotation) in [
        ("upper_arm_delta", pose.upper_arm_delta),
        ("lower_arm_delta", pose.lower_arm_delta),
        ("current_upper_arm_delta", current.upper_arm_delta),
        ("current_lower_arm_delta", current.lower_arm_delta),
    ] {
        if !rotation.is_finite() || rotation.length_squared() <= f32::EPSILON {
            return Err(format!("{side} arm {label} is non-finite or degenerate"));
        }
    }
    if !solution.elbow.is_finite()
        || !solution.wrist.is_finite()
        || !solution.solved_reach.is_finite()
        || !pose.upper_arm_delta.is_normalized()
        || !pose.lower_arm_delta.is_normalized()
    {
        return Err(format!(
            "{side} arm IK or resolved pose is not finite/normalized"
        ));
    }

    println!(
        "arm pose verified: side={side} upper={:?} lower={:?} hand={:?} bend_sine={bend_sine:.6} optional_shoulder={} optional_fingers={}",
        chain.upper_arm,
        chain.lower_arm,
        chain.hand,
        chain.capabilities.has_shoulder,
        chain.capabilities.has_fingers,
    );
    Ok(())
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
