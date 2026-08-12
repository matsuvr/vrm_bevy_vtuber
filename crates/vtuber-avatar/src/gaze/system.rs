//! Bevy eye-bone gaze fallback.

use bevy::prelude::*;
use bevy_vrm1::prelude::RestTransform;

use crate::binding::AvatarBinding;
use crate::capabilities::GazeMode;
use crate::gaze::bone::{EyeBoneGazeSettings, compute_eye_bone_rotation};
use crate::gaze::expression::RawGazeInput;
use crate::lifecycle::{AvatarLifecycle, AvatarLifecycleState};
use crate::unload::ActiveControlFrame;

/// Applies direct eye-bone gaze when the avatar has no look-direction
/// expression fallback. Rest local rotations are used every frame to avoid
/// cumulative drift.
pub fn apply_tracked_eye_gaze(
    lifecycle: Res<AvatarLifecycle>,
    control_frame: Res<ActiveControlFrame>,
    binding_query: Query<&AvatarBinding>,
    mut eyes: Query<(&mut Transform, &RestTransform)>,
) {
    if lifecycle.state() != AvatarLifecycleState::Ready {
        return;
    }
    let Some(capabilities) = lifecycle.capabilities() else {
        return;
    };
    if capabilities.gaze != GazeMode::EyeBones {
        return;
    }
    let Some(gaze) = control_frame
        .frame
        .as_ref()
        .map(|frame| frame.gaze)
        .filter(|gaze| gaze.is_available())
    else {
        return;
    };
    let Some(root) = lifecycle.active_root() else {
        return;
    };
    let Ok(binding) = binding_query.get(root) else {
        return;
    };
    let delta = compute_eye_bone_rotation(
        &RawGazeInput {
            yaw_rad: gaze.horizontal,
            pitch_rad: gaze.vertical,
        },
        &EyeBoneGazeSettings::default(),
    );
    for target in [binding.left_eye, binding.right_eye].into_iter().flatten() {
        if let Ok((mut transform, rest)) = eyes.get_mut(target) {
            transform.rotation = (rest.0.rotation * delta).normalize();
        }
    }
}
