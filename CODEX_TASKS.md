# CODEX_TASKS.md

基準日: 2026-08-04  
対象: Windows／macOS、VRM 1.0、Bevy 0.19.0、`bevy_vrm1`

このファイルは`DESIGN.md`をCodexが安全に実装できるPR単位へ分割する。一度に一つのtask IDだけを実装する。

---

## 0. 全task共通の完了条件

- task外の機能を実装しない。
- `DESIGN.md`と`AGENTS.md`へ従う。
- public APIへ単位、値域、座標系、thread safetyを記す。
- user input、camera、model、VRM、configのfailureでpanicしない。
- 変更責務に対応するtestを同じPRへ入れる。
- `cargo fmt --all -- --check`を通す。
- 対象crateのClippyを`-D warnings`で通す。
- 対象testを通す。
- design deviationはADR amendmentを含める。
- 実行していないcommandを成功と報告しない。
- 完了報告にchanged files、commands、結果、受入条件、残課題を記す。

---

## 1. 依存グラフ

```text
G0-01
 ├─ G0-02 ─ G0-03 ─ G0-08
 ├─ G0-04
 ├─ G0-05
 ├─ G0-06
 └─ G0-07

G0-04 + G0-05 + G0-06 + G0-07
 └─ M1-01 ─ M1-02 ─ M1-03

G0-02 + G0-03 + G0-08
 └─ M1-04 ─ M1-05 ─ M1-06

M1-03 + M1-06
 └─ M1-07 ─ M1-08 ─ M1-09

M1-09
 ├─ Q2-01
 ├─ Q2-02
 ├─ Q2-03
 ├─ Q2-04
 └─ Q2-05

Q2-01 + Q2-03
 └─ R3-01
```

---

# Gate 0 — 不確実性を先に潰す

## G0-01: workspace、toolchain、品質基盤

### 目的

Windows／macOS専用workspaceと依存方向を作り、VRM処理を`bevy_vrm1`へ集約する基盤を固定する。

### 作業

- root `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`（task実行時にBevy 0.19と全依存で検証したexact stable toolchainを固定）
- `rustfmt.toml`
- `deny.toml`
- crates:
  - `vtuber-core`
  - `vtuber-camera`
  - `vtuber-inference`
  - `vtuber-tracking`
  - `vtuber-avatar`
  - `vtuber-app`
  - `apps/desktop`（package名`vtuber-desktop`）
  - `tools/xtask`（package名`xtask`）
- workspace lints
- Windows／macOS CI skeleton
- forbidden dependency check skeleton
- `vtuber-core`へplaceholder typesのみ
- root READMEへcrate責務

### 制約

- Windows／macOS以外のapp crate、backend、feature、CI jobを作らない。
- camera、inference、Bevy、`bevy_vrm1`の実依存はまだ追加しない。
- `unsafe`は原則禁止。

### 受入条件

- 全crateが空でもbuild／testできる。
- dependency cycleがない。
- `vtuber-core`がplatform／Bevy非依存。
- CI matrixがWindowsとmacOSだけ。
- lockfileがcommit対象。
- `rustc -Vv`を完了報告へ記録し、toolchainがexactに固定される。

### 検証

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

---

## G0-02: Bevy 0.19 + pinned bevy_vrm1 baseline

依存: G0-01

### 目的

Bevy 0.19.0と固定revisionの`bevy_vrm1`でVRM 1.0 sampleを表示する最小desktop appを作る。

### 作業

- `bevy = =0.19.0`
- `bevy_vrm1`を`DESIGN.md`記載revisionへ固定
- `DefaultPlugins` + `VrmPlugin`
- camera、light、groundまたはbackground
- repository内fixtureを`VrmHandle`でload
- `Vrm`／`Initialized`検知
- `HeadBoneEntity`存在をlog／UIへ出す
- release／debug profile設定

### 非対象

- user-selected absolute path
- Webカメラ
- 顔推論
- bone制御
- Expression制御

### 受入条件

