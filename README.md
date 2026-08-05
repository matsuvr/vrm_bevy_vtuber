# Full-Rust VTuber / Bevy + bevy_vrm1 設計資料

基準日: 2026-08-04
設計版: 2.0

Webカメラで一人の顔を追跡し、VRM 1.0モデルを動かすWindows／macOS向けVTuberアプリの設計一式である。Bevy 0.19.0と固定revisionの`bevy_vrm1`を使用し、アプリ固有実装をcamera、pure-Rust inference、tracking math、VRM adapterへ分離する。

## 固定方針

- 対象formatはVRM 1.0のみ
- 対象OSはWindows 11とmacOS 13以降
- Bevyは`=0.19.0`
- `bevy_vrm1`はGit revision `f9593fd78136fb9e0507bcae111e09291ec9b82a`
- VRM load、Humanoid、MToon、Expression、SpringBone、Node Constraintは`bevy_vrm1`へ委譲
- 顔追跡結果だけを`vtuber-avatar`からVRM bone／Expressionへ適用
- camera、inference、Bevy main threadを分離
- frame transportは容量1件のlatest-value方式
- production inference runtimeはpure Rust
- macOS対応を初期CI、camera設計、package設計へ最初から含める

## 読む順序

1. `DESIGN.md` — 主設計書
2. `AGENTS.md` — Codexが常時守る拘束条件
3. `CODEX_TASKS.md` — PR単位の実装task
4. `docs/adr/` — 主要な設計判断
5. `REFERENCES.md` — 一次資料と固定source
6. `CODEX_BOOTSTRAP_PROMPT.md` — 最初のtaskを開始するprompt
7. `REVISION_NOTES.md` — 前版からの変更点

補助資料:

- `docs/MODEL_COMPATIBILITY_TEMPLATE.md`
- `docs/PERFORMANCE_TEST_PLAN.md`

## Codexでの使い方

設計一式をrepository rootへ配置し、最初に`CODEX_BOOTSTRAP_PROMPT.md`を渡す。以後は`CODEX_TASKS.md`からtask IDを一つだけ指定する。

Gate 0を省略しない。特に、顔modelのpure-Rust runtime互換性、対象VRMの`bevy_vrm1`互換性、Windows／macOS camera、macOS app bundle permissionを実測してから縦断MVPへ進む。

## 完成時の最小成果

- Windows／macOSで起動するdesktop app
- user-selected VRM 1.0のimport、表示、transactional replacement
- webcam capture、single-face tracking、head yaw／pitch／roll
- blinkとmouth-open expression
- calibration、tracking loss、neutral return
- stage別latency、FPS、drop、confidenceのdiagnostics
- reproducible compatibility reportとperformance report

## Workspace crates

| Crate | Responsibility |
|---|---|
| `vtuber-core` | Platform- and engine-independent data and synchronization contracts. |
| `vtuber-camera` | OS camera backends and capture worker. |
| `vtuber-inference` | Face model loading, preprocessing, and pure-Rust inference. |
| `vtuber-tracking` | Calibration, pose solving, filtering, and tracking state. |
| `vtuber-avatar` | Bevy and `bevy_vrm1` adapter. |
| `vtuber-app` | Orchestration, UI, settings, model import, and diagnostics. |
| `vtuber-desktop` | Desktop executable entry point. |
| `xtask` | Repository automation tasks. |

## Commands

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
```
