# ADR-020: Direct MediaPipe と experimental GNM の frame-boundary 統合

Status: Accepted for Issue #50 child #57

## Context

The application already has a production Direct MediaPipe path and Issue #50
adds an experimental GNM Head v3 path. The two paths must consume the same
validated MediaPipe observation without allowing a partially prepared GNM
model, stale detailed coefficients, or a missing Perfect Sync morph binding to
change the default avatar behavior.

The repository does not yet contain a redistributable GNM Head v3 model
artifact. Therefore the application cannot claim runtime GNM readiness in this
leaf. The GNM crate must still expose the complete numeric handoff so a later
model-artifact task can supply the model and decoder without changing the UI or
avatar authority contract.

## Decision

- `FaceRetargetingMode::DirectMediaPipe` is the default requested and active
  mode.
- A GNM request is retained as user intent, but its active authority remains
  Direct until readiness is `Ready` and the active avatar has at least one
  effective Perfect Sync channel. Partial Perfect Sync capability is allowed;
  the avatar adapter remains the final per-channel effective-bind filter.
- The application changes authority only at the latest control-frame boundary.
  When Direct is active, `detailed_face` is cleared before publication. This
  causes the existing expression tracker to emit zero commands for detailed
  channels that disappeared during a GNM-to-Direct switch.
- GNM runtime conversion is model-owned in `vtuber-gnm`: canonical MediaPipe
  478 landmarks are mapped to the fixed sparse 68-point contract, passed to the
  bounded neutral/temporal fitter, and decoded to ARKit52 without reading the
  MediaPipe teacher coefficients at runtime.
- A deterministic numeric A/B evaluator reports residuals, decoder
  confidence, channel MAE, variance, first-difference energy, second-
  difference energy, and latency percentiles. It is diagnostic-only and does
  not declare quality from Direct/GNM differences.
- No GNM worker, model load, or solver is started by default. Camera and
  visual acceptance remain separate from automated contract tests.

## Consequences

The Direct path remains safe and fully usable when GNM is unavailable,
uncalibrated, learning, degraded, or in error. The UI can show the requested
mode, active mode, readiness, fallback reason, and Perfect Sync present versus
effective counts. A subsequent model-artifact/runtime task can publish
readiness and feed the existing GNM handoff without introducing a second VRM
runtime or a mixed per-channel authority.

## Verification

- `cargo test -p vtuber-core --no-fail-fast`
- `cargo test -p vtuber-app --no-fail-fast`
- `cargo test -p vtuber-gnm --no-fail-fast`
- `cargo clippy -p vtuber-gnm --all-targets --all-features -- -D warnings`
- `git diff --check`

Windows camera/VRM visual acceptance and macOS acceptance are `NOT VERIFIED`
for this leaf. No GitHub Actions workflow is used.