- VRM 1.0 sampleがWindowsとmacOSでbuild対象になる。
- VRM runtime dependencyが`bevy_vrm1`だけである。
- MToon表示は`bevy_vrm1`へ委譲されている。
- app側に独自VRM schema／loaderがない。

### 検証

```bash
cargo tree -p vtuber-desktop
cargo check -p vtuber-desktop
cargo test -p vtuber-avatar
cargo clippy -p vtuber-avatar -p vtuber-desktop --all-targets -- -D warnings
```

手動:

```text
VRM sampleを表示し、head bone capabilityを確認する
```

---

## G0-03: user asset sourceとVRM 1.0 preflight

依存: G0-02

### 目的

任意pathを無制限にAssetServerへ許可せず、安全にVRM 1.0をapplication-managed asset sourceへ取り込む。

### 作業

- project data directory resolver
- named asset source `user`
- import service
- file size上限
- SHA-256
- temp write + atomic rename
- `gltf`／`serde_json::Value`によるtolerant preflight
- VRM 1.0判定
- meta summary
- required `hips`／`head`検査
- external URI検査
- typed errors
- CLI `--avatar`またはBevy file-dropからimport request

### 非対象

- VRM runtime再実装
- native file dialog
- recent list

### 受入条件

- `user://avatars/<sha256>/model.vrm`から`VrmHandle`をloadできる。
- `VRMC_vrm`を持たないfileは`MODEL_NOT_VRM1`で拒否される。
- invalid GLB、directory、symlink、external URIを安全に扱う。
- default 256 MiBのsize limitと、変更不能な1 GiB hard capを検証する。
- `UnapprovedPathMode::Allow`をglobalに設定しない。
- 同一fileの再importが重複copyを作らない。

### test

- valid VRM 1.0
- `VRMC_vrm`を持たないlegacy fixture
- invalid bytes
- missing head
- oversized metadata-only fake
- hash idempotency
- path traversal相当

---

## G0-04: Windows／macOS camera smoke

依存: G0-01

### 目的

`nokhwa 0.10.11` native backendでdevice列挙、format選択、RGB frame取得が両OSで成立することを確認する。

### 作業

- `nokhwa`をtarget-specificに追加する（Windows: `input-msmf`、macOS: `input-avfoundation`。必要なdecode featureだけを有効化）
- `CameraDevice`／`CameraFormatInfo` domain type
- camera enumeration CLIまたはdebug UI
- deterministic format selection
- 10frame取得smoke command
- RGB8 owned frame
- backend／stream objectをsmoke worker内でconstruct・open・capture・stop・drop
- macOS `nokhwa_initialize`
- cameraなしでも通常unit testが成功する構成

### 非対象

- long-running worker
- preview
- inference
- reconnect

### 受入条件

- WindowsでMSMF backendが選択される。
- macOSでAVFoundation backendが選択される。
- camera indexだけをpersistent keyにしない。
- 1280×720／30fpsを第一候補、640×480／30fpsをfallbackとするformat選定testがある。
- hardware testは`#[ignore]`またはxtaskで明示実行できる。

### 検証

```bash
cargo test -p vtuber-camera
cargo test -p vtuber-camera -- --ignored
```

macOS手動:

- bundle外smokeの限界を記録
- permission状態を記録

---

## G0-05: face model provenanceとpure-Rust runtime gate

依存: G0-01

### 目的

使用model、runtime、tensor契約、license、hashを確定し、Windows／macOSで同じgolden inputを実行する。

### 作業

- `assets/models/manifest.toml`
- source、version、license、SHA-256
- model fetch／verify xtask
- detector／landmark／必要ならblendshape artifact抽出
- `tract-tflite 0.23.0` load／optimize／run probe
- operator inventory
- input shape／dtype／layout
- output names／indices／shape
- fixed inputのgolden output
- unsupported operatorのtyped report
- ADR-001更新

### 分岐

TFLiteがblockerの場合:

- app実装へ進まず、exact operatorとreproductionを記録
- `tract-onnx 0.23.4`候補を別commitで検証
- C runtimeへ切り替えない

### 受入条件

