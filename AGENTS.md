# AGENTS.md

## Mission

Implement the application described by `DESIGN.md` and `AI_AGENT_TASKS.md`: a full-Rust, desktop-only VTuber application for VRM 1.0 using Bevy 0.19 and `bevy_vrm1`, with webcam face tracking on Windows and macOS.

Work on exactly one task ID from `AI_AGENT_TASKS.md` at a time.

## Sources of truth

Order of authority:

1. Current user instruction
2. `DESIGN.md`
3. `AI_AGENT_TASKS.md`
4. Accepted ADRs under `docs/adr/`
5. Official upstream documentation and source pinned by revision
6. Existing code and tests

When code and design disagree, do not silently preserve code. Report the mismatch and update the relevant ADR or design in the same change only when the active task permits it.

## Fixed scope

Supported:

- VRM 1.0 only
- Windows 11 x86_64
- macOS 13+; Apple Silicon is Tier 1
- Bevy 0.19.0
- `bevy_vrm1` as the VRM runtime

Explicitly unsupported:

- VRM 0.x
- Android
- iOS
- Linux product support
- WebAssembly
- browser camera APIs
- VRMA playback
- full-body and hand tracking

Do not add abstraction, feature flags, crates, CI jobs, manifests, or documentation solely for unsupported platforms.

## GitHub Actions 禁止

- 本プロジェクトでは GitHub Actions を一切利用しない。Actions の実行枠を使い切っており、実行を試みるだけでエラーになり開発を妨げるためである。
- `.github/workflows/`、workflow YAML、`actions/*` の参照、Actions badge、`workflow_dispatch` など、GitHub Actions を作成・有効化・実行・再導入する変更を禁止する。
- 検証は開発者環境で PowerShell、Cargo、`xtask`、および明示的な Windows/macOS 実機手順を実行する。GitHub 上の push／pull request を検証トリガーにしてはならない。
- 過去の workflow、実行履歴、旧タスク文書に残る CI 記述は履歴であり、現行の検証手段・受入根拠・実装指示として再利用しない。

## VRM runtime rules

- Use `bevy_vrm1`; do not implement a replacement VRM runtime.
- Do not add `vrm-utils-rs`, `vrm-spec`, or `bevy_vrm`.
- Do not create a custom `.vrm` AssetLoader.
- Do not reimplement MToon, SpringBone, Node Constraint, Humanoid binding, or Expression morph accumulation.
- Isolate all `bevy_vrm1` dependencies in `vtuber-avatar`.
- Do not expose `bevy_vrm1` types through `vtuber-core` or `vtuber-tracking` APIs.
- Pin `bevy_vrm1` to the exact approved revision. Upgrade only in a dependency-only task or ADR.
- Do not fork `bevy_vrm1` without a target-model reproducer, regression test, spec citation, and ADR. `Q2-06-001` and `Q2-06-002` are the approved exceptions for source-derived vendored patches limited respectively to direct-pose `BodyTracking` and direct head-relative LookAt; preserve the upstream license and base revision and keep the dependency immutable.

## bevy_vrm1 known-path restrictions

Until a task explicitly changes these rules:

- Do not insert a cursor/target `bevy_vrm1::LookAt` for webcam gaze. Use the `Q2-06-002` direct head-relative LookAt input; it must not create a synthetic world-space target.
- Use the `Q2-06-001` direct-pose extension of `bevy_vrm1::BodyTracking` as the sole writer for tracked head, neck, upper-chest, chest, and spine rotation.
- Feed calibrated yaw, pitch, and roll directly to `BodyTracking`; do not create a synthetic `LookAt` target for face pose.
- Inspect a model before loading it. Reject missing `VRMC_vrm`, non-1.0 VRM, missing hips, missing head, invalid node indices, and external URIs.
- Use `ExpressionEntityMap` to build expression capabilities.
- Use `ModifyExpressions` for procedural expression updates.
- Let `bevy_vrm1` resolve binary and override behavior.
- Apply direct-pose `BodyTracking` after Bevy `AnimationSystems` and before `VrmSystemSets::Constraints`.
- Estimate webcam eye gaze separately from head pose, but apply it as a head-relative LookAt delta.
- Resolve LookAt and look expressions after tracked head/body pose and before Node Constraint and SpringBone, following the VRM 1.0 execution order.
- No eye writer may set an independent world transform or change eye translation/scale.
- Do not disable constraints or SpringBone merely to simplify scheduling.

## Architecture boundaries

### `vtuber-core`

May contain only platform- and engine-independent data and synchronization contracts. It must not depend on Bevy, `bevy_vrm1`, `nokhwa`, or tract.

### `vtuber-camera`

Owns OS camera backends and `nokhwa`. Never expose backend buffers or OS handles. Output owned `VideoFrame` values. Construct, open, use, stop, and drop the native camera object inside the capture worker; do not require the stream object itself to cross threads.

### `vtuber-inference`

Owns model loading, preprocessing, inference, and output decoding. The approved production face backend is the official MediaPipe Tasks 0.10.35 native runtime accessed only through the pinned `mediapipe-rs` revision recorded in ADR-009. It must not depend on Bevy. Construct and own the inference runtime inside the inference worker instead of moving a live runtime object across threads.

### `vtuber-tracking`

Owns calibration, pose solving, filtering, tracking state, and expression coefficients. It must not depend on Bevy or `bevy_vrm1`.

### `vtuber-avatar`

Owns all interaction with Bevy entities and `bevy_vrm1` APIs.

### `vtuber-app`

