# Windows M1 Acceptance Report

**Status:** BLOCKED — the C922 composite/guided camera protocol and automated M1-08-015〜017 gates pass; GUI/VRM motion, performance, and 30-minute soak acceptance remain NOT RUN
**Date:** 2026-08-11
**Commit SHA:** `c193e1bc96b148d9ece041e879bf970f246a881b`
**Binary:** `vtuber-desktop` (release profile)

---

## 1. Test Environment

| Item | Value |
|------|-------|
| OS | Windows 11 Pro 10.0.26200 (build 26200) |
| CPU | NOT RECORDED — no hardware claim |
| GPU | NOT RECORDED — no hardware claim |
| RAM | NOT RECORDED — no hardware claim |
| Screen | NOT RECORDED — no hardware claim |
| Camera 1 | c922 Pro Stream Webcam — MSMF index 0 |
| Camera 2 (if available) | NOT RUN |
| Build profile | release |
| Rust toolchain | rustc 1.97.1 / cargo 1.97.1 |
| Binary SHA-256 | `97F82B36E7C97C621C7C41672220F828ECDD971D9593CF59A35871AE66E34F00` |

### Model Manifest

| # | Model Name | Source | License | SHA-256 | VRM Version | Notes |
|---|-----------|--------|---------|---------|-------------|-------|
| 1 | UltraFace RFB-320 | ONNX Model Zoo / Hugging Face mirror | MIT | `34CD7E60AEFF28744C657DE7A3DC64E872D506741DE66987F3426F2B79F88017` | n/a | Full-frame detector, `[1,3,240,320]` |
| 2 | Peppa_Pig_Face_Landmark student 256x256 | upstream GitHub + PINTO model zoo | Apache-2.0 | `73EDF90954F05EBEF4639E7FA8620C5F83CCA09D2476DE66AB100F26C2B25E7A` | n/a | Crop landmark model, `[1,98,3]` |
| 3 | — | — | — | — | n/a | NOT RUN |

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

The GUI import, VRM motion, and 30-minute soak protocol below were not run in
this agent session. The composite diagnostic was exercised on Windows with
the connected `c922 Pro Stream Webcam`:

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

- [ ] Start app with model loaded
- [ ] Face camera in neutral position
- [ ] Click "Begin Calibration"
- [ ] Hold neutral pose until complete
- [ ] Verify: tracking responds to head movement from neutral

### 3.2 Head Pose (yaw / pitch / roll)

| Axis | Direction | Expected | Actual | Result |
|------|-----------|----------|--------|--------|
| Yaw | Turn right (image right) | Head turns right | | |
| Yaw | Turn left | Head turns left | | |
| Pitch | Chin up | Head tilts up | | |
| Pitch | Chin down | Head tilts down | | |
| Roll | Tilt right (clockwise) | Head tilts right | | |
| Roll | Tilt left (counter-clockwise) | Head tilts left | | |

### 3.3 Blink

| Capability | Expected | Actual | Result |
|-----------|----------|--------|--------|
| Per-eye (blinkLeft + blinkRight) | Independent left/right blink | | |
| Combined (blink only) | Both eyes blink together | | |
| No blink preset | No blink response (not a failure) | | |

### 3.4 Mouth

| Capability | Expected | Actual | Result |
|-----------|----------|--------|--------|
| Full (aa/ih/ou/ee/oh) | Vowel shapes respond | | |
| aa-only | Mouth opens with "aa" | | |
| No mouth preset | No mouth response (not a failure) | | |

### 3.5 Gaze

| Mode | Expected | Actual | Result |
|------|----------|--------|--------|
| Expression (lookLeft/Right/Up/Down) | Eyes move with gaze | | |
| Eye bones | Eye bones rotate | | |
| None | No gaze response (not a failure) | | |

### 3.6 Functional Summary

- [ ] No panic or fatal render issue
- [ ] Head pose three axes move in intended directions
- [ ] Blink/mouth respond according to capability
- [ ] Unsupported capabilities show as limitations, not errors

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

The physical checklist below remains `NOT RUN` until the release binary is
tested with a Windows camera and two imported VRM 1.0 models. In particular,
the automated preflight does not prove camera unplug/replug, GUI avatar
replacement, thread-count stability, or the two-second reacquisition target.

### 4.1 Face Loss / Return

- [ ] Move face out of frame
- [ ] Verify: tracking state → Lost
- [ ] Verify: avatar holds neutral or last pose (no freeze)
- [ ] Return face to frame
- [ ] Verify: tracking resumes within 2 seconds
- [ ] Verify: no stale pose remains

### 4.2 Camera Stop / Restart