- model file名だけでなく完全contractがmanifestにある。
- Windows／macOSで同じmodelをloadできる。
- golden output toleranceが文書化される。
- license fileが保存される。
- model hash mismatchで起動を続行しない。

---

## G0-06: canonical coordinateとhead pose proof

依存: G0-01

### 目的

neutral-relative weighted Kabschとcoordinate mappingを、実写に依存せず数値testで固定する。

### 作業

- canonical coordinate types
- stable landmark subset contract
- weighted centering
- SVD／reflection correction
- rotation matrix → quaternion
- model coordinate → canonical basis
- synthetic point cloud fixtures
- noise／missing point handling
- degeneracy error

### 受入条件

- yaw／pitch／rollの符号が`DESIGN.md`と一致する。
- 既知rotationを小さな誤差で復元する。
- reflectionをrotationとして採用しない。
- insufficient／collinear pointsでtyped error。
- mirror preview flagがmathへ入らない。

### 検証

```bash
cargo test -p vtuber-tracking pose
cargo test -p vtuber-core coordinate
```

---

## G0-07: LatestSlotとworker shutdown proof

依存: G0-01

### 目的

capacity 1のdata pathと、停止可能なworker controllerを先に完成させる。

### 作業

- `LatestSlot<T>`
- sequence
- wait timeout
- close
- overwrite metric
- bounded control channel
- generic worker supervisor test double
- named thread
- explicit stop／join
- disconnect／panic status

### 受入条件

- 10万publishしても保持件数が1を超えない。
- slow consumerが最新値へ追いつく。
- close時にwaiterが解除される。
- shutdown testがhangしない。
- data channelをunboundedにしない。

---

## G0-08: bevy_vrm1 compatibility gate

依存: G0-02、G0-03

### 目的

固定revisionの`bevy_vrm1`を実利用予定のVRM 1.0に対して検証し、fork patchの要否を判断する。

### 対象model

1. VRM specification公式sample
2. VRoid Studio export
3. 実利用予定model

### 検査

- import／preflight
- load／Initialized
- head／neck／eyes
- Expression一覧
- MToon
- outline
- SpringBone
- duplicate material name
- transparent material
- lookAt type
- missing optional field variant
- Windows／macOS

### 作業

- compatibility report templateへ記録
- known failureを最小fixture化
- upstream source／issue確認
- blockerの場合だけfork patch proposal
- ADR-002をAcceptedまたはAmendedにする

### 受入条件

- 各modelのSHA-256とexporterが記録される。
- valid VRMでpanicした場合はstack traceとtestがある。
- `LookAt`／`BodyTracking`をproduct pathで使わないことを確認する。
- forkするか否かが明文化される。

---

# Milestone 1 — 縦断MVP

## M1-01: production capture service

依存: G0-04、G0-07

### 目的

camera lifecycle、capture worker、metrics、reconnectを実装する。

### 作業

- `CaptureController`
- start／stop／select device
- capture worker loop
- `LatestSlot<VideoFrame>`
- format report
- capture／decode timing
- reconnect state machine
- device removal handling
- clean shutdown

### 受入条件

- UI threadをblockしない。
- live camera objectをmain threadからworkerへmoveしない。
- stop後にthreadがjoinされる。
- camera抜去でprocessが落ちない。
- frame queueが1を超えない。
- reconnect上限がある。

---

## M1-02: production inference worker

依存: M1-01、G0-05

### 目的

最新camera frameだけを処理し、`RawFaceObservation`をpublishする。

### 作業

- model descriptorだけをworkerへ渡し、runtime objectをworker startup内でconstruct／load
- preprocess buffer再利用
- detector cadence
- ROI state
- landmark decode
- basic blink／mouth observation
- inference timing
- drop accounting
- typed failure
- stop／join

### 受入条件

- same frameを重複推論しない。
- camera 30fps、inference 15fpsでもlatency backlogが増えない。
- model load failureをmainへ報告する。
- live inference runtime objectをmain threadからworkerへmoveしない。
- model objectを毎frame再構築しない。

---

