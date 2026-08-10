//! Bridge from tracking's latest control slot to the active avatar generation.

use bevy::prelude::*;
use vtuber_avatar::{
    ActiveControlFrame, AvatarLifecycle, AvatarLifecycleState, PoseApplyMetrics,
    set_active_control_frame,
};

use crate::diagnostics::DiagnosticsSnapshot;
use crate::tracking_runtime::TrackingRuntime;

/// Publishes the latest tracking frame only while the active avatar is ready.
///
/// The avatar generation is attached on the Bevy side, so an old frame cannot
/// be applied to a replacement avatar. Frames produced while loading,
/// unloading, or failed are dropped and counted by the avatar apply systems.
pub fn publish_control_frame_system(
    mut tracking: ResMut<TrackingRuntime>,
    lifecycle: Res<AvatarLifecycle>,
    mut active: ResMut<ActiveControlFrame>,
) {
    if lifecycle.state() != AvatarLifecycleState::Ready {
        active.frame = None;
        return;
    }
    let slot = tracking.control_slot();
    let generation = slot.generation();
    if generation == 0 {
        return;
    }
    // `TrackingRuntime` retains the last value, but this generation check
    // ensures the bridge publishes at most one copy per latest-slot advance.
    if active.frame.as_ref().is_some_and(|frame| {
        tracking
            .latest_control
            .as_ref()
            .is_some_and(|value| frame.produced_at == value.produced_at)
    }) {
        return;
    }
    let Some(frame) = tracking.latest_control.take() else {
        return;
    };
    let _ = set_active_control_frame(
        &lifecycle,
        lifecycle.current_generation(),
        frame,
        &mut active,
    );
}

/// Mirrors real avatar binding/apply metrics into the application diagnostics.
pub fn sync_avatar_diagnostics(
    lifecycle: Option<Res<AvatarLifecycle>>,
    pose_metrics: Option<Res<PoseApplyMetrics>>,
    mut diagnostics: ResMut<DiagnosticsSnapshot>,
) {
    if let Some(lifecycle) = lifecycle {
        diagnostics.avatar_capabilities = lifecycle.capabilities().map(|caps| caps.summary());
    }
    if let Some(metrics) = pose_metrics {
        diagnostics.avatar_frames_applied = metrics.frames_applied;
        diagnostics.avatar_frames_skipped = metrics
            .skipped_not_ready
            .saturating_add(metrics.skipped_generation_mismatch)
            .saturating_add(metrics.skipped_stale_entity);
        if metrics.latency_sample_count() > 0 {
            diagnostics.capture_to_apply_p50_ms = Some(metrics.capture_to_apply_p50_ms() as f32);
            diagnostics.capture_to_apply_p95_ms = Some(metrics.capture_to_apply_p95_ms() as f32);
        } else {
            diagnostics.capture_to_apply_p50_ms = None;
            diagnostics.capture_to_apply_p95_ms = None;
        }
    }
}
