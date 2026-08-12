# Windows M1 Acceptance Report

**Status:** BLOCKED — live MediaPipe tracking, face loss/reacquire, Start/Stop, avatar replace/unload/reload, latency observation, and 30-minute soak were exercised; direct avatar motion and capture-to-apply latency remain unverified
**Date:** 2026-08-12
**Commit SHA:** `ee88dfa`
**Binary:** `vtuber-desktop` (release profile)

---

## 1. Test Environment

| Item | Value |
|------|-------|
| OS | Windows 11 Pro 10.0.26200 (build 26200) |
| CPU | 13th Gen Intel(R) Core(TM) i9-13900 |
| GPU | Virtual Desktop Monitor; driver 13.50.53.699 |
| RAM | NOT RECORDED — no hardware claim |
| Screen | NOT RECORDED — no hardware claim |
| Camera 1 | c922 Pro Stream Webcam — MSMF index 0 |
| Camera 2 (if available) | NOT RUN |
| Build profile | release |
| Rust toolchain | rustc 1.97.1 / cargo 1.97.1 |
| Binary SHA-256 | `69B71344032ABDB18C5DE1EAD785AB9ECFE98BBE75B4240B4470B94B70831C3E` |

### Model Manifest

| # | Model Name | Source | License | SHA-256 | VRM Version | Notes |
|---|-----------|--------|---------|---------|-------------|-------|
| 1 | MediaPipe Face Landmarker task bundle | Official MediaPipe model URL; ADR-009 | Apache-2.0 | `64184E229B263107BC2B804C6625DB1341FF2BB731874B0BCC2FE6544E0BC9FF` | n/a | CPU VIDEO mode; 478 landmarks, 52 blendshapes, one matrix |
| 2 | inore-vrm1.vrm | Local approved fixture | project fixture | `B5A3D4126C4A30EF3BFBCFC764A24DC48511B558799D98D4C2FF1DB0BDC7AB01` | VRM 1.0 | Imported in GUI; Ready |

The former UltraFace/Peppa entries and their hashes are retained only in the
historical composite evidence below. They are not the current production
backend.

### Previous MediaPipe GUI run (partial evidence)

The release binary was launched on Windows and exercised through the desktop
UI. C922 was enumerated and selected as `MSMF index 0`, `inore-vrm1.vrm` was
imported and reached `Avatar lifecycle: Ready`, and Start/Stop completed without
a process restart. Diagnostics while the camera was running showed:

```text
capture rate: 30.0 Hz
inference rate: 30.0 Hz
capture worker: Running
inference worker: Running
no-face frames: 447
landmark/tracking rate: 0.0 Hz
```

The input contained no detectable face during this run. Therefore no neutral
sample, Recenter, head pose, expression, gaze, face-loss/reacquire, avatar
replace, or unplug/reconnect result is claimed. The observed no-face state was
normal tracking state, not an inference failure.

### Current MediaPipe GUI run (live acceptance)

On 2026-08-12 the release binary was exercised with the user present in front
of the connected `c922 Pro Stream Webcam` (MSMF index 0). The approved
`inore-vrm1.vrm` fixture reached `Avatar lifecycle: Ready`. Auto-neutral
calibration completed, and Live showed:

```text
Tracking: Tracking
Confidence: 1.00
Face detected: yes
```

The six head-pose directions, blink, mouth-open, and left/right gaze prompts
were executed while the pipeline remained live. Face loss was observed as
`Face detected: no`, `Tracking: Initializing`, `Confidence: 0.00`; returning to
the camera reacquired `Face detected: yes`, `Tracking: Tracking`, and
`Confidence: 1.00` in the same process. Three Stop/Start cycles completed, and
avatar replace, unload (`Avatar lifecycle: None`), and reload (`Ready`) also
completed without a process restart. After reload, tracking reacquired again.

The UI did not expose a direct numeric avatar-apply counter or a
capture-to-apply latency value, and the viewport framing did not provide a
recordable head/expression visual result in this run. Therefore this evidence
does not claim the VRM head, blink, mouth, or gaze visual checks as PASS.

The 60-second diagnostics observation showed approximately:

