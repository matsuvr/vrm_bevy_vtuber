//! Integration tests for loss hold, neutral decay, and recovery blend.
//!
//! These tests verify that `LossRecovery` behaves deterministically using
//! only caller-supplied durations and monotonic timestamps, without any
//! wall-clock dependency.

use std::time::Duration;

use approx::assert_relative_eq;

use vtuber_core::types::{
    AvatarControlFrame, ExpressionCoefficients, FrameSeq, GazeSignal, HeadPose, MonoTimeNs,
    TrackingState,
};
use vtuber_tracking::loss_recovery::{
    LossRecovery, LossRecoveryConfigError, LossRecoveryParams, MAX_DECAY_DURATION,
    MAX_HOLD_DURATION, MAX_RECOVERY_DURATION, MIN_DECAY_DURATION, MIN_HOLD_DURATION,
    MIN_RECOVERY_DURATION,
};
use vtuber_tracking::pose::semantic_pose_to_quaternion;

fn test_params() -> LossRecoveryParams {
    LossRecoveryParams {
        hold_duration: Duration::from_millis(100),
        decay_duration: Duration::from_millis(200),
        recovery_duration: Duration::from_millis(100),
    }
}

fn frame(seq: u64, yaw: f32, pitch: f32, roll: f32, expression_value: f32) -> AvatarControlFrame {
    AvatarControlFrame {
        source_seq: FrameSeq(seq),
        captured_at: MonoTimeNs(seq * 33_333_333),
        produced_at: MonoTimeNs(seq * 33_333_333),
        confidence: 0.9,
        state: TrackingState::Tracking,
        head: HeadPose {
            yaw_rad: yaw,
            pitch_rad: pitch,
            roll_rad: roll,
        },
        gaze: GazeSignal::UNAVAILABLE,
        expressions: ExpressionCoefficients {
            aa: expression_value,
            ..ExpressionCoefficients::default()
        },
    }
}

fn rotation_angle_from_identity(pose: HeadPose) -> f32 {
    semantic_pose_to_quaternion(pose).angle()
}

#[test]
fn loss_recovery_hold_preserves_source_sequence() {
    let mut recovery = LossRecovery::new(test_params()).unwrap();
    let tracked = frame(7, 0.3, 0.1, -0.2, 0.5);

    let _ = recovery.update(
        TrackingState::Tracking,
        Duration::from_millis(16),
        Some(tracked.clone()),
        MonoTimeNs(16_000_000),
    );
    let held = recovery
        .update(
            TrackingState::LostHold,
            Duration::from_millis(50),
            None,
            MonoTimeNs(66_000_000),
        )
        .expect("held frame should be emitted");

    assert_eq!(held.source_seq, tracked.source_seq);
    assert_eq!(held.captured_at, tracked.captured_at);
    assert_eq!(held.state, TrackingState::LostHold);
    assert!(
        rotation_angle_from_identity(held.head) > 0.01,
        "held pose should not already be neutral"
    );
}

#[test]
fn loss_recovery_neutral_return_uses_shortest_arc() {
    let mut recovery = LossRecovery::new(test_params()).unwrap();
    // Yaw just shy of +pi. The shortest arc to identity stays positive and
    // decreases in magnitude; the long way would wrap through negative yaw.
    let tracked = frame(1, 179.0f32.to_radians(), 0.0, 0.0, 0.0);

    let _ = recovery.update(
        TrackingState::Tracking,
        Duration::from_millis(16),
        Some(tracked.clone()),
        MonoTimeNs(16_000_000),
    );
    // Spend exactly the hold duration in LostHold.
    let _ = recovery.update(
        TrackingState::LostHold,
        Duration::from_millis(100),
        None,
        MonoTimeNs(116_000_000),
    );

    let mut last_yaw = tracked.head.yaw_rad;
    let mut last_angle = rotation_angle_from_identity(tracked.head);

    for step in 1..=5 {
        let out = recovery
            .update(
                TrackingState::ReturningNeutral,
                Duration::from_millis(40),
                None,
                MonoTimeNs(116_000_000 + step as u64 * 40_000_000),
            )
            .expect("returning frame should be emitted");

        let angle = rotation_angle_from_identity(out.head);
        assert!(
            angle <= last_angle + 1e-5,
            "rotation angle should not increase during return: step {step}: {angle} > {last_angle}"
        );
        assert!(
            out.head.yaw_rad >= -0.01,
            "shortest arc should stay on the positive side of yaw: step {step}: {}",
            out.head.yaw_rad
        );
        assert!(
            out.head.yaw_rad.abs() <= last_yaw.abs() + 1e-5,
            "yaw magnitude should decrease: step {step}: {} > {}",
            out.head.yaw_rad.abs(),
            last_yaw.abs()
        );

        last_yaw = out.head.yaw_rad;
        last_angle = angle;
    }

    // After the full decay duration has elapsed, the output should be neutral.
    let neutral = recovery
        .update(
            TrackingState::ReturningNeutral,
            Duration::from_millis(200),
            None,
            MonoTimeNs(500_000_000),
        )
        .expect("neutral frame should be emitted");
    assert_relative_eq!(neutral.head.yaw_rad, 0.0, epsilon = 1e-4);
    assert_relative_eq!(neutral.head.pitch_rad, 0.0, epsilon = 1e-4);
    assert_relative_eq!(neutral.head.roll_rad, 0.0, epsilon = 1e-4);
}

