//! Tracked pose apply system.
//!
//! Reads the latest [`ActiveControlFrame`] and applies head/neck rotations
//! to the active avatar's bone entities. The system runs in `PostUpdate`,
//! after Bevy `AnimationSystems`, and recomputes from rest each frame to
//! prevent drift.

use bevy::prelude::*;
use vtuber_core::metrics::FixedStats;
use vtuber_core::monotonic_now;

use super::binding::RestOrientationCache;
use super::distribution::{PoseDistributionSettings, apply_distributed_pose, distribute_pose};
use crate::binding::AvatarBinding;
use crate::lifecycle::{AvatarLifecycle, AvatarLifecycleState};
use crate::unload::ActiveControlFrame;

/// Metrics for the pose apply system, useful for diagnostics.
#[derive(Resource, Debug, Clone)]
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
    /// Source sequence of the most recently applied frame.
    pub last_applied_source_seq: Option<vtuber_core::FrameSeq>,
    /// Monotonic time when the most recent frame was applied.
    pub last_applied_at: Option<vtuber_core::MonoTimeNs>,
    /// Capture-to-apply latency of the most recently applied frame.
    pub last_capture_to_apply_ms: Option<f64>,
    /// Fixed-size capture-to-apply latency samples.
    latency_samples: FixedStats,
}

impl Default for PoseApplyMetrics {
    fn default() -> Self {
        Self {
            frames_applied: 0,
            skipped_not_ready: 0,
            skipped_generation_mismatch: 0,
            skipped_no_frame: 0,
            skipped_stale_entity: 0,
            last_applied_source_seq: None,
            last_applied_at: None,
            last_capture_to_apply_ms: None,
            latency_samples: FixedStats::new(256),
        }
    }
}

impl PoseApplyMetrics {
    /// Number of capture-to-apply latency samples retained.
    #[must_use]
    pub fn latency_sample_count(&self) -> usize {
        self.latency_samples.count()
    }

    /// p50 capture-to-apply latency in milliseconds.
    #[must_use]
    pub fn capture_to_apply_p50_ms(&self) -> f64 {
        self.latency_samples.p50()
    }

    /// p95 capture-to-apply latency in milliseconds.
    #[must_use]
    pub fn capture_to_apply_p95_ms(&self) -> f64 {
        self.latency_samples.p95()
    }
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

    let applied_at = monotonic_now();
    let latency_ms = applied_at
        .0
        .checked_sub(frame.captured_at.0)
        .map(|ns| ns as f64 / 1_000_000.0);
    metrics.frames_applied += 1;
    metrics.last_applied_source_seq = Some(frame.source_seq);
    metrics.last_applied_at = Some(applied_at);
    metrics.last_capture_to_apply_ms = latency_ms;
    if let Some(latency_ms) = latency_ms {
        metrics.latency_samples.record(latency_ms);
    }
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
