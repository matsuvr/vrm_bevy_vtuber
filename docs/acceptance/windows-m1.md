# Windows M1 Acceptance Report

**Status:** BLOCKED — software gates and the still-image detector probe pass; the MSMF probe still saw no face, so the full face-in-frame protocol is NOT RUN
**Date:** 2026-08-11
**Commit SHA:** see `git rev-parse HEAD` for the commit containing this report
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
| Camera 1 | NOT RUN — no physical camera descriptor recorded |
| Camera 2 (if available) | NOT RUN |
| Build profile | release |
| Rust toolchain | rustc 1.97.1 / cargo 1.97.1 |
| Binary SHA-256 | `2C4ED40CDBB1B4E7DC39F55B62AF3875433D50943C0A5855BCD45FA2A6198BB9` |

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

This is a real MSMF capture and composite-runtime run, but it is not a face
success: no face was present in the sampled view. The 60-second neutral run,
face out/return protocol, directional movement, edge protocol, and three
Stop/Start repetitions remain NOT RUN.

| Row | Model | Camera | Protocol | Result | Notes |
|-----|-------|--------|----------|--------|-------|
| 1 | Model 1 | Camera 1 | Full | NOT RUN | Physical camera and GUI protocol unavailable in this session |
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

**Overall Gate:** BLOCKED — the still-image detector probe, detector/crop/landmark software gate, and real MSMF no-face run pass, but a face-in-frame run yielding finite 98 landmarks and planar pose has not been completed.

---

## 8. Blocker Classification (M1-08-008)

| # | Blocker | Category | Severity | Fix Required | Blocks M1-09? |
|---|---------|----------|----------|-------------|---------------|
| 1 | The 10-second MSMF probe captured frames but observed no face, while the still-image detector probe succeeded; detector→crop→landmark→pose could not be measured live | test-environment | High | Place a face in the selected camera view and execute the full M1-08-013-009 protocol | Yes |
| 2 | Physical Windows GUI, VRM motion, and 30-minute soak were not run in this session | test-environment | High | Execute the Windows acceptance protocol on target hardware | Yes |

Categories: correctness, compatibility, performance, hardware-specific, test-environment

**M1-09 Go/No-Go Decision:** HOLD — M1-08-013/019 are not accepted; macOS work remains explicitly deferred.

---

## 9. Artifact Manifest

| Artifact | SHA-256 | Path |
|----------|---------|------|
| vtuber-desktop.exe | `2C4ED40CDBB1B4E7DC39F55B62AF3875433D50943C0A5855BCD45FA2A6198BB9` | target/release/vtuber-desktop.exe |
| Model 1 | `73EDF90954F05EBEF4639E7FA8620C5F83CCA09D2476DE66AB100F26C2B25E7A` | assets/models/peppapig_student_1x3x256x256.onnx |
| Model 2 | — | not present |
| Model 3 | — | not present |
| Metrics CSV | — | not generated — protocol not run |
| Soak metrics | — | not generated — protocol not run |

---

_Report generated by acceptance template. Fill in values during actual test run._