#[test]
fn loss_recovery_reacquire_limits_jump() {
    let mut recovery = LossRecovery::new(test_params()).unwrap();
    let first = frame(1, 0.0, 0.0, 0.0, 0.0);

    // Track a neutral pose.
    let _ = recovery.update(
        TrackingState::Tracking,
        Duration::from_millis(16),
        Some(first.clone()),
        MonoTimeNs(16_000_000),
    );

    // Lose the face and let it decay partway to neutral.
    let _ = recovery.update(
        TrackingState::LostHold,
        Duration::from_millis(100),
        None,
        MonoTimeNs(116_000_000),
    );
    let before_reacquire = recovery
        .update(
            TrackingState::ReturningNeutral,
            Duration::from_millis(100),
            None,
            MonoTimeNs(216_000_000),
        )
        .unwrap();

    // Reacquire with a pose that is far from the current recovered pose.
    let target = frame(2, -1.2, 0.6, -0.4, 0.9);
    let during_recovery = recovery
        .update(
            TrackingState::Tracking,
            Duration::from_millis(50),
            Some(target.clone()),
            MonoTimeNs(266_000_000),
        )
        .unwrap();

    // The recovery frame must not snap directly to the target.
    assert!(
        (during_recovery.head.yaw_rad - target.head.yaw_rad).abs() > 0.1,
        "recovery should not jump to target yaw immediately"
    );

    // The rotation should move toward the target, not away from it.
    let before_q = semantic_pose_to_quaternion(before_reacquire.head);
    let target_q = semantic_pose_to_quaternion(target.head);
    let during_q = semantic_pose_to_quaternion(during_recovery.head);

    let before_to_target = before_q.angle_to(&target_q);
    let during_to_target = during_q.angle_to(&target_q);
    assert!(
        during_to_target < before_to_target,
        "recovery should move closer to target: before_to_target={before_to_target}, during_to_target={during_to_target}"
    );

    // Finish the recovery.
    let after_recovery = recovery
        .update(
            TrackingState::Tracking,
            Duration::from_millis(100),
            Some(target.clone()),
            MonoTimeNs(366_000_000),
        )
        .unwrap();
    assert_relative_eq!(
        after_recovery.head.yaw_rad,
        target.head.yaw_rad,
        epsilon = 1e-4
    );
    assert!(!recovery.is_recovering());
}

#[test]
fn loss_recovery_settings_enforce_fixed_ranges() {
    assert!(LossRecoveryParams::default().validate().is_ok());

    assert!(matches!(
        LossRecoveryParams {
            hold_duration: Duration::ZERO,
            ..LossRecoveryParams::default()
        }
        .validate()
        .unwrap_err(),
        LossRecoveryConfigError::ZeroDuration {
            field: "hold_duration"
        }
    ));

    assert!(
        LossRecoveryParams {
            hold_duration: MIN_HOLD_DURATION - Duration::from_millis(1),
            ..LossRecoveryParams::default()
        }
        .validate()
        .is_err()
    );

    assert!(
        LossRecoveryParams {
            hold_duration: MAX_HOLD_DURATION + Duration::from_millis(1),
            ..LossRecoveryParams::default()
        }
        .validate()
        .is_err()
    );

    assert!(
        LossRecoveryParams {
            decay_duration: MIN_DECAY_DURATION - Duration::from_millis(1),
            ..LossRecoveryParams::default()
        }
        .validate()
        .is_err()
    );

    assert!(
        LossRecoveryParams {
            decay_duration: MAX_DECAY_DURATION + Duration::from_millis(1),
            ..LossRecoveryParams::default()
        }
        .validate()
        .is_err()
    );

    assert!(
        LossRecoveryParams {
            recovery_duration: MIN_RECOVERY_DURATION - Duration::from_millis(1),
            ..LossRecoveryParams::default()
        }
        .validate()
        .is_err()
    );

    assert!(
        LossRecoveryParams {
            recovery_duration: MAX_RECOVERY_DURATION + Duration::from_millis(1),
            ..LossRecoveryParams::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn loss_recovery_does_not_publish_stale_observation_as_new_frame() {
    let mut recovery = LossRecovery::new(test_params()).unwrap();
    let tracked = frame(5, 0.4, 0.0, 0.0, 0.7);

    let _ = recovery.update(
        TrackingState::Tracking,
        Duration::from_millis(16),
        Some(tracked.clone()),
        MonoTimeNs(16_000_000),
    );

    // Emit several synthetic frames while lost. Their source sequence must
    // remain the last valid sequence, not increment.
    let last_seq = tracked.source_seq;
    for step in 1..=10 {
        let state = if step <= 3 {
            TrackingState::LostHold
        } else {
            TrackingState::ReturningNeutral
        };
        let out = recovery
            .update(
                state,
                Duration::from_millis(50),
                None,
                MonoTimeNs(16_000_000 + step as u64 * 50_000_000),
            )
            .expect("synthetic frame should be emitted");
        assert_eq!(
            out.source_seq, last_seq,
            "stale observation should not be republished with a new sequence"
        );
    }

    // Reacquire with a new observation. Only after recovery completes should
    // the source sequence advance.
    let reacquired = frame(6, -0.4, 0.0, 0.0, 0.0);
    let mut seen_new_seq = false;
    for step in 1..=5 {
        let out = recovery
            .update(
                TrackingState::Tracking,
                Duration::from_millis(30),
                Some(reacquired.clone()),
                MonoTimeNs(600_000_000 + step as u64 * 30_000_000),
            )
            .expect("frame should be emitted during recovery");
        if out.source_seq == reacquired.source_seq {
            assert!(
                !recovery.is_recovering(),
                "source sequence must not advance until recovery is complete"
            );
            seen_new_seq = true;
            break;
        }
    }
    assert!(
        seen_new_seq,
        "recovery should eventually publish the new observation"
    );
}