```text
capture/inference: 30 Hz
tracking: 30-31 Hz
inference wait: p50 ~25 ms, p95 ~41.9 ms
inference total: p50 ~6.0 ms, p95 ~6.8 ms
capture-to-apply p50/p95: (none)
```

The subsequent 30-minute soak completed with the process responsive and the
workers still Running. Final diagnostics were `tracking 30.0 Hz`,
`slot overwrites 0`, `no-face frames 106`, inference wait p95 `41.46 ms`, and
inference total p95 `7.45 ms`. No crash, worker exit, or hang was observed.

---

## 2. Test Matrix

Automated verification completed on the environment above:

- `cargo fmt --all -- --check`
- `cargo check -p vtuber-desktop`
- `cargo clippy -p vtuber-app -p vtuber-avatar -p vtuber-camera -p vtuber-tracking --all-targets -- -D warnings`
- `cargo clippy -p vtuber-inference --all-targets -- -D warnings`
- `cargo run -p xtask -- acceptance verify assets/models/manifest.toml`
- `cargo test --workspace --no-fail-fast`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build -p vtuber-desktop --release`

The current live GUI run above covers the MediaPipe worker and recovery
evidence. The historical composite diagnostic below is retained only as
superseded evidence and was exercised on Windows with the connected
`c922 Pro Stream Webcam`:

```text
cargo run -p xtask -- face-pipeline-smoke --duration 10 --json
camera: msmf:0 (1280x720 @ 30/1 Rgb8)
frames_captured: 297
face_count/no_face_count: 0/30
detector_hz/landmark_hz: 2.836/0.000
stage_error: none
```

The detector was also exercised directly with the user-supplied local JPEG
`ookawa721_0I9A4938_TP_V4.jpg` (not committed to the repository):

```text
cargo run -p xtask -- face-image-probe ookawa721_0I9A4938_TP_V4.jpg
dimensions: 800x533
result: FACE_DETECTED
confidence: 0.9999
rect: (0.4314,0.1891,0.1228,0.2397)
```

This confirms that the pinned UltraFace artifact, JPEG/RGBA8 preprocessing,
and detector decode can detect a face in a still image. It does not substitute
for the live camera composite gate because it does not exercise crop,
landmark, or planar-pose output.

The first live MSMF run returned no face even though the captured frame showed
the user. A post-fix run found that the landmark model emitted visibility
values above 1.0; the ONNX landmark decoder now clamps visibility to the
engine contract `[0,1]`. The resulting 20-second C922 run was:

```text
cargo run -p xtask -- face-pipeline-smoke --camera 0 --duration 20 --snapshot <temp>/vrm-c922-snapshot-after-fix.jpg --json
format: 1280x720 @ 30/1 Rgb8
frames_captured: 593
face_count/no_face_count: 6/0
detector_hz/landmark_hz: 0.293/0.293
detector_confidence: 0.610928
finite_landmarks: 98
finite_pose_count: 5
pose yaw/pitch/roll ranges: [-1.402699,0.477511] [-1.695925,2.557055] [-1.651763,0.366612]
stage_error: none
```

The full M1-08-013 camera protocol was then run with the guided CUI mode. It
prints each phase and a one-second countdown for the last three seconds:

```text
cargo run -p xtask -- face-pipeline-smoke --camera 0 --guided-protocol --json
```

```text
camera: c922 Pro Stream Webcam (MSMF index 0)
format: 1280x720 @ 30/1 Rgb8
frames_captured: 4944
face_count/no_face_count: 50/19
detector_hz/landmark_hz: 0.387/0.347
stage_error: none
detector_confidence: 0.779927
finite_landmarks/finite_pose_count: 98/44
pose yaw/pitch/roll ranges: [-5.145444,2.186486] [-2.124861,4.208844] [-2.604939,1.199027]
```

The guided run emitted and executed the 60-second neutral, five loss/return
cycles, left/right/up/down, edge, and three capture Stop/Start phases. The
`no_face_count` is expected during the out-of-frame phases. The first run
returned exit code 1 after printing the complete summary because the exact
deadline bookkeeping did not mark the protocol complete; the command now
normalizes that boundary and has a regression test.

A later 60-second requested-duration run was executed after the release build
was rebuilt. The command completed the full guided protocol, including the
three capture Stop/Start phases, and returned exit code 0. It is recorded as a
composite camera result only: the low face count means it is not evidence for
GUI calibration or avatar motion.

```text
cargo run -p xtask -- face-pipeline-smoke --camera 0 --guided-protocol --duration 60 --json
camera: c922 Pro Stream Webcam (MSMF index 0)
format: 1280x720 @ 30/1 Rgb8
frames_captured: 4942
face_count/no_face_count: 12/418
detector_hz/landmark_hz: 2.450/0.074
stage_error: none
detector_confidence: 0.884251
finite_landmarks/finite_pose_count: 98/12
pose yaw/pitch/roll ranges: [-1.762825,1.524811] [-2.552518,8.091341] [-1.380595,1.612097]
```

The implementation gates for M1-08-015〜M1-08-017 also passed in this
environment: observation freshness/reset, real-source avatar bridge
contracts, synthetic-source exclusivity compile check, worker failure
recovery, diagnostics, and clean-shutdown unit/integration coverage. The
managed VRM compatibility command reached `Ready` for the approved local VRM
fixture. These automated results do not replace the GUI functional gate.

| Row | Model | Camera | Protocol | Result | Notes |
|-----|-------|--------|----------|--------|-------|
| 1 | Model 1 | Camera 1 | M1-08-013 guided camera | PASS | Full guided camera phases completed; GUI/VRM acceptance is recorded separately as NOT RUN |
| 2 | Model 2 | Camera 1 | Full | NOT RUN | No manifest model |
| 3 | Model 3 | Camera 1 | Full | NOT RUN | No manifest model |
| 4 | Model 1 | Camera 2 | Full | NOT RUN | No physical camera inventory |

Skip conditions:
- Camera 2 not available → rows 4+ marked SKIP with reason
- Model requires expressions not present → capability limitation noted, not failure

---

## 3. Functional Protocol (M1-08-003)

### 3.1 Neutral Calibration

- [x] Start app with model loaded
- [x] Face camera in neutral position
- [x] Auto-neutral calibration completed without a blocking calibration phase
- [x] Hold neutral pose until calibration completed
- [x] Live state reached `Tracking` with confidence `1.00`

### 3.2 Head Pose (yaw / pitch / roll)

| Axis | Direction | Expected | Actual | Result |
|------|-----------|----------|--------|--------|
| Yaw | Turn right (image right) | Head turns right | Prompt executed; viewport result not recorded | NOT VERIFIED |
| Yaw | Turn left | Head turns left | Prompt executed; viewport result not recorded | NOT VERIFIED |
| Pitch | Chin up | Head tilts up | Prompt executed; viewport result not recorded | NOT VERIFIED |
| Pitch | Chin down | Head tilts down | Prompt executed; viewport result not recorded | NOT VERIFIED |
| Roll | Tilt right (clockwise) | Head tilts right | Prompt executed; viewport result not recorded | NOT VERIFIED |
| Roll | Tilt left (counter-clockwise) | Head tilts left | Prompt executed; viewport result not recorded | NOT VERIFIED |

### 3.3 Blink

| Capability | Expected | Actual | Result |
|-----------|----------|--------|--------|
| Per-eye (blinkLeft + blinkRight) | Independent left/right blink | Prompt executed; viewport result not recorded | NOT VERIFIED |
| Combined (blink only) | Both eyes blink together | Prompt executed; viewport result not recorded | NOT VERIFIED |
| No blink preset | No blink response (not a failure) | Not applicable to selected fixture | NOT RUN |

### 3.4 Mouth

| Capability | Expected | Actual | Result |
|-----------|----------|--------|--------|
| Full (aa/ih/ou/ee/oh) | Vowel shapes respond | Prompt executed; viewport result not recorded | NOT VERIFIED |
| aa-only | Mouth opens with "aa" | Prompt executed; viewport result not recorded | NOT VERIFIED |
| No mouth preset | No mouth response (not a failure) | Not applicable to selected fixture | NOT RUN |

### 3.5 Gaze

| Mode | Expected | Actual | Result |
|------|----------|--------|--------|
| Expression (lookLeft/Right/Up/Down) | Eyes move with gaze | Prompts executed; viewport result not recorded | NOT VERIFIED |
| Eye bones | Eye bones rotate | Not separately isolated | NOT RUN |
| None | No gaze response (not a failure) | Not applicable to selected fixture | NOT RUN |

### 3.6 Functional Summary

- [x] No panic or fatal render issue observed
- [ ] Head pose three axes move in intended directions (viewport evidence missing)
- [ ] Blink/mouth respond according to capability (viewport evidence missing)
- [x] No unsupported-capability error was shown

---

## 4. Recovery Protocol (M1-08-004)

### 4.0 Automated recovery preflight

実機操作に先立つソフトウェア回復ゲートを 2026-08-11 に実施した。
これはWindows GUI／物理cameraの受入結果ではなく、回復契約と状態遷移を
自動検証した結果である。

| Recovery contract | Evidence | Result |
|---|---|---|
| Repeated inference result is not replayed forever | `vtuber-app` observation-gate tests | PASS |
| Stale inference becomes face loss after 250ms | `vtuber-app` observation-gate tests | PASS |
| Missing inference output enters normal no-face path | `vtuber-app` observation-gate tests | PASS |
| Lost hold / neutral decay / reacquisition | `vtuber-tracking` loss-recovery tests | PASS |
| Camera Stop removes retained frame | `vtuber-camera` capture tests | PASS |
| Camera reconnect preserves capture sequence | `vtuber-camera` capture tests | PASS |
| Inactive tracking clears retained avatar control frame | `vtuber-app` avatar-bridge test | PASS |

Executed commands:

```text
cargo test -p vtuber-core -p vtuber-camera -p vtuber-app --lib --tests
cargo clippy -p vtuber-core -p vtuber-camera -p vtuber-app --all-targets -- -D warnings
```

The live checklist below was exercised with one imported VRM 1.0 model. Camera
unplug/replug, thread-count measurement, and a timed two-second reacquisition
measurement were not performed.

### 4.1 Face Loss / Return

- [x] Move face out of frame
- [x] Verify: `Face detected: no`, `Tracking: Initializing`, confidence `0.00`
- [ ] Verify: avatar holds neutral or last pose (no direct viewport evidence)
- [x] Return face to frame
- [ ] Verify: tracking resumes within 2 seconds (not timed)
- [ ] Verify: no stale pose remains (no direct viewport evidence)

### 4.2 Camera Stop / Restart

- [x] Click "Stop" and verify lifecycle reached `Idle`
- [x] Click "Start" and verify lifecycle reached `Running`
- [x] Repeat Stop/Start three times in the same process
- [ ] If possible: unplug camera, replug
- [ ] Verify: app handles disconnect gracefully, reconnect works

### 4.3 Avatar Replace

- [x] While tracking, import a VRM replacement
- [x] Verify: lifecycle returned to `Ready` without a process restart
- [x] Verify: replacement model loads and binds
- [x] Verify: tracking continues after replacement/reload
- [ ] Verify: no stale state from old model (no direct viewport evidence)

### 4.4 Recovery Summary

- [ ] No permanent freeze on face loss (viewport evidence missing)
- [x] Camera restart recovers tracking
- [ ] Avatar replace leaves no stale state (viewport evidence missing)
- [ ] Thread count returns to baseline after each operation (not measured)

---

## 5. Latency & Rate (M1-08-005)

### 5.1 Measurement Setup

- Warm-up: first 10 seconds excluded
- Sample window: 60 seconds minimum
- Clock source: monotonic (same domain for all timestamps)

### 5.2 Results

| Metric | Target | Actual | Result |
|--------|--------|--------|--------|
| Render FPS | ≥ 30 | Not exposed in diagnostics | NOT RUN |
| Tracking Hz | ≥ 15 | 30-31 Hz | PASS |
| p50 capture-to-apply | — | `(none)` | NOT VERIFIED |
| p95 capture-to-apply | ≤ 180ms | `(none)` | NOT VERIFIED |
| Queue depth | ≤ 1 | Slot overwrites 0 | PASS |
| Slot overwrite count | — | 0 | PASS |

### 5.3 Raw Data

_Metrics CSV/JSON artifact path: not generated; diagnostics values were read
from the live release UI._

---

## 6. 30-Minute Soak (M1-08-006)

### 6.1 Setup

- Model: `inore-vrm1.vrm`
- Camera: `c922 Pro Stream Webcam`, MSMF index 0
- Duration: 30 minutes
- Sampling interval: every 60 seconds

### 6.2 Results

| Metric | Start | Mid | End | Trend |
|--------|-------|-----|-----|-------|
| RSS (MB) | Not measured | Not measured | Not measured | NOT VERIFIED |
| p95 latency (ms) | wait ~41.9 / total ~6.8 | wait ~41.8 / total ~7.8 | wait 41.46 / total 7.45 | stable sampled |
| Render FPS | Not exposed | Not exposed | Not exposed | NOT RUN |
| Tracking Hz | 30-31 | 30-31 | 30.0 | stable |
| Worker threads | Running | Running | Running | no exit observed |

### 6.3 Soak Summary

- [x] No process crash
- [ ] No memory continuous increase trend (RSS not measured)
- [x] No latency continuous increase trend in sampled diagnostics
- [x] Worker threads terminated on Stop (user confirmed Stop after soak)
- [ ] Clean shutdown confirmed (process remained open after Stop)

---

## 7. Acceptance Criteria Summary

| # | Criterion | Target | Actual | Verdict |
|---|-----------|--------|--------|---------|
| 1 | Render FPS | ≥ 30 | Not exposed | NOT RUN |
| 2 | Tracking Hz | ≥ 15 | 30-31 Hz | PASS |
| 3 | p95 capture-to-apply | ≤ 180ms | `(none)` | NOT VERIFIED |
| 4 | Queue depth | ≤ 1 | Slot overwrites 0 | PASS |
| 5 | No memory/latency increase | stable | latency stable sampled; RSS not measured | PARTIAL |
| 6 | No process crash | 0 crashes | 0 observed | PASS |
| 7 | Report saved | yes | recorded | PASS |

**Overall Gate:** BLOCKED — live tracking, loss/reacquire, lifecycle recovery, latency stage timing, and the 30-minute soak are evidenced, but direct avatar motion and capture-to-apply latency remain unverified.

---

## 8. Blocker Classification (M1-08-008)

| # | Blocker | Category | Severity | Fix Required | Blocks M1-09? |
|---|---------|----------|----------|-------------|---------------|
| 1 | Direct avatar head/expression/gaze visual result was not recorded | test-environment | High | Repeat the live protocol with viewport evidence or an avatar-apply metric | Yes |
| 2 | capture-to-apply diagnostics are `(none)` and RSS/thread-count artifacts were not exported | performance | High | Expose/export capture-to-apply and resource metrics, then rerun the gate | Yes |

Categories: correctness, compatibility, performance, hardware-specific, test-environment

**M1-09 Go/No-Go Decision:** HOLD — M1-08-019 and the broader Windows gate are not accepted; macOS work remains explicitly deferred.

---

## 9. Artifact Manifest

| Artifact | SHA-256 | Path |
|----------|---------|------|
| vtuber-desktop.exe | `69B71344032ABDB18C5DE1EAD785AB9ECFE98BBE75B4240B4470B94B70831C3E` | target/release/vtuber-desktop.exe |
| face_landmarker.task | `64184E229B263107BC2B804C6625DB1341FF2BB731874B0BCC2FE6544E0BC9FF` | assets/models/face_landmarker.task |
| inore-vrm1.vrm | `B5A3D4126C4A30EF3BFBCFC764A24DC48511B558799D98D4C2FF1DB0BDC7AB01` | tests/fixtures/vrm/inore-vrm1.vrm |
| Metrics CSV | — | not generated; live diagnostics recorded in this report |
| Soak metrics | — | no exported artifact; 30-minute live observation recorded |

---

_Report generated from the automated, live GUI, latency-observation, and
30-minute soak evidence above. Direct GUI avatar motion and capture-to-apply
latency remain intentionally unverified._
