# Windows M1 Acceptance Report

**Status:** NOT_RUN  
**Date:** _pending_  
**Commit SHA:** _pending_  
**Binary:** `vtuber-desktop` (release profile)

---

## 1. Test Environment

| Item | Value |
|------|-------|
| OS | Windows 11 _version_ |
| CPU | _model_ |
| GPU | _model + driver version_ |
| RAM | _amount_ |
| Screen | _resolution_ |
| Camera 1 | _device name + descriptor_ |
| Camera 2 (if available) | _device name + descriptor_ |
| Build profile | release |
| Rust toolchain | _from rust-toolchain.toml_ |
| Binary SHA-256 | _sha256sum of vtuber-desktop.exe_ |

### Model Manifest

| # | Model Name | Source | License | SHA-256 | VRM Version | Notes |
|---|-----------|--------|---------|---------|-------------|-------|
| 1 | _name_ | _source_ | _license_ | _hash_ | 1.0 | _notes_ |
| 2 | _name_ | _source_ | _license_ | _hash_ | 1.0 | _notes_ |
| 3 | _name_ | _source_ | _license_ | _hash_ | 1.0 | _notes_ |

---

## 2. Test Matrix

| Row | Model | Camera | Protocol | Result | Notes |
|-----|-------|--------|----------|--------|-------|
| 1 | Model 1 | Camera 1 | Full | _PASS/FAIL/SKIP_ | |
| 2 | Model 2 | Camera 1 | Full | _PASS/FAIL/SKIP_ | |
| 3 | Model 3 | Camera 1 | Full | _PASS/FAIL/SKIP_ | |
| 4 | Model 1 | Camera 2 | Full | _PASS/FAIL/SKIP_ | _if available_ |

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
| Render FPS | ≥ 30 | _tbd_ | |
| Tracking Hz | ≥ 15 | _tbd_ | |
| p50 capture-to-apply | — | _tbd_ | |
| p95 capture-to-apply | ≤ 180ms | _tbd_ | |
| Queue depth | ≤ 1 | _tbd_ | |
| Slot overwrite count | — | _tbd_ | |

### 5.3 Raw Data

_Metrics CSV/JSON artifact path: _pending__

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
| 1 | Render FPS | ≥ 30 | _tbd_ | |
| 2 | Tracking Hz | ≥ 15 | _tbd_ | |
| 3 | p95 capture-to-apply | ≤ 180ms | _tbd_ | |
| 4 | Queue depth | ≤ 1 | _tbd_ | |
| 5 | No memory/latency increase | stable | _tbd_ | |
| 6 | No process crash | 0 crashes | _tbd_ | |
| 7 | Report saved | yes | _tbd_ | |

**Overall Gate:** _PASS / FAIL / CONDITIONAL_

---

## 8. Blocker Classification (M1-08-008)

| # | Blocker | Category | Severity | Fix Required | Blocks M1-09? |
|---|---------|----------|----------|-------------|---------------|
| | | | | | |

Categories: correctness, compatibility, performance, hardware-specific, test-environment

**M1-09 Go/No-Go Decision:** _pending_

---

## 9. Artifact Manifest

| Artifact | SHA-256 | Path |
|----------|---------|------|
| vtuber-desktop.exe | _hash_ | target/release/vtuber-desktop.exe |
| Model 1 | _hash_ | _path_ |
| Model 2 | _hash_ | _path_ |
| Model 3 | _hash_ | _path_ |
| Metrics CSV | _hash_ | _path_ |
| Soak metrics | _hash_ | _path_ |

---

_Report generated by acceptance template. Fill in values during actual test run._