- [ ] Click "Stop"
- [ ] Verify: all workers stop, tracking state → Idle
- [ ] Click "Start"
- [ ] Verify: tracking resumes
- [ ] If possible: unplug camera, replug
- [ ] Verify: app handles disconnect gracefully, reconnect works

### 4.3 Avatar Replace

- [ ] While tracking, import a different model
- [ ] Verify: old model is fully despawned
- [ ] Verify: new model loads and binds
- [ ] Verify: tracking continues with new model
- [ ] Verify: no stale state from old model

### 4.4 Recovery Summary

- [ ] No permanent freeze on face loss
- [ ] Camera restart recovers tracking
- [ ] Avatar replace leaves no stale state
- [ ] Thread count returns to baseline after each operation

---

## 5. Latency & Rate (M1-08-005)

### 5.1 Measurement Setup

- Warm-up: first 10 seconds excluded
- Sample window: 60 seconds minimum
- Clock source: monotonic (same domain for all timestamps)

### 5.2 Results

| Metric | Target | Actual | Result |
|--------|--------|--------|--------|
| Render FPS | ≥ 30 | NOT RUN | |
| Tracking Hz | ≥ 15 | NOT RUN | |
| p50 capture-to-apply | — | NOT RUN | |
| p95 capture-to-apply | ≤ 180ms | NOT RUN | |
| Queue depth | ≤ 1 | NOT RUN | |
| Slot overwrite count | — | NOT RUN | |

### 5.3 Raw Data

_Metrics CSV/JSON artifact path: not generated — protocol not run._

---

## 6. 30-Minute Soak (M1-08-006)

### 6.1 Setup

- Model: _selected model_
- Camera: _selected camera_
- Duration: 30 minutes
- Sampling interval: every 60 seconds

### 6.2 Results

| Metric | Start | Mid | End | Trend |
|--------|-------|-----|-----|-------|
| RSS (MB) | | | | |
| p95 latency (ms) | | | | |
| Render FPS | | | | |
| Tracking Hz | | | | |
| Worker threads | | | | |

### 6.3 Soak Summary

- [ ] No process crash
- [ ] No memory continuous increase trend
- [ ] No latency continuous increase trend
- [ ] Worker threads terminate cleanly on Stop
- [ ] Clean shutdown confirmed

---

## 7. Acceptance Criteria Summary

| # | Criterion | Target | Actual | Verdict |
|---|-----------|--------|--------|---------|
| 1 | Render FPS | ≥ 30 | NOT RUN | NOT RUN |
| 2 | Tracking Hz | ≥ 15 | NOT RUN | NOT RUN |
| 3 | p95 capture-to-apply | ≤ 180ms | NOT RUN | NOT RUN |
| 4 | Queue depth | ≤ 1 | NOT RUN | NOT RUN |
| 5 | No memory/latency increase | stable | NOT RUN | NOT RUN |
| 6 | No process crash | 0 crashes | NOT RUN | NOT RUN |
| 7 | Report saved | yes | recorded | PASS |

**Overall Gate:** BLOCKED — M1-08-013 camera correctness is PASS, but GUI/VRM functional checks, latency measurements, and the 30-minute soak have not been completed.

---

## 8. Blocker Classification (M1-08-008)

| # | Blocker | Category | Severity | Fix Required | Blocks M1-09? |
|---|---------|----------|----------|-------------|---------------|
| 1 | The M1-08-013 camera protocol is complete, but the broader Windows GUI/VRM acceptance has not been run | test-environment | High | Execute the Windows functional, recovery, latency, and soak protocol | Yes |
| 2 | Physical Windows GUI, VRM motion, and 30-minute soak were not run in this session | test-environment | High | Execute the Windows acceptance protocol on target hardware | Yes |

Categories: correctness, compatibility, performance, hardware-specific, test-environment

**M1-09 Go/No-Go Decision:** HOLD — M1-08-019 and the broader Windows gate are not accepted; macOS work remains explicitly deferred.

---

## 9. Artifact Manifest

| Artifact | SHA-256 | Path |
|----------|---------|------|
| vtuber-desktop.exe | `97F82B36E7C97C621C7C41672220F828ECDD971D9593CF59A35871AE66E34F00` | target/release/vtuber-desktop.exe |
| Model 1 | `73EDF90954F05EBEF4639E7FA8620C5F83CCA09D2476DE66AB100F26C2B25E7A` | assets/models/peppapig_student_1x3x256x256.onnx |
| Model 2 | — | not present |
| Model 3 | — | not present |
| Metrics CSV | — | not generated — protocol not run |
| Soak metrics | — | not generated — protocol not run |

---

_Report generated from the automated Windows evidence above. GUI/VRM motion,
latency export, and the 30-minute soak remain intentionally unfilled._