Owns orchestration, UI, settings, model import, diagnostics, and application state. It must not contain model-specific inference math or VRM runtime internals.

## Full-Rust boundary

Production runtime must not use:

- Python subprocesses
- TensorFlow Lite C API
- ONNX Runtime
- OpenCV
- Unity
- arbitrary native inference plugins or an unreviewed native runtime hidden behind a Rust wrapper

The sole approved exception is the official MediaPipe Tasks 0.10.35 runtime,
accessed through the exact reviewed `mediapipe-rs` revision in ADR-009. This
exception is audited, worker-owned, CPU-only for the current Windows gate, and
must remain behind the `vtuber-inference` boundary. Application crates still
use `#![forbid(unsafe_code)]`; no Python, OpenCV, Unity, ONNX Runtime, or
sidecar process may be added.

OS camera FFI used by `nokhwa`, Bevy/wgpu internals, Windows manifests, and macOS bundle metadata are permitted.

## Dependency policy

- Pin Bevy exactly to `0.19.0`.
- Pin the approved `bevy_vrm1` Git revision exactly.
- Pin MediaPipe Tasks to 0.10.35 through `mediapipe-rs` revision
  `527037fa0fe1339750140283930bbb9560460e9e`; do not use an unpinned branch.
- Use target-specific `nokhwa` features: MSMF on Windows and AVFoundation on macOS.
- Do not enable both TFLite and ONNX inference production features simultaneously.
- Record source, version, purpose, license, and alternatives for every new direct dependency.
- Commit `Cargo.lock`.
- Do not update unrelated dependencies while implementing a feature task.

The approved face task bundle is `assets/models/face_landmarker.task` with
SHA-256
`64184E229B263107BC2B804C6625DB1341FF2BB731874B0BCC2FE6544E0BC9FF`.
The task bundle is consumed as a bundle; its internal models must not be
extracted into a replacement application pipeline. The verified native
runtime may be downloaded on first use through the binding's documented
MediaPipe 0.10.35 package path. Offline release packaging is a later task and
must not be represented as complete here.

## Concurrency rules

- Camera capture, inference, and Bevy ECS execute in separate ownership domains.
- Workers must never access Bevy `World`, `Commands`, `Entity`, or `Assets`.
- Frame transport must use capacity-one latest-value semantics.
- Do not use unbounded channels for frames or inference results.
- Do not spawn a thread per frame.
- Every worker must have a stop token, a retained join handle, and deterministic shutdown.
- Never detach a worker.
- Use monotonic timestamps and retain source frame sequence through the pipeline.

## Coordinates and semantics

The engine-independent convention is fixed:

- Unmirrored image right turn: positive yaw
- Chin up: positive pitch
- Clockwise tilt as viewed in the unmirrored image: positive roll
- Preview mirroring must not alter inference or tracking values
- Angles are radians unless a field name or documentation explicitly says degrees
- Expression weights are in `[0, 1]`

Any conversion to Bevy local transforms belongs in `vtuber-avatar` and requires synthetic sign tests.

## Error handling

- User input, camera failure, model failure, permission denial, and no-face conditions must not panic.
- Use typed errors with stable error codes from `DESIGN.md`.
- Preserve source chains for technical logs and show a separate user-facing message.
- Do not use `unwrap` or `expect` on external data or OS results.
- `expect` is permitted only for a proven internal invariant and must state the invariant.
- No-face is a normal tracking state, not an error.
- A malformed MediaPipe result is a typed output-contract error, not `NoFace`.
- MediaPipe face processing is on-device. Camera frames and landmarks must not
  be transmitted. The MediaPipe project may collect performance/utilization
  metrics; do not claim zero network activity without measurement.

## Unsafe code

Application crates should use `#![forbid(unsafe_code)]`.

If unsafe is unavoidable:

- isolate it in one platform module;
- add an ADR;
- document every safety invariant;
- add tests and a platform smoke procedure;
- do not broaden the unsafe surface while fixing an unrelated issue.

## Model provenance

- Do not commit or download a model without a manifest.
- Manifest must include original source, license, redistribution status, SHA-256, tensor contract, preprocessing, and output schema.
- Reject hash mismatch.
- Do not use an unverified converted model from an aggregation site.
- If conversion is necessary, version and record the converter and compare outputs against the original model.

## Testing

Each task must add tests appropriate to its layer.

Mandatory classes over the project lifecycle:

- LatestSlot and worker shutdown
- camera format selection and mock disconnect
- preprocessing and inference golden tests
- Kabsch synthetic rotations
- filter time invariance
- tracking lost/recovery
- VRM 1.0 inspection and non-VRM-1.0 rejection
- avatar binding retry and timeout
- head/neck pose mapping
- expression capability fallback
- Windows and macOS のローカル自動検証
- target model compatibility matrix
- soak and latency reports

Do not claim camera, MToon, permission, or performance success without an actual platform run.

## Documentation

- Keep `DESIGN.md`, `AI_AGENT_TASKS.md`, ADRs, and public APIs consistent.
- Record tested OS, hardware, model SHA, Bevy version, `bevy_vrm1` revision, and commands in compatibility/performance reports.
- Do not write unsupported claims such as “cross-platform” when only one OS was tested.

## Completion report

For each task report:

- task ID;
- files changed;
- important design decisions;
- tests and exact commands;
- Windows result;
- macOS result;
- measured values where relevant;
- known limitations;
- whether any design deviation or upstream issue remains.

Do not merge, push, publish a release, or modify upstream repositories unless the user explicitly requests it.