## M1-03: calibration、filter、loss recovery

依存: M1-02、G0-06

### 目的

raw observationを安定した`AvatarControlFrame`へ変換する。

### 作業

- calibration collector
- neutral reference
- head rotation filter
- blink／mouth normalization
- confidence hysteresis
- Searching／Acquiring／Tracking／Degraded／LostHold／ReturningNeutral
- hold／neutral decay／recovery blend
- settings
- deterministic replay test

### 受入条件

- calibration不足を保存しない。
- lost時にlast poseへ永久固着しない。
- quaternion境界jumpがない。
- recorded observation replayがdeterministic。

---

## M1-04: avatar lifecycleとcapability discovery

依存: G0-03、G0-08

### 目的

一体のactive VRMを安全にload／unloadし、bone／Expression capabilityを構築する。

### 作業

- `VtuberAvatarPlugin`
- avatar lifecycle resource
- import result → `VrmHandle`
- `Initialized`検知
- required head
- optional neck／eyes
- `ExpressionEntityMap`
- `AvatarCapabilities`
- unload cleanup
- error surface

### 受入条件

- active avatarは一体。
- model差し替え時にold entity／stateが残らない。
- headなしをtyped error。
- capability discoveryを毎frame繰り返さない。
- `bevy_vrm1`型はavatar crate外へ漏れない。

---

## M1-05: tracked head／neck pose integration

依存: M1-03、M1-04

### 目的

`AvatarControlFrame.head`を、VRM 1.0の任意rest rotationを保ったままhead／neckへ適用する。

### 作業

- semantic pose → VRM 1.0 model-space quaternion
- binding時のroot／bone rest orientation cache
- model-space deltaからbone-local deltaへの共役変換
- head／neck distribution
- range clamp
- 毎frame rest poseから再計算してdrift防止
- system order after `AnimationSystems`
- before `VrmSystemSets::Constraints`
- avatar unload cleanup
- ADR-004の符号と式をsynthetic testへ固定

### 受入条件

- neutralで`RestTransform`を保つ。
- tracking deltaが累積driftしない。
- non-identity rest rotationを持つfixtureでもyaw／pitch／roll方向が正しい。
- neckなしmodelではheadだけで動作する。
- MVPへanimation base detectionを持ち込まない。
- synthetic integration testがある。

---

## M1-06: blink、mouth、gaze integration

依存: M1-04、M1-05

### 目的

Expression capabilityに応じてblink／mouth／gazeを適用する。

### 作業

- one `ModifyExpressions` event per avatar per frame
- blink fallback
- `aa` fallback
- gaze expression mapping
- eye bone gaze fallback
- dead zone／clamp
- change epsilon
- product pathへ`LookAt`／`BodyTracking`を入れないtest

### 受入条件

- 左右blink対応modelで左右別に動く。
- `blink`のみでも動く。
- mouth presetなしでpanicしない。
- gaze modeがcapabilityとして表示される。
- gaze systemがVRM schedule順に入る。

---

## M1-07: desktop UI、preview、diagnostics

依存: M1-03、M1-06

### 目的

setupからlive動作までを一つのdesktop appとして操作可能にする。

### 作業

- `bevy_egui`
- Setup／Live／Diagnostics
- file drop
- camera select
- start／stop
- calibration
- preview texture
- mirror preview
- lifecycle indicator
- FPS／latency／drop metrics
- error display

### 受入条件

- UI systemからcamera APIを直接呼ばない。
- preview textureを再利用する。
- preview OFFでもtrackingが継続する。
- mirror ON/OFFでtracking数値が変わらない。
- errorから再操作できる。

---

## M1-08: Windows vertical acceptance

依存: M1-07

### 目的

Windows 11でMVP縦断動作と30分安定性を確認する。

### protocol

- VRM三種類
- camera二種類があれば両方
- neutral calibration
- yaw／pitch／roll
- blink
- mouth
- face loss／return
- camera stop／restart
- avatar replace
- 30分run

### 受入条件

