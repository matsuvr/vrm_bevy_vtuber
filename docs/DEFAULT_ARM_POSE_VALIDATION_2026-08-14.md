# Default-arm pose validation report

Date: 2026-08-14
Scope: GitHub Issue #19 and Issues #14–#18
Code baseline before this report: `dbcaf19`
Status: local automated regression PASS; real-VRM visual validation NOT RUN

## Environment

| Field | Value |
|---|---|
| OS | Windows 11 Pro 10.0.26200 (x86_64) |
| Shell | PowerShell |
| rustc | 1.97.1 (8bab26f4f 2026-07-14) |
| cargo | 1.97.1 (c980f4866 2026-06-30) |
| Bevy | 0.19.0 |
| `bevy_vrm1` | `f9593fd78136fb9e0507bcae111e09291ec9b82a` |
| camera / GPU / real model | NOT RUN for this issue |

## Representative-model visual gate

The requested real-model gate was intentionally skipped because it requires
manual model loading and visual inspection. No third-party VRM was copied into
the repository, and no visual PASS is claimed.

| Model class | SHA-256 / provenance | Front | 45° | Side | Result |
|---|---|---|---|---|---|
| VRM Consortium / author official sample | NOT PROVIDED; not committed | NOT RUN | NOT RUN | NOT RUN | PENDING |
| VRoid Studio VRM 1.0 export | NOT PROVIDED; not committed | NOT RUN | NOT RUN | NOT RUN | PENDING |
| Intended private/user model | NOT PROVIDED; not committed | NOT RUN | NOT RUN | NOT RUN | PENDING |

The following visual checks therefore remain pending: arm spread, elbow lock or
flip, shoulder shrug, wrist orientation, finger relaxation, arm/torso
penetration, left/right asymmetry, MToon, outline, transparent materials, and
SpringBone appearance.

## Automated regression matrix

| Acceptance behavior | Local evidence | Result |
|---|---|---|
| Non-identity rest rotations and rest-space conjugation | `cargo test -p vtuber-avatar --test arm_ik --test arm_pose` | PASS |
| Asymmetric upper/forearm lengths and mirrored sides | `crates/vtuber-avatar/tests/arm_ik.rs` | PASS |
| Optional shoulder/finger absence and degenerate side no-op | `cargo test -p vtuber-avatar --test binding --test arm_pose` | PASS |
| Intermediate hierarchy propagation and authored wrist orientation | `cargo test -p vtuber-avatar --test arm_pose` | PASS |
| Animation base preservation and no accumulation | `cargo test -p vtuber-avatar --test arm_pose` | PASS |
| Avatar replacement / generation cleanup | `cargo test -p vtuber-avatar --test arm_pose --test binding --test unload` | PASS |
| Finite degenerate-input behavior | `cargo test -p vtuber-avatar --test arm_ik --test arm_profile` | PASS |
| Per-model bounded override, reset, no cross-model leak | `cargo test -p vtuber-avatar --test arm_profile` | PASS |
| 30/60/120 FPS transition equivalence | `cargo test -p vtuber-avatar --test arm_profile` | PASS |
| Shortest-arc quaternion and invalid-time safety | `cargo test -p vtuber-avatar --test arm_profile` | PASS |
| Runtime control order and no synthetic world target | `cargo test -p vtuber-avatar --test schedule` | PASS |

The complete local workspace run also passed:

```text
cargo fmt --all -- --check                         PASS
cargo check --workspace                            PASS
cargo clippy --workspace --all-targets -- -D warnings PASS
cargo test --workspace --no-fail-fast              PASS
cargo deny check                                   PASS (existing duplicate/dead license warnings)
git diff --check                                   PASS
```

No GitHub Actions workflow, trigger, or remote CI result was used.

## Final control-order contract

```text
AnimationSystems
  -> direct-pose bevy_vrm1 BodyTracking (spine..head)
  -> model-adaptive DefaultArmPose (arm subtree only)
  -> direct head-relative LookAt / GazeControl
  -> ModifyExpressions / Expressions
  -> VrmSystemSets::Constraints
  -> transform propagation
  -> SpringBone
```

`RestTransform` and `RestGlobalTransform` are immutable authorities. The arm
compositor applies rest-relative local deltas to the animation base and
propagates only affected real `ChildOf` subtrees. It does not create a
synthetic world-space gaze target, write hand world transforms, or replace the
direct-pose `BodyTracking` writer for head-to-spine bones. Missing optional
shoulder/fingers and invalid or degenerate arm geometry fall back to a safe
no-op for that side.

## Limitations and follow-up

- Windows real camera and real VRM visual acceptance: `NOT RUN`.
- macOS compile/real VRM/camera validation: `NOT RUN`.
- The automated suite establishes finite/control-order/regression behavior only;
  it cannot establish model appearance, MToon behavior, permission behavior, or
  hardware performance.
- Run the existing local manual compatibility protocol before claiming Issue
  #19 visual acceptance.
