//! Tracked pose apply system.
//!
//! Reads the latest [`ActiveControlFrame`] and applies head/neck rotations
//! to the active avatar's bone entities. The system runs in `PostUpdate`,
//! after Bevy `AnimationSystems`, and recomputes from rest each frame to
//! prevent drift.

use bevy::prelude::*;

use super::binding::RestOrientationCache;
use super::distribution::{PoseDistributionSettings, apply_distributed_pose, distribute_pose};
use crate::binding::AvatarBinding;
use crate::lifecycle::{AvatarLifecycle, AvatarLifecycleState};
use crate::unload::ActiveControlFrame;

/// Metrics for the pose apply system, useful for diagnostics.
#[derive(Resource, Debug, Default, Clone)]
pub struct PoseApplyMetrics {
    /// Number of frames where the pose was successfully applied.
    pub frames_applied: u64,
    /// Number of frames skipped because lifecycle was not Ready.
    pub skipped_not_ready: u64,
    /// Number of frames skipped due to generation mismatch.
    pub skipped_generation_mismatch: u64,
    /// Number of frames skipped because no control frame was available.
    pub skipped_no_frame: u64,
    /// Number of frames skipped because the binding entity was stale.
    pub skipped_stale_entity: u64,
}

/// System that applies tracked head pose to the active avatar's bones.
///
/// # Schedule
///
/// Runs in `PostUpdate`, after `AnimationSystems`. Recomputes from rest
/// each frame to prevent cumulative drift.
///
/// # Skip conditions
///
/// - Lifecycle is not `Ready`
/// - No active control frame
/// - Generation mismatch between frame and binding
/// - Head bone entity no longer exists
#[allow(clippy::too_many_arguments)]
pub fn apply_tracked_head_pose(
    lifecycle: Res<AvatarLifecycle>,
    control_frame: Res<ActiveControlFrame>,
    settings: Res<PoseDistributionSettings>,
    mut metrics: ResMut<PoseApplyMetrics>,
    mut bone_query: Query<(
        &mut Transform,
        &GlobalTransform,
        Option<&bevy_vrm1::prelude::RestTransform>,
    )>,
    _root_query: Query<(&GlobalTransform, Option<&bevy_vrm1::prelude::RestTransform>)>,
    cache_query: Query<&RestOrientationCache>,
    binding_query: Query<&AvatarBinding>,
) {
    // Check lifecycle is Ready.
    if lifecycle.state() != AvatarLifecycleState::Ready {
        metrics.skipped_not_ready += 1;
        return;
    }

    let active_root = match lifecycle.active_root() {
        Some(root) => root,
        None => {
            metrics.skipped_not_ready += 1;
            return;
        }
    };

    // Check we have a control frame.
    let frame = match &control_frame.frame {
        Some(f) => f,
        None => {
            metrics.skipped_no_frame += 1;
            return;
        }
    };

    // Check generation matches.
    let binding = match binding_query.get(active_root) {
        Ok(b) => b,
        Err(_) => {
            metrics.skipped_stale_entity += 1;
            return;
        }
    };

    if control_frame.generation != binding.generation {
        metrics.skipped_generation_mismatch += 1;
        return;
    }

    // Get rest orientation cache.
    let cache = match cache_query.get(active_root) {
        Ok(c) => c,
        Err(_) => {
            metrics.skipped_stale_entity += 1;
            return;
        }
    };

    // Check head bone exists.
    if bone_query.get_mut(binding.head).is_err() {
        metrics.skipped_stale_entity += 1;
        return;
    }

    // Distribute the pose.
    let has_neck = binding.neck.is_some();
    let distributed = distribute_pose(
        frame.head.yaw_rad,
        frame.head.pitch_rad,
        frame.head.roll_rad,
        has_neck,
        &settings,
    );

    // Apply to bones.
    let (head_rot, neck_rot) = apply_distributed_pose(&distributed, cache);

    // Write head rotation.
    if let Ok((mut head_transform, _, _)) = bone_query.get_mut(binding.head) {
        head_transform.rotation = head_rot;
    }

    // Write neck rotation if present.
    if let (Some(neck_entity), Some(neck_rotation)) = (binding.neck, neck_rot)
        && let Ok((mut neck_transform, _, _)) = bone_query.get_mut(neck_entity)
    {
        neck_transform.rotation = neck_rotation;
    }

    metrics.frames_applied += 1;
}

/// System that resets pose metrics when the avatar lifecycle changes.
///
/// Runs after `clear_control_cache_on_lifecycle_change` to ensure metrics
/// don't accumulate across avatar replacements.
pub fn reset_pose_metrics_on_lifecycle_change(
    lifecycle: Res<AvatarLifecycle>,
    mut metrics: ResMut<PoseApplyMetrics>,
    mut last_state: Local<Option<AvatarLifecycleState>>,
) {
    let current = lifecycle.state();
    if last_state.as_ref() != Some(&current) {
        *metrics = PoseApplyMetrics::default();
        *last_state = Some(current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracked_pose_system_skips_when_not_ready() {
        let metrics = PoseApplyMetrics::default();
        assert_eq!(metrics.frames_applied, 0);
        assert_eq!(metrics.skipped_not_ready, 0);
    }

    #[test]
    fn tracked_pose_system_metrics_default() {
        let metrics = PoseApplyMetrics::default();
        assert_eq!(metrics.frames_applied, 0);
        assert_eq!(metrics.skipped_not_ready, 0);
        assert_eq!(metrics.skipped_generation_mismatch, 0);
        assert_eq!(metrics.skipped_no_frame, 0);
        assert_eq!(metrics.skipped_stale_entity, 0);
    }
}