- render最低30fps
- tracking最低15Hz
- p95 capture-to-apply 180ms以下
- queue 1以下
- memory／latency増加傾向なし
- process crashなし
- report保存

---

## M1-09: macOS vertical acceptance

依存: M1-08

### 目的

macOS `.app`形態でcamera permissionを含む同等縦断動作を確認する。

### 作業

- minimal `.app` bundle
- `Info.plist`
- `NSCameraUsageDescription`
- resource locator
- Apple Silicon build
- camera permission flow
- M1-08と同じprotocol

### 受入条件

- app自身にcamera permissionが付与される。
- AVFoundation captureが動く。
- MToon／SpringBoneに致命的差がない。
- same compatibility report format。
- 30分runを通す。

---

# Quality 2 — 完成度向上

## Q2-01: 5母音と表情品質

依存: M1-09

### 目的

backendが提供するblendshapeを検証し、5母音とより自然なblink／gazeへ拡張する。

### 作業

- blendshape output contract
- neutral baseline
- 5 vowel normalization
- coarticulation smoothing
- confidence gating
- expression conflict test
- UI calibration values

### 受入条件

- modelごとのoutput indexをhard-codeせずmanifestへ置く。
- mouth weights合計／clamp方針がtest済み。
- unsupported backendではMVP fallbackが維持される。

---

## Q2-02: settings、recent avatar、import UX

依存: M1-09

### 作業

- versioned `config.toml`
- atomic write
- broken config backup
- per-camera calibration
- recent avatar
- missing file cleanup
- optional native file dialog

### 受入条件

- user dataをapp bundleへ書かない。
- camera indexだけを保存しない。
- schema migration test。

---

## Q2-03: performance tuningとlatency budget

依存: M1-09

### 作業

- fixed-size latency histogram
- per-stage timing
- allocation profile
- frame buffer reuse
- preview throttle
- detector cadence tuning
- tract thread behavior計測
- Windows／macOS comparison

### 受入条件

- 最適化前後report。
- profile根拠のないpool／unsafeを追加しない。
- Windows: tracking 25Hz以上、capture-to-apply p95 110ms以下を目標。
- macOS Apple Silicon: tracking 25Hz以上、capture-to-apply p95 120ms以下を目標。
- 未達の場合はstage別blockerを数値で示し、ADR-001または性能目標を明示的にamendする。

---

## Q2-04: release packaging

依存: M1-09

### 作業

- ADR-007を実測結果に基づいてAccepted、Amended、またはSupersededへ更新
- `xtask package-windows`
- portable zip
- `xtask package-macos`
- `.app`
- resources／licenses
- version metadata
- hash manifest
- ad-hoc signing instructions

### 受入条件

- install directory以外から起動してresourceを見つける。
- macOS camera permission文字列が入る。
- model hash検査が通る。
- source license一覧を同梱する。

---

## Q2-05: release hardeningとprivacy audit

依存: M1-09

### 作業

- debug frame feature audit
- log redaction
- forbidden dependency scan
- path／size fuzz-like tests
- worker panic handling
- camera reconnect edge cases
- release profile
- crash-free shutdown

### 受入条件

- release buildでpixel保存pathが無効。
- camera image／landmarkが通常logへ出ない。
- unexpected worker exitがUIへ出る。
- `cargo deny`とlicense review成功。

---

# Research 3 — 自由研究としての評価

## R3-01: smoothingとlatencyの比較実験

依存: Q2-01、Q2-03

### 目的

「滑らかさ」と「反応速度」のtrade-offを再現可能に比較する。

### 比較候補

- fixed exponential smoothing
- One Euro filter
- quaternion log-space adaptive filter
- output-only slerp

### protocol

- 同じrecorded observation stream
- slow sine motion
- step rotation
- fast turn
- noisy neutral
- blink pulse
- face loss／return

### 指標

- RMS jitter
- 10–90% rise time
- overshoot
- phase lag
- p50／p95 capture-to-apply
- subjective visual note

### 成果物

- `docs/experiments/filter-comparison.md`
- raw CSV
- reproducible command
- recommended defaults
