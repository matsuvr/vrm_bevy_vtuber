//! Bridge from tracking's latest control slot to the active avatar generation.

use bevy::prelude::*;
use vtuber_avatar::{
    ActiveControlFrame, AvatarLifecycle, AvatarLifecycleState, PoseApplyMetrics,
    set_active_control_frame,
};

use crate::diagnostics::DiagnosticsSnapshot;
use crate::orchestrator::Orchestrator;
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
    orchestrator: Option<Res<Orchestrator>>,
) {
    if lifecycle.state() != AvatarLifecycleState::Ready || !tracking.control_active {
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
    // Detailed ARKit52 values are authoritative only for an explicitly
    // selected, ready GNM session.  Clearing them at this single frame
    // boundary prevents stale experimental values from leaking into Direct
    // MediaPipe after a mode switch; the avatar expression tracker emits the
    // required zero commands for disappeared channels.
    let gnm_authority = orchestrator
        .as_ref()
        .is_some_and(|orchestrator| orchestrator.retargeting_status().uses_gnm_authority());
    let mut frame = frame;
    if !gnm_authority {
        frame.detailed_face = None;
    }
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
    orchestrator: Option<ResMut<Orchestrator>>,
) {
    if let Some(lifecycle) = lifecycle.as_ref() {
        diagnostics.avatar_capabilities = lifecycle.capabilities().map(|caps| caps.summary());
    }
    if let Some(mut orchestrator) = orchestrator {
        if let Some(lifecycle) = lifecycle.as_ref() {
            if let Some(caps) = lifecycle.capabilities() {
                orchestrator.set_perfect_sync_capability(
                    caps.perfect_sync.present_count(),
                    caps.perfect_sync.effective_count(),
                );
            } else {
                orchestrator.set_perfect_sync_capability(0, 0);
            }
        }
        diagnostics.face_retargeting = Some(orchestrator.retargeting_status());
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

#[cfg(test)]
mod tests {
    use super::*;
    use vtuber_avatar::lifecycle::AvatarLifecycle;

    #[test]
    fn inactive_tracking_session_clears_retained_control_frame() {
        let mut app = App::new();
        app.init_resource::<TrackingRuntime>()
            .init_resource::<AvatarLifecycle>()
            .init_resource::<ActiveControlFrame>()
            .add_systems(Update, publish_control_frame_system);

        let root = app.world_mut().spawn_empty().id();
        {
            let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
            lifecycle.request_load(root).unwrap();
            lifecycle.start_binding(root);
            lifecycle.finish_ready();
        }
        let generation = app
            .world()
            .resource::<AvatarLifecycle>()
            .current_generation();
        app.world_mut()
            .resource_mut::<ActiveControlFrame>()
            .generation = generation;
        app.world_mut().resource_mut::<ActiveControlFrame>().frame =
            Some(vtuber_core::AvatarControlFrame {
                source_seq: vtuber_core::FrameSeq(1),
                captured_at: vtuber_core::MonoTimeNs(1),
                produced_at: vtuber_core::MonoTimeNs(1),
                confidence: 1.0,
                state: vtuber_core::TrackingState::Tracking,
                head: vtuber_core::HeadPose::default(),
                gaze: vtuber_core::GazeSignal::UNAVAILABLE,
                expressions: vtuber_core::ExpressionCoefficients::default(),
                detailed_face: None,
            });

        app.update();

        assert!(app.world().resource::<ActiveControlFrame>().frame.is_none());
    }

    #[test]
    fn ready_avatar_receives_the_latest_real_tracking_frame() {
        let mut app = App::new();
        app.init_resource::<TrackingRuntime>()
            .init_resource::<AvatarLifecycle>()
            .init_resource::<ActiveControlFrame>()
            .add_systems(Update, publish_control_frame_system);

        let root = app.world_mut().spawn_empty().id();
        {
            let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
            lifecycle.request_load(root).unwrap();
            lifecycle.start_binding(root);
            lifecycle.finish_ready();
        }
        let expected_generation = app
            .world()
            .resource::<AvatarLifecycle>()
            .current_generation();

        let frame = vtuber_core::AvatarControlFrame {
            source_seq: vtuber_core::FrameSeq(4),
            captured_at: vtuber_core::MonoTimeNs(10),
            produced_at: vtuber_core::MonoTimeNs(12),
            confidence: 0.9,
            state: vtuber_core::TrackingState::Tracking,
            head: vtuber_core::HeadPose {
                yaw_rad: 0.2,
                ..Default::default()
            },
            gaze: vtuber_core::GazeSignal::UNAVAILABLE,
            expressions: vtuber_core::ExpressionCoefficients::default(),
            detailed_face: None,
        };
        {
            let mut tracking = app.world_mut().resource_mut::<TrackingRuntime>();
            tracking.control_active = true;
            tracking.latest_control = Some(frame.clone());
            assert!(tracking.control_slot.publish(frame));
        }

        app.update();

        let active = app.world().resource::<ActiveControlFrame>();
        assert_eq!(active.generation, expected_generation);
        assert_eq!(
            active.frame.as_ref().map(|frame| frame.source_seq.0),
            Some(4)
        );
    }

    #[test]
    fn direct_authority_clears_stale_detailed_face_values() {
        let mut app = App::new();
        app.init_resource::<TrackingRuntime>()
            .init_resource::<AvatarLifecycle>()
            .init_resource::<ActiveControlFrame>()
            .init_resource::<Orchestrator>()
            .add_systems(Update, publish_control_frame_system);

        let root = app.world_mut().spawn_empty().id();
        {
            let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
            lifecycle.request_load(root).unwrap();
            lifecycle.start_binding(root);
            lifecycle.finish_ready();
        }
        let frame = vtuber_core::AvatarControlFrame {
            source_seq: vtuber_core::FrameSeq(9),
            captured_at: vtuber_core::MonoTimeNs(20),
            produced_at: vtuber_core::MonoTimeNs(22),
            confidence: 1.0,
            state: vtuber_core::TrackingState::Tracking,
            head: vtuber_core::HeadPose::default(),
            gaze: vtuber_core::GazeSignal::UNAVAILABLE,
            expressions: vtuber_core::ExpressionCoefficients::default(),
            detailed_face: Some(vtuber_core::Arkit52Coefficients::default()),
        };
        {
            let mut tracking = app.world_mut().resource_mut::<TrackingRuntime>();
            tracking.control_active = true;
            tracking.latest_control = Some(frame.clone());
            assert!(tracking.control_slot.publish(frame));
        }

        app.update();

        assert_eq!(
            app.world()
                .resource::<ActiveControlFrame>()
                .frame
                .as_ref()
                .and_then(|frame| frame.detailed_face),
            None
        );
    }

    #[test]
    fn ready_gnm_authority_preserves_detailed_face_values() {
        let mut app = App::new();
        app.init_resource::<TrackingRuntime>()
            .init_resource::<AvatarLifecycle>()
            .init_resource::<ActiveControlFrame>()
            .init_resource::<Orchestrator>()
            .add_systems(Update, publish_control_frame_system);
        {
            let mut orchestrator = app.world_mut().resource_mut::<Orchestrator>();
            orchestrator.process_action(&crate::actions::UiAction::SelectFaceRetargetingMode {
                mode: vtuber_core::FaceRetargetingMode::GnmPerfectSync,
            });
            orchestrator.set_perfect_sync_capability(52, 52);
            orchestrator.set_gnm_readiness(vtuber_core::GnmReadiness::Ready);
        }

        let root = app.world_mut().spawn_empty().id();
        {
            let mut lifecycle = app.world_mut().resource_mut::<AvatarLifecycle>();
            lifecycle.request_load(root).unwrap();
            lifecycle.start_binding(root);
            lifecycle.finish_ready();
        }
        let frame = vtuber_core::AvatarControlFrame {
            source_seq: vtuber_core::FrameSeq(10),
            captured_at: vtuber_core::MonoTimeNs(30),
            produced_at: vtuber_core::MonoTimeNs(32),
            confidence: 1.0,
            state: vtuber_core::TrackingState::Tracking,
            head: vtuber_core::HeadPose::default(),
            gaze: vtuber_core::GazeSignal::UNAVAILABLE,
            expressions: vtuber_core::ExpressionCoefficients::default(),
            detailed_face: Some(vtuber_core::Arkit52Coefficients::default()),
        };
        {
            let mut tracking = app.world_mut().resource_mut::<TrackingRuntime>();
            tracking.control_active = true;
            tracking.latest_control = Some(frame.clone());
            assert!(tracking.control_slot.publish(frame));
        }

        app.update();

        assert!(
            app.world()
                .resource::<ActiveControlFrame>()
                .frame
                .as_ref()
                .is_some_and(|frame| frame.detailed_face.is_some())
        );
    }
}
