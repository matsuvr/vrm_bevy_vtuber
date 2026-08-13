# AI_AGENT_TASKS.md

基準日: 2026-08-04
subtask細分化改訂日: 2026-08-07
Windows縦断再計画改訂日: 2026-08-09
M1-08-013 blocker突破計画改訂日: 2026-08-11
対象: Windows 11を現在の開発・受入対象、macOSを保留中の次期対象、VRM 1.0、Bevy 0.19.0、`bevy_vrm1`

このファイルは`DESIGN.md`を実装単位へ分割する。親task ID（`G0-XX`、`M1-XX`、`Q2-XX`、`R3-XX`）は既存の進捗・PR・履歴を維持するため変更しない。実際のコーディングエージェントへの委嘱単位は、通常は`M1-02-001`、correctness blockerのrepair branchでは`M1-08-013-001`のようなleaf subtask IDとする。

---

## 0. subtask運用規約

### 0.1 進捗引継ぎ

基準日: 2026-08-11
repository基準: `main`が少なくとも次を含むこと。

- `4f7a4e7cc5ff3dacdd221c9c6d7b7f75df1636b8`: M1-08 acceptance基盤・report・metrics scaffold
- `f69c41d52c261a3e246384f84ae5174f6d4a73da`: `bevy_egui`によるSetup／Live／Diagnostics GUI補完
- `ab6b0b6daf9536c6e595f7e6e97c6c7168adc501`: GUI importからmanaged asset／avatar lifecycleへの接続
- `0c5294703485f0658a3cdf9bc0e9253f903af46a`: dev-only synthetic tracking
- `01dce2753ef483069c0eb83f7b740b3814e65349`: Windows MSMF camera backend
- `bf32be62cb1b5f386b18dd7215d8b3195272d923`: inference／tracking runtime、planar pose、diagnostics、monotonic timing等を含む現在の基準

確認済みrelease binary SHA-256:

```text
69B71344032ABDB18C5DE1EAD785AB9ECFE98BBE75B4240B4470B94B70831C3E
```

| 範囲 | 状態 | 扱い |
| --- | --- | --- |
| `G0-01`〜`M1-06` | `LEGACY_PROGRESS` | 既存実装を正とし、親task全体を再実装しない。不足修正時だけ該当subtaskを監査して差分を実装する。 |
| `M1-07` | `LEGACY_PROGRESS` | action／view-model／orchestrator境界と`bevy_egui 0.41.1` GUIが実装済み。GUI frameworkを再選定・再実装しない。 |
| `M1-08-001`〜`M1-08-008` | `LEGACY_PROGRESS` | acceptance用の文書・matrix・metrics基盤まで実装済み。ただしWindows物理受入は未実施であり、PASSと解釈しない。 |
| `M1-08-009`〜`M1-08-012` | `DONE` | GUI import、avatar lifecycle、synthetic tracking、Windows camera契約、capture／preview接続まで実装・自動検証済み。 |
| `M1-08-013` | `DONE` | UltraFace→crop→Peppa 98点landmark→planar poseのWindows C922実機gateを、face loss／return、方向、edge、capture Stop／Startを含むguided protocolで確認済み。 |
| `M1-08-014` | `DONE` | managed VRM import／avatar lifecycleの既存blockerを解消し、自動互換性検査済み。 |
| `M1-08-015` | `DONE` | MediaPipe rewrite、C922 functional／recovery gate、bounded performance gateを完了した。旧composite trackingの実機証拠は新backendの受入証拠として再利用しない。 |
| `M1-08-016` | `DONE` | MediaPipe canonical tracking sourceからreal-VRM bridgeへの接続、generation／stale排他、synthetic排他を自動検証済み。 |
| `M1-08-017` | `DONE` | MediaPipe identity／contract diagnostics、worker recovery、reverse shutdown、retry、no-face通常状態を自動検証済み。 |
| `M1-08-018` | `DONE` | C922 symbolic-link選択、実preview、real-VRM head／blink／mouth／gaze、Stop／Start 3回、既存のloss／reacquire・replug・replace証拠を統合してfunctional／recovery gateを完了した。 |
| `M1-08-019` | `DONE` | release appで31点のbounded metrics／resource exportを完了し、render、tracking、capture-to-apply、capacity-one、resource stability、clean shutdownのWindows final gateをPASSした。 |
| `M1-08-020` | `DONE` | Live preview texture登録とavatar骨基準viewport framingを修正し、自動検証とapproved VRMのrelease GUI framing確認を完了した。 |
| `M1-08-021` | `DONE` | production bindingでgeneration一致のrest-orientation cacheを構築し、実cameraでhead／blink／mouth／gazeとavatar apply latencyを確認した。 |
| `M1-08-022` | `DONE` | C922とELECOMの起動時列挙、symbolic-link identity選択、C922実previewを確認した。 |
| `M1-09` | `DEFERRED` | macOS開発環境へ移るまで保留。削除・DONE扱いはしないが、Windows-only Quality 2の開始条件にはしない。 |
| `Q2-01`〜`Q2-05` | `PENDING` | Windows部分は`M1-08-019`のWindows gate PASS後に開始可能。macOS固有・両OS比較部分は`M1-09`完了まで保留する。 |
| `Q2-06` | `IN_PROGRESS` | `Q2-06-001`〜`002`を実装済み。review blockerを`Q2-06-002-001`〜`004`で修正中。 |
| `R3-01` | `PENDING` | Windows実験は`Q2-01`のWindows経路と`Q2-03-007`完了後に開始可能。macOS比較は後日追補する。 |

M1-08のWindows gateは完了した。次の実行単位は、**`Q2-01`〜`Q2-05`のWindows部分から一つのtask ID**である。M1-08-018はC922 symbolic-link明示選択後のreal preview、real-VRM head／blink／mouth／gaze、capture-to-apply、Stop／Start 3回と既存recovery evidenceを統合して`DONE`、M1-08-019はbounded performance gateをPASSした。`M1-09`は`DEFERRED`のままとする。

`LEGACY_PROGRESS`は、この文書の現行subtask単位で全成果を再監査済みという意味ではない。既存成果を捨てて作り直さないための状態である。特に`M1-08-001`〜`M1-08-008`は「acceptance infrastructureが存在する」ことだけを引き継ぎ、実際のWindows受入結果をPASSと解釈してはならない。

### 0.2 軽量コーディングエージェントへの委嘱単位

- 一回の依頼では、必ず一つのleaf subtask IDだけを指定する。
- 通常の実行単位は`M1-02-001`形式とする。
- 既存subtaskがcorrectness blockerへ到達した場合だけ、そのIDを親gateとして`M1-08-013-001`形式のrepair leafを追加できる。既存の`M1-08-014`以降をrenumberしない。
- blocker parent自体（現在は`M1-08-013`）を軽量agentへ再委嘱しない。repair leafを番号順に一件ずつ委嘱する。
- agentは`AGENTS.md`、親taskの目的・制約・受入条件、指定leaf、指定された`DESIGN.md`／ADR節だけを読む。隣接leafを先回りしない。
- 最初に現repositoryを検索し、既存type、file、test、命名を再利用する。文書の候補pathと実repositoryが異なる場合、同じ責務の既存fileを優先し、重複実装を作らない。
- 原則として確認質問をせず、既存設計とcodeから最小の妥当な判断を行い、仮定を完了報告へ記録する。secret、実hardware、権利不明asset等が本質的に不足する場合だけ`BLOCKED`として具体的な不足を報告する。
- subtask外のrefactor、dependency update、format変更、UI改善を行わない。必要性を発見した場合は「後続候補」として報告するだけにする。
- 一つのleafは単一責務で、可能なら1〜5 source files＋対応testに収める。generated file、manifest、lockfileはこの目安から除く。
- 作業終了時に、指定leafの完了条件を一項目ずつ照合する。未達が一つでもあれば`DONE`と報告しない。
- agentはcommit、push、PR作成を明示指示なしに行わない。

### 0.3 subtask index

| 親task | subtask範囲 | 件数 | 現在状態 |
| --- | --- | ---: | --- |
| `G0-01` | `G0-01-001`〜`G0-01-008` | 8 | `LEGACY_PROGRESS` |
| `G0-02` | `G0-02-001`〜`G0-02-008` | 8 | `LEGACY_PROGRESS` |
| `G0-03` | `G0-03-001`〜`G0-03-009` | 9 | `LEGACY_PROGRESS` |
| `G0-04` | `G0-04-001`〜`G0-04-008` | 8 | `LEGACY_PROGRESS` |
| `G0-05` | `G0-05-001`〜`G0-05-008` | 8 | `LEGACY_PROGRESS` |
| `G0-06` | `G0-06-001`〜`G0-06-008` | 8 | `LEGACY_PROGRESS` |
| `G0-07` | `G0-07-001`〜`G0-07-008` | 8 | `LEGACY_PROGRESS` |
| `G0-08` | `G0-08-001`〜`G0-08-008` | 8 | `LEGACY_PROGRESS` |
| `M1-01` | `M1-01-001`〜`M1-01-008` | 8 | `LEGACY_PROGRESS` |
| `M1-02` | `M1-02-001`〜`M1-02-010` | 10 | `LEGACY_PROGRESS` |
| `M1-03` | `M1-03-001`〜`M1-03-010` | 10 | `LEGACY_PROGRESS` |
| `M1-04` | `M1-04-001`〜`M1-04-009` | 9 | `LEGACY_PROGRESS` |
| `M1-05` | `M1-05-001`〜`M1-05-008` | 8 | `LEGACY_PROGRESS` |
| `M1-06` | `M1-06-001`〜`M1-06-009` | 9 | `LEGACY_PROGRESS` |
| `M1-07` | `M1-07-001`〜`M1-07-010` | 10 | `LEGACY_PROGRESS`＋GUI補完済み。`M1-07-010`はControls windowのsession内開閉修正。 |
| `M1-08` | top-level `M1-08-001`〜`022`、repair `M1-08-013-001`〜`009` | 22 + repair 9 | `DONE`（Windows gate PASS、M1-09は別途DEFERRED） |
| `M1-09` | `M1-09-001`〜`M1-09-008` | 8 | `DEFERRED` |
| `Q2-01` | `Q2-01-001`〜`Q2-01-008` | 8 | `PENDING` |
| `Q2-02` | `Q2-02-001`〜`Q2-02-008` | 8 | `PENDING` |
| `Q2-03` | `Q2-03-001`〜`Q2-03-008` | 8 | `PENDING`（008はmacOS再開までdeferred） |
| `Q2-04` | `Q2-04-001`〜`Q2-04-008` | 8 | `PENDING`（004〜008はmacOS再開までdeferred） |
| `Q2-05` | `Q2-05-001`〜`Q2-05-008` | 8 | `PENDING` |
| `Q2-06` | `Q2-06-001`〜`Q2-06-002`、repair `Q2-06-002-001`〜`004` | 2 + repair 4 | `IN_PROGRESS` |
| `R3-01` | `R3-01-001`〜`R3-01-010` | 10 | `PENDING` |

### 0.4 status更新

使用可能な状態は`LEGACY_PROGRESS`、`PENDING`、`IN_PROGRESS`、`DONE`、`BLOCKED`、`DEFERRED`である。

- `DEFERRED`は、実装不要という意味ではなく、platform／hardware等の明示された再開条件を待つ状態である。
- `BLOCKED` parentのrepair leafは`PENDING`→`IN_PROGRESS`→`DONE`と進める。全required leaf完了後だけparentを`DONE`へ変更する。
- repair leafが新たなcorrectness blockerへ到達した場合、既存番号を変えず末尾に次のleafを追加する。候補を無断で別runtimeへ切り替えない。
- `M1-09`やmacOS依存subtaskをWindows agentが勝手に`DONE`へ変更してはならない。
- 実作業時に文書を更新する場合、原則として指定leafの`状態`だけを変更する。親task IDや他subtaskの番号・順序を変更しない。

### 0.5 委嘱prompt template

```text
Implement <SUBTASK-ID> only.

Read AGENTS.md, the parent task in AI_AGENT_TASKS.md, the selected subtask, and only the DESIGN.md sections referenced by that parent/subtask.
Inspect the current repository before editing. Preserve all existing work, especially LEGACY_PROGRESS tasks. Do not reimplement the parent task or continue to the next subtask.

Follow the exact Implementation instructions, exclusions, acceptance conditions, and verification commands. Resolve routine details from the existing code without asking questions. If an external prerequisite is genuinely unavailable, complete every non-blocked part and report a precise blocker.

Do not commit, push, or open a PR unless explicitly instructed. End with: status, changed files, exact commands/results, acceptance checklist, assumptions, and remaining blocker/follow-up.
```

### 0.6 Windows-first gateと実行順

1. 現在は`M1-08-013-001`から`M1-08-013-009`まで番号順に進める。
2. detector候補は、まず**UltraFace RFB-320 ONNX**へ固定してexactな`tract-onnx 0.23.4`互換性を検証する。`M1-08-013-002`で失敗した場合はoperator、model SHA-256、再現commandを残して停止し、同じleaf内で別runtimeへ切り替えない。
3. `M1-08-013-009`がPASSし、full camera frameからface detection→crop→98 landmarks→finite head poseまで再現できた場合だけ、parent `M1-08-013`を`DONE`へ変更する。
4. その後は`M1-08-014`〜`M1-08-019`を進める。既存worker／tracking／avatar bridge／diagnostics／shutdownを再実装せず、composite pipeline対応と実機監査の差分だけを行う。
5. `M1-08-019`が`PASS`またはcorrectness blockerのない明示的`CONDITIONAL PASS`になるまでは、Q2の機能追加・最適化・packagingを始めない。
6. `M1-09`はmacOS実機へ移るまで`DEFERRED`とし、Windows agentへ委嘱しない。
7. Windows gate後はQ2のWindows部分だけを開始できる。macOS固有実装、両OS比較、macOS packageは引き続き`M1-09`へ依存する。
8. `docs/acceptance/windows-m1.md`が`NOT_RUN`／`BLOCKED`のままなら、M1-08を完了扱いにしない。

---

## 1. 全task共通の完了条件

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

## 2. 親task依存グラフ

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
 └─ M1-07 ─ M1-08-001..008 (legacy acceptance scaffold)
                    └─ M1-08-009..012 (DONE)
                         └─ M1-08-013 (BLOCKED gate)
                              └─ M1-08-013-001
                                  ─ M1-08-013-002
                                  ─ M1-08-013-003
                                  ─ M1-08-013-004
                                  ─ M1-08-013-005
                                  ─ M1-08-013-006
                                  ─ M1-08-013-007
                                  ─ M1-08-013-008
                                  ─ M1-08-013-009
                                       └─ M1-08-014 ─ ... ─ M1-08-019

M1-08-019 Windows gate PASS
 ├─ Q2-01 Windows implementation／evaluation
 ├─ Q2-02 Windows settings／import UX
 ├─ Q2-03-001..007 Windows performance work
 ├─ Q2-04-001..003 Windows packaging
 ├─ Q2-05 Windows hardening
 └─ R3-01（Q2-01 + Q2-03-007後）

M1-08-019 PASS
 └─ M1-09 (DEFERRED until macOS development resumes)
      ├─ Q2-03-008 cross-platform performance report
      ├─ Q2-04-004..008 macOS／cross-platform packaging
      └─ 各Q2のmacOS verification／platform comparison追補
```

---

# Gate 0 — 不確実性を先に潰す

## G0-01: workspace、toolchain、品質基盤
状態: `LEGACY_PROGRESS`
実行単位: `G0-01-NNN`
重点参照: DESIGN.md §9、§10、§21.4、§26、§27


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

### 実行subtask

#### G0-01-001: root workspace manifestを固定する

状態: `LEGACY_PROGRESS`
依存: `なし`
親参照: DESIGN.md §9、§10、§21.4、§26、§27

**変更候補**

- `Cargo.toml`
- `Cargo.lock`


**実装指示**

- 最初に`cargo metadata`とdirectory treeを確認し、既存member名を正として重複crateを作らない。
- workspace `resolver = "2"`、member一覧、共通package metadata、workspace dependenciesの置き場所を定義する。
- package名は設計どおり`vtuber-*`と`xtask`を維持し、Windows／macOS以外のmemberを追加しない。
- `Cargo.lock`が生成済みなら維持し、生成されていなければworkspace checkで生成してcommit対象にする。


**このsubtaskで行わないこと**

- Bevy、camera、tract、VRM関連依存を追加しない。
- 既存crateをrenameしない。


**完了条件**

- `cargo metadata --no-deps`が全memberを一意に返す。
- dependency cycleや同名packageがない。
- このsubtaskでは実装依存を追加していない。


**検証**

```bash
cargo metadata --no-deps --format-version 1
cargo check --workspace
```

#### G0-01-002: workspace crate skeletonを揃える

状態: `LEGACY_PROGRESS`
依存: `G0-01-001`
親参照: DESIGN.md §9、§10、§21.4、§26、§27

**変更候補**

- `crates/vtuber-core/`
- `crates/vtuber-camera/`
- `crates/vtuber-inference/`
- `crates/vtuber-tracking/`
- `crates/vtuber-avatar/`
- `crates/vtuber-app/`
- `apps/desktop/`
- `tools/xtask/`


**実装指示**

- 不足しているcrateだけを追加し、各`Cargo.toml`のpackage名とlibrary/binary targetを設計に合わせる。
- 各crateは最小の`lib.rs`または`main.rs`でbuild可能にし、責務外のplaceholder APIを増やさない。
- `vtuber-desktop`と`xtask`以外は原則library crateとする。
- crate間依存は設計のdependency directionを超えない最小構成にする。


**このsubtaskで行わないこと**

- 将来用featureを先回りして作らない。
- 空traitや巨大なfacadeを作らない。


**完了条件**

- 全memberが空実装でもcompileする。
- `vtuber-core`にBevy／OS／camera依存がない。
- workspace treeにAndroid、WASM、Linux製品crateがない。


**検証**

```bash
cargo check --workspace
cargo test --workspace --no-fail-fast
```

#### G0-01-003: toolchain、format、lint policyを固定する

状態: `LEGACY_PROGRESS`
依存: `G0-01-002`
親参照: DESIGN.md §9、§10、§21.4、§26、§27

**変更候補**

- `rust-toolchain.toml`
- `rustfmt.toml`
- `Cargo.toml`
- `各crate/Cargo.toml`


**実装指示**

- 現在の全workspaceが通るexact stable toolchainを確認し、channelを完全なversionで固定する。
- workspace lint tableを定義し、各crateが`lints.workspace = true`を継承する。
- `unsafe_code`の既定方針とClippy warning policyをAGENTS.mdに一致させる。
- `rustfmt.toml`はstable rustfmtで解釈可能なoptionだけに限定する。


**このsubtaskで行わないこと**

- nightly専用featureを追加しない。
- 警告をまとめてallowしない。


**完了条件**

- `rustc -Vv`のreleaseが`rust-toolchain.toml`と一致する。
- `cargo fmt --check`とClippyが空workspaceで成功する。
- lint抑制はcrate全体の`allow`ではなく必要最小限である。


**検証**

```bash
rustc -Vv
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

#### G0-01-004: license／dependency policy skeletonを作る

状態: `LEGACY_PROGRESS`
依存: `G0-01-003`
親参照: DESIGN.md §9、§10、§21.4、§26、§27

**変更候補**

- `deny.toml`
- `Cargo.toml`
- `tools/xtask/src/`


**実装指示**

- `cargo-deny`用のlicense、advisory、source、banの最小policyを作る。
- VRM runtimeの重複、native inference runtime、禁止platform依存を後続taskで検出できるcheck入口を用意する。
- 現時点で存在しないdependencyを仮にbanする場合は、crate名と理由をcommentで明示する。
- `xtask`には将来のcheck commandを追加できる最小dispatcherだけを置く。


**このsubtaskで行わないこと**

- 実依存を追加しない。
- shell scriptだけにpolicyを埋め込まない。


**完了条件**

- `cargo deny check`が現workspaceで成功する。
- policyがGit dependencyを全面禁止して`bevy_vrm1`固定revisionを妨げない。
- 禁止理由が文書化されている。


**検証**

```bash
cargo deny check
cargo run -p xtask -- --help
```

#### G0-01-005: `vtuber-core`の最小placeholder契約を定義する

状態: `LEGACY_PROGRESS`
依存: `G0-01-004`
親参照: DESIGN.md §9、§10、§21.4、§26、§27

**変更候補**

- `crates/vtuber-core/src/lib.rs`
- `crates/vtuber-core/src/error.rs（必要な場合）`


**実装指示**

- 後続crateがcompileするために本当に必要なID／timestampのplaceholderだけを定義する。
- 単位、値域、clock originが未確定の型は作らず、`TODO`型で設計を先取りしない。
- error型を置く場合はplatform errorやBevy errorを含めない。
- public APIには最小限のrustdocを付ける。


**このsubtaskで行わないこと**

- VideoFrameやtracking modelの本実装を始めない。
- generic utility集を作らない。


**完了条件**

- `cargo tree -p vtuber-core`にplatform／render／camera依存がない。
- public typeの意味がrustdocから判断できる。
- placeholderが後続実装を拘束しすぎない。


**検証**

```bash
cargo test -p vtuber-core
cargo clippy -p vtuber-core --all-targets -- -D warnings
cargo tree -p vtuber-core
```

#### G0-01-006: root READMEへcrate責務とdependency directionを書く

状態: `LEGACY_PROGRESS`
依存: `G0-01-005`
親参照: DESIGN.md §9、§10、§21.4、§26、§27

**変更候補**

- `README.md`


**実装指示**

- 各crateの責務を一文ずつ記載する。
- 許可される依存方向を矢印で示し、`vtuber-core`が最下層であることを明記する。
- 対象OSがWindows／macOS、対象VRMが1.0のみであることを明記する。
- build／testの最小commandを記載し、未実装機能を完成済みと表現しない。


**このsubtaskで行わないこと**

- 製品紹介やUI仕様を膨らませない。
- 未検証の性能値を書かない。


**完了条件**

- READMEとworkspace memberが一致する。
- Android／WASMを将来予定として記載していない。
- 依存方向がDESIGN.mdと矛盾しない。


**検証**

```bash
cargo metadata --no-deps --format-version 1
python -m compileall tools 2>/dev/null || true
```

#### G0-01-007: Windows／macOS CI skeletonを追加する

状態: `LEGACY_PROGRESS`
依存: `G0-01-006`
親参照: DESIGN.md §9、§10、§21.4、§26、§27

**変更候補**

- `.github/workflows/ci.yml`


**実装指示**

- matrixを`windows-latest`と`macos-latest`だけにする。
- toolchain fileを使用し、format、check、Clippy、testのjobを定義する。
- hardware cameraやGUI smokeを通常CIへ入れない。
- cache keyへtoolchainとCargo filesを含め、失敗を隠す`continue-on-error`を使わない。


**このsubtaskで行わないこと**

- release packagingを始めない。
- GUIを起動するCI stepを追加しない。


**完了条件**

- workflow YAMLがparse可能である。
- Linux、Android、WASM jobがない。
- 各OSで少なくとも`cargo check --workspace`とtestを実行する。


**検証**

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

#### G0-01-008: Gate 0 workspace baselineを総合検証する

状態: `LEGACY_PROGRESS`
依存: `G0-01-007`
親参照: DESIGN.md §9、§10、§21.4、§26、§27

**変更候補**

- `変更済みworkspace全体`


**実装指示**

- 前subtaskの差分を再実装せず、acceptance criteriaとの不足だけを修正する。
- directory tree、metadata、dependency tree、lint、test、denyを順に実行する。
- toolchain versionと実行commandの結果を作業報告へ記録する。
- 失敗が後続依存由来なら、この親task内で勝手に依存を追加せずblockerとして残す。


**このsubtaskで行わないこと**

- 次のG0-02へ進まない。
- 検証失敗をwarning抑制で消さない。


**完了条件**

- G0-01の全受入条件が満たされる。
- 検証commandがすべて成功するか、実行不能理由が具体的に記録される。
- task外機能の差分がない。


**検証**

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

## G0-02: Bevy 0.19 + pinned bevy_vrm1 baseline
状態: `LEGACY_PROGRESS`
実行単位: `G0-02-NNN`
重点参照: DESIGN.md §7.2〜§7.4、§16、§19、§26


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

### 実行subtask

#### G0-02-001: Bevyとbevy_vrm1のversion／featureを固定する

状態: `LEGACY_PROGRESS`
依存: `G0-01`
親参照: DESIGN.md §7.2〜§7.4、§16、§19、§26

**変更候補**

- `Cargo.toml`
- `apps/desktop/Cargo.toml`
- `crates/vtuber-avatar/Cargo.toml`
- `Cargo.lock`


**実装指示**

- `bevy = "=0.19.0"`を必要crateだけへ追加する。
- `bevy_vrm1`はDESIGN.md記載のcommit SHAへ固定し、branch／tag／semver rangeを使わない。
- 同じ機能を提供するVRM crateや`vrm-utils-rs`がdependency treeへ入らないことを確認する。
- Bevy featureはbaseline表示に必要なものだけを有効にする。


**このsubtaskで行わないこと**

- 独自VRM schema crateを追加しない。
- 最新mainへ追従するためrevisionを動かさない。


**完了条件**

- `cargo tree`でBevy versionが一本化される。
- `bevy_vrm1`のsourceとrevisionがlockfileで固定される。
- VRM runtime dependencyが`bevy_vrm1`だけである。


**検証**

```bash
cargo tree -p vtuber-desktop -d
cargo tree -p vtuber-avatar
cargo check -p vtuber-desktop
```

#### G0-02-002: 最小Bevy desktop appを起動する

状態: `LEGACY_PROGRESS`
依存: `G0-02-001`
親参照: DESIGN.md §7.2〜§7.4、§16、§19、§26

**変更候補**

- `apps/desktop/src/main.rs`
- `crates/vtuber-app/src/lib.rs（必要なら）`


**実装指示**

- `DefaultPlugins`で単一windowと`Camera3d`が起動する最小Appを作る。
- app constructionを一箇所へ集約し、desktop binaryへ業務ロジックを詰め込まない。
- window title、clear color、終了処理だけをbaselineとして設定する。
- headless test用にplugin assemblyを関数化する場合も過剰なbuilder abstractionを作らない。


**このsubtaskで行わないこと**

- UI frameworkを追加しない。
- 複数windowやrender tuningを始めない。


**完了条件**

- `cargo run -p vtuber-desktop`がwindow初期化まで到達する。
- unit test時にwindowを開かない経路がある。
- VRM、camera、inference処理はまだない。


**検証**

```bash
cargo check -p vtuber-desktop
cargo clippy -p vtuber-desktop --all-targets -- -D warnings
```

#### G0-02-003: `VrmPlugin`とbaseline sceneを登録する

状態: `LEGACY_PROGRESS`
依存: `G0-02-002`
親参照: DESIGN.md §7.2〜§7.4、§16、§19、§26

**変更候補**

- `crates/vtuber-avatar/src/lib.rs`
- `crates/vtuber-app/src/lib.rs`
- `apps/desktop/src/main.rs`


**実装指示**

- `VrmPlugin`をavatar側pluginまたはapp assemblyから一度だけ追加する。
- directional light、camera、groundまたは単色backgroundを最小構成でspawnする。
- scene setup component／resource名は責務を明確にし、`bevy_vrm1`型をdesktop binary全体へ拡散させない。
- MToonやSpringBoneの独自systemを追加しない。


**このsubtaskで行わないこと**

- MToon materialをアプリ側で作らない。
- camera framingの完成実装をしない。


**完了条件**

- empty sceneがWindows／macOS targetでcompileする。
- `VrmPlugin`が重複登録されない。
- render setupがfixture固有値に依存しない。


**検証**

```bash
cargo check -p vtuber-avatar -p vtuber-desktop
cargo clippy -p vtuber-avatar -p vtuber-desktop --all-targets -- -D warnings
```

#### G0-02-004: VRM 1.0 sample fixtureとprovenanceを追加する

状態: `LEGACY_PROGRESS`
依存: `G0-02-003`
親参照: DESIGN.md §7.2〜§7.4、§16、§19、§26

**変更候補**

- `assets/vrm/`
- `assets/vrm/README.mdまたはfixture manifest`
- `LICENSES/`


**実装指示**

- licenseが明確なVRM 1.0 sampleだけをrepository fixtureとして追加する。
- source、author、license、取得version、SHA-256をfixture manifestへ記録する。
- binary sizeが不必要に大きい場合はGit LFS方針を明記し、無断で圧縮変換しない。
- fixture pathを一箇所のconstantまたはtest helperへまとめる。


**このsubtaskで行わないこと**

- 権利不明モデルを追加しない。
- VRM 0.x fixtureを追加しない。


**完了条件**

- fixtureがVRM 1.0であり、再配布条件を満たす。
- hashをcommandで再計算できる。
- 外部downloadがなくてもbaseline smokeを実行できる。


**検証**

```bash
sha256sum assets/vrm/*.vrm 2>/dev/null || shasum -a 256 assets/vrm/*.vrm
cargo check -p vtuber-desktop
```

#### G0-02-005: `VrmHandle`でsampleをloadする

状態: `LEGACY_PROGRESS`
依存: `G0-02-004`
親参照: DESIGN.md §7.2〜§7.4、§16、§19、§26

**変更候補**

- `crates/vtuber-avatar/src/`
- `apps/desktop/src/`


**実装指示**

- fixture asset pathを`AssetServer`へ渡し、root entityへ`VrmHandle`をinsertする。
- spawn要求とload状態を区別するcomponent／resourceを最小限追加する。
- load failureはlogまたはdiagnostic resourceへ出し、panicしない。
- absolute pathやuser asset sourceはまだ扱わない。


**このsubtaskで行わないこと**

- import cacheを先行実装しない。
- GLBを独自parseしない。


**完了条件**

- sample VRMがsceneへspawnされる。
- missing fixture時にprocessがpanicしない。
- アプリ側に`.vrm` AssetLoader実装がない。


**検証**

```bash
cargo check -p vtuber-avatar -p vtuber-desktop
cargo test -p vtuber-avatar
```

#### G0-02-006: `Initialized`とhead bone capabilityを観測する

状態: `LEGACY_PROGRESS`
依存: `G0-02-005`
親参照: DESIGN.md §7.2〜§7.4、§16、§19、§26

**変更候補**

- `crates/vtuber-avatar/src/`
- `crates/vtuber-avatar/tests/`


**実装指示**

- `With<Vrm>`かつ`Added<Initialized>`を一度だけ観測するsystemを追加する。
- VRM rootから`HeadBoneEntity`を取得し、成功／欠落をstructured diagnosticへ変換する。
- 毎frame hierarchyを探索せず、検知結果をcacheする。
- public boundaryへ`bevy_vrm1::HeadBoneEntity`を直接返さない。


**このsubtaskで行わないこと**

- bone rotationを書き換えない。
- Expression discoveryを同時実装しない。


**完了条件**

- sample load後にhead capabilityが一度だけ報告される。
- head取得失敗がtyped stateとして表現される。
- systemが未load中にbusy loopしない。


**検証**

```bash
cargo test -p vtuber-avatar
cargo clippy -p vtuber-avatar --all-targets -- -D warnings
```

#### G0-02-007: baseline debug／release profileを設定する

状態: `LEGACY_PROGRESS`
依存: `G0-02-006`
親参照: DESIGN.md §7.2〜§7.4、§16、§19、§26

**変更候補**

- `Cargo.toml`


**実装指示**

- Bevy開発時のdependency最適化とworkspace crate最適化を明示する。
- releaseはLTO等を過剰に固定せず、DESIGN.mdのbaseline範囲に限定する。
- profile変更理由をcommentまたはREADMEへ短く記録する。
- platform別profileを作らない。


**このsubtaskで行わないこと**

- 根拠なくcodegen-unitsやstripを極端に設定しない。
- 性能測定前にunsafe最適化を入れない。


**完了条件**

- debug buildが実用的な速度でcompile／runできる設定である。
- release profileがpanic abort等でdiagnostic要件を壊さない。
- profile設定が全workspaceへ一貫して適用される。


**検証**

```bash
cargo check -p vtuber-desktop
cargo build -p vtuber-desktop --release
```

#### G0-02-008: Bevy／bevy_vrm1 baselineを総合検証する

状態: `LEGACY_PROGRESS`
依存: `G0-02-007`
親参照: DESIGN.md §7.2〜§7.4、§16、§19、§26

**変更候補**

- `G0-02で変更した全file`


**実装指示**

- sample表示、`Initialized`、head capabilityの縦断経路を確認する。
- `cargo tree`からVRM runtime重複とBevy重複を検査する。
- Windows／macOSでcompile可能なcfgだけが使われていることを確認する。
- 手動確認結果を短いbaseline reportまたは完了報告へ残す。


**このsubtaskで行わないこと**

- G0-03のuser importを始めない。
- 確認できないOSの成功を推測で報告しない。


**完了条件**

- G0-02の全受入条件を満たす。
- 独自VRM loader／schema／MToon実装がない。
- sample表示失敗時に再現手順がある。


**検証**

```bash
cargo tree -p vtuber-desktop
cargo check -p vtuber-desktop
cargo test -p vtuber-avatar
cargo clippy -p vtuber-avatar -p vtuber-desktop --all-targets -- -D warnings
```

## G0-03: user asset sourceとVRM 1.0 preflight
状態: `LEGACY_PROGRESS`
実行単位: `G0-03-NNN`
重点参照: DESIGN.md §17、§20.1、§21.1


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

### 実行subtask

#### G0-03-001: VRM import domain typesとtyped errorsを定義する

状態: `LEGACY_PROGRESS`
依存: `G0-02`
親参照: DESIGN.md §17、§20.1、§21.1

**変更候補**

- `crates/vtuber-avatar/src/import/`
- `crates/vtuber-core/src/（共有型が必要な場合のみ）`


**実装指示**

- `ImportRequest`、`ImportedAvatar`、`AvatarAssetId`、meta summary、error codeを定義する。
- error codeはinvalid path、not regular file、size、invalid GLB、not VRM1、unsupported version、missing bone、external URI、I/Oを区別する。
- path、size、hashの単位とownershipをrustdocへ記す。
- Bevy `AssetPath`とfilesystem pathを同一型で表現しない。


**このsubtaskで行わないこと**

- native file dialogを追加しない。
- VRM schema全体を型定義しない。


**完了条件**

- errorからUI向けcodeと内部causeを取得できる。
- domain typeがuser pathを無検証でAssetServerへ渡せない形になっている。
- unit testで主要error codeを生成できる。


**検証**

```bash
cargo test -p vtuber-avatar import
cargo clippy -p vtuber-avatar --all-targets -- -D warnings
```

#### G0-03-002: project data directoryとnamed asset source `user`を用意する

状態: `LEGACY_PROGRESS`
依存: `G0-03-001`
親参照: DESIGN.md §17、§20.1、§21.1

**変更候補**

- `crates/vtuber-app/src/`
- `apps/desktop/src/`
- `crates/vtuber-avatar/src/import/`


**実装指示**

- Windows／macOSのapplication data directory resolverを一箇所に実装する。
- `user` named asset sourceのrootを`<AppData>/user-assets`へ固定する。
- directory作成とpermission failureをtyped errorにする。
- Asset source登録はApp構築時に一度だけ行い、global unapproved path許可を使わない。


**このsubtaskで行わないこと**

- user home直下へ固定folderを作らない。
- `UnapprovedPathMode::Allow`をglobal設定しない。


**完了条件**

- OSごとのpath resolver testがenvironment overrideまたはtemp dirで実行できる。
- `user://`以外の任意absolute pathをAssetServerが直接読まない。
- directory作成失敗がpanicしない。


**検証**

```bash
cargo test -p vtuber-app data_dir
cargo check -p vtuber-desktop
```

#### G0-03-003: 入力pathのfilesystem検証を実装する

状態: `LEGACY_PROGRESS`
依存: `G0-03-002`
親参照: DESIGN.md §17、§20.1、§21.1

**変更候補**

- `crates/vtuber-avatar/src/import/path_validation.rs`
- `crates/vtuber-avatar/tests/`


**実装指示**

- 入力をcanonicalizeする前後でsymlink／directory／non-regular fileを検査する。
- file sizeをmetadataから取得しdefault 256 MiB、hard cap 1 GiBを適用する。
- TOCTOUを減らすため、検証後はopen済みfile handleからhash／copy／parseする設計にする。
- pathをlogへ出す場合はprivacy方針に従い、通常logへfull pathを残さない。


**このsubtaskで行わないこと**

- path extensionだけでVRM判定しない。
- canonicalize failureを握り潰さない。


**完了条件**

- directory、symlink、missing file、oversizeを個別errorで拒否する。
- hard capをconfigで引き上げられない。
- temp fixtureでplatform非依存testが通る。


**検証**

```bash
cargo test -p vtuber-avatar path_validation
cargo clippy -p vtuber-avatar --all-targets -- -D warnings
```

#### G0-03-004: streaming SHA-256とatomic import copyを実装する

状態: `LEGACY_PROGRESS`
依存: `G0-03-003`
親参照: DESIGN.md §17、§20.1、§21.1

**変更候補**

- `crates/vtuber-avatar/src/import/storage.rs`
- `crates/vtuber-avatar/tests/`


**実装指示**

- open済みsourceからbounded bufferでSHA-256を計算し、同時または二巡目でtemp fileへcopyする。
- 最終pathを`avatars/<sha256>/model.vrm`へ固定する。
- temp fileを同一filesystem上へ書き、flush／必要ならsync後にatomic renameする。
- 既存hash directoryが正しい場合は再copyせずidempotentに返す。


**このsubtaskで行わないこと**

- 全fileを一度にmemoryへ読まない。
- hash以外の可変IDをasset pathに使わない。


**完了条件**

- 同一file再importでduplicate directoryを作らない。
- 中断時に完成fileとして見えるpartial copyがない。
- hash mismatchやrename failureがtyped errorになる。


**検証**

```bash
cargo test -p vtuber-avatar import_storage
cargo clippy -p vtuber-avatar --all-targets -- -D warnings
```

#### G0-03-005: GLB containerと`VRMC_vrm`をtolerant preflightする

状態: `LEGACY_PROGRESS`
依存: `G0-03-004`
親参照: DESIGN.md §17、§20.1、§21.1

**変更候補**

- `crates/vtuber-avatar/src/import/preflight.rs`
- `crates/vtuber-avatar/tests/fixtures/`


**実装指示**

- `gltf` crateでGLB／glTF documentをparseし、raw extension JSONへアクセスする。
- `VRMC_vrm`の存在と`specVersion == "1.0"`だけを必要最小限のtolerant JSONとして読む。
- unknown fieldやoptional field欠落でpreflight全体を失敗させない。
- invalid GLBとvalid non-VRM GLBを別errorにする。


**このsubtaskで行わないこと**

- `bevy_vrm1`内部Serde型をpreflight契約に流用しない。
- VRM 0.xを変換しない。


**完了条件**

- invalid bytesは`MODEL_INVALID_GLTF`相当で拒否される。
- `VRMC_vrm`欠落は`MODEL_NOT_VRM1`で拒否される。
- version違いは`MODEL_UNSUPPORTED_VERSION`で拒否される。


**検証**

```bash
cargo test -p vtuber-avatar preflight_vrm_version
cargo clippy -p vtuber-avatar --all-targets -- -D warnings
```

#### G0-03-006: humanoid必須bone、external URI、meta summaryを検査する

状態: `LEGACY_PROGRESS`
依存: `G0-03-005`
親参照: DESIGN.md §17、§20.1、§21.1

**変更候補**

- `crates/vtuber-avatar/src/import/preflight.rs`
- `crates/vtuber-avatar/tests/fixtures/`


**実装指示**

- `VRMC_vrm.humanoid.humanBones.hips/head.node`を取得し、node index範囲を検査する。
- buffer／image URIを走査し、embedded／data URI以外のexternal referenceを拒否する。
- metaのname、authors、version、thumbnail indexをbest-effortでsummaryへ格納する。
- 欠落optional metaはerrorにしない。


**このsubtaskで行わないこと**

- 全VRM license ruleを独自解釈しない。
- thumbnail imageをdecodeしない。


**完了条件**

- missing hips／head、out-of-range node、external URIが個別testで拒否される。
- meta欠落でもvalid modelとして通る。
- summaryにraw arbitrary JSONを保持しない。


**検証**

```bash
cargo test -p vtuber-avatar preflight_humanoid
cargo test -p vtuber-avatar preflight_external_uri
```

#### G0-03-007: import manifestとcache idempotencyを完成させる

状態: `LEGACY_PROGRESS`
依存: `G0-03-006`
親参照: DESIGN.md §17、§20.1、§21.1

**変更候補**

- `crates/vtuber-avatar/src/import/manifest.rs`
- `<AppData>/user-assets layout tests`


**実装指示**

- import結果にsource hash、size、import timestamp、preflight summary、app schema versionを保存する。
- manifestはtemp write＋atomic renameで書く。
- 既存manifestが破損している場合はmodel fileを再検査して再生成する。
- import resultから`user://avatars/<sha256>/model.vrm`を型安全に生成する。


**このsubtaskで行わないこと**

- source absolute pathをmanifestへ必須保存しない。
- JSON pathを文字列連結だけで作らない。


**完了条件**

- cache hitでもpreflight summaryを返せる。
- 破損manifestがprocess panicを起こさない。
- AssetServer用pathにpath traversalを混入できない。


**検証**

```bash
cargo test -p vtuber-avatar import_manifest
cargo test -p vtuber-avatar import_idempotency
```

#### G0-03-008: CLIまたはfile-drop import requestを接続する

状態: `LEGACY_PROGRESS`
依存: `G0-03-007`
親参照: DESIGN.md §17、§20.1、§21.1

**変更候補**

- `apps/desktop/src/`
- `crates/vtuber-app/src/`
- `crates/vtuber-avatar/src/`


**実装指示**

- `--avatar <path>`またはBevy file-drop eventのうち既存UIに近い入口を一つ実装する。
- 入口はpathをimport serviceへ渡すだけにし、filesystem／GLB処理をUI systemへ置かない。
- import成功時にtyped asset pathをload requestへ変換する。
- import中、成功、失敗をapp stateへ反映する。


**このsubtaskで行わないこと**

- native file dialogとrecent listを追加しない。
- UI threadでlarge fileを同期copyしない。


**完了条件**

- valid fileが`user://.../model.vrm`からloadされる。
- invalid fileのerror codeがUI／logへ届く。
- 同一requestの連打で同時copyが競合しない。


**検証**

```bash
cargo check -p vtuber-desktop
cargo test -p vtuber-app import_request
```

#### G0-03-009: VRM import adversarial testsと総合検証を追加する

状態: `LEGACY_PROGRESS`
依存: `G0-03-008`
親参照: DESIGN.md §17、§20.1、§21.1

**変更候補**

- `crates/vtuber-avatar/tests/`
- `tools/xtask/src/（必要なら）`


**実装指示**

- valid VRM1、non-VRM GLB、invalid bytes、missing head、oversize metadata fake、hash idempotency、path traversal相当をfixture化する。
- 巨大fileを実際に1 GiB生成せず、size policyをinject可能にしてtestする。
- symlink testはplatform capabilityに応じてskip理由を明示する。
- 全G0-03受入条件を一つのtest/reportから追跡できるようtest名を整理する。


**このsubtaskで行わないこと**

- fuzzing frameworkを先行導入しない。
- 実ユーザーファイルをtestへ含めない。


**完了条件**

- G0-03の列挙testがすべて自動化される。
- testがuserの実AppDataを汚さない。
- error codeのregressionを検知できる。


**検証**

```bash
cargo test -p vtuber-avatar import --no-fail-fast
cargo clippy -p vtuber-avatar -p vtuber-desktop --all-targets -- -D warnings
```

## G0-04: Windows／macOS camera smoke
状態: `LEGACY_PROGRESS`
実行単位: `G0-04-NNN`
重点参照: DESIGN.md §4.3、§13、§21.4


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

### 実行subtask

#### G0-04-001: nokhwaをOS別featureで追加する

状態: `LEGACY_PROGRESS`
依存: `G0-01`
親参照: DESIGN.md §4.3、§13、§21.4

**変更候補**

- `crates/vtuber-camera/Cargo.toml`
- `Cargo.lock`


**実装指示**

- Windows targetに`input-msmf`、macOS targetに`input-avfoundation`を指定する。
- default featuresを切り、必要なYUV／MJPEG decode featureだけを有効にする。
- `nokhwa = "=0.10.11"`をexact pinし、両backendを全OSで同時compileさせない。
- dependency treeへOpenCV／FFmpeg backendが入らないことを確認する。


**このsubtaskで行わないこと**

- `input-native`で不要backendをまとめて有効化しない。
- OpenCV／FFmpegを追加しない。


**完了条件**

- Windows cfgでMSMF、macOS cfgでAVFoundationが選択される。
- unsupported backend dependencyがtreeにない。
- cameraなし環境でも`cargo check`できる。


**検証**

```bash
cargo tree -p vtuber-camera
cargo check -p vtuber-camera
```

#### G0-04-002: camera domain typesとbackend非依存APIを定義する

状態: `LEGACY_PROGRESS`
依存: `G0-04-001`
親参照: DESIGN.md §4.3、§13、§21.4

**変更候補**

- `crates/vtuber-camera/src/lib.rs`
- `crates/vtuber-camera/src/types.rs`


**実装指示**

- `CameraDeviceId`、`CameraDevice`、`CameraFormatInfo`、`RgbFrame`、typed errorを定義する。
- persistent device identityはindexだけでなくhuman-readable name／backend metadataを含める。
- resolution、fps、pixel format、stride、timestampの単位をrustdocへ記す。
- public APIへ`nokhwa`型を漏らさない。


**このsubtaskで行わないこと**

- long-running controllerを実装しない。
- Bevy `Image`をcamera domain typeにしない。


**完了条件**

- domain typesがWindows／macOS共通でcompileする。
- RGB frameがowned bufferとしてworker外へ渡せる。
- camera index単独を永続IDとして扱わない。


**検証**

```bash
cargo test -p vtuber-camera types
cargo clippy -p vtuber-camera --all-targets -- -D warnings
```

#### G0-04-003: macOS camera permission初期化を隔離する

状態: `LEGACY_PROGRESS`
依存: `G0-04-002`
親参照: DESIGN.md §4.3、§13、§21.4

**変更候補**

- `crates/vtuber-camera/src/platform/macos.rs`
- `crates/vtuber-camera/src/platform/mod.rs`


**実装指示**

- macOSのみ`nokhwa_initialize`／authorization statusを包むAPIを作る。
- callback結果をtyped stateへ変換し、permission denied／not determinedを区別する。
- Windowsではno-opの共通APIを提供するが、macOS実装をcompile対象にしない。
- main thread要件をrustdocとcall site assertionで明示する。


**このsubtaskで行わないこと**

- `.app` packagingを始めない。
- permissionを常にgrantedと仮定しない。


**完了条件**

- permission APIがOS cfgで正しく分離される。
- unit testでは実permission dialogを出さない。
- denied状態をpanicせず返せる。


**検証**

```bash
cargo check -p vtuber-camera
cargo test -p vtuber-camera permission
```

#### G0-04-004: device enumerationを実装する

状態: `LEGACY_PROGRESS`
依存: `G0-04-003`
親参照: DESIGN.md §4.3、§13、§21.4

**変更候補**

- `crates/vtuber-camera/src/enumerate.rs`
- `tools/xtask/src/またはapps/desktop debug command`


**実装指示**

- OSに応じたbackendを明示してdevice listを取得する。
- `nokhwa`情報をdomain typeへ変換し、index、name、description、backendを保持する。
- deviceなしを空listとして扱い、backend初期化失敗とは区別する。
- debug commandはJSONまたは安定したtextで列挙結果を出す。


**このsubtaskで行わないこと**

- UIを追加しない。
- device 0が必ず存在すると仮定しない。


**完了条件**

- cameraなしでprocessが成功終了し空listを返せる。
- 同名deviceが複数あってもIDが衝突しない。
- raw backend errorがtyped causeとして保持される。


**検証**

```bash
cargo test -p vtuber-camera enumeration
cargo run -p xtask -- camera list --help
```

#### G0-04-005: deterministic format selectionをpure functionで実装する

状態: `LEGACY_PROGRESS`
依存: `G0-04-004`
親参照: DESIGN.md §4.3、§13、§21.4

**変更候補**

- `crates/vtuber-camera/src/format_selection.rs`
- `crates/vtuber-camera/src/format_selection_tests.rs`


**実装指示**

- 候補format listから1280×720/30fpsを第一候補、640×480/30fpsをfallbackとしてscoreする。
- exact matchがなければ解像度、fps、decode可能pixel formatの優先順位を明示する。
- 入力順に依存せず同じ結果を返すstable tie-breakerを入れる。
- unsupported formatだけならtyped errorを返す。


**このsubtaskで行わないこと**

- hardware APIをpure selection testへ持ち込まない。
- 最大解像度を無条件選択しない。


**完了条件**

- 候補順をshuffleしても選択結果が同じ。
- 設計上の第一候補／fallback testが通る。
- 0fpsや0-sizeの異常候補を選ばない。


**検証**

```bash
cargo test -p vtuber-camera format_selection
cargo clippy -p vtuber-camera --all-targets -- -D warnings
```

#### G0-04-006: 10-frame capture smokeをworker内ownershipで実装する

状態: `LEGACY_PROGRESS`
依存: `G0-04-005`
親参照: DESIGN.md §4.3、§13、§21.4

**変更候補**

- `crates/vtuber-camera/src/smoke.rs`
- `tools/xtask/src/camera.rs`


**実装指示**

- worker thread内でcamera objectをconstruct、format設定、open、10 frame capture、stop、dropする。
- 各frameをRGB8 owned bufferへdecodeし、width／height／buffer lengthを検証する。
- main threadからlive camera objectをmoveせず、device descriptorとformatだけを渡す。
- timeoutとstop flagを設け、camera blockで無期限hangしないようにする。


**このsubtaskで行わないこと**

- production reconnectを実装しない。
- camera frameをdisk保存しない。


**完了条件**

- 10frameが取得できた場合にsequenceとbasic timingを返す。
- open／capture／decode／stopの失敗位置が区別される。
- worker終了時にthreadがjoinされる。


**検証**

```bash
cargo test -p vtuber-camera --lib
cargo run -p xtask -- camera smoke --help
```

#### G0-04-007: hardware-free test doubleとignored hardware testを整える

状態: `LEGACY_PROGRESS`
依存: `G0-04-006`
親参照: DESIGN.md §4.3、§13、§21.4

**変更候補**

- `crates/vtuber-camera/src/test_support.rs`
- `crates/vtuber-camera/tests/`


**実装指示**

- format selection／decode boundaryを検証できるfake backendまたはfixture bufferを用意する。
- 実camera testは`#[ignore]`またはxtaskだけで明示実行する。
- CI通常testがcamera permissionやdeviceを要求しないことを確認する。
- hardware testの必要環境変数／device selectorを文書化する。


**このsubtaskで行わないこと**

- CIで実deviceを必須にしない。
- testだけのunsafe backendを作らない。


**完了条件**

- cameraなしCIでunit testが成功する。
- ignored testは明示commandでだけ実行される。
- fakeがproduction code pathを過度に分岐させない。


**検証**

```bash
cargo test -p vtuber-camera
cargo test -p vtuber-camera -- --ignored --list
```

#### G0-04-008: Windows／macOS camera smoke結果を記録する

状態: `LEGACY_PROGRESS`
依存: `G0-04-007`
親参照: DESIGN.md §4.3、§13、§21.4

**変更候補**

- `docs/camera-smoke.mdまたは完了report`


**実装指示**

- Windowsでbackend、device、selected format、10frame timingを記録する。
- macOSでpermission status、bundle外実行の限界、backend、format、timingを記録する。
- 一方のOSを実行できない場合はcompile結果と未実行理由を明示する。
- G0-04受入条件と結果を一対一で照合する。


**このsubtaskで行わないこと**

- 未実行OSの成功を推測しない。
- production captureへ進まない。


**完了条件**

- MSMF／AVFoundationの実選択が確認される。
- hardware結果とunit test結果が混同されない。
- 既知のpermission／device quirkが次taskへ引き継がれる。


**検証**

```bash
cargo test -p vtuber-camera
cargo test -p vtuber-camera -- --ignored
```

## G0-05: face model provenanceとpure-Rust runtime gate
状態: `LEGACY_PROGRESS`
実行単位: `G0-05-NNN`
重点参照: DESIGN.md §3、§10.3、§14.2〜§14.4、§25 R1


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

### 実行subtask

#### G0-05-001: model manifest schemaを定義する

状態: `LEGACY_PROGRESS`
依存: `G0-01`
親参照: DESIGN.md §3、§10.3、§14.2〜§14.4、§25 R1

**変更候補**

- `assets/models/manifest.toml`
- `crates/vtuber-inference/src/model_manifest.rs`


**実装指示**

- detector／landmark／blendshape artifactごとにsource、version、license、SHA-256、size、runtime formatを記録するschemaを作る。
- input／output tensor契約をshape、dtype、layout、nameまたはindexで表現する。
- manifest versionを持たせ、unknown fieldは将来互換の方針を明示する。
- model file pathをhard-codeせずmanifestから解決する。


**このsubtaskで行わないこと**

- model binaryをまだ追加しない。
- 不明なlicenseを仮定しない。


**完了条件**

- manifest parse testが通る。
- 必須hash／license／tensor契約欠落をtyped errorにする。
- model indexをsource codeへ散在させない。


**検証**

```bash
cargo test -p vtuber-inference model_manifest
cargo clippy -p vtuber-inference --all-targets -- -D warnings
```

#### G0-05-002: model fetch／verify xtaskを実装する

状態: `LEGACY_PROGRESS`
依存: `G0-05-001`
親参照: DESIGN.md §3、§10.3、§14.2〜§14.4、§25 R1

**変更候補**

- `tools/xtask/src/model.rs`
- `assets/models/README.md`


**実装指示**

- manifestのsourceからartifactをdownloadするcommandと、offline hash verify commandを分ける。
- downloadはtemp fileへ保存し、size／SHA-256検証後にrenameする。
- license textまたはsource noticeを所定pathへ保存する。
- network failure、HTTP status、hash mismatchを明確に報告する。


**このsubtaskで行わないこと**

- runtime起動時に自動downloadしない。
- hash checkを省略するforce optionを作らない。


**完了条件**

- `xtask model verify`がofflineで再実行できる。
- hash mismatch artifactをruntimeが使用しない。
- download済み正しいartifactは再取得しない。


**検証**

```bash
cargo test -p xtask
cargo run -p xtask -- model --help
```

#### G0-05-003: TFLite tensor／operator inventory probeを作る

状態: `LEGACY_PROGRESS`
依存: `G0-05-002`
親参照: DESIGN.md §3、§10.3、§14.2〜§14.4、§25 R1

**変更候補**

- `crates/vtuber-inference/src/probe.rs`
- `tools/xtask/src/model.rs`


**実装指示**

- TFLite flatbufferからsubgraph、input/output tensor、operator codeを列挙するprobeを実装する。
- manifest contractと実artifactのshape／dtype／indexを比較する。
- operator inventoryをstable JSONまたはtext artifactとして出力する。
- probe failureをruntime inference failureと別errorにする。


**このsubtaskで行わないこと**

- Netron等の手作業だけを契約根拠にしない。
- 推論loopを実装しない。


**完了条件**

- artifactごとのinput/output contract差分が機械的に検出される。
- operator一覧が再現可能な順序で出力される。
- unknown operator codeを数値と名称で報告する。


**検証**

```bash
cargo test -p vtuber-inference probe
cargo run -p xtask -- model inspect --help
```

#### G0-05-004: tract-tflite load／optimize／run probeを実装する

状態: `LEGACY_PROGRESS`
依存: `G0-05-003`
親参照: DESIGN.md §3、§10.3、§14.2〜§14.4、§25 R1

**変更候補**

- `crates/vtuber-inference/Cargo.toml`
- `crates/vtuber-inference/src/tract_probe.rs`


**実装指示**

- `tract-tflite = "=0.23.0"`を追加し、model load、input fact設定、optimize、runnable化を段階別に実行する。
- 各段階のerrorへartifact名、hash、stageを付加する。
- runtime objectはprobe関数内でconstructし、global singletonにしない。
- native C/C++ runtimeへfallbackしない。


**このsubtaskで行わないこと**

- ONNX fallbackを同じsubtaskで追加しない。
- C API bindingを追加しない。


**完了条件**

- 対象modelがload／optimize／run可能かstage別に判定できる。
- unsupported operatorがexact reproductionとともに返る。
- Windows／macOSで同じRust APIを使う。


**検証**

```bash
cargo test -p vtuber-inference tract_probe
cargo clippy -p vtuber-inference --all-targets -- -D warnings
```

#### G0-05-005: fixed golden inputのpreprocessを定義する

状態: `LEGACY_PROGRESS`
依存: `G0-05-004`
親参照: DESIGN.md §3、§10.3、§14.2〜§14.4、§25 R1

**変更候補**

- `crates/vtuber-inference/src/preprocess.rs`
- `crates/vtuber-inference/tests/data/`


**実装指示**

- 小さなlicense明確なfixed imageまたはsynthetic tensorをgolden inputにする。
- resize、color order、normalization、layout変換をmanifest契約から実装する。
- preprocess output hashまたはselected valuesをtest固定する。
- buffer allocationを計測できるよう入力／出力buffer ownershipを明示する。


**このsubtaskで行わないこと**

- Webカメラframeをgolden sourceにしない。
- image処理libraryを複数追加しない。


**完了条件**

- 同じinputからbitwiseまたは許容誤差内で同じtensorを作る。
- RGB/BGR、NHWC/NCHWの取り違えをtestが検知する。
- preprocessだけをmodelなしでunit testできる。


**検証**

```bash
cargo test -p vtuber-inference preprocess
cargo clippy -p vtuber-inference --all-targets -- -D warnings
```

#### G0-05-006: golden inference outputとtoleranceを固定する

状態: `LEGACY_PROGRESS`
依存: `G0-05-005`
親参照: DESIGN.md §3、§10.3、§14.2〜§14.4、§25 R1

**変更候補**

- `crates/vtuber-inference/tests/golden.rs`
- `assets/models/golden/`
- `assets/models/manifest.toml`


**実装指示**

- fixed inputをtract runtimeへ渡し、必要output tensorを保存または要約する。
- platform差を考慮したabsolute／relative toleranceをoutputごとに定義する。
- output shape、NaN／Inf、代表値、landmark範囲を検証する。
- golden更新手順を明記し、silent auto-updateを禁止する。


**このsubtaskで行わないこと**

- golden failure時に期待値を自動上書きしない。
- 目視のみで推論成功としない。


**完了条件**

- Windows／macOSで同じgolden testを実行できる。
- toleranceが過度に広くない。
- model hash変更時にgolden testが明示的に失敗する。


**検証**

```bash
cargo test -p vtuber-inference golden -- --nocapture
```

#### G0-05-007: unsupported operator時の分岐記録を整える

状態: `LEGACY_PROGRESS`
依存: `G0-05-006`
親参照: DESIGN.md §3、§10.3、§14.2〜§14.4、§25 R1

**変更候補**

- `docs/adr/ADR-001-*.md`
- `crates/vtuber-inference/src/error.rs`
- `assets/models/operator-inventory.*`


**実装指示**

- tract blockerがある場合、operator、node、input facts、model hash、reproduction commandを記録する。
- blockerがなければtract採用判断と制約をADRへ記録する。
- ONNX検証が必要な場合は別commit／別subtask相当の追補として扱い、TFLite結果を消さない。
- C runtimeへ切り替えない設計判断を維持する。


**このsubtaskで行わないこと**

- 問題を「tractが動かない」だけで終わらせない。
- 未検証runtimeを採用済みと書かない。


**完了条件**

- 採用／blocker判断がartifactとcommandで追跡できる。
- runtime errorがUIへ伝播可能なtyped categoryを持つ。
- ADR状態が現実と一致する。


**検証**

```bash
cargo test -p vtuber-inference error
cargo run -p xtask -- model inspect --help
```

#### G0-05-008: model provenance gateを総合検証する

状態: `LEGACY_PROGRESS`
依存: `G0-05-007`
親参照: DESIGN.md §3、§10.3、§14.2〜§14.4、§25 R1

**変更候補**

- `assets/models/`
- `LICENSES/`
- `crates/vtuber-inference/`
- `docs/adr/ADR-001-*.md`


**実装指示**

- manifest、artifact hash、license、operator inventory、tensor contract、golden outputを一括検証する。
- Windows／macOSの実行結果を区別して記録する。
- model load failure時にapp実装へ進めないgate conditionを明文化する。
- secretやprivate URLなしで別環境が再現できるか確認する。


**このsubtaskで行わないこと**

- production workerを実装しない。
- 検証未完のmodelをdefault assetにしない。


**完了条件**

- G0-05の全受入条件を満たす。
- model hash mismatchでcommandがnon-zero終了する。
- license fileとsource metadataが揃う。


**検証**

```bash
cargo run -p xtask -- model verify
cargo test -p vtuber-inference --no-fail-fast
cargo clippy -p vtuber-inference -p xtask --all-targets -- -D warnings
```

## G0-06: canonical coordinateとhead pose proof
状態: `LEGACY_PROGRESS`
実行単位: `G0-06-NNN`
重点参照: DESIGN.md §11.4〜§11.6、§15.1〜§15.3、§21.1


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

### 実行subtask

#### G0-06-001: canonical coordinate／pose typesを固定する

状態: `LEGACY_PROGRESS`
依存: `G0-01`
親参照: DESIGN.md §11.4〜§11.6、§15.1〜§15.3、§21.1

**変更候補**

- `crates/vtuber-core/src/coordinate.rs`
- `crates/vtuber-core/src/pose.rs`


**実装指示**

- camera image、model output、canonical face、semantic yaw/pitch/rollの座標契約を別type／newtypeで表現する。
- 右手系／左手系、forward、up、positive rotationをrustdocへ明記する。
- `HeadPose`は単位radを型またはfield docで固定する。
- preview mirrorは表示属性として分離し、math typeへ入れない。


**このsubtaskで行わないこと**

- generic `Vec3`だけで座標意味を隠さない。
- Bevy Quatをcoreへ持ち込まない。


**完了条件**

- 座標変換の向きがAPI名から判別できる。
- degree／radianを混在させない。
- `vtuber-core`がBevy非依存を維持する。


**検証**

```bash
cargo test -p vtuber-core coordinate
cargo clippy -p vtuber-core --all-targets -- -D warnings
```

#### G0-06-002: stable landmark subset contractを定義する

状態: `LEGACY_PROGRESS`
依存: `G0-06-001`
親参照: DESIGN.md §11.4〜§11.6、§15.1〜§15.3、§21.1

**変更候補**

- `crates/vtuber-tracking/src/pose/landmark_subset.rs`
- `crates/vtuber-core/src/landmark.rs`


**実装指示**

- head poseに使用するlandmark index、weight、必須／optionalをmodel manifestと対応づける。
- 口／瞼等の変形が強いpointをsubsetから除外する。
- missing／low-confidence pointをfilterし、最低point数とweight合計を検証する。
- index out-of-rangeをtyped errorにする。


**このsubtaskで行わないこと**

- 顔全pointを無条件使用しない。
- 実写調整値を根拠なく固定しない。


**完了条件**

- subsetがdeterministicな順序を持つ。
- insufficient pointが明確なerrorになる。
- model別indexがsource codeへ無秩序に散らばらない。


**検証**

```bash
cargo test -p vtuber-tracking landmark_subset
cargo clippy -p vtuber-tracking --all-targets -- -D warnings
```

#### G0-06-003: weighted centeringとcovariance計算を実装する

状態: `LEGACY_PROGRESS`
依存: `G0-06-002`
親参照: DESIGN.md §11.4〜§11.6、§15.1〜§15.3、§21.1

**変更候補**

- `crates/vtuber-tracking/src/pose/kabsch.rs`


**実装指示**

- neutral／current point setを同じ有効indexで揃える。
- weight合計でweighted centroidを計算し、centered pointsと3×3 covarianceを作る。
- zero／non-finite weightとnon-finite coordinateを拒否する。
- 内部計算precisionと最終f32変換位置を一貫させる。


**このsubtaskで行わないこと**

- SVDやquaternion変換を同時実装しない。
- unchecked indexingを使わない。


**完了条件**

- translationだけのpoint setでidentity rotationになる。
- weight scaleを一様倍しても結果が変わらない。
- NaN入力がsilent propagationしない。


**検証**

```bash
cargo test -p vtuber-tracking weighted_centering
cargo clippy -p vtuber-tracking --all-targets -- -D warnings
```

#### G0-06-004: SVDとreflection correctionを実装する

状態: `LEGACY_PROGRESS`
依存: `G0-06-003`
親参照: DESIGN.md §11.4〜§11.6、§15.1〜§15.3、§21.1

**変更候補**

- `crates/vtuber-tracking/src/pose/kabsch.rs`
- `crates/vtuber-tracking/tests/kabsch.rs`


**実装指示**

- 3×3 covarianceのSVDからrotation matrixを構築する。
- determinantが負の場合に最後のsingular vectorを反転してproper rotationへ補正する。
- singular valueからcollinear／degenerate判定を行う。
- 数学libraryのerror／non-convergenceをtyped errorへ変換する。


**このsubtaskで行わないこと**

- reflectionをabs determinantで握り潰さない。
- unsafe SIMDを追加しない。


**完了条件**

- mirror point setをrotationとして採用しない。
- rotation matrix determinantが+1近傍になる。
- collinear fixtureがdegenerate errorになる。


**検証**

```bash
cargo test -p vtuber-tracking kabsch_reflection
cargo test -p vtuber-tracking kabsch_degenerate
```

#### G0-06-005: rotation matrixからsemantic head poseへ変換する

状態: `LEGACY_PROGRESS`
依存: `G0-06-004`
親参照: DESIGN.md §11.4〜§11.6、§15.1〜§15.3、§21.1

**変更候補**

- `crates/vtuber-tracking/src/pose/mod.rs`
- `crates/vtuber-core/src/pose.rs`


**実装指示**

- rotation matrixをnormalized quaternionへ変換する。
- model coordinateからcanonical basisへの前後変換を明示的なmatrixで行う。
- semantic yaw／pitch／rollを定義済みEuler orderで抽出する。
- quaternion sign ambiguityを連続性比較で扱えるhelperを用意する。


**このsubtaskで行わないこと**

- preview mirror flagで符号を反転しない。
- Euler角を内部累積状態に使わない。


**完了条件**

- identity、±yaw、±pitch、±roll fixtureが期待符号を返す。
- quaternion normが1近傍である。
- Euler境界のtestがある。


**検証**

```bash
cargo test -p vtuber-tracking pose_signs
cargo test -p vtuber-core coordinate
```

#### G0-06-006: synthetic point cloud fixture generatorを作る

状態: `LEGACY_PROGRESS`
依存: `G0-06-005`
親参照: DESIGN.md §11.4〜§11.6、§15.1〜§15.3、§21.1

**変更候補**

- `crates/vtuber-tracking/tests/support/`
- `crates/vtuber-tracking/tests/pose_synthetic.rs`


**実装指示**

- 非対称な3D neutral point cloudを固定seedで生成または静的定義する。
- known rotation、translation、noise、missing pointを適用するhelperを作る。
- expected quaternion／semantic anglesをfixture metadataへ持たせる。
- test fixtureはcamera／model runtimeに依存させない。


**このsubtaskで行わないこと**

- random testだけに依存しない。
- 実人物landmarkをfixtureに含めない。


**完了条件**

- 同じseedで同じfixtureが生成される。
- 対称性による180度曖昧性がない。
- known transformを直接比較できる。


**検証**

```bash
cargo test -p vtuber-tracking pose_synthetic -- --nocapture
```

#### G0-06-007: noise、missing point、degeneracyの数値testを追加する

状態: `LEGACY_PROGRESS`
依存: `G0-06-006`
親参照: DESIGN.md §11.4〜§11.6、§15.1〜§15.3、§21.1

**変更候補**

- `crates/vtuber-tracking/tests/pose_synthetic.rs`


**実装指示**

- 小noise下のangle error上限を定義する。
- optional point欠落でも最低条件を満たせば回復するtestを追加する。
- insufficient、collinear、zero weight、NaNをtyped errorとしてtestする。
- tolerance値と選定理由をtest commentへ記載する。


**このsubtaskで行わないこと**

- 実測前に過度に厳しい性能testを入れない。
- 失敗caseを`unwrap_err`不能なpanicにしない。


**完了条件**

- known rotationを小さな誤差で復元する。
- 異常入力でpanicしない。
- reflection／degenerate caseがfalse positiveで成功しない。


**検証**

```bash
cargo test -p vtuber-tracking pose --no-fail-fast
```

#### G0-06-008: coordinate／head pose proofを文書化して総合検証する

状態: `LEGACY_PROGRESS`
依存: `G0-06-007`
親参照: DESIGN.md §11.4〜§11.6、§15.1〜§15.3、§21.1

**変更候補**

- `DESIGN.mdまたはADR-004 amendment（差分がある場合のみ）`
- `crates/vtuber-tracking/`


**実装指示**

- 実装したbasis、Euler order、positive direction、reflection処理を設計と照合する。
- 設計から逸脱した場合だけADR amendmentを追加し、コードに合わせて黙って文書を書き換えない。
- core／trackingの全coordinate testを実行する。
- 手動実写確認をGate 0の数値proofの代替にしない。


**このsubtaskで行わないこと**

- M1 calibration／filterを始めない。
- 目視だけで符号を決めない。


**完了条件**

- G0-06の全受入条件を満たす。
- yaw／pitch／roll符号が一箇所で定義される。
- mirror previewがmathへ影響しないことをtestで確認する。


**検証**

```bash
cargo test -p vtuber-tracking pose
cargo test -p vtuber-core coordinate
cargo clippy -p vtuber-core -p vtuber-tracking --all-targets -- -D warnings
```

## G0-07: LatestSlotとworker shutdown proof
状態: `LEGACY_PROGRESS`
実行単位: `G0-07-NNN`
重点参照: DESIGN.md §12、§20.2〜§20.3、§21.1


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

### 実行subtask

#### G0-07-001: `LatestSlot<T>`のAPIとstateを定義する

状態: `LEGACY_PROGRESS`
依存: `G0-01`
親参照: DESIGN.md §12、§20.2〜§20.3、§21.1

**変更候補**

- `crates/vtuber-core/src/latest_slot.rs`


**実装指示**

- single latest value、sequence、closed flag、condvarまたは同等primitiveを持つ構造を定義する。
- producer／consumer handle分離が必要なら最小のtypeで表現する。
- `publish`、`try_take_after`または同等API、timeout wait、closeの意味をrustdocへ書く。
- `T: Send`等のtrait boundを必要箇所だけに置く。


**このsubtaskで行わないこと**

- FIFO queueを内部に使わない。
- async runtimeを追加しない。


**完了条件**

- 保持可能件数が構造上1件である。
- sequence overflow方針が明示される。
- closed後のAPI結果が一貫する。


**検証**

```bash
cargo test -p vtuber-core latest_slot_api
cargo clippy -p vtuber-core --all-targets -- -D warnings
```

#### G0-07-002: publish／overwrite semanticsを実装する

状態: `LEGACY_PROGRESS`
依存: `G0-07-001`
親参照: DESIGN.md §12、§20.2〜§20.3、§21.1

**変更候補**

- `crates/vtuber-core/src/latest_slot.rs`


**実装指示**

- publish時に旧valueを置換しsequenceを進める。
- 未読valueを上書きした場合だけoverwrite metricを増やす。
- consumerが取得したsequenceを記録してduplicate deliveryを防ぐ。
- lock保持中にuser codeを呼ばない。


**このsubtaskで行わないこと**

- value historyを保持しない。
- sequence比較をtimestampだけに依存しない。


**完了条件**

- slow consumerが最終publish valueを取得する。
- 同じsequenceを二度返さない。
- overwrite countが期待値と一致する。


**検証**

```bash
cargo test -p vtuber-core latest_slot_overwrite
```

#### G0-07-003: timeout waitとclose wake-upを実装する

状態: `LEGACY_PROGRESS`
依存: `G0-07-002`
親参照: DESIGN.md §12、§20.2〜§20.3、§21.1

**変更候補**

- `crates/vtuber-core/src/latest_slot.rs`
- `crates/vtuber-core/tests/latest_slot.rs`


**実装指示**

- 指定sequenceより新しいvalueをtimeout付きで待てるようにする。
- close時に全waiterをwakeし、closed resultを返す。
- spurious wake-upをloop predicateで処理する。
- timeoutとcloseの競合をdeterministic testで扱う。


**このsubtaskで行わないこと**

- busy pollingを使わない。
- close後publishをsilent successにしない。


**完了条件**

- valueなしtimeoutが指定時間近傍で戻る。
- closeでwaiterが即時解除される。
- hangするtestがない。


**検証**

```bash
cargo test -p vtuber-core latest_slot_wait -- --nocapture
```

#### G0-07-004: LatestSlot metrics snapshotを追加する

状態: `LEGACY_PROGRESS`
依存: `G0-07-003`
親参照: DESIGN.md §12、§20.2〜§20.3、§21.1

**変更候補**

- `crates/vtuber-core/src/latest_slot.rs`
- `crates/vtuber-core/src/metrics.rs（必要なら）`


**実装指示**

- published、consumed、overwritten、last sequence、closedを固定size counterで取得できるようにする。
- metrics取得がvalue cloneを要求しない。
- counter memory orderingまたはlock一貫性をcommentで説明する。
- metrics APIをcamera／inference固有名にしない。


**このsubtaskで行わないこと**

- unbounded histogramを追加しない。
- global metrics singletonを作らない。


**完了条件**

- 100k publish testでcounterが期待値を示す。
- 保持件数が常に0または1と証明できる。
- metrics取得がproducerを長時間blockしない。


**検証**

```bash
cargo test -p vtuber-core latest_slot_metrics
```

#### G0-07-005: bounded worker control channelとstatus contractを定義する

状態: `LEGACY_PROGRESS`
依存: `G0-07-004`
親参照: DESIGN.md §12、§20.2〜§20.3、§21.1

**変更候補**

- `crates/vtuber-core/src/worker.rs`


**実装指示**

- Start／Stopが必要ならcontroller側state、worker側command、status eventを明示する。
- control channel capacityを固定し、Stopが他commandで永久に塞がれないpolicyを定める。
- Running、Stopped、Failed、Panicked、Disconnectedをtyped statusにする。
- workerへlive backend objectを渡さずfactory inputだけを渡せるcontractにする。


**このsubtaskで行わないこと**

- actor frameworkを導入しない。
- workerをdetachしない。


**完了条件**

- control channelがboundedである。
- unexpected disconnectをstatusとして観測できる。
- generic contractがcamera／inference双方で再利用可能である。


**検証**

```bash
cargo test -p vtuber-core worker_contract
cargo clippy -p vtuber-core --all-targets -- -D warnings
```

#### G0-07-006: generic worker supervisor test doubleを実装する

状態: `LEGACY_PROGRESS`
依存: `G0-07-005`
親参照: DESIGN.md §12、§20.2〜§20.3、§21.1

**変更候補**

- `crates/vtuber-core/src/worker.rs`
- `crates/vtuber-core/tests/worker.rs`


**実装指示**

- named threadをspawnし、factory closure内でresourceをconstructするsupervisorを作る。
- explicit stop signal、join、join timeout相当のtestabilityを提供する。
- worker return errorとpanic payloadをstatusへ変換する。
- Dropだけにshutdownを依存せず、明示APIを主経路にする。


**このsubtaskで行わないこと**

- panicをprocess abortに変換しない。
- JoinHandleを捨てない。


**完了条件**

- normal stopでthreadがjoinされる。
- worker error／panicがmain側で区別される。
- resource dropがworker thread内で起きるtestがある。


**検証**

```bash
cargo test -p vtuber-core worker_supervisor -- --nocapture
```

#### G0-07-007: LatestSlot stress testを追加する

状態: `LEGACY_PROGRESS`
依存: `G0-07-006`
親参照: DESIGN.md §12、§20.2〜§20.3、§21.1

**変更候補**

- `crates/vtuber-core/tests/latest_slot_stress.rs`


**実装指示**

- producerが100k valueをpublishし、consumerを意図的に遅延させる。
- 最終value、sequence monotonicity、overwrite count、保持件数を検証する。
- test durationに上限を設け、failure時にhangせず終了する。
- 複数consumerをサポートしない設計なら、そのことをtest／docで明示する。


**このsubtaskで行わないこと**

- benchmarkをtest成功条件にしない。
- sleepだけに依存するflaky assertionを避ける。


**完了条件**

- 10万publishしても保持件数が1を超えない。
- consumerが最終valueへ追いつく。
- data race sanitizerなしでもdeterministicに再現する。


**検証**

```bash
cargo test -p vtuber-core latest_slot_stress -- --nocapture
```

#### G0-07-008: shutdown proofを総合検証する

状態: `LEGACY_PROGRESS`
依存: `G0-07-007`
親参照: DESIGN.md §12、§20.2〜§20.3、§21.1

**変更候補**

- `crates/vtuber-core/`
- `G0-07関連test`


**実装指示**

- close待機解除、normal stop、worker error、panic、control disconnectを全て実行する。
- 各testに最大待機時間を設定し、hangをfailureへ変換する。
- API rustdocと実挙動を照合する。
- 後続camera／inferenceが使う最小exampleをdoc testまたはunit testへ残す。


**このsubtaskで行わないこと**

- production workerを実装しない。
- timeout failureをignoreしない。


**完了条件**

- G0-07の全受入条件を満たす。
- unbounded data channelがdependency／sourceにない。
- worker thread leakをtestが検知できる。


**検証**

```bash
cargo test -p vtuber-core latest_slot worker --no-fail-fast
cargo clippy -p vtuber-core --all-targets -- -D warnings
```

## G0-08: bevy_vrm1 compatibility gate
状態: `LEGACY_PROGRESS`
実行単位: `G0-08-NNN`
重点参照: DESIGN.md §7.2〜§7.4、§16、§21.3、§25 R2


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

### 実行subtask

#### G0-08-001: compatibility model inventoryとreport entryを作る

状態: `LEGACY_PROGRESS`
依存: `G0-02、G0-03`
親参照: DESIGN.md §7.2〜§7.4、§16、§21.3、§25 R2

**変更候補**

- `docs/MODEL_COMPATIBILITY_TEMPLATE.md`
- `tests/fixtures/vrm/manifest.tomlまたはdocs compatibility inventory`


**実装指示**

- 公式sample、VRoid Studio export、実利用予定modelの3区分を一意IDで登録する。
- 各modelのSHA-256、exporter／version、取得日、license／保管場所を記録する。
- private modelはrepositoryへcommitせず、local path／hashだけでtest harnessへ渡せるようにする。
- 結果欄をWindows／macOSで分離する。


**このsubtaskで行わないこと**

- 権利不明modelをbundleへ追加しない。
- 結果未記入をpass扱いしない。


**完了条件**

- 3区分がreportで追跡可能である。
- hashなしmodelをtest対象として扱わない。
- private assetを誤commitしないignore policyがある。


**検証**

```bash
sha256sum tests/fixtures/vrm/*.vrm 2>/dev/null || true
cargo check -p vtuber-desktop
```

#### G0-08-002: compatibility harnessでimport／load／Initializedを自動確認する

状態: `LEGACY_PROGRESS`
依存: `G0-08-001`
親参照: DESIGN.md §7.2〜§7.4、§16、§21.3、§25 R2

**変更候補**

- `crates/vtuber-avatar/tests/compatibility.rs`
- `tools/xtask/src/vrm_compat.rs`


**実装指示**

- model pathを受け取り、G0-03 preflight、import cache、`VrmHandle` loadを順に実行するharnessを作る。
- `Initialized`までのtimeoutとstructured resultを実装する。
- panicをcatch可能なtest境界で記録し、stack trace取得手順を出す。
- GUIが必要な確認とheadless可能な確認を分離する。


**このsubtaskで行わないこと**

- 独自loaderへ迂回しない。
- failureをwarningだけで継続しない。


**完了条件**

- 各modelのload成功／失敗stageが機械的に記録される。
- timeoutが無期限hangしない。
- invalid modelのfailureがcompatibility passにならない。


**検証**

```bash
cargo test -p vtuber-avatar compatibility -- --ignored --nocapture
cargo run -p xtask -- vrm-compat --help
```

#### G0-08-003: humanoid boneとExpression capabilityを収集する

状態: `LEGACY_PROGRESS`
依存: `G0-08-002`
親参照: DESIGN.md §7.2〜§7.4、§16、§21.3、§25 R2

**変更候補**

- `crates/vtuber-avatar/src/compatibility.rs`
- `crates/vtuber-avatar/tests/compatibility.rs`


**実装指示**

- head、neck、left/right eyeの存在をroot componentsから確認する。
- `ExpressionEntityMap`からpreset名をstable順序で収集する。
- required／optional capabilityをreport schemaへ記録する。
- 同じmodelでloadごとにcapabilityが変わらないことを検証する。


**このsubtaskで行わないこと**

- boneをName検索だけで再実装しない。
- Expressionを実際に適用しない。


**完了条件**

- head必須、neck／eyes optionalの判定が正しい。
- blink、mouth、gaze presetの有無をreportできる。
- capability discoveryが毎frame実行されない。


**検証**

```bash
cargo test -p vtuber-avatar compatibility_capabilities -- --ignored
```

#### G0-08-004: MToon／outline／material casesを目視検証可能にする

状態: `LEGACY_PROGRESS`
依存: `G0-08-003`
親参照: DESIGN.md §7.2〜§7.4、§16、§21.3、§25 R2

**変更候補**

- `apps/desktop/src/compatibility_scene.rs`
- `docs/MODEL_COMPATIBILITY_TEMPLATE.md`


**実装指示**

- 固定camera／light条件とスクリーンショット手順を用意する。
- duplicate material name、transparent material、outline有無をmodelごとに確認する。
- MToon parameterをアプリ側で補正せず、`bevy_vrm1`出力をそのまま評価する。
- pass／minor／blockerの判定基準をreportへ書く。


**このsubtaskで行わないこと**

- 見た目差を無断でmaterial overrideして隠さない。
- 画像比較基盤を過剰実装しない。


**完了条件**

- 同じscene条件でWindows／macOS比較ができる。
- dark material、missing outline、transparent orderingを区別して記録できる。
- アプリ側独自shaderがない。


**検証**

```bash
cargo check -p vtuber-desktop
cargo run -p vtuber-desktop -- --help
```

#### G0-08-005: SpringBone、lookAt type、optional-field variantsを検査する

状態: `LEGACY_PROGRESS`
依存: `G0-08-004`
親参照: DESIGN.md §7.2〜§7.4、§16、§21.3、§25 R2

**変更候補**

- `crates/vtuber-avatar/src/compatibility.rs`
- `docs/MODEL_COMPATIBILITY_TEMPLATE.md`


**実装指示**

- SpringBone component／registry初期化の有無とruntime panicを確認する。
- raw preflight情報から`lookAt.type`をbone／expression／noneで記録する。
- optional field欠落variant fixtureをloadし、strict deserialization failureを検出する。
- product pathで`LookAt`／`BodyTracking` componentを付与しないassertionを追加する。


**このsubtaskで行わないこと**

- `todo!()`へ到達する`LookAt`を使用しない。
- upstream bugをsilent fallbackで隠さない。


**完了条件**

- expression lookAt modelで危険pathを実行しない。
- SpringBoneなしmodelも正常loadする。
- optional field起因failureがfixtureとstack traceで再現できる。


**検証**

```bash
cargo test -p vtuber-avatar compatibility_vrm_features -- --ignored
```

#### G0-08-006: known failureを最小fixture／regression testへ縮小する

状態: `LEGACY_PROGRESS`
依存: `G0-08-005`
親参照: DESIGN.md §7.2〜§7.4、§16、§21.3、§25 R2

**変更候補**

- `crates/vtuber-avatar/tests/fixtures/`
- `crates/vtuber-avatar/tests/compatibility_regression.rs`


**実装指示**

- 実model failureから必要なGLB／extension fieldだけを残した最小再現fixtureを作る。
- 元modelのlicenseが再配布を許さない場合はsynthetic fixtureを生成する。
- panic、missing material、deserialization等のfailureを個別test名にする。
- fixture生成手順とhashを記録する。


**このsubtaskで行わないこと**

- 巨大な実modelをそのままtestへ複製しない。
- binaryを手編集して由来不明にしない。


**完了条件**

- failureが小さなfixtureで安定再現する。
- private model dataを含まない。
- upstream update時にregression解消を検知できる。


**検証**

```bash
cargo test -p vtuber-avatar compatibility_regression -- --nocapture
```

#### G0-08-007: Windows／macOS compatibility reportを埋める

状態: `LEGACY_PROGRESS`
依存: `G0-08-006`
親参照: DESIGN.md §7.2〜§7.4、§16、§21.3、§25 R2

**変更候補**

- `docs/model-compatibility-*.mdまたはtemplate instance`


**実装指示**

- 各modelを両OSで同一protocolに通し、import、load、bone、Expression、MToon、outline、SpringBone、transparent、lookAtを記録する。
- OS、GPU backend、Bevy、bevy_vrm1 revision、model hashをreport headerへ書く。
- 未実行項目を`N/A`ではなく`NOT RUN`として理由付きで残す。
- blockerへ再現commandとlog locationを付ける。


**このsubtaskで行わないこと**

- 未実行OSをpassにしない。
- 手動結果を口頭だけで残さない。


**完了条件**

- 全対象modelにWindows／macOS result rowがある。
- pass判定が判定基準に従う。
- valid VRMのpanicにはstack traceがある。


**検証**

```bash
cargo run -p xtask -- vrm-compat --help
```

#### G0-08-008: fork patch要否を決定しADR-002を更新する

状態: `LEGACY_PROGRESS`
依存: `G0-08-007`
親参照: DESIGN.md §7.2〜§7.4、§16、§21.3、§25 R2

**変更候補**

- `docs/adr/ADR-002-bevy-vrm1-runtime.md`
- `REFERENCES.md（必要な場合）`


**実装指示**

- compatibility blockerをupstream source／issue／commitと照合する。
- product blockerがなければpinned upstream採用をAcceptedとする。
- blockerがある場合だけ最小fork patch proposal、patch範囲、upstream追従方法を記載する。
- fork判断と別に、`LookAt`／`BodyTracking`非使用方針を維持する。


**このsubtaskで行わないこと**

- 単なる改善希望でforkしない。
- revisionを未検証commitへ更新しない。


**完了条件**

- forkする／しないが明文化される。
- 判断がcompatibility reportに紐づく。
- G0-08の全受入条件を満たす。


**検証**

```bash
cargo test -p vtuber-avatar compatibility_regression
cargo clippy -p vtuber-avatar -p vtuber-desktop --all-targets -- -D warnings
```

## M1-01: production capture service
状態: `LEGACY_PROGRESS`
実行単位: `M1-01-NNN`
重点参照: DESIGN.md §12、§13、§20.2〜§20.3


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

### 実行subtask

#### M1-01-001: `CaptureController`のpublic contractとstateを定義する

状態: `LEGACY_PROGRESS`
依存: `G0-04、G0-07`
親参照: DESIGN.md §12、§13、§20.2〜§20.3

**変更候補**

- `crates/vtuber-camera/src/controller.rs`
- `crates/vtuber-camera/src/lib.rs`


**実装指示**

- Idle、Starting、Running、Reconnecting、Stopping、Failedのstateを定義する。
- start、stop、select device、status snapshotの同期境界を明示する。
- controllerはdevice descriptor／format requestだけをworkerへ渡す。
- 二重start、stop中start、drop時の挙動をtyped resultにする。


**このsubtaskで行わないこと**

- Bevy resourceをcamera crateへ持ち込まない。
- reconnect loopをまだ実装しない。


**完了条件**

- public APIがUI threadをblockしない。
- invalid state transitionがtest可能である。
- live camera objectがcontroller fieldにない。


**検証**

```bash
cargo test -p vtuber-camera capture_controller_state
cargo clippy -p vtuber-camera --all-targets -- -D warnings
```

#### M1-01-002: capture worker startup／resource ownershipを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-01-001`
親参照: DESIGN.md §12、§13、§20.2〜§20.3

**変更候補**

- `crates/vtuber-camera/src/worker.rs`


**実装指示**

- generic supervisorを使い、worker startup内でnokhwa cameraをconstructする。
- permission済みdescriptor、selected format、output slot、control receiverだけをworkerへ渡す。
- open成功後にRunning statusをpublishする。
- camera objectのstop／dropがworker thread内で行われる。


**このsubtaskで行わないこと**

- frame capture loopを複雑化しない。
- global camera singletonを作らない。


**完了条件**

- main threadからcamera objectをmoveしない。
- open failureがcontrollerへ返る。
- named threadとjoin handleが管理される。


**検証**

```bash
cargo test -p vtuber-camera capture_worker_startup
```

#### M1-01-003: capture loopと`LatestSlot<VideoFrame>` publishを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-01-002`
親参照: DESIGN.md §12、§13、§20.2〜§20.3

**変更候補**

- `crates/vtuber-camera/src/worker.rs`
- `crates/vtuber-core/src/frame.rs（必要なら）`


**実装指示**

- frameをcapture／RGB decodeし、monotonic timestampとsequenceを付ける。
- owned bufferを`LatestSlot`へpublishし、旧未読frameを蓄積しない。
- Stop commandを各capture iteration間で確認する。
- decode size／stride異常をtyped failureとして扱う。


**このsubtaskで行わないこと**

- preview変換をworkerへ入れない。
- FIFO channelを追加しない。


**完了条件**

- frame queueが1件を超えない。
- sequenceが単調増加する。
- consumerなしでもmemoryが増え続けない。


**検証**

```bash
cargo test -p vtuber-camera capture_loop_fake
cargo test -p vtuber-core latest_slot
```

#### M1-01-004: capture／decode timingとmetricsを追加する

状態: `LEGACY_PROGRESS`
依存: `M1-01-003`
親参照: DESIGN.md §12、§13、§20.2〜§20.3

**変更候補**

- `crates/vtuber-camera/src/metrics.rs`
- `crates/vtuber-camera/src/worker.rs`


**実装指示**

- captured、decoded、published、dropped／overwritten、capture duration、decode durationをfixed-size metricsで集計する。
- timestampはmonotonic clockに統一する。
- metrics snapshotをcontrollerからnon-blockingに取得できるようにする。
- raw frameやlandmarkをmetricsへ保存しない。


**このsubtaskで行わないこと**

- unbounded sample vectorを持たない。
- wall-clock timeをlatency基準にしない。


**完了条件**

- fake backend testでcounterが期待値と一致する。
- metrics collectionがframe pathへ大きなallocationを追加しない。
- overwrite countがLatestSlotと整合する。


**検証**

```bash
cargo test -p vtuber-camera capture_metrics
```

#### M1-01-005: 明示stop／joinとdrop fallbackを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-01-004`
親参照: DESIGN.md §12、§13、§20.2〜§20.3

**変更候補**

- `crates/vtuber-camera/src/controller.rs`
- `crates/vtuber-camera/src/worker.rs`


**実装指示**

- stopでcontrol signal、slot close、worker joinを順序立てて行う。
- 複数stopをidempotentに扱う。
- Dropはbest-effort fallbackに限定し、通常経路は明示stopを使う。
- join failure／panicをstatusへ反映する。


**このsubtaskで行わないこと**

- JoinHandleをdetachしない。
- Drop内で無期限blockしない。


**完了条件**

- stop後にthreadが残らない。
- blocked fake captureでもtimeout／closeでtestが終了する。
- panic時にprocess全体が落ちない。


**検証**

```bash
cargo test -p vtuber-camera capture_shutdown -- --nocapture
```

#### M1-01-006: bounded reconnect state machineを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-01-005`
親参照: DESIGN.md §12、§13、§20.2〜§20.3

**変更候補**

- `crates/vtuber-camera/src/reconnect.rs`
- `crates/vtuber-camera/src/worker.rs`


**実装指示**

- recoverable capture errorをStopped／Fatalと区別し、bounded retryへ送る。
- retry count、initial delay、max delay、total limitを設定値として固定範囲にする。
- Stop要求がbackoff待機を中断できるようにする。
- reopenごとにcamera objectをworker内で再constructする。


**このsubtaskで行わないこと**

- 無限retryしない。
- 全errorをrecoverable扱いしない。


**完了条件**

- reconnect上限到達でFailedへ移る。
- stop中に再openしない。
- fake error sequenceでstate transitionをdeterministic testできる。


**検証**

```bash
cargo test -p vtuber-camera reconnect_state
```

#### M1-01-007: device removal／device selection変更を処理する

状態: `LEGACY_PROGRESS`
依存: `M1-01-006`
親参照: DESIGN.md §12、§13、§20.2〜§20.3

**変更候補**

- `crates/vtuber-camera/src/controller.rs`
- `crates/vtuber-camera/src/reconnect.rs`


**実装指示**

- device unavailableを専用statusへ変換し、process crashを防ぐ。
- running中のselect deviceはstop→join→new startの明確なtransitionにする。
- index再割当を考慮し、descriptor matchingをname／backend metadataで再解決する。
- 選択失敗時に旧workerが残らないようにする。


**このsubtaskで行わないこと**

- OS hotplug watcherを過剰実装しない。
- index 0へ勝手にfallbackしない。


**完了条件**

- fake device removalでFailedまたはReconnectingへ遷移する。
- device切替後にworkerが一つだけ存在する。
- camera抜去でpanicしない。


**検証**

```bash
cargo test -p vtuber-camera device_removal
cargo test -p vtuber-camera device_switch
```

#### M1-01-008: production capture serviceを総合検証する

状態: `LEGACY_PROGRESS`
依存: `M1-01-007`
親参照: DESIGN.md §12、§13、§20.2〜§20.3

**変更候補**

- `crates/vtuber-camera/`
- `tools/xtask/src/camera.rs`


**実装指示**

- fake backendでstart、frames、slow consumer、stop、reconnect、remove、switchを通す。
- 実hardware smokeで長時間ではなく基本start／stopを確認する。
- metrics、thread count、LatestSlot countを報告する。
- G0-04 smoke codeと重複した処理をproduction serviceへ統合し、二重実装を残さない。


**このsubtaskで行わないこと**

- inference workerを始めない。
- hardware未実行を成功扱いしない。


**完了条件**

- M1-01の全受入条件を満たす。
- UI threadをblockするAPIがない。
- queue 1、bounded reconnect、clean joinがtestで保証される。


**検証**

```bash
cargo test -p vtuber-camera --no-fail-fast
cargo clippy -p vtuber-camera --all-targets -- -D warnings
cargo test -p vtuber-camera -- --ignored
```

## M1-02: production inference worker
状態: `LEGACY_PROGRESS`
実行単位: `M1-02-NNN`
重点参照: DESIGN.md §12、§14、§20.1〜§20.3


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

### 実行subtask

#### M1-02-001: inference worker contract／state／statusを定義する

状態: `LEGACY_PROGRESS`
依存: `M1-01、G0-05`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/src/controller.rs`
- `crates/vtuber-inference/src/lib.rs`


**実装指示**

- Idle、LoadingModel、Running、Stopping、Failedのstateを定義する。
- 入力は`LatestSlot<VideoFrame>`、出力は`LatestSlot<RawFaceObservation>`、controlはbounded channelにする。
- model descriptor、runtime settings、stop handleだけをcontrollerから渡す。
- model load failureとper-frame inference failureを別statusにする。


**このsubtaskで行わないこと**

- runtime objectをcontrollerに保持しない。
- tracking filterを追加しない。


**完了条件**

- public APIがcamera crate／Bevyに依存しない。
- state transition testがある。
- unbounded frame queueがない。


**検証**

```bash
cargo test -p vtuber-inference worker_state
cargo clippy -p vtuber-inference --all-targets -- -D warnings
```

#### M1-02-002: worker startup内でmodel runtimeをconstructする

状態: `LEGACY_PROGRESS`
依存: `M1-02-001`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/src/worker.rs`
- `crates/vtuber-inference/src/backend/tract.rs`


**実装指示**

- worker thread起動後にmanifest verify、model load、optimize、runnable化を行う。
- 成功時だけRunning statusをpublishする。
- runtime objectはworker localに保持し、main threadからmoveしない。
- startup failure時にinput slotを消費せず、exact stage／causeを返す。


**このsubtaskで行わないこと**

- 毎frame modelをloadしない。
- C runtimeへfallbackしない。


**完了条件**

- model objectを一回だけconstructする。
- hash mismatch／load errorがmainへ届く。
- resource dropがworker threadで起きる。


**検証**

```bash
cargo test -p vtuber-inference worker_model_startup
```

#### M1-02-003: preprocess buffersをworker内で再利用する

状態: `LEGACY_PROGRESS`
依存: `M1-02-002`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/src/worker.rs`
- `crates/vtuber-inference/src/preprocess.rs`


**実装指示**

- 入力RGB resize／normalize用bufferとtract tensor backingをworker startup時に確保する。
- frame size変更時だけ必要なbufferをresizeし、通常frameでallocationしない。
- preprocess contractはG0-05 goldenと同じ関数を使用する。
- stride／resolution mismatchをtyped frame errorにする。


**このsubtaskで行わないこと**

- 別のpreprocess実装をworker内に複製しない。
- frameごとにVecを新規作成しない。


**完了条件**

- 連続同size frameでcapacityが増えない。
- golden preprocess testを再利用する。
- invalid frameがpanicしない。


**検証**

```bash
cargo test -p vtuber-inference preprocess_reuse
```

#### M1-02-004: 最新frame consumeとduplicate suppressionを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-02-003`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/src/worker.rs`


**実装指示**

- last processed sequenceより新しいframeだけをLatestSlotから取得する。
- 推論中に複数frameが到着した場合は次loopで最新sequenceだけを取る。
- same frame sequenceを二度inferしないassertion／metricを入れる。
- wait timeout中にStop commandを確認する。


**このsubtaskで行わないこと**

- FIFO consumeしない。
- timestampだけでduplicate判定しない。


**完了条件**

- camera 30fps／fake inference 15fpsでbacklogが増えない。
- processed sequenceがstrictly increasingである。
- stopがinputなしでも完了する。


**検証**

```bash
cargo test -p vtuber-inference latest_frame_consumption -- --nocapture
```

#### M1-02-005: detector cadenceとROI stateを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-02-004`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/src/pipeline.rs`
- `crates/vtuber-inference/src/roi.rs`


**実装指示**

- 初期／lost時はdetector、tracking中は設定cadenceまたはROI confidenceに応じて再detectする。
- ROIをframe coordinate、rotation、scale付きのtyped valueとして保持する。
- ROI update failure／out-of-boundsを再detectへ遷移させる。
- cadence counterをframe sequenceに基づける。


**このsubtaskで行わないこと**

- tracking state machine M1-03を実装しない。
- ROIをuntyped tupleにしない。


**完了条件**

- detectorが毎frame無条件実行されない。
- lost後にdetectorへ戻る。
- ROI state transitionをsynthetic resultでtestできる。


**検証**

```bash
cargo test -p vtuber-inference detector_cadence
cargo test -p vtuber-inference roi_state
```

#### M1-02-006: landmark output decodeを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-02-005`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/src/decode/landmarks.rs`
- `crates/vtuber-inference/tests/decode.rs`


**実装指示**

- manifest output contractからtensorを取得し、landmark座標／confidenceをdecodeする。
- ROI-local座標をimage／canonical入力座標へ戻す。
- shape、dtype、element count、NaN／Infを検証する。
- output indexをsource codeへ重複hard-codeしない。


**このsubtaskで行わないこと**

- head pose filterを実装しない。
- silent truncate／zipでshape mismatchを隠さない。


**完了条件**

- golden outputから期待landmark countを得る。
- invalid tensor shapeがtyped errorになる。
- ROI transformのround-trip testがある。


**検証**

```bash
cargo test -p vtuber-inference landmark_decode
```

#### M1-02-007: basic blink／mouth raw observationを抽出する

状態: `LEGACY_PROGRESS`
依存: `M1-02-006`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/src/decode/expressions.rs`
- `crates/vtuber-core/src/observation.rs`


**実装指示**

- backend blendshape outputがある場合はmanifest mappingでblink／mouth raw係数を取得する。
- ない場合はlandmark ratioからbasic blink／mouth opennessを算出する最小fallbackを実装する。
- 値域をraw observation contractへ正規化し、confidenceを別fieldで保持する。
- 左右、口開度、必要なraw係数だけをMVP出力にする。


**このsubtaskで行わないこと**

- 5母音品質処理を先行実装しない。
- model-specific indexをcodeへ直書きしない。


**完了条件**

- 左右blinkとmouth raw値がfinite／boundedである。
- backend unsupported時にpanicせずfallbackまたはNoneを返す。
- calibration済み表情値をここで作らない。


**検証**

```bash
cargo test -p vtuber-inference expression_decode
cargo test -p vtuber-core observation
```

#### M1-02-008: inference timing／drop accountingを追加する

状態: `LEGACY_PROGRESS`
依存: `M1-02-007`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/src/metrics.rs`
- `crates/vtuber-inference/src/worker.rs`


**実装指示**

- wait、preprocess、detector、landmark、decode、total durationをfixed-size metricsへ記録する。
- input overwritten count、skipped sequence、processed count、output overwritten countを集計する。
- capture timestampをRawFaceObservationへ引き継ぐ。
- metrics snapshot取得をnon-blockingまたは短いlockにする。


**このsubtaskで行わないこと**

- performance tuningを始めない。
- wall clockをcapture-to-applyに使わない。


**完了条件**

- fake pipelineでstage timingとdrop countが期待値に一致する。
- camera 30／inference 15 scenarioでlatency backlog増加がない。
- unbounded duration listを保持しない。


**検証**

```bash
cargo test -p vtuber-inference inference_metrics
```

#### M1-02-009: typed failure、stop、joinを完成させる

状態: `LEGACY_PROGRESS`
依存: `M1-02-008`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/src/controller.rs`
- `crates/vtuber-inference/src/worker.rs`
- `crates/vtuber-inference/src/error.rs`


**実装指示**

- model load、preprocess、runtime、decode、input closed、panicを区別する。
- recoverable per-frame errorの連続上限を定め、超過でFailedへ移る。
- Stopでinput waitを解除し、workerをjoinする。
- Dropはbest-effort fallbackに限定する。


**このsubtaskで行わないこと**

- infinite retryしない。
- errorをlogだけで握り潰さない。


**完了条件**

- model load failureがmainへ報告される。
- stop後にthreadが残らない。
- worker panicがstatusへ変換される。


**検証**

```bash
cargo test -p vtuber-inference inference_shutdown -- --nocapture
cargo test -p vtuber-inference worker_failure
```

#### M1-02-010: production inference workerを総合検証する

状態: `LEGACY_PROGRESS`
依存: `M1-02-009`
親参照: DESIGN.md §12、§14、§20.1〜§20.3

**変更候補**

- `crates/vtuber-inference/`
- `M1-02関連test／report`


**実装指示**

- fake runtimeと実golden modelの双方でstartup→frames→output→stopを通す。
- same frame duplicate、30/15fps backlog、model load failure、decode failureを検証する。
- allocation reuseとruntime一回constructをtest counterまたはinstrumentationで確認する。
- M1-02の既存実装を基準に不足だけを修正し、全面rewriteしない。


**このsubtaskで行わないこと**

- M1-03 filterへ進まない。
- 既存進捗を削除して作り直さない。


**完了条件**

- M1-02の全受入条件を満たす。
- `RawFaceObservation`がLatestSlotへpublishされる。
- clean stop／joinとtyped failureが確認される。


**検証**

```bash
cargo test -p vtuber-inference --no-fail-fast
cargo clippy -p vtuber-inference --all-targets -- -D warnings
```

## M1-03: calibration、filter、loss recovery
状態: `LEGACY_PROGRESS`
実行単位: `M1-03-NNN`
重点参照: DESIGN.md §11.5〜§11.8、§15、§18.1


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

### 実行subtask

#### M1-03-001: calibration settingsとsession stateを定義する

状態: `LEGACY_PROGRESS`
依存: `M1-02、G0-06`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/calibration/types.rs`
- `crates/vtuber-core/src/control.rs`


**実装指示**

- 必要sample数、最大duration、confidence threshold、motion thresholdをversioned settingsとして定義する。
- NotStarted、Collecting、Ready、Rejected、Completedのsession stateを型で表現する。
- calibration inputと保存済みneutral profileを別typeにする。
- default値の根拠と単位をrustdocへ記載する。


**このsubtaskで行わないこと**

- UIを実装しない。
- neutral dataをglobal singletonにしない。


**完了条件**

- invalid settingsをconstructorで拒否する。
- session state transitionをunit testできる。
- camera／Bevy依存がtracking crateへ入らない。


**検証**

```bash
cargo test -p vtuber-tracking calibration_types
cargo clippy -p vtuber-tracking --all-targets -- -D warnings
```

#### M1-03-002: calibration sample collectorを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-03-001`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/calibration/collector.rs`
- `crates/vtuber-tracking/tests/calibration.rs`


**実装指示**

- `RawFaceObservation`をsequence順に受け取り、confidence／face count／finite値を検証する。
- head motion、blink、mouth movementが閾値内のneutral候補だけを採用する。
- duplicate sequenceとtimestamp逆行を拒否またはskip metricへ送る。
- sample上限を固定し、unbounded vectorを作らない。


**このsubtaskで行わないこと**

- filter済み値を収集しない。
- 顔なしsampleをneutralとして採用しない。


**完了条件**

- 有効sampleだけがcollectorへ残る。
- 不足／動き過多／低confidenceのreject理由を取得できる。
- 同じinput streamで同じ採否になる。


**検証**

```bash
cargo test -p vtuber-tracking calibration_collector
```

#### M1-03-003: neutral referenceを集約／検証する

状態: `LEGACY_PROGRESS`
依存: `M1-03-002`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/calibration/neutral.rs`
- `crates/vtuber-tracking/tests/calibration.rs`


**実装指示**

- landmarkはmedianまたはrobust mean、expression raw値はbaseline統計として集約する。
- head pose spread、landmark spread、sample countから品質を判定する。
- 必須landmark欠落やdegenerate point cloudをG0-06 errorへ接続する。
- profileへschema version、model hash、camera descriptor fingerprintを保存できるようにする。


**このsubtaskで行わないこと**

- 永続化I/Oを同時実装しない。
- 単純first-frameをneutralにしない。


**完了条件**

- 品質不足profileをCompletedとして返さない。
- outlier一件でneutralが大きく変わらない。
- model hash違いprofileを再利用できない。


**検証**

```bash
cargo test -p vtuber-tracking neutral_reference
```

#### M1-03-004: neutral-relative head pose生成を接続する

状態: `LEGACY_PROGRESS`
依存: `M1-03-003`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/pipeline.rs`
- `crates/vtuber-tracking/src/pose/`


**実装指示**

- current landmarksとneutral referenceをG0-06 weighted Kabschへ渡す。
- semantic `HeadPose`とpose confidenceを生成する。
- insufficient／degenerate resultをtracking failure reasonへ変換する。
- timestampとsource sequenceを維持する。


**このsubtaskで行わないこと**

- loss recoveryを先行実装しない。
- Euler角を累積しない。


**完了条件**

- neutral observationでidentity近傍になる。
- known synthetic streamで期待poseを返す。
- pose error時に前回値をこの層で勝手に再利用しない。


**検証**

```bash
cargo test -p vtuber-tracking neutral_relative_pose
```

#### M1-03-005: head rotation filterをquaternion中心で実装する

状態: `LEGACY_PROGRESS`
依存: `M1-03-004`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/filter/head.rs`
- `crates/vtuber-tracking/tests/filter.rs`


**実装指示**

- 最初は設計既定のexponential／slerp filterを実装する。
- delta timeをmonotonic timestampから計算し、極端なdtをclampする。
- quaternion shortest arcとsign continuityを扱う。
- reset／reacquire APIを明示する。


**このsubtaskで行わないこと**

- One Euro等の比較候補を同時実装しない。
- Euler各軸を独立低域通過しない。


**完了条件**

- constant inputへ収束する。
- quaternion符号反転でjumpしない。
- large dt／zero dt／timestamp逆行でpanicしない。


**検証**

```bash
cargo test -p vtuber-tracking head_filter
```

#### M1-03-006: blink／mouth normalizationを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-03-005`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/filter/expression.rs`
- `crates/vtuber-tracking/tests/expression_normalization.rs`


**実装指示**

- neutral baseline、closed/open calibration rangeから0..1へnormalizeする。
- left/right blinkを別channelとして扱う。
- mouth opennessとbasic mouth coefficientにdead zone、clamp、attack/release smoothingを適用する。
- missing raw channelにはNone／fallback policyを明示する。


**このsubtaskで行わないこと**

- VRM preset mappingを実装しない。
- 5母音coarticulationを先行実装しない。


**完了条件**

- neutralで0近傍、blink／mouth maxで1近傍になる。
- range逆転／zero spanをtyped calibration errorにする。
- 全出力がfiniteかつ0..1内である。


**検証**

```bash
cargo test -p vtuber-tracking expression_normalization
```

#### M1-03-007: confidence hysteresisを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-03-006`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/confidence.rs`
- `crates/vtuber-tracking/tests/confidence.rs`


**実装指示**

- detector、landmark、pose、expression availabilityからframe confidenceを合成する。
- enter thresholdとexit thresholdを分離してhysteresisを作る。
- 連続good／bad frame countをbounded counterで扱う。
- confidence sourceの欠落を0扱いするかignoredにするかをfieldごとに明記する。


**このsubtaskで行わないこと**

- 一つのmagic thresholdだけで全stateを決めない。
- camera FPSに依存する固定秒数をframe countだけで表さない。


**完了条件**

- threshold付近でstateが毎frame振動しない。
- good／bad sequenceから期待acquire／degrade signalが出る。
- NaN confidenceをrejectする。


**検証**

```bash
cargo test -p vtuber-tracking confidence_hysteresis
```

#### M1-03-008: tracking state machineを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-03-007`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/state_machine.rs`
- `crates/vtuber-tracking/tests/state_machine.rs`


**実装指示**

- Searching、Acquiring、Tracking、Degraded、LostHold、ReturningNeutralを明示的なenumとtransitionで実装する。
- transition inputをconfidence signal、elapsed time、new observation、calibration availabilityに限定する。
- 各transitionでfilter reset／hold開始／neutral return開始のactionを返す。
- illegal transitionを発生させないtable-driven実装にする。


**このsubtaskで行わないこと**

- Bevy stateをtracking crateへ持ち込まない。
- 状態ごとに散在したboolを増やさない。


**完了条件**

- 全stateからの主要transitionをtable testできる。
- 顔復帰時にSearchingから直接不連続Trackingへ飛ばない。
- LostHoldが永続しない。


**検証**

```bash
cargo test -p vtuber-tracking tracking_state_machine
```

#### M1-03-009: loss hold、neutral decay、recovery blendを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-03-008`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/loss_recovery.rs`
- `crates/vtuber-tracking/tests/loss_recovery.rs`


**実装指示**

- LostHold中は有限時間だけlast valid frameを保持する。
- ReturningNeutralではhead quaternionをidentity、expressionsをneutralへ時間基準で補間する。
- reacquire時はcurrent tracked frameへ短いblendを行いjumpを抑える。
- hold／decay／recovery durationをsettingsで固定範囲にする。


**このsubtaskで行わないこと**

- last observationを新規frameとしてpublishし続けない。
- wall clock依存でtestをflakyにしない。


**完了条件**

- 顔lost後にlast poseへ永久固着しない。
- neutral returnがquaternion shortest arcで進む。
- reacquire境界で大きなjumpがない。


**検証**

```bash
cargo test -p vtuber-tracking loss_recovery
```

#### M1-03-010: `AvatarControlFrame` assemblyとdeterministic replayを完成させる

状態: `LEGACY_PROGRESS`
依存: `M1-03-009`
親参照: DESIGN.md §11.5〜§11.8、§15、§18.1

**変更候補**

- `crates/vtuber-tracking/src/pipeline.rs`
- `crates/vtuber-tracking/tests/replay.rs`
- `crates/vtuber-core/src/control.rs`


**実装指示**

- pose、blink、mouth、gaze raw、state、confidenceを一つの`AvatarControlFrame`へ組み立てる。
- recorded／synthetic `RawFaceObservation` streamをtimestamp付きでreplayできるtest harnessを作る。
- 同じsettings＋streamからbitwiseまたは定義tolerance内で同じoutputを得る。
- pipeline reset／new calibration適用の境界をtestする。


**このsubtaskで行わないこと**

- avatar／Bevy適用を始めない。
- recorded dataに個人画像を含めない。


**完了条件**

- M1-03の全受入条件を満たす。
- output sequence／timestampがinput由来で追跡できる。
- calibration不足profileを保存／利用しない。


**検証**

```bash
cargo test -p vtuber-tracking replay -- --nocapture
cargo test -p vtuber-tracking --no-fail-fast
cargo clippy -p vtuber-core -p vtuber-tracking --all-targets -- -D warnings
```

## M1-04: avatar lifecycleとcapability discovery
状態: `LEGACY_PROGRESS`
実行単位: `M1-04-NNN`
重点参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4


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

### 実行subtask

#### M1-04-001: `VtuberAvatarPlugin`とlifecycle domainを定義する

状態: `LEGACY_PROGRESS`
依存: `G0-03、G0-08`
親参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4

**変更候補**

- `crates/vtuber-avatar/src/plugin.rs`
- `crates/vtuber-avatar/src/lifecycle.rs`
- `crates/vtuber-avatar/src/lib.rs`


**実装指示**

- NoAvatar、Loading、Binding、Ready、Unloading、Failedのlifecycle stateを定義する。
- LoadAvatar、UnloadAvatar、ReplaceAvatar requestとtyped resultを用意する。
- active avatarは最大一体というinvariantをresource／componentで表現する。
- `bevy_vrm1`型はcrate public facadeから漏らさない。


**このsubtaskで行わないこと**

- camera／tracking stateをavatar resourceへ混ぜない。
- multi-avatar対応を先行実装しない。


**完了条件**

- invalid request orderをtestできる。
- lifecycle stateがUIからread-only snapshotとして取得できる。
- active avatar二体を同時Readyにできない。


**検証**

```bash
cargo test -p vtuber-avatar lifecycle_state
cargo clippy -p vtuber-avatar --all-targets -- -D warnings
```

#### M1-04-002: import resultから`VrmHandle` rootをspawnする

状態: `LEGACY_PROGRESS`
依存: `M1-04-001`
親参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4

**変更候補**

- `crates/vtuber-avatar/src/load.rs`


**実装指示**

- `ImportedAvatar`のtyped `user://` pathを`AssetServer`でloadし、専用root entityへ`VrmHandle`をinsertする。
- request IDとroot entityをlifecycle stateへ関連付ける。
- load開始前に旧avatarがある場合はReplace policyに従う。
- asset path conversion failureをtyped errorにする。


**このsubtaskで行わないこと**

- preflightを再実装しない。
- Initialized前にbone queryしない。


**完了条件**

- load request一件につきroot entity一体だけをspawnする。
- absolute pathを直接`AssetServer`へ渡さない。
- missing assetでpanicしない。


**検証**

```bash
cargo test -p vtuber-avatar avatar_load_request
cargo check -p vtuber-desktop
```

#### M1-04-003: `Initialized`／load failureを一度だけ観測する

状態: `LEGACY_PROGRESS`
依存: `M1-04-002`
親参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4

**変更候補**

- `crates/vtuber-avatar/src/bind.rs`
- `crates/vtuber-avatar/src/lifecycle.rs`


**実装指示**

- `Added<Initialized>`を対象rootだけで観測し、Bindingへ遷移する。
- AssetServer load state／timeoutからfailureを検出する。
- 同じrootへbind処理を二度triggerしないmarkerを付ける。
- stale requestのlate completionをactive avatarへ採用しない。


**このsubtaskで行わないこと**

- 毎frame全`Vrm`をscanして重い探索をしない。
- asset failureを永久Loadingにしない。


**完了条件**

- Initialized一回につきbinding一回である。
- load timeout／failureがFailed stateへ届く。
- replace中の旧request completionが無視される。


**検証**

```bash
cargo test -p vtuber-avatar avatar_initialized_once
```

#### M1-04-004: required／optional humanoid bone bindingを作る

状態: `LEGACY_PROGRESS`
依存: `M1-04-003`
親参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4

**変更候補**

- `crates/vtuber-avatar/src/binding.rs`
- `crates/vtuber-avatar/tests/binding.rs`


**実装指示**

- root componentの`HeadBoneEntity`をrequired、`NeckBoneEntity`／eye entitiesをoptionalとして取得する。
- entity存在と`Transform`／`RestTransform` availabilityを検証する。
- 取得結果を内部`AvatarBinding`へcacheする。
- head欠落／despawn済みentityをtyped bind errorにする。


**このsubtaskで行わないこと**

- Nameによるbone再探索を主経路にしない。
- bone rotationをまだ変更しない。


**完了条件**

- headなしmodelがReadyにならない。
- neck／eyesなしでもhead-only capabilityでReadyになれる。
- binding後に毎frameroot component lookupを繰り返さない。


**検証**

```bash
cargo test -p vtuber-avatar humanoid_binding
```

#### M1-04-005: Expression capability discoveryを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-04-004`
親参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4

**変更候補**

- `crates/vtuber-avatar/src/capabilities.rs`
- `crates/vtuber-avatar/tests/capabilities.rs`


**実装指示**

- `ExpressionEntityMap`を一度読み、available expression nameを内部集合へ変換する。
- blinkLeft／blinkRight／blink、aa／ih／ou／ee／oh、look directions、emotion presetsを分類する。
- custom expressionは未知名として保持できるがMVP mappingへ自動採用しない。
- Expression map欠落をempty capabilityとして扱う。


**このsubtaskで行わないこと**

- morph targetを直接探索しない。
- Expression weightをまだ適用しない。


**完了条件**

- fallback選択に必要なcapabilityを取得できる。
- preset順序に依存しない。
- Expressionなしmodelでpanicしない。


**検証**

```bash
cargo test -p vtuber-avatar expression_capabilities
```

#### M1-04-006: `AvatarCapabilities`公開snapshotを組み立てる

状態: `LEGACY_PROGRESS`
依存: `M1-04-005`
親参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4

**変更候補**

- `crates/vtuber-avatar/src/capabilities.rs`
- `crates/vtuber-avatar/src/lib.rs`


**実装指示**

- head／neck／eyes、blink mode、mouth mode、gaze candidates、SpringBone有無等をengine-neutral enumへ変換する。
- UI向けhuman-readable summaryとmachine-readable fieldsを分離する。
- internal entity IDや`bevy_vrm1` component typeをpublic structへ含めない。
- binding完了時に一度生成しlifecycle Readyへ格納する。


**このsubtaskで行わないこと**

- public APIにEntityを露出しない。
- capabilityを毎frame再構築しない。


**完了条件**

- UIがcapabilityをBevy内部queryなしで表示できる。
- model差し替え時にsnapshotが更新される。
- unknown／unsupportedを明示できる。


**検証**

```bash
cargo test -p vtuber-avatar avatar_capability_snapshot
```

#### M1-04-007: single-active avatar replacementを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-04-006`
親参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4

**変更候補**

- `crates/vtuber-avatar/src/lifecycle.rs`
- `crates/vtuber-avatar/src/load.rs`


**実装指示**

- Replace requestで旧avatarをUnloadingへ移し、despawn完了後に新avatarをspawnする。
- old request ID、binding、capability、pending expression stateを明示的に破棄する。
- new load failure時に旧avatarを復活させるかemptyにするか設計方針どおり固定する。
- 連続replace requestをlatest requestへcoalesceする。


**このsubtaskで行わないこと**

- scene asset自体を手動mutationしない。
- 複数avatar queueを作らない。


**完了条件**

- 同時active rootが二体にならない。
- old entity／resource／eventが残らない。
- rapid replace testがdeterministicに終了する。


**検証**

```bash
cargo test -p vtuber-avatar avatar_replace
```

#### M1-04-008: unload cleanupとstale control rejectionを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-04-007`
親参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4

**変更候補**

- `crates/vtuber-avatar/src/unload.rs`
- `crates/vtuber-avatar/src/lifecycle.rs`


**実装指示**

- root hierarchyをrecursive despawnし、binding／capability／control cacheを削除する。
- unload後に届いたold `AvatarControlFrame`をrequest／generation IDで拒否する。
- SpringBone／MToon等は`bevy_vrm1`scene cleanupへ委譲する。
- despawn済みentity accessをtyped stale bindingへ変換する。


**このsubtaskで行わないこと**

- asset cache全体を無条件clearしない。
- bevy_vrm1内部componentを個別cleanupしない。


**完了条件**

- unload後にVRM entityがworldへ残らない。
- stale frameが新avatarへ適用されない。
- 二重unloadがidempotentである。


**検証**

```bash
cargo test -p vtuber-avatar avatar_unload_cleanup
```

#### M1-04-009: avatar lifecycle／capabilityを総合検証する

状態: `LEGACY_PROGRESS`
依存: `M1-04-008`
親参照: DESIGN.md §16.1〜§16.4、§16.11、§17.4

**変更候補**

- `crates/vtuber-avatar/`
- `apps/desktop/src/diagnostic hooks（最小）`


**実装指示**

- load、Initialized、binding、Ready、replace、unload、failureをfixtureで通す。
- headなし、neckなし、Expressionなしmodel／synthetic worldをtestする。
- `bevy_vrm1`型がavatar crate public API外へ漏れていないかAPI reviewする。
- M1-04受入条件を完了報告で一対一確認する。


**このsubtaskで行わないこと**

- pose／Expression適用を始めない。
- 実modelだけに依存するtestにしない。


**完了条件**

- M1-04の全受入条件を満たす。
- active avatar一体invariantがtestされる。
- error surfaceがappへ届く。


**検証**

```bash
cargo test -p vtuber-avatar --no-fail-fast
cargo clippy -p vtuber-avatar -p vtuber-desktop --all-targets -- -D warnings
```

## M1-05: tracked head／neck pose integration
状態: `LEGACY_PROGRESS`
実行単位: `M1-05-NNN`
重点参照: DESIGN.md §11.6、§16.5〜§16.7、ADR-004


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

### 実行subtask

#### M1-05-001: semantic pose adapterの入力／出力契約を定義する

状態: `LEGACY_PROGRESS`
依存: `M1-03、M1-04`
親参照: DESIGN.md §11.6、§16.5〜§16.7、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/pose/types.rs`
- `crates/vtuber-avatar/src/pose/mod.rs`


**実装指示**

- `AvatarControlFrame.head_pose`のyaw／pitch／roll radをVRM model-space deltaへ変換する純粋関数境界を作る。
- axis、Euler order、positive direction、neutral identityをADR-004と一致させる。
- input clamp前／後のtypeまたはfunction名を区別する。
- Bevy systemからmathを分離してunit test可能にする。


**このsubtaskで行わないこと**

- tracking crateの座標契約を上書きしない。
- bone entityへまだ書き込まない。


**完了条件**

- neutral inputでidentity deltaになる。
- ±yaw／pitch／rollのsign testがある。
- 単位がradに固定される。


**検証**

```bash
cargo test -p vtuber-avatar pose_semantics
```

#### M1-05-002: root／bone rest orientation cacheをbindingへ追加する

状態: `LEGACY_PROGRESS`
依存: `M1-05-001`
親参照: DESIGN.md §11.6、§16.5〜§16.7、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/binding.rs`
- `crates/vtuber-avatar/src/pose/binding.rs`


**実装指示**

- binding時にroot rest global rotation、head／neck rest local／global rotationを取得する。
- scale／non-uniform transformの扱いを検証し、rotation extraction failureをtyped errorにする。
- cacheをavatar generationへ紐付ける。
- 毎frame`RestTransform` queryをしない設計にする。


**このsubtaskで行わないこと**

- rest poseをidentityと仮定しない。
- GlobalTransformを毎framecloneし続けない。


**完了条件**

- non-identity rest rotationを保存できる。
- neckなしではhead cacheだけが作られる。
- despawn後cacheが破棄される。


**検証**

```bash
cargo test -p vtuber-avatar pose_rest_cache
```

#### M1-05-003: semantic poseからVRM model-space quaternionへ変換する

状態: `LEGACY_PROGRESS`
依存: `M1-05-002`
親参照: DESIGN.md §11.6、§16.5〜§16.7、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/pose/math.rs`


**実装指示**

- ADR-004で固定した候補`YXZ`とsign mappingを純粋関数で実装する。
- quaternionをnormalizeし、non-finite inputをrejectする。
- angle clamp前にsemantic axisごとのraw deltaを保持できるようにする。
- basis conversionが必要なら明示matrixで行う。


**このsubtaskで行わないこと**

- 目視だけでsignを調整しない。
- Euler outputを次frameへ累積しない。


**完了条件**

- synthetic axis testが期待rotationを返す。
- input orderを変えた誤実装をtestが検知する。
- result determinant／normが正しい。


**検証**

```bash
cargo test -p vtuber-avatar model_space_pose
```

#### M1-05-004: model-space deltaをbone-local deltaへ共役変換する

状態: `LEGACY_PROGRESS`
依存: `M1-05-003`
親参照: DESIGN.md §11.6、§16.5〜§16.7、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/pose/math.rs`
- `crates/vtuber-avatar/tests/pose_math.rs`


**実装指示**

- `R_local_delta = inverse(R_bone_rest_model) * R_model_delta * R_bone_rest_model`を実装する。
- root rest globalからbone rest model orientationを計算する。
- head／neck共通の純粋関数にする。
- non-identity、rotated root、rotated bone fixtureを追加する。


**このsubtaskで行わないこと**

- 単純`rest * delta`だけで済ませない。
- matrix／quat conventionを混在させない。


**完了条件**

- identity restではmodel deltaとlocal deltaが一致する。
- non-identity rest fixtureでworld方向が期待どおりになる。
- quaternion multiplication order regressionをtestが検知する。


**検証**

```bash
cargo test -p vtuber-avatar local_pose_conjugation
```

#### M1-05-005: head／neck distributionとrange clampを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-05-004`
親参照: DESIGN.md §11.6、§16.5〜§16.7、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/pose/distribution.rs`
- `crates/vtuber-avatar/tests/pose_distribution.rs`


**実装指示**

- yaw／pitch／rollをhead／neck weightへ分配し、weight合計policyを明示する。
- neck欠落時はheadへ再配分する。
- 軸別最大角をclampし、clamp前後をdiagnostic可能にする。
- default値をsettings typeへ置き、model個別hard-codeを避ける。


**このsubtaskで行わないこと**

- BodyTrackingのweight設定を流用しない。
- bone limitをVRM spec値と誤認しない。


**完了条件**

- head＋neck distributionの合成が意図したtotal deltaになる。
- neckなしでも動作する。
- 極端なinputが設定range内に収まる。


**検証**

```bash
cargo test -p vtuber-avatar pose_distribution
```

#### M1-05-006: tracked pose apply systemを正しいscheduleへ登録する

状態: `LEGACY_PROGRESS`
依存: `M1-05-005`
親参照: DESIGN.md §11.6、§16.5〜§16.7、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/pose/system.rs`
- `crates/vtuber-avatar/src/plugin.rs`


**実装指示**

- latest `AvatarControlFrame`を一度読み、active bindingのhead／neck `Transform`へ適用する。
- systemを`AnimationSystems`後、`VrmSystemSets::Constraints`前へ明示orderingする。
- 各frameで`RestTransform.rotation * local_delta`から再計算する。
- generation mismatch／stale entity／not Readyをskipしmetricへ記録する。


**このsubtaskで行わないこと**

- VRMA animation base detectionを追加しない。
- PostUpdate orderingを暗黙にしない。


**完了条件**

- tracking deltaが毎frame累積しない。
- system ordering test／app schedule introspectionで位置を確認できる。
- not Ready時にTransformを書き換えない。


**検証**

```bash
cargo test -p vtuber-avatar tracked_pose_system
cargo check -p vtuber-desktop
```

#### M1-05-007: loss／neutral frameとavatar unloadをpose systemへ接続する

状態: `LEGACY_PROGRESS`
依存: `M1-05-006`
親参照: DESIGN.md §11.6、§16.5〜§16.7、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/pose/system.rs`
- `crates/vtuber-avatar/src/lifecycle.rs`


**実装指示**

- ReturningNeutral frameは通常のidentity方向deltaとして同じapply pathを通す。
- LostHold中もframe timestamp／generationを検証する。
- unload時にpose cache／last frameを削除する。
- 新avatarへ旧avatarのlast poseを適用しない。


**このsubtaskで行わないこと**

- avatar側で独自loss interpolationを追加しない。
- last poseをglobal resourceへ残さない。


**完了条件**

- avatar replace後にneutral restから開始する。
- lost／return中にdriftしない。
- unload直後のsystem tickでdespawn entity panicがない。


**検証**

```bash
cargo test -p vtuber-avatar pose_lifecycle
```

#### M1-05-008: non-identity rest synthetic integration testを追加する

状態: `LEGACY_PROGRESS`
依存: `M1-05-007`
親参照: DESIGN.md §11.6、§16.5〜§16.7、ADR-004

**変更候補**

- `crates/vtuber-avatar/tests/pose_integration.rs`


**実装指示**

- root、neck、headに非identity rest rotationを持つminimal Bevy world fixtureを作る。
- neutral、yaw、pitch、roll、combined、clamp、neck missingをtestする。
- 複数frame同じdeltaを適用しdriftがないことを確認する。
- schedule ordering込みでsystem testを実行する。


**このsubtaskで行わないこと**

- 目視sampleだけをacceptanceにしない。
- M1-06 Expressionを同時実装しない。


**完了条件**

- M1-05の全受入条件をtestで追跡できる。
- world-space期待方向とlocal rotationの双方を検証する。
- fixtureが実VRM assetに依存しない。


**検証**

```bash
cargo test -p vtuber-avatar pose_integration -- --nocapture
cargo clippy -p vtuber-avatar --all-targets -- -D warnings
```

## M1-06: blink、mouth、gaze integration
状態: `LEGACY_PROGRESS`
実行単位: `M1-06-NNN`
重点参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004


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

### 実行subtask

#### M1-06-001: per-frame expression command builderを定義する

状態: `LEGACY_PROGRESS`
依存: `M1-04、M1-05`
親参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/expression/command.rs`


**実装指示**

- `AvatarControlFrame`と`AvatarCapabilities`からname→weight集合を作る純粋builderを実装する。
- weightを0..1へclampし、non-finite値をdrop／error metricへ送る。
- 一つのexpression nameを同一frameで重複insertしない。
- outputが空ならeventを送らないpolicyを明示する。


**このsubtaskで行わないこと**

- morph weightへ直接アクセスしない。
- `SetExpressions`を使わない。


**完了条件**

- stable inputからstable mappingが得られる。
- duplicate／NaN／range外をtestできる。
- Bevy event送信とmapping logicが分離される。


**検証**

```bash
cargo test -p vtuber-avatar expression_command
```

#### M1-06-002: blink capability fallbackを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-06-001`
親参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/expression/blink.rs`
- `crates/vtuber-avatar/tests/expression_mapping.rs`


**実装指示**

- 左右presetがあれば`blinkLeft`／`blinkRight`へ別weightを割り当てる。
- `blink`だけなら左右値のmaxまたは設計指定集約値を割り当てる。
- presetなしなら何も出さずunsupported capabilityを維持する。
- threshold／epsilonをmapping層で過度に重複適用しない。


**このsubtaskで行わないこと**

- 左右presetがないのに存在を仮定しない。
- 自動瞬きを追加しない。


**完了条件**

- 左右対応modelで非対称blinkが保持される。
- `blink`のみmodelで両目入力が一つへ集約される。
- presetなしでpanicしない。


**検証**

```bash
cargo test -p vtuber-avatar blink_mapping
```

#### M1-06-003: mouth preset mappingと`aa` fallbackを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-06-002`
親参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/expression/mouth.rs`
- `crates/vtuber-avatar/tests/expression_mapping.rs`


**実装指示**

- MVP raw mouth opennessをavailable presetへ変換する。
- 5母音係数が存在する場合はavailable presetだけを利用し、未対応名を送らない。
- `aa`だけならmouth opennessを`aa`へ割り当てる。
- mouth presetなしならmappingを空にする。


**このsubtaskで行わないこと**

- coarticulation品質処理をここで実装しない。
- `ModifyExpressions::mouth`とfrom_iterを同一frameで混在させない。


**完了条件**

- `aa` fallbackが動く。
- unsupported vowelを送信しない。
- mouth presetなしでpanicしない。


**検証**

```bash
cargo test -p vtuber-avatar mouth_mapping
```

#### M1-06-004: gaze mode selectionをcapabilityから決定する

状態: `LEGACY_PROGRESS`
依存: `M1-06-003`
親参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/gaze/mode.rs`
- `crates/vtuber-avatar/src/capabilities.rs`


**実装指示**

- 4方向expressionが揃う場合、部分的に揃う場合、eye bonesのみの場合、unsupportedをenum化する。
- modelの`lookAt.type`はdiagnosticに保持するが`bevy_vrm1::LookAt`選択には使わない。
- 優先順位をexpression→eye bones→disabled等、設計どおり固定する。
- modeをbinding時に一度決定する。


**このsubtaskで行わないこと**

- `LookAt`／`BodyTracking` componentをinsertしない。
- 毎framemode判定しない。


**完了条件**

- gaze modeが`AvatarCapabilities`へ表示される。
- expression lookAt modelでもpanic pathを選ばない。
- 部分capabilityのfallbackがdeterministicである。


**検証**

```bash
cargo test -p vtuber-avatar gaze_mode
```

#### M1-06-005: gaze expression mappingを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-06-004`
親参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/gaze/expression.rs`
- `crates/vtuber-avatar/tests/gaze.rs`


**実装指示**

- gaze yaw／pitchへdead zone、軸別max、normalized weightを適用する。
- left/right、up/downを相互排他的または設計指定blendでmappingする。
- available presetだけをcommand builderへ追加する。
- blink／mouth override conflictを`bevy_vrm1`のExpression resolverへ委譲する。


**このsubtaskで行わないこと**

- VRM range mapを独自推測しない。
- head poseをgazeへ二重加算しない。


**完了条件**

- center gazeで全weightが0になる。
- ±yaw／pitchで正しいdirection presetが増える。
- partial presetでもfinite outputになる。


**検証**

```bash
cargo test -p vtuber-avatar gaze_expression_mapping
```

#### M1-06-006: eye bone gaze fallbackを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-06-005`
親参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/gaze/bone.rs`
- `crates/vtuber-avatar/tests/gaze.rs`


**実装指示**

- left／right eye rest orientationをbinding cacheへ追加する。
- yaw／pitchを軸別clampし、restからlocal rotationを再計算する。
- systemをConstraints後のpropagationとExpressionsの順序を壊さない位置へ登録する。
- 片目欠落時のpolicyをdisabledまたはavailable eye onlyで明示する。


**このsubtaskで行わないこと**

- `bevy_vrm1::LookAt`を呼ばない。
- 前frame rotationへ累積乗算しない。


**完了条件**

- eyesありmodelでcenterがrestを保つ。
- 極端なgazeがclampされる。
- eyesなしでsystemがskipする。


**検証**

```bash
cargo test -p vtuber-avatar eye_bone_gaze
```

#### M1-06-007: expression event coalescingとchange epsilonを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-06-006`
親参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/expression/system.rs`


**実装指示**

- 一avatar／一frameにつき最大一つの`ModifyExpressions::from_iter` eventを送る。
- 前回送信mappingとの差がepsilon未満なら送信をskipする。
- 消えたexpressionを0へ戻す必要がある場合は明示zeroを含める。
- avatar generation変更時にprevious mapping cacheをresetする。


**このsubtaskで行わないこと**

- `SetExpressions`と混在させない。
- epsilonで重要なblink pulseを消さない。


**完了条件**

- 同一frameに複数Modify eventを送らない。
- steady neutralでevent stormが起きない。
- previous nonzeroがneutral時に確実に0へ戻る。


**検証**

```bash
cargo test -p vtuber-avatar expression_coalescing
```

#### M1-06-008: VRM schedule orderingと禁止path guardを追加する

状態: `LEGACY_PROGRESS`
依存: `M1-06-007`
親参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004

**変更候補**

- `crates/vtuber-avatar/src/plugin.rs`
- `crates/vtuber-avatar/tests/schedule.rs`


**実装指示**

- gaze bone、expression command、Modify eventのsystem set順を明示する。
- `VrmSystemSets::GazeControl`／`Expressions`とのbefore／after関係をtestまたは構築codeで固定する。
- product app worldに`LookAt`／`BodyTracking`が存在しないassertion testを追加する。
- SpringBoneまでのtransform propagationを壊していないことを確認する。


**このsubtaskで行わないこと**

- bevy_vrm1内部systemをforkせず無効化しない。
- 曖昧な`.after()`だけで順序を放置しない。


**完了条件**

- gaze systemがVRM更新順に入る。
- 禁止componentのproduct-path testが通る。
- schedule cycleがない。


**検証**

```bash
cargo test -p vtuber-avatar avatar_schedule
cargo check -p vtuber-desktop
```

#### M1-06-009: blink／mouth／gaze integrationを総合検証する

状態: `LEGACY_PROGRESS`
依存: `M1-06-008`
親参照: DESIGN.md §11.7〜§11.8、§16.8〜§16.9、ADR-004

**変更候補**

- `crates/vtuber-avatar/tests/expression_integration.rs`
- `M1-06関連実装`


**実装指示**

- 左右blink model、blink-only、aa-only、no-mouth、expression gaze、eye-bone gaze、no-gazeをfixture worldでtestする。
- one event per frame、epsilon、zero reset、generation resetを確認する。
- 実VRM sampleで手動expression変化を確認する。
- M1-06受入条件を完了報告へ対応づける。


**このsubtaskで行わないこと**

- UIを実装しない。
- Q2表情品質へ進まない。


**完了条件**

- M1-06の全受入条件を満たす。
- missing capabilityでpanicしない。
- product pathに`LookAt`／`BodyTracking`がない。


**検証**

```bash
cargo test -p vtuber-avatar expression gaze --no-fail-fast
cargo clippy -p vtuber-avatar -p vtuber-desktop --all-targets -- -D warnings
```

## M1-07: desktop UI、preview、diagnostics
状態: `LEGACY_PROGRESS`
実行単位: `M1-07-NNN`
重点参照: DESIGN.md §13.6、§18、§20


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

### 実行subtask

#### M1-07-001: UI action／view model boundaryを定義する

状態: `LEGACY_PROGRESS`
依存: `M1-03、M1-06`
親参照: DESIGN.md §13.6、§18、§20

**変更候補**

- `crates/vtuber-app/src/ui_model.rs`
- `crates/vtuber-app/src/actions.rs`


**実装指示**

- camera、avatar、calibration、tracking lifecycleをUI向けimmutable snapshotへ変換する。
- UI actionをSelectCamera、ImportAvatar、Start、Stop、BeginCalibration等のcommand enumにする。
- UI codeがcamera／filesystem／VRM APIを直接呼ばないようapp orchestratorへ送る。
- error display modelにcode、user message、recoverable actionを持たせる。


**このsubtaskで行わないこと**

- egui widgetを同時実装しない。
- errorをString一つに潰さない。


**完了条件**

- UI modelがBevy queryの詳細を隠す。
- actionからdomain service呼出しをtestできる。
- button clickで同期camera openしない。


**検証**

```bash
cargo test -p vtuber-app ui_model
cargo clippy -p vtuber-app --all-targets -- -D warnings
```

#### M1-07-002: `bevy_egui` shellとSetup／Live／Diagnostics画面を作る

状態: `LEGACY_PROGRESS`
依存: `M1-07-001`
親参照: DESIGN.md §13.6、§18、§20

**変更候補**

- `apps/desktop/Cargo.toml`
- `crates/vtuber-app/src/ui/`
- `apps/desktop/src/`


**実装指示**

- Bevy 0.19互換`bevy_egui`をexact互換versionで追加する。
- 単一window内にSetup、Live、Diagnosticsのtabまたはstate画面を作る。
- 画面はview modelを描画しactionをemitするだけにする。
- layoutはMVP操作に必要な最小項目に限定する。


**このsubtaskで行わないこと**

- theme frameworkやcustom rendererを追加しない。
- 設定永続化を先行実装しない。


**完了条件**

- 三画面へ切り替えられる。
- UI draw systemがdomain mutable resourceを直接操作しない。
- empty／error stateでもpanicしない。


**検証**

```bash
cargo check -p vtuber-desktop
cargo test -p vtuber-app ui
```

#### M1-07-003: Setup画面へavatar importとcamera selectionを接続する

状態: `LEGACY_PROGRESS`
依存: `M1-07-002`
親参照: DESIGN.md §13.6、§18、§20

**変更候補**

- `crates/vtuber-app/src/ui/setup.rs`
- `crates/vtuber-app/src/orchestrator.rs`


**実装指示**

- file-drop状態、import中／成功／失敗、active avatar summaryを表示する。
- enumerated camera listとselected descriptor／formatを表示する。
- Refresh camera、Select camera、Import actionをorchestratorへ送る。
- 操作不可stateではbuttonをdisableし理由を表示する。


**このsubtaskで行わないこと**

- native file dialogを必須にしない。
- recent avatarを実装しない。


**完了条件**

- UIから直接import service／nokhwaを呼ばない。
- cameraなし／avatarなしstateが明確である。
- import error後に再操作できる。


**検証**

```bash
cargo test -p vtuber-app setup_ui_actions
cargo check -p vtuber-desktop
```

#### M1-07-004: Start／Stop lifecycle orchestrationを接続する

状態: `LEGACY_PROGRESS`
依存: `M1-07-003`
親参照: DESIGN.md §13.6、§18、§20

**変更候補**

- `crates/vtuber-app/src/orchestrator.rs`
- `crates/vtuber-app/src/ui/live.rs`


**実装指示**

- Start actionでpreconditionを検証しcapture→inference→tracking順に起動する。
- 途中失敗時は起動済みworkerを逆順にstop／joinする。
- Stop actionでinference→capture等、設計した安全順序で停止する。
- UIへStarting／Running／Stopping／Failedを反映する。


**このsubtaskで行わないこと**

- worker objectをUI resourceへ直接置かない。
- 失敗時にprocessを終了しない。


**完了条件**

- 二重start／stopが安全である。
- partial startup failureでthread leakがない。
- UIがworker起動中にblockしない。


**検証**

```bash
cargo test -p vtuber-app lifecycle_orchestration
```

#### M1-07-005: calibration UIとstate feedbackを接続する

状態: `LEGACY_PROGRESS`
依存: `M1-07-004`
親参照: DESIGN.md §13.6、§18、§20

**変更候補**

- `crates/vtuber-app/src/ui/calibration.rs`
- `crates/vtuber-app/src/orchestrator.rs`


**実装指示**

- Begin／Cancel／Retry calibration actionをtracking pipelineへ送る。
- sample progress、reject reason、quality score、completionを表示する。
- Readyでないcamera／inference時は開始不可にする。
- calibration profileをこのtaskでは永続化せずsessionへ適用する。


**このsubtaskで行わないこと**

- UI側でneutral計算しない。
- 自動calibrationを暗黙開始しない。


**完了条件**

- 不足sampleを完了扱いしない。
- cancel後にcollector stateがresetされる。
- reject理由がuserに判別できる。


**検証**

```bash
cargo test -p vtuber-app calibration_ui
```

#### M1-07-006: camera preview texture pipelineを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-07-005`
親参照: DESIGN.md §13.6、§18、§20

**変更候補**

- `crates/vtuber-app/src/preview.rs`
- `crates/vtuber-app/src/ui/live.rs`


**実装指示**

- LatestSlotの最新RGB frameを一定上限fpsでBevy `Image`へ更新する。
- Image handleとpixel buffer capacityを再利用する。
- mirror previewはtexture uploadまたはUV表示だけで行い、inference inputを変えない。
- preview OFF時はtexture updateだけを停止しcapture／trackingは継続する。


**このsubtaskで行わないこと**

- camera frameをdisk保存しない。
- previewを推論sourceにしない。


**完了条件**

- preview texture assetを毎frame新規作成しない。
- mirror ON/OFFでtracking数値が同一になるtestがある。
- preview OFFでもworker metricsが進む。


**検証**

```bash
cargo test -p vtuber-app preview
cargo check -p vtuber-desktop
```

#### M1-07-007: DiagnosticsへFPS／latency／drop／stateを表示する

状態: `LEGACY_PROGRESS`
依存: `M1-07-006`
親参照: DESIGN.md §13.6、§18、§20

**変更候補**

- `crates/vtuber-app/src/diagnostics.rs`
- `crates/vtuber-app/src/ui/diagnostics.rs`


**実装指示**

- render FPS、capture rate、inference rate、tracking state、slot overwrite、stage timingsをsnapshotへ集約する。
- p50／p95等は既存fixed-size statsから取得し、UIで全sampleを保持しない。
- model hash短縮値、camera backend、avatar capabilityを表示する。
- pixel／landmark座標やfull local pathを通常表示しない。


**このsubtaskで行わないこと**

- chart libraryを追加しない。
- unbounded historyを持たない。


**完了条件**

- worker停止時にもlast stateとerrorを表示できる。
- diagnostics表示でdata pathがblockしない。
- privacy方針に反するraw dataがない。


**検証**

```bash
cargo test -p vtuber-app diagnostics_snapshot
```

#### M1-07-008: recoverable error UXを実装する

状態: `LEGACY_PROGRESS`
依存: `M1-07-007`
親参照: DESIGN.md §13.6、§18、§20

**変更候補**

- `crates/vtuber-app/src/error_presenter.rs`
- `crates/vtuber-app/src/ui/`


**実装指示**

- camera denied／missing、model invalid、worker failed、calibration rejectedをuser messageへmappingする。
- Retry、Select another camera、Import another model、Stop等の可能actionを表示する。
- technical causeはDiagnosticsへ残し、main UIには安全な要約を出す。
- error dismissalだけでdomain failure stateを消さない。


**このsubtaskで行わないこと**

- modalを無限stackしない。
- panic dialogに依存しない。


**完了条件**

- error後に適切な再操作ができる。
- 同じerrorを毎frame再追加しない。
- full path／raw frameがmessageへ出ない。


**検証**

```bash
cargo test -p vtuber-app error_presenter
```

#### M1-07-009: desktop UI vertical smokeを総合検証する

状態: `LEGACY_PROGRESS`
依存: `M1-07-008`
親参照: DESIGN.md §13.6、§18、§20

**変更候補**

- `crates/vtuber-app/`
- `apps/desktop/`


**実装指示**

- file drop→avatar Ready、camera select→Start、calibration→Live、Stop、error recoveryを手動protocolで通す。
- UI action boundaryをunit testし、domain service直接呼出しがないかreviewする。
- preview mirror／off、Diagnostics更新を確認する。
- M1-07受入条件をreportへ対応づける。


**このsubtaskで行わないこと**

- 長時間待機を伴うacceptanceを追加しない。
- UI polishをスコープ拡大しない。


**完了条件**

- M1-07の全受入条件を満たす。
- UI thread blockによる長いfreezeがない。
- errorからprocess再起動なしで回復できる。


**検証**

```bash
cargo test -p vtuber-app --no-fail-fast
cargo check -p vtuber-desktop
cargo clippy -p vtuber-app -p vtuber-desktop --all-targets -- -D warnings
```

#### M1-07-010: Controls windowをsession内で開閉可能にする

状態: `DONE`
依存: `M1-07-009`
親参照: DESIGN.md §18.2

**変更候補**

- `crates/vtuber-app/src/ui/shell.rs`
- `DESIGN.md`

**実装指示**

- Controls windowをdefault表示とし、windowのclose操作、明示的なHide操作、`F1`で表示／非表示を切り替えられるようにする。
- 非表示時は画面上の小さい再表示buttonと`F1`で必ず戻せるようにし、Controlsを閉じたことで操作不能な状態を作らない。
- 表示切替はUI sessionだけに限定し、camera、tracking、avatar lifecycle、previewの実行状態を変更しない。
- settings schema、依存、rendererを追加しない。表示状態の永続化はQ2-02へ送る。

**完了条件**

- 起動時はControlsが表示される。
- Hide、window close、`F1`で非表示にでき、非表示後もbuttonまたは`F1`で再表示できる。
- 表示状態のunit testがあり、既存UI action／domain境界を変更しない。

**検証**

```powershell
cargo fmt --all -- --check
cargo test -p vtuber-app ui::shell
cargo check -p vtuber-desktop
cargo clippy -p vtuber-app -p vtuber-desktop --all-targets -- -D warnings
git diff --check
```

## M1-08: Windows vertical implementation and acceptance
状態: `BLOCKED`
実行単位: top-level `M1-08-NNN`、blocker repair `M1-08-013-NNN`
重点参照: DESIGN.md §3、§6、§11、§14〜§17、§20〜§21、§24、ADR-001、docs/PERFORMANCE_TEST_PLAN.md

依存: M1-07

### 2026-08-11 blocker突破再計画

`M1-08-009`〜`M1-08-012`と、その後に実装されたworker／tracking／avatar bridge／diagnostics／shutdownの基盤は保持する。現在のcorrectness blockerは一つに絞られている。

```text
Windows full camera frame
  → face detector 〔欠落〕
  → stable square crop 〔欠落〕
  → Peppa 98-point landmark ONNX 〔実装済み〕
  → planar pose solver 〔実装済み〕
  → TrackingPipeline／avatar bridge 〔基盤実装済み〕
```

Peppa upstream自身もface detectorとlandmark detectorを別段として扱っており、landmark modelだけをfull frameへ直接適用する設計ではない。したがって、Peppa modelやplanar solverを捨てて別の大規模stackへ移るのではなく、pure-Rustで検証可能なdetector／crop段を前置する。

### detectorの固定第一候補

第一候補を**UltraFace RFB-320 ONNX**に固定する。

- authoritative model entry: ONNX Model Zoo `validated/vision/body_analysis/ultraface`
- upstream implementation: `Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB`
- license: MIT
- file: `version-RFB-320.onnx`
- exact SHA-256: `34cd7e60aeff28744c657de7a3dc64e872d506741de66987f3426f2b79f88017`
- exact size: `1,270,727` bytes
- optional official model-with-test-data SHA-256: `628d0dd3e0288adb821f211e13d4e97f6d6f4527237339606732dffa6f19d381`
- optional official model-with-test-data size: `2,015,397` bytes
- ONNX opset: 9
- input: `[1, 3, 240, 320]` F32 NCHW RGB
- preprocess: `(pixel - 127) / 128`
- outputs: scores `[1, 4420, 2]`、boxes `[1, 4420, 4]`
- initial postprocess: score threshold `0.7`、NMS IoU `0.3`
- pure-Rust feasibility evidence: 別projectでUltraFace RFB-320を`tract-onnx 0.22`からload／runしている実装例が存在する。ただし本projectの合格根拠にはせず、`tract-onnx 0.23.4`でexact artifactを再検証する。

参照先:

- `https://github.com/onnx/models/tree/main/validated/vision/body_analysis/ultraface`
- `https://github.com/onnx/models/blob/main/ONNX_HUB_MANIFEST.json`
- `https://github.com/Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB`
- `https://github.com/imazen/zensally/blob/43d8ec2776416ee5822266dac3db56bad21e190f/crates/zensally-tract/src/ultraface.rs`
- `https://github.com/610265158/Peppa_Pig_Face_Landmark`

### blocker突破の原則

- runtimeは引き続きpure Rust、`tract-onnx 0.23.4`とする。
- MediaPipe TFLite、Python sidecar、OpenCV、ONNX Runtime、native C／C++へ切り替えない。
- UltraFaceのexact probeが失敗した場合、同じleafで別modelやruntimeへ逃げない。operator／shape／model hashを記録して新repair leafを追加する。
- detectorとlandmarkのmodel artifact、前処理、後処理、crop変換はmanifest-drivenにする。
- runtime起動時にnetwork downloadしない。取得はdevelopment commandでtemp download→size／SHA検証→atomic renameとする。
- detectorを省略するframeでもlandmark inferenceまで省略してはならない。detector cadenceはROI更新頻度であり、全pipelineのskip頻度ではない。
- no-faceはtyped outcomeであり、runtime errorとして数えない。
- `M1-08-013`本体はgate ownerであり、軽量agentへ直接委嘱しない。

### 目的

Windows 11で、GUIからのVRM import、実camera capture、公式MediaPipe Tasks推論、tracking、`bevy_vrm1` avatar適用を一本につなぎ、MVP縦断動作とbounded pipeline／lifecycleを確認する。

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
- warm-up 10秒＋60秒以上の定常測定（人の連続待機は要求しない）

### 受入条件

- production tracking pathからfiniteなpose／expressionが得られ、real VRMのhead／blink／mouth／gazeへ反映される。
- render最低30fps、tracking最低15Hz
- p95 capture-to-apply 180ms以下、queue保持件数1以下
- face loss／return、Stop／Start、avatar replace後にtrackingへ復帰する。
- process crashがなく、終了時に全workerをjoinできる。
- PASS／FAIL／NOT RUNとmetrics artifactをreportへ保存する。

---

### 実行subtask

> `M1-08-001`〜`M1-08-008`は履歴保持のため残す。`M1-08-009`〜`M1-08-012`はDONEである。repair branchは`M1-08-013-004`を次に実行する。


#### M1-08-001: Windows acceptance buildとtest environmentを固定する

状態: `LEGACY_PROGRESS`
依存: `M1-07`
親参照: DESIGN.md §6、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/acceptance/windows-m1.md`
- `tools/xtask/src/acceptance.rs（必要なら）`


**実装指示**

- Windows version、CPU、GPU、driver、camera、screen resolution、build profile、commit SHAをreport headerへ記録する。
- releaseまたは指定profileのbinaryとmodel hashesを固定する。
- logging／metrics output directoryをrunごとに一意にする。
- acceptance commandを一つにまとめる場合もtest protocolを隠さない。


**このsubtaskで行わないこと**

- 測定中にdebug buildへ切り替えない。
- 環境情報を省略しない。


**完了条件**

- 別runとartifactが混ざらない。
- binary／model／configのversionが再現可能である。
- 実環境不足が明記される。


**検証**

```bash
cargo build -p vtuber-desktop --release
cargo run -p xtask -- acceptance --help
```

#### M1-08-002: VRM／camera test matrixを確定する

状態: `LEGACY_PROGRESS`
依存: `M1-08-001`
親参照: DESIGN.md §6、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/acceptance/windows-m1.md`


**実装指示**

- compatibility gateのVRM三種類をhash付きで列挙する。
- camera二種類があればdevice descriptorとformatを列挙し、一種類なら制約を明記する。
- 各組合せで必要な最小protocolとskip条件を表にする。
- model／camera選択順を固定してcache／permission影響を記録する。


**このsubtaskで行わないこと**

- 都合の良いmodel一体だけで合格にしない。
- camera indexだけを記録しない。


**完了条件**

- 全対象組合せにtest rowがある。
- skip理由が事前定義される。
- hashなしmodelを使用しない。


**検証**

```bash
cargo run -p xtask -- model verify
```

#### M1-08-003: functional motion／expression protocolを実施する

状態: `LEGACY_PROGRESS`
依存: `M1-08-002`
親参照: DESIGN.md §6、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/acceptance/windows-m1.md`
- `run logs`


**実装指示**

- neutral calibration、yaw／pitch／roll、左右blink、mouth、gazeを一定順序／時間で実施する。
- 各stepでtracking state、visual result、error、capability fallbackを記録する。
- modelごとのunsupported項目をfailureとcapability limitationで区別する。
- 必要なら短いscreen recordingをartifactとして保存するがraw camera frameを公開しない。


**このsubtaskで行わないこと**

- 目視結果だけでlatency合格を判断しない。
- privacy許可なしにcamera映像を添付しない。


**完了条件**

- head pose三軸が意図方向へ動く。
- blink／mouthがcapabilityに応じて動く。
- panic／fatal render issueがない。


**検証**

```bash
手動protocol。実行commandとbinary hashをreportへ記録する。
```

#### M1-08-004: face loss／camera／avatar recovery protocolを実施する

状態: `LEGACY_PROGRESS`
依存: `M1-08-003`
親参照: DESIGN.md §6、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/acceptance/windows-m1.md`
- `run logs`


**実装指示**

- faceを外す→LostHold→ReturningNeutral→再入場を確認する。
- camera Stop／Start、可能なら抜去／再接続を確認する。
- tracking中にavatarを別modelへreplaceする。
- 各操作後にthread count、state、stale pose有無を記録する。


**このsubtaskで行わないこと**

- OS再起動を通常回復手段にしない。
- crash後再起動を成功扱いしない。


**完了条件**

- face lossで永久freezeしない。
- camera restart後にtrackingへ復帰できる。
- avatar replace後に旧stateが残らない。


**検証**

```bash
手動protocol＋Diagnostics log確認
```

#### M1-08-005: capture-to-apply latencyとrateを測定する

状態: `LEGACY_PROGRESS`
依存: `M1-08-004`
親参照: DESIGN.md §6、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/acceptance/windows-m1.md`
- `metrics CSV／JSON`


**実装指示**

- capture timestamp、inference complete、control frame、avatar apply timestampを同一monotonic domainで記録する。
- warm-upを除外する規則とsample windowを固定する。
- render FPS、tracking Hz、p50／p95 capture-to-apply、overwrite countを算出する。
- UI表示値だけでなくraw bounded metrics exportを保存する。


**このsubtaskで行わないこと**

- 異なるclockを単純減算しない。
- 少数sampleだけでp95を出さない。


**完了条件**

- render 30fps以上、tracking 15Hz以上を判定できる。
- p95 180ms以下を数値で判定できる。
- queue保持件数が1以下である。


**検証**

```bash
cargo test -p vtuber-app diagnostics_snapshot
acceptance run metrics export
```

#### M1-08-006: bounded実行とshutdownを確認する

状態: `LEGACY_PROGRESS`
依存: `M1-08-005`
親参照: DESIGN.md §6、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/acceptance/windows-m1.md`
- `bounded metrics artifact`


**実装指示**

- 固定model／cameraで10秒warm-up後に60秒以上、300 sample以上のbounded metricsを取得する。測定中に人の連続操作は要求しない。
- latency p95、rates、queue depth、overwrite、worker statusを記録する。
- 測定前後に短いface loss／returnとStop／Startを各1回行う。
- 終了時に明示Stopし、全workerのjoinとclean shutdownを確認する。


**このsubtaskで行わないこと**

- 途中失敗runを削除しない。
- RSSの短期変動だけを合否根拠にしない。


**完了条件**

- render 30fps以上、tracking 15Hz以上、capture-to-apply p95 180ms以下を満たす。
- queue保持件数が1以下で、overwriteを記録できる。
- Stop／Start後にtrackingへ復帰し、終了時にworker threadが残らない。
- process crashなし。


**検証**

```bash
実機run（10秒warm-up＋60秒以上、300 sample以上）＋Stop／Start＋metrics artifact保存
```

#### M1-08-007: Windows acceptance reportを完成させる

状態: `LEGACY_PROGRESS`
依存: `M1-08-006`
親参照: DESIGN.md §6、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/acceptance/windows-m1.md`
- `artifact manifest`


**実装指示**

- matrix、functional、recovery、latency、bounded実行結果を一つのreportへ統合する。
- 各受入条件をPASS／FAIL／NOT RUNで記録する。
- FAILにはissue候補、再現手順、log／artifact pathを付ける。
- model、binary、config、artifactのhash manifestを付ける。


**このsubtaskで行わないこと**

- reportを成功要約だけにしない。
- raw personal dataをbundleしない。


**完了条件**

- 結果を第三者が追跡できる。
- 未実行項目がpassへ混入しない。
- M1-08 gate判断が明示される。


**検証**

```bash
sha256sum docs/acceptance/artifacts/* 2>/dev/null || true
```

#### M1-08-008: Windows blockerを分類しM1-09進行可否を決める

状態: `LEGACY_PROGRESS`
依存: `M1-08-007`
親参照: DESIGN.md §6、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/acceptance/windows-m1.md`
- `AI_AGENT_TASKS.md status（運用時）`


**実装指示**

- blockerをcorrectness、compatibility、performance、hardware-specific、test-environmentに分類する。
- M1-09へ進める条件と先に修正すべき条件を明記する。
- fixが必要なら既存subtaskへ戻るか新repair taskを別途提案し、親IDをrenumberしない。
- 全command／manual protocol結果を完了報告へ記録する。


**このsubtaskで行わないこと**

- 重大FAILを既知制約として無条件許容しない。
- M1-09のmacOS作業を同時開始しない。


**完了条件**

- M1-08の全受入条件が判定済みである。
- 進行可否に数値／再現根拠がある。
- task番号を変更していない。


**検証**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```



#### M1-08-009: GUI importをavatar lifecycleへ接続する

状態: `DONE`
依存: `M1-08-008`
親参照: DESIGN.md §7.2〜§7.4、§16、§17、§18.4、§21.3

**変更候補**

- `crates/vtuber-app/src/orchestrator.rs`
- `crates/vtuber-app/src/import.rs`
- `crates/vtuber-app/src/runtimeまたはavatar_bridge.rs`
- `crates/vtuber-avatar/src/load.rs`
- `apps/desktop/src/main.rs`
- `crates/vtuber-app/tests/avatar_import_flow.rs`

**実装指示**

- 現repositoryの`ImportedModel`、`ImportedAvatar`、`AvatarAssetId`、`UserAssetPath`、`LoadImportedAvatarRequest`、`LoadImportedAvatarResult`、`AvatarLifecycle`を再利用する。
- `import_vrm()`がcopyするmanaged asset rootと、Bevyのnamed `user` AssetSourceのrootを同一にする。`user://avatars/<sha256>/model.vrm`が解決できることをtestする。
- `UiAction::ImportAvatar`のpreflight／copy成功後、`ImportedModel.id`からcanonical `UserAssetPath`を構築し、exactly onceで`LoadImportedAvatarRequest`を発行する。
- absolute filesystem pathを`AssetServer`へ渡さない。
- `LoadImportedAvatarResult`と`AvatarLifecycleState`をconsumeし、`UiViewModel.avatar.lifecycle`、`is_ready`、recoverable errorを更新する。
- file copy成功だけでは`Ready`にしない。`bevy_vrm1::Initialized`、humanoid bind、lifecycle `Ready`を満たした時だけready表示する。
- 二体目importでは既存replace／coalescing pathを使い、old hierarchyを残さない。
- CLI `--model`経路をmanaged asset source invariantへ統合し、別loaderを残さない。

**このsubtaskで行わないこと**

- camera、inference、tracking、synthetic motionを実装しない。
- `bevy_vrm1` loaderを置換しない。
- UI rendererからfilesystemやVRM APIを直接呼ばない。

**完了条件**

- GUIで選択したvalid VRMが3D sceneへ表示され、lifecycle `Ready`になる。
- model replacementでactive avatarが常に一体以下である。
- unload後にold root／binding／expression stateが残らない。
- invalid model／load failureがpanicせずUIへ表示される。
- `AssetServer`へabsolute pathを渡すcode pathがない。

**検証**

```powershell
cargo fmt --all -- --check
cargo check -p vtuber-app -p vtuber-avatar -p vtuber-desktop
cargo clippy -p vtuber-app -p vtuber-avatar -p vtuber-desktop --all-targets -- -D warnings
cargo test -p vtuber-app avatar_import
cargo test -p vtuber-avatar
cargo test -p vtuber-desktop
```

#### M1-08-010: dev-only synthetic trackingでavatar適用経路を証明する

状態: `DONE`
依存: `M1-08-009`
親参照: DESIGN.md §11、§15、§16、§20.1、ADR-004

**変更候補**

- `crates/vtuber-app/src/synthetic_tracking.rs`
- `crates/vtuber-avatar/src/plugin.rs`
- `apps/desktop/src/main.rs`
- `crates/vtuber-app/tests/synthetic_tracking.rs`

**実装指示**

- `AvatarControlFrame`を生成するdev-only sourceを作り、release defaultでは無効にする。feature名またはCLI flagは一つに限定する。
- yaw、pitch、roll、左右blink、mouth、gazeを一定周期で再現可能に生成する。
- lifecycle `Ready`のactive generationへ`tag_control_frame`／`set_active_control_frame`を通して投入する。
- head bone、expression morph、eye／gazeを直接操作せず、M1-05／M1-06で作った既存apply systemを通す。
- 既存apply systemがpluginへ未登録なら登録だけを行い、同責務を再実装しない。
- model replace時は新generationへ自動的に切り替え、stale frameを拒否する。
- deterministic unit testでは固定時刻を注入し、GPU／cameraなしで各phaseのcontrol値を検証する。

**このsubtaskで行わないこと**

- production inputとしてsynthetic modeを既定有効にしない。
- camera／inferenceの不足をsynthetic値で隠さない。
- bone nameによる独自探索を追加しない。

**完了条件**

- cameraなしでVRMの頭・目・口が目視で動く。
- active avatarがない時はframeを安全にdropする。
- replace後に旧generation frameが適用されない。
- synthetic mode無効時にproduction経路へ影響しない。

**検証**

```powershell
cargo test -p vtuber-app synthetic_tracking
cargo test -p vtuber-avatar
cargo check -p vtuber-desktop --features dev-synthetic-input
```

#### M1-08-011: Windows Nokhwa／MSMF backendを実装しdevice選択契約を修正する

状態: `DONE`
依存: `M1-08-010`
親参照: DESIGN.md §10.2、§12、§13、§20.2、§21.2

**変更候補**

- `crates/vtuber-camera/src/device.rs`
- `crates/vtuber-camera/src/backend/windows.rs`
- `crates/vtuber-camera/src/capture.rs`
- `crates/vtuber-camera/src/lib.rs`
- `crates/vtuber-camera/tests/windows_contract.rs`

**実装指示**

- `CameraBackend::open`が選択した`CameraDescriptor`を明示的に受け取る契約へ変更し、mockとcontrollerを同時更新する。
- `nokhwa 0.10.11`のMSMF backendでcameraを列挙し、display labelと再列挙中に使えるdevice identityを`CameraDescriptor`へ変換する。
- selected descriptorを実際のNokhwa camera index／identifierへ解決してopenする。first camera固定やindex無視を禁止する。
- requestに対してcompatible formatを列挙し、既存`select_format`で1280x720@30付近を選ぶ。実formatをmetricsへ記録する。
- native camera objectのconstruct、open、frame、stop、dropをcapture worker内に閉じ込める。
- MJPEG／YUYV等を`VideoFrame`のcanonical pixel formatへdecodeし、strideとtimestampを正しく設定する。
- permission、busy、disconnect、decode failureを既存typed errorへmapする。
- Windows以外のbackendをこのsubtaskで変更しない。

**このsubtaskで行わないこと**

- OpenCV、Media Foundation C++ wrapper、別camera crateを追加しない。
- camera indexだけを永続identityにしない。
- main threadでblocking frame captureしない。

**完了条件**

- 実Windows cameraが少なくとも一台列挙され、選択したdeviceをopenできる。
- mock testで二台の選択が取り違えられない。
- Start／Stop／drop後にdevice handleがworker内で解放される。
- disconnectがprocess panicではなくservice stateへ出る。

**検証**

```powershell
cargo fmt --all -- --check
cargo clippy -p vtuber-camera --all-targets -- -D warnings
cargo test -p vtuber-camera
cargo check -p vtuber-desktop
```

#### M1-08-012: 実captureをorchestratorとpreviewへ接続する

状態: `DONE`
依存: `M1-08-011`
親参照: DESIGN.md §12、§13、§17.1、§20.2〜§20.4、§21.2

**変更候補**

- `crates/vtuber-app/src/orchestrator.rs`
- `crates/vtuber-app/src/runtime.rs`
- `crates/vtuber-app/src/preview.rs`
- `crates/vtuber-app/src/diagnostics.rs`
- `crates/vtuber-app/tests/capture_orchestration.rs`

**実装指示**

- stub camera listを削除し、Windows backendの実enumeration結果を`UiViewModel.camera.available_cameras`へ変換する。
- selected descriptorとrequest formatを保持し、Startで`CaptureController` workerを起動してselected deviceをcaptureする。
- Stopでcaptureを明示停止し、再Startできるようにする。controllerをdetachしない。
- capture `LatestSlot<VideoFrame>`からpreview更新用の最新frameだけを読む。
- `PreviewState.image_handle`を一度作成して既存`Image` assetを更新し、frameごとに新しいassetを生成しない。
- preview OFFはtexture更新をthrottle／skipしてもcaptureを停止しない。mirrorは表示UVだけへ適用する。
- app lifecycleが`Running`でもtracking stateはまだ`Idle`であることを明示し、full tracking成功を偽装しない。
- camera error、disconnect、BackOffをDiagnosticsとrecoverable errorへ伝える。

**このsubtaskで行わないこと**

- inference／trackingを開始しない。
- preview用に別capture streamを開かない。
- unbounded frame historyを保持しない。

**完了条件**

- GUIで実cameraを選択し、Start後にlive previewが表示される。
- Stop／Startを三回繰り返して再接続できる。
- preview OFFでもcapture workerが意図せず停止しない。
- slot depthは1以下で、memoryがframe数に比例して増えない。

**検証**

```powershell
cargo test -p vtuber-app
cargo test -p vtuber-camera
cargo clippy -p vtuber-app -p vtuber-camera -p vtuber-desktop --all-targets -- -D warnings
cargo run -p vtuber-desktop --release
```

#### M1-08-013: detector→crop→Peppa landmarkのproduction face pipelineを確定する

状態: `BLOCKED`（repair branch実行中）
依存: `M1-08-012`
実行単位: `M1-08-013-001`〜`M1-08-013-009`
親参照: DESIGN.md §3、§11.4〜§11.6、§14、§15、ADR-001

### blocker

現行Peppa ONNXは、あらかじめface cropへ整形された入力から98点landmarkを推定するmodelである。full camera frameを直接resizeして入力しても、face detection／ROI選択／crop contractを満たさない。

完了済みのもの:

- `tract-onnx 0.23.4`でPeppa modelをload／optimize／runするruntime
- 98点2D landmark decode
- `peppapig-98`専用planar pose solver
- capture、preview、worker、tracking、avatar bridge、diagnostics、shutdownの基盤

不足しているもの:

- fixed inputとWindows実cameraで、required loss/recovery、movement、edge、Stop/Start protocolを含むfull end-to-end gate

20秒のC922 MSMF probeでは、face 6件、98 finite landmarks、finite planar pose 5件を確認済み。初回の `NO_FACE` はlandmark visibility値の範囲外をONNX decoderがclampしていなかった実装不備で、修正済み。

### fixed direction

第一候補としてUltraFace RFB-320 ONNXを採用候補に固定する。exact artifactが`tract-onnx 0.23.4`で動くことを`M1-08-013-002`で証明するまでは「採用済み」と記述しない。

authoritative contract:

```text
file        version-RFB-320.onnx
source      ONNX Model Zoo / UltraFace
license     MIT
SHA-256     34cd7e60aeff28744c657de7a3dc64e872d506741de66987f3426f2b79f88017
size        1,270,727 bytes
opset       9
input       [1, 3, 240, 320] F32 NCHW RGB
normalize   (pixel - 127.0) / 128.0
outputs     scores [1, 4420, 2], boxes [1, 4420, 4]
threshold   0.7 initial value
NMS IoU     0.3 initial value
```

UltraFace以外のdetectorへ変更する場合は、`M1-08-013-002`のexact failureを記録した後、新repair leafとADR amendmentを追加する。同じleafで候補を次々に試して根拠を失わせない。

### parent完了条件

次をすべて満たした時だけ`M1-08-013`を`DONE`へ変更する。

- detector／landmark両artifactのsource、license、SHA-256、tensor contractがmanifestにある。
- UltraFace exact artifactが`tract-onnx 0.23.4`でload／optimize／runできる。
- full frame→face box→square crop→Peppa 98 landmarks→source image座標へのinverse mappingがpure Rustで成立する。
- no-faceはtyped outcomeとなり、recoverable runtime failureと区別される。
- fixed inputとWindows cameraの双方からfiniteなlandmarks／head poseが得られる。
- detector cadence中もlandmark inferenceは毎対象frame継続する。
- model／crop変更時にgolden testが検知する。
- ADR-001、manifest、Windows acceptance reportが実装と一致する。

---

#### M1-08-013-001: UltraFace artifactと二段pipeline manifest schemaを固定する

状態: `DONE`
依存: `M1-08-012`
親参照: M1-08-013、ADR-001

**変更候補**

- `assets/models/manifest.toml`
- `crates/vtuber-app/src/model_catalog.rs`
- `crates/vtuber-inference/src/descriptor.rs`
- `tools/xtask/src/acceptance.rs`
- `LICENSES/`または既存license inventory

**実装指示**

- 現在の`models[0]`前提を廃止し、modelをstable ID／roleで解決するmanifest schemaへ変更する。
- 少なくとも`role = "face_detector"`と`role = "face_landmarks"`を表現し、二つを一つのproduction pipeline IDへ束縛する。
- UltraFace entryへ上記fixed contract、authoritative source、upstream、MIT license、exact SHA-256、exact byte sizeを記録する。
- Peppa entryは保持し、`role = "face_landmarks"`、crop-required、landmark coordinate encoding、planar pose methodを明示する。
- detector postprocessにscore threshold `0.7`、NMS IoU `0.3`、max pre-NMS candidates `256`、max post-NMS detections `16`をmanifest値として置く。
- crop configにprovisional baselineとして、square scale `1.35`、center Y offset `-0.05` face-box heights、output `256x256`、bilinear、outside fill=`normalization mean`を置く。これらは`013-009`で受入値へ確定する。
- `FacePipelineDescriptor`等のplain data contractを定義する。detector／landmark model runtime objectを含めない。
- `acceptance verify`を二artifact対応にし、file不存在、size不一致、hash不一致、role重複、pipeline参照切れをtyped failureにする。
- runtime起動時downloadは行わない。development fetchを追加する場合はtemp file→size／hash verify→atomic renameを必須にする。

**このleafで行わないこと**

- detector runtimeを実装しない。
- Peppa artifact／planar solverを置換しない。
- model file名やtensor indexをapp sourceへ重複hard-codeしない。
- hash checkを無効化するforce optionを作らない。

**完了条件**

- production pipelineをIDで一意に解決できる。
- manifest parserがarray先頭順に依存しない。
- detector／landmark双方のfile、size、hash、license、input／output contractを検証できる。
- existing Peppa-only manifest testをpipeline manifest testへ更新できる。
- model binaryがない環境でもerrorがどのartifact不足か特定できる。

**検証**

```powershell
cargo fmt --all -- --check
cargo test -p vtuber-app model_catalog
cargo test -p vtuber-inference descriptor
cargo test -p xtask
cargo run -p xtask -- acceptance verify assets/models/manifest.toml
cargo clippy -p vtuber-app -p vtuber-inference -p xtask --all-targets -- -D warnings
```

#### M1-08-013-002: UltraFaceをtract-onnx 0.23.4でexact probeする

状態: `DONE`
依存: `M1-08-013-001`
親参照: M1-08-013、ADR-001

**変更候補**

- `crates/vtuber-inference/src/probe.rs`
- `crates/vtuber-inference/tests/ultraface_probe.rs`
- `assets/models/operator-inventory-ultraface.*`
- `docs/adr/ADR-001-face-inference-runtime-and-model.md`

**実装指示**

- manifestで検証済みのexact UltraFace artifactだけを対象にする。
- `tract-onnx = 0.23.4`でmodel load、input fact設定、optimize、runnable化、runを段階別に実行する。
- input factは`F32 [1,3,240,320]`へ固定し、manifestとmodelの実shape／dtypeを照合する。
- zero／mean inputに加え、ONNX Model Zooのmodel-with-test-data artifactを利用できる場合は、そのSHA-256も記録してreference input／outputを実行する。
- output名、順序、shape、dtypeをprobe結果から記録する。runtime実装はoutput indexだけを無条件に仮定しない。
- output全値のfinite性、scores／boxesのelement countを確認する。
- operator inventoryをstable順序で保存する。
- stage別errorへmodel ID、SHA-256、stage、operator／node情報を含める。
- zensally等の既存Tract実装はfeasibility evidenceに留め、このprojectのprobe結果で代用しない。

**このleafで行わないこと**

- probe failure時にONNX Runtime、OpenCV、TFLiteへfallbackしない。
- 別UltraFace variant、int8版、YuNet等を同時採用しない。
- output toleranceを広げてNaN／shape mismatchを通さない。

**完了条件**

- exact modelがload／optimize／runを通る、またはexact blockerが再現可能に記録される。
- output contractが`[1,4420,2]`と`[1,4420,4]`へ一致する。
- probe成功時、次leafが利用できるrunnable construction functionがある。
- probe失敗時、`M1-08-013`はBLOCKEDのままで、`013-003`へ進まない。

**検証**

```powershell
cargo test -p vtuber-inference ultraface_probe --features onnx -- --nocapture
cargo clippy -p vtuber-inference --features onnx --all-targets -- -D warnings
cargo run -p xtask -- acceptance verify assets/models/manifest.toml
```

#### M1-08-013-003: UltraFace preprocessとdetector stage runtimeを実装する

状態: `DONE`
依存: `M1-08-013-002`
親参照: M1-08-013、DESIGN.md §14

**変更候補**

- `crates/vtuber-inference/src/detector/mod.rs`
- `crates/vtuber-inference/src/detector/preprocess.rs`
- `crates/vtuber-inference/src/detector/runtime.rs`
- `crates/vtuber-inference/tests/detector_preprocess.rs`
- `crates/vtuber-inference/tests/detector_runtime.rs`

**実装指示**

- `VideoFrame`のsupported canonical pixel formatsをRGBへ読み、320x240へbilinear direct resizeする。official UltraFace contractどおり、この段階ではletterboxへ変えない。
- RGB各channelへ`(pixel - 127.0) / 128.0`を適用し、NCHW `[1,3,240,320]`へ格納する。
- source strideを尊重し、packed width前提にしない。
- preprocess bufferをworker-ownedで再利用し、steady stateでframeごとの大容量allocationを行わない。
- detector stage runtimeは`FaceInference`のlandmark固有outputへ押し込まず、detector専用typed APIとする。
- model runnableはworker thread内でconstruct／dropされる構造を維持する。
- unsupported pixel format、zero dimension、short buffer、non-finite normalization設定をtyped errorにする。
- testはsmall synthetic RGB patternからchannel order、resize orientation、normalization、NCHW indexを固定する。

**このleafで行わないこと**

- box decode／NMS／cropを実装しない。
- preview imageを推論入力として再encodeしない。
- main threadでmodel runtimeをconstructしない。
- unsafe SIMDを追加しない。

**完了条件**

- fixed frameからmanifest contractどおりのdetector tensorを再現できる。
- RGB／BGR、width／height、NCHW／NHWCの取り違えをtestが検知する。
- buffer reuseがAPIから明確である。
- UltraFace runnableへtensorを渡しraw outputsを取得できる。

**検証**

```powershell
cargo test -p vtuber-inference detector_preprocess --features onnx
cargo test -p vtuber-inference detector_runtime --features onnx
cargo clippy -p vtuber-inference --features onnx --all-targets -- -D warnings
```

#### M1-08-013-004: UltraFace output decode、NMS、primary face selectionを実装する

状態: `DONE`
依存: `M1-08-013-003`
親参照: M1-08-013、DESIGN.md §14

**変更候補**

- `crates/vtuber-inference/src/detector/decode.rs`
- `crates/vtuber-inference/src/detector/nms.rs`
- `crates/vtuber-inference/src/roi.rs`
- `crates/vtuber-inference/tests/detector_decode.rs`

**実装指示**

- raw outputsをvalidated output name／shape contractからscoresとboxesへ解決する。
- face class scoreをthresholdし、boxesを`[xmin,ymin,xmax,ymax]` normalized source coordinatesへ変換する。
- NaN／Inf、反転box、zero area、著しく範囲外のboxをrejectし、微小な範囲超過だけclampする。
- pre-NMS candidateをconfidence順に最大256へbounded化する。
- hard NMSをIoU `0.3`で実装し、post-NMS最大16件へ制限する。
- parameterはmanifestから取得し、decode moduleにmagic numberを散らさない。
- single-user primary face policyをpure functionにする。
  - previous ROIとIoU `>= 0.2`の候補があればIoU最大、同値ならconfidence最大。
  - continuity候補がなければconfidence最大、同値ならarea最大。
- `NoFace`、`Detections(Vec<...>)`、malformed outputを区別する。
- no-faceは通常の観測結果であり、worker consecutive failure countを増やさない。
- known boxesでIoU、threshold、NMS、continuity selectionをdeterministic testする。

**このleafで行わないこと**

- multi-person UIを追加しない。
- no-faceを空の全画面ROIへfallbackしない。
- unbounded Vec historyを保持しない。
- detector boxからcropをまだ作らない。

**完了条件**

- official output shapeからface boxesをboundedにdecodeできる。
- overlap boxのNMS結果が既知fixtureと一致する。
- faceの一時的なconfidence順変動でprimary identityが不必要に切り替わらない。
- no-faceとruntime failureをcallerが判別できる。

**検証**

```powershell
cargo test -p vtuber-inference detector_decode --features onnx
cargo test -p vtuber-inference detector_nms --features onnx
cargo test -p vtuber-inference primary_face_selection --features onnx
```

#### M1-08-013-005: detector boxからPeppa cropへのtyped transformを実装する

状態: `DONE`
依存: `M1-08-013-004`
親参照: M1-08-013、DESIGN.md §11、§14

**変更候補**

- `crates/vtuber-inference/src/crop.rs`
- `crates/vtuber-inference/src/preprocess.rs`
- `crates/vtuber-inference/src/decode.rs`
- `crates/vtuber-inference/tests/face_crop.rs`

**実装指示**

- `FaceCropTransform`を定義し、source normalized coordinates、source pixels、crop pixels、landmark model coordinatesの相互変換を一箇所へ集約する。
- detector boxをmanifestのcrop configで拡張し、square ROIを作る。
  - initial scale `1.35`
  - initial center Y offset `-0.05 * detector_box_height`
  - output `256x256`
- source外へはみ出すROIを切り詰めて形を変えず、paddingとして扱う。
- paddingはPeppaのImageNet meanへ正規化後0になる値を使う。
- crop resizeはbilinear、RGB、Peppa manifestのNCHW／mean／stdに従う。
- Peppa outputのx／y coordinate encodingをprobeで確定し、`normalized_0_1`、`crop_pixels`等のenumで表す。現状の暗黙的な「すでにnormalized」という仮定を禁止する。
- 98 landmarksをcrop座標からsource normalized座標へinverse mappingし、`RawFaceObservation.roi`へ実ROIを記録する。
- edge／cornerのROI、1px box、outside padding、round tripの誤差をpure testする。
- crop configは`013-009`の実camera結果で調整可能だが、accepted valueはmanifest／ADR／goldenを同時更新する。

**このleafで行わないこと**

- OpenCV affine／resizeを導入しない。
- bboxを単純clampしてaspect ratioを壊さない。
- mirror preview flagをcrop／inference座標へ入れない。
- crop parameterをmodel_catalog以外へ重複hard-codeしない。

**完了条件**

- source→crop→source round tripが許容誤差内で一致する。
- frame edgeのfaceでもpanic／short readがない。
- Peppaへ常にvalid `[1,3,256,256]` tensorを渡せる。
- output landmarksがsource normalized coordinateへ戻る。
- crop scale／offsetを変更するとgolden testが明示的に変化する。

**検証**

```powershell
cargo test -p vtuber-inference face_crop --features onnx
cargo test -p vtuber-inference crop_round_trip --features onnx
cargo clippy -p vtuber-inference --features onnx --all-targets -- -D warnings
```

#### M1-08-013-006: frame-level composite inference contractを導入する

状態: `DONE`
依存: `M1-08-013-005`
親参照: M1-08-013、DESIGN.md §12、§14

**変更候補**

- `crates/vtuber-inference/src/descriptor.rs`
- `crates/vtuber-inference/src/runtime.rs`
- `crates/vtuber-inference/src/pipeline.rs`
- `crates/vtuber-app/src/model_catalog.rs`
- `crates/vtuber-inference/tests/composite_contract.rs`

**実装指示**

- single model用`ModelDescriptor`をstage descriptorとして再利用し、production用`FacePipelineDescriptor`にdetector、landmark、postprocess、crop configを束ねる。
- production boundaryをpreprocessed tensor単体ではなく`VideoFrame`単位にする。
- 例として`FrameFaceInference::infer_frame(&mut self, &VideoFrame) -> Result<FrameInferenceOutcome>`相当のtyped contractを導入する。名称は既存codeに合わせてよい。
- outcomeは少なくとも`Face(RawFaceObservation)`と`NoFace`を区別する。
- individual ONNX runnableを扱うinternal stage APIと、end-to-end frame APIを分離する。
- detectorとPeppa runnableは同じinference worker内でconstruct、所有、dropする。controller／Bevy resourceへlive model objectを出さない。
- `model_catalog`はproduction pipeline IDを解決し、first `models[]`前提を残さない。
- existing `FaceInference`をtest probe用に残す場合も、production workerがsingle modelへ直接結合しないようにする。
- public contractへcoordinate space、ownership、thread confinement、no-face semanticsを記す。

**このleafで行わないこと**

- worker loopへまだ接続しない。
- app／tracking／avatar crateへdetector-specific tensor型を漏らさない。
- general graph／plugin frameworkを作らない。
- 二つのruntimeを別threadへ分割しない。

**完了条件**

- plain descriptorだけがcontrollerからworkerへ渡る。
- composite runtimeがdetectorとlandmarkの双方をworker内で所有できる。
- no-faceがerrorと区別される。
- appはpipeline IDから完全なdescriptorを構築できる。
- single-model whole-frame Peppa経路をproduction contractから選べない。

**検証**

```powershell
cargo test -p vtuber-inference composite_contract --features onnx
cargo test -p vtuber-app model_catalog
cargo clippy -p vtuber-inference --features onnx --all-targets -- -D warnings
cargo clippy -p vtuber-app --all-targets -- -D warnings
```

#### M1-08-013-007: detector cadenceとROI recoveryを含むcomposite runtimeを実装する

状態: `DONE`
依存: `M1-08-013-006`
親参照: M1-08-013、DESIGN.md §12、§14、§20.2

**変更候補**

- `crates/vtuber-inference/src/composite.rs`
- `crates/vtuber-inference/src/worker.rs`
- `crates/vtuber-inference/src/pipeline.rs`
- `crates/vtuber-inference/src/metrics.rs`
- `crates/vtuber-inference/tests/composite_runtime.rs`

**実装指示**

- composite runtimeの一frame処理を次の順序へ固定する。
  1. ROIなし／lost／detector期限ならUltraFace実行。
  2. primary detectionがあればexpanded square cropを更新。
  3. active ROIがあればPeppa landmarkを毎対象frame実行。
  4. landmarkをsource座標へ戻して`RawFaceObservation`を構築。
  5. confidence不足／malformed landmarkならROIをinvalidateし、次frameでdetectorを強制。
- correctness baselineではsearch中detectorを毎frame実行する。
- tracking中のinitial detector intervalは既存settingsの5 framesを使用してよいが、**detectorをskipしたframeでもlandmark stageは実行する**。
- previous ROI continuity、face loss、reacquireをexplicit stateにする。
- no detectionは`NoFace`を返し、consecutive runtime errorを増やさない。
- source `FrameSeq`、captured timestamp、inference start／finishを保持する。stageごとに別epochを作らない。
- detector、crop、landmark、decode、totalのbounded metricsを追加する。
- detector／landmark failureをstage付きerrorにし、どちらのmodelが失敗したか区別する。
- output slotはlatest-onlyを維持し、ROI history／frame historyを蓄積しない。
- old logicの`should_run_detector == false`で全inferenceをskipする挙動を除去する。
- mock detector／landmark stageでdetector cadence、landmark毎frame、loss→reacquireをdeterministic testする。

**このleafで行わないこと**

- optical flow tracker、Kalman tracker、multi-face identity trackerを追加しない。
- performance tuningを先行しない。
- detector／landmarkを別workerへ分けない。
- no-face時に古いlandmarkを新観測として再発行しない。

**完了条件**

- stable ROI中、detector intervalが5でもlandmark outputは各入力frameで生成される。
- loss時にdetector searchへ戻り、face復帰後に自動reacquireする。
- detector／landmarkのerrorとno-faceがmetrics／statusで区別される。
- latest-only／bounded memory invariantを維持する。
- composite runtimeのunit testがcameraなしで通る。

**検証**

```powershell
cargo test -p vtuber-inference composite_runtime --features onnx
cargo test -p vtuber-inference detector_cadence --features onnx
cargo test -p vtuber-inference roi_recovery --features onnx
cargo clippy -p vtuber-inference --features onnx --all-targets -- -D warnings
```

#### M1-08-013-008: detector→crop→landmarkのgolden／replay gateを作る

状態: `DONE`
依存: `M1-08-013-007`
親参照: M1-08-013、DESIGN.md §14、§21

**変更候補**

- `crates/vtuber-inference/tests/composite_golden.rs`
- `crates/vtuber-inference/tests/data/`
- `assets/models/golden/`
- `docs/adr/ADR-001-face-inference-runtime-and-model.md`
- `assets/models/manifest.toml`

**実装指示**

- UltraFaceのofficial model-with-test-dataが利用できる場合、そのsource、license、SHA-256を記録しdetector output goldenへ使う。
- end-to-endには権利・出典が明確な単一顔画像を使用する。任意の検索画像や個人camera frameをrepositoryへcommitしない。public-domain／permissive assetを使用し、source URL、license、SHA-256をmanifestまたはtest-data READMEへ残す。
- blank／mean imageで`NoFace`を確認する。
- single-face imageで少なくとも次を固定する。
  - primary face box
  - crop transform
  - landmark count 98
  - all finite
  - source coordinate range
  - detector／landmark confidence
- multi-face referenceを合法に用意できる場合はprimary selectionをgolden化する。用意できない場合はsynthetic detection listのpure testを正とし、実画像を捏造しない。
- golden toleranceをoutput categoryごとに定義し、model／crop config hash変更でsilent passしないようにする。
- planar poseのaxis signは既存synthetic testを維持し、single portraitの主観だけでyaw／pitchをgolden化しない。
- replay harnessはfixed frame sequenceをcomposite runtimeへ流し、same sequence、same outcome、bounded metricsを再現する。
- golden更新は明示command／reviewを必要とし、自動上書きしない。

**このleafで行わないこと**

- raw user camera recordingをcommitしない。
- toleranceを顔box全域が許容されるほど広くしない。
- model hash変更時にgoldenを自動更新しない。
- physical camera試験をこのleafの自動testで代替しない。

**完了条件**

- no-faceとsingle-faceのend-to-end結果を自動再現できる。
- detector→crop→98 landmarksの各境界を個別に診断できる。
- license／source／hash不明のtest assetがない。
- model／crop contractの破壊をtestが検知する。
- replayでmemoryがframe数に比例しない。

**検証**

```powershell
cargo test -p vtuber-inference composite_golden --features onnx -- --nocapture
cargo test -p vtuber-inference composite_replay --features onnx
cargo run -p xtask -- acceptance verify assets/models/manifest.toml
```

#### M1-08-013-009: Windows full-frame camera probeでparent gateを解除する

状態: `DONE`
依存: `M1-08-013-008`
親参照: M1-08-013、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `tools/xtask/src/face_pipeline_smoke.rs`
- `tools/xtask/src/face_image_probe.rs`
- `tools/xtask/src/main.rs`
- `tools/xtask/Cargo.toml`
- `crates/vtuber-inference/src/runtime.rs`
- `crates/vtuber-inference/src/diagnostic.rs（必要なら）`
- `docs/acceptance/windows-m1.md`
- `docs/adr/ADR-001-face-inference-runtime-and-model.md`
- `assets/models/manifest.toml`

**実装指示**

- main desktop appへ接続する前に、既存Windows MSMF captureとcomposite runtimeだけを直結するdiagnostic commandを作る。
- command例は`cargo run -p xtask -- face-pipeline-smoke ...`とし、camera descriptor、duration、pipeline ID、optional JSON outputを受け取る。`--guided-protocol`ではCUIが各操作の開始時刻と残り3秒を表示し、neutral、face loss/recovery、方向、edge、capture Stop/Startを定刻で案内する。
- command内でcapture controllerとcomposite inference workerを正規のthread ownership規約どおり起動し、終了時にinference→captureの順でstop／joinする。
- avatar、Bevy renderer、eguiを起動しない。これによりdetector／crop／landmark blockerをapp integrationから分離する。
- console／bounded JSON summaryへ次を記録する。
  - detector state
  - detector confidence
  - normalized ROI
  - detector Hz／landmark Hz
  - face／no-face count
  - detector／crop／landmark stage error
  - crop scale／offset
  - 98 landmark finite count
  - planar pose finite countとyaw／pitch／roll range
- raw frame／crop／landmark全列をdiskへ自動保存しない。debug画像出力を追加する場合は明示flag、単発、local-only、default offとする。
- 実cameraで少なくとも次を実施する。
  - centered neutral face 60秒
  - face out／returnを5回
  - 左右／上下へ通常範囲で移動
  - frame edge近くへ移動
  - Stop／Startを3回
- neutral face runで継続的に98 finite landmarksを得る。
- planar pose calibration相当のneutral referenceをdiagnostic command内で明示収集し、yaw／pitch／rollがfiniteで、正負方向が既存semantic conventionと一致することを確認する。
- provisional crop scale／Y offsetが額、顎、両頬を安定して含むか確認する。変更した場合はmanifest、golden、ADRを同一差分で更新する。
- detector／crop／landmarkのどこでlossしたかをreportへ記録する。
- success時に`M1-08-013`を`DONE`へ変更し、`M1-08-014`を次実行単位とする。
- failure時は`M1-08-013`をBLOCKEDのままにし、exact failureを分類して`M1-08-013-010`以降のrepair leafを追加する。`M1-08-014`へ進まない。

**Windows実機結果（2026-08-11）**

`c922 Pro Stream Webcam`（MSMF index 0、1280x720 @ 30/1、Rgb8）で、次の
commandを実行してguided protocolの全フェーズを完了した。顔を画面外へ
出すフェーズを含むため、`no_face_count`は失敗ではなく期待される観測である。

```text
cargo run -p xtask -- face-pipeline-smoke --camera 0 --guided-protocol --json
frames_captured: 4944
face_count/no_face_count: 50/19
detector_hz/landmark_hz: 0.387/0.347
stage_error: none
detector_confidence: 0.779927
finite_landmarks/finite_pose_count: 98/44
pose yaw/pitch/roll ranges: [-5.145444,2.186486] [-2.124861,4.208844] [-2.604939,1.199027]
```

最初の実行は全フェーズの指示とsummaryを出力した後、終了時刻ちょうどの
境界で完了フラグを立て損ねて終了コード1になった。このCUI bookkeeping
bugを修正し、exact deadlineの回帰テストを追加した。summaryの実測値と
worker／cameraの終了処理には異常はなかった。

**このleafで行わないこと**

- desktop orchestrator、tracking runtime、avatar bridgeへ接続しない。
- guided functional smokeをperformance acceptanceとして流用しない。
- crop parameterを目視だけでcodeへ直書きしない。
- detector missを古いROIの永久保持で隠さない。
- physical test未実施をPASSにしない。

**完了条件**

- full camera frameからdetector→crop→Peppa→98 landmarksが継続する。
- face loss／returnでsearch／reacquireが成立する。
- finiteなneutral-relative poseを生成できる。
- detector／landmark Hzとfailure stageをsummaryで観測できる。
- command終了後にcamera handleとworker threadが残らない。
- manifest、ADR、reportが実測値と一致する。
- parent `M1-08-013`の全完了条件を満たす。

**検証**

```powershell
cargo fmt --all -- --check
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- acceptance verify assets/models/manifest.toml
cargo run -p xtask -- face-pipeline-smoke --help
cargo build -p vtuber-desktop --release
# 上記に加え、本leafのWindows手動protocolを実施し、結果をreportへ記録する。
```


#### M1-08-014: composite runtimeを既存InferenceRuntime／orchestratorへ接続する

状態: `DONE`
依存: `M1-08-013-009`
親参照: DESIGN.md §12、§14、§17、§20.2〜§20.3、§21.2

**現状の扱い**

`crates/vtuber-app/src/inference_runtime.rs`、`model_catalog.rs`、`crates/vtuber-inference/src/controller.rs`／`worker.rs`は既に存在する。これらを捨てて別workerを作らず、single-model前提を`M1-08-013`で確定したcomposite pipelineへ置換する差分だけを実装する。

**blocker resolution note (2026-08-11)**

The avatar load blocker was the pinned `bevy_vrm1` lifecycle boundary: it removes `VrmHandle` before adding `Initialized`, while the adapter required both `Added<Initialized>` and `&VrmHandle` on the same root. Successful initialization is now observed from `Initialized` independently, transient handle-less roots remain in `Loading`, and successful humanoid binding restores root visibility before `Ready`. Automated validation, the managed-source runtime gate, and the physical Windows GUI/camera acceptance all passed; `M1-08-014` is `DONE`.

**変更候補**

- `crates/vtuber-app/src/inference_runtime.rs`
- `crates/vtuber-app/src/model_catalog.rs`
- `crates/vtuber-app/src/orchestrator.rs`
- `crates/vtuber-inference/src/controller.rs`
- `crates/vtuber-inference/src/worker.rs`
- `crates/vtuber-inference/src/state.rs`
- `crates/vtuber-app/tests/inference_orchestration.rs`

**実装指示**

- controllerへ渡すload commandをsingle `ModelDescriptor`から`FacePipelineDescriptor`へ変更する。
- runtime objectは既存どおりworker内でconstruct／dropする。detectorとlandmarkの双方を同一workerが所有する。
- appのmodel catalogはmanifestのproduction pipeline IDからdescriptorを構築する。
- Start時はcapture worker→composite model load→inference readyの順に進め、各stageの成功を確認してからapp lifecycleをRunningへする。
- Stop時はinferenceを先にstop／joinし、その後captureをstopする。closed input slot待ちでhangしない。
- existing capture `LatestSlot<VideoFrame>`をcomposite workerへそのまま渡し、previewと別captureを開かない。
- `FrameInferenceOutcome::Face`相当だけをobservation slotへpublishし、`NoFace`はtyped state／metricとしてtracking側へ伝えられるようにする。必要ならno-face signal用のlatest stateを追加するが、fake observationは作らない。
- detector／landmark／crop／total metrics、last source sequence、ROI state、stage failureをexisting statusへ追加する。
- existing detector cadence設定をcomposite runtimeへ渡し、landmark毎frame invariantを壊さない。
- model load、detector infer、landmark infer、malformed outputをstable error codeで区別し、previewだけは可能な範囲で継続できる。
- old whole-frame Peppa pathとsingle-model production constructorを検索し、production appから到達不能にする。
- testsはmock composite runtimeを注入し、Start／Face／NoFace／runtime failure／Stopのstate transitionをcameraなしで確認する。

**このsubtaskで行わないこと**

- tracking math、calibration、avatar applyを再実装しない。
- detector／landmarkを別threadへ分けない。
- FIFO frame queueを追加しない。
- runtime error時にprocess abortしない。
- previewを停止して推論failureを隠さない。

**完了条件**

- GUI Startでexisting capture slotからcomposite workerがframeを受け取る。
- actual face frameで`RawFaceObservation`がlatest-onlyで発行される。
- no-faceとruntime failureがstatus上で区別される。
- Stop後にinference workerが残らず、再Startできる。
- single-model whole-frame Peppa production pathがない。
- model hash mismatch／detector failure／landmark failureをUIへ区別して伝えられる。

**検証**

```powershell
cargo fmt --all -- --check
cargo test -p vtuber-inference --features onnx --no-fail-fast
cargo test -p vtuber-app inference_orchestration
cargo clippy -p vtuber-inference -p vtuber-app -p vtuber-desktop --all-targets -- -D warnings
cargo build -p vtuber-desktop --release
```


#### M1-08-015: 既存TrackingRuntimeをcomposite observationsへ適合・実機検証する

状態: `DONE`
備考: MediaPipe canonical pipeline、C922 real-VRM functional／recovery acceptance、bounded performance acceptanceまで完了した。
依存: `M1-08-014`
親参照: DESIGN.md §11、§15、§17.2、§20.1、§21.1

> **2026-08-11 rewrite:** この親taskの旧UltraFace／PeppaPig／planar-pose
> 指示と実装証拠はproduction decisionとしてsupersededである。以下の
> `M1-08-015-001`〜`M1-08-015-012`を現在の実行順序とし、旧camera gateを
> MediaPipeの受入証拠として再利用しない。旧コードは後続leafでproduction
> reachabilityを除去する。

以下の「現状の扱い」「変更候補」「実装指示」「このsubtaskで行わないこと」
「完了条件」「検証」は旧composite tracking pathの履歴であり、現在の実行指示
ではない。現在は直後の`M1-08-015 rewrite leaf sequence`だけを実行する。

**現状の扱い**

`crates/vtuber-app/src/tracking_runtime.rs`、`CalibrationCollector`、`CalibrationSession`、`TrackingPipeline`、planar pose branchは既に実装されている。新しいtracking pipelineを作らず、detector／crop後のsource-normalized 98 landmarksとtyped no-face outcomeを既存runtimeへ正しく流す。

**変更候補**

- `crates/vtuber-app/src/tracking_runtime.rs`
- `crates/vtuber-app/src/inference_runtime.rs`
- `crates/vtuber-app/src/orchestrator.rs`
- `crates/vtuber-app/src/ui_model.rs`
- `crates/vtuber-tracking/src/pipeline.rs`
- `crates/vtuber-app/tests/tracking_orchestration.rs`

**実装指示**

- composite inferenceのlatest `Face` outcomeを既存`TrackingRuntime`へ渡す。
- `NoFace`は`TrackingPipeline::update(None, ...)`へ渡し、lost hold／neutral decay／recoveryを既存state machineで処理する。
- no-face中に最後のlandmark observationを新frameとして再利用しない。
- `peppapig-98` landmarksがsource normalized x-right／y-downであることをschema contractとtestで確認する。
- crop座標のままcalibrationへ流れないことをassert／testする。
- `UiAction::BeginCalibration`、`CancelCalibration`、`RetryCalibration`は既存collector／sessionへ接続したまま維持する。
- sample count、reject reason、quality、complete状態を既存`UiViewModel.calibration`へ同期する。
- calibration完了後、existing planar pose branchでneutral-relative yaw／pitch／rollを生成する。
- detector confidence、landmark confidence、reprojection errorをconfidence synthesisへどう渡すか一箇所で定義する。欠落値を無条件1.0にしない。
- detector ROIが変わってもsource coordinateへ戻したlandmarksを使うことで、crop translation／scaleがhead poseへ混入しないことをrecorded testで確認する。
- camera device、model pipeline、avatar generation変更時にexisting calibration／filter reset policyを維持する。
- existing monotonic clockを使い、`dt`を別epochから作らない。
- mock／recorded outcomeでcalibration→tracking→loss→reacquireをdeterministic testする。

**このsubtaskで行わないこと**

- UI内でcalibration mathを行わない。
- planar solverを別実装へ置換しない。
- fixed alphaの追加filterを作らない。
- control frameをavatarへ直接適用しない。

**完了条件**

- 実cameraでneutral calibrationを完了できる。
- detector crop位置が変化してもneutral faceのpose driftが許容範囲内である。
- head motionに応じたfiniteな`AvatarControlFrame`が生成される。
- face loss／returnでstate transitionがUIへ反映される。
- cancel／retry後にprocess再起動なしで再校正できる。
- no-faceをruntime failureとして誤表示しない。

**検証**

```powershell
cargo test -p vtuber-tracking --no-fail-fast
cargo test -p vtuber-app tracking_orchestration
cargo test -p vtuber-app tracking_crop_invariance
cargo clippy -p vtuber-tracking -p vtuber-app --all-targets -- -D warnings
```

### M1-08-015 rewrite leaf sequence (current)

#### M1-08-015-001: Record failure baseline and supersede the old design

状態: `DONE`

`AGENTS.md`、`DESIGN.md`、ADR-001をMediaPipe rewriteに整合させ、ADR-009を追加する。旧rate／pose range、Peppa normalization mismatch、98-point expression index mismatchをfailure baselineとして記録する。ADR-001の旧production decisionをsupersededとし、MediaPipe native exceptionを監査済みbinding経由に限定する。M1-08-016／017を`PENDING`へ戻し、018／019は`BLOCKED`、M1-09は`DEFERRED`のまま維持する。

Commit: `docs(mocap): supersede custom planar tracking design`

#### M1-08-015-002: MediaPipe dependency and standalone Windows gate

状態: `DONE`

Windows C922 gate passed on 2026-08-12: 28.085 result Hz, 1,487 face results,
162 no-face results, 478 landmarks, 52 blendshapes, one valid matrix, zero
contract failures, zero capture overwrites, and three Stop/Start cycles. The
task bundle hash matched and the native library loaded from the verified cache.

`mediapipe-rs`のexact revision、official task bundle、既存camera layer、worker-owned VIDEO modeを追加し、`mediapipe-face-smoke`でWindows gateを実施する。60秒測定、15 Hz以上、478 landmarks、52 blendshapes、one valid matrix、queue depth 1以下、Stop/Start 3回を証明する。旧backendへfallbackしない。

Commit: `feat(mocap): add MediaPipe Face Landmarker Windows gate`

#### M1-08-015-003: Add canonical face-tracking contracts

状態: `DONE`

`vtuber-core`へtyped `FaceTrackingOutcome`、face sample、camera transform、landmark、typed blendshape set、quality fieldsを追加し、fake backendとunit testを適合させる。`NoFace`とmalformed output errorを区別する。

Commit: `refactor(mocap): add canonical MediaPipe face sample contract`

#### M1-08-015-004: Replace the production worker backend

状態: `DONE`

MediaPipe VIDEO modeをexisting inference workerへ統合する。strict timestamp、stride-aware pixel conversion、latest-only output、typed error、worker内construct/drop、deterministic shutdownを満たし、Bevyへ依存しない。

`InferenceController::load_mediapipe`からworker内でCPU／VIDEO modeの`FaceLandmarker`を構築し、canonical `FaceTrackingOutcome`をcapacity-one slotへ発行する。task bundle hash、timestamp、pixel layout、478 landmarks、52 blendshapes、matrix、NoFaceをtyped contractとして検証する。2026-08-12のWindows C922 gateは29.011 Hz、1,728 face results、478 landmarks、52 blendshapes、one valid matrix、zero contract failures、zero capture overwrites、three Stop/Start cyclesを再確認した。これはnative binding／task gateの実機証拠であり、GUI／VRM／pose sign acceptanceではない。

Commit: `feat(mocap): replace production worker with MediaPipe Face Landmarker`

#### M1-08-015-005: Implement matrix conversion and guided sign proof

状態: `DONE`

column-major matrix validation、proper rotation extraction、relative transform、basis mapping、synthetic fixtures、`mediapipe-pose-probe`を実装した。MediaPipeのyawはimage-rightを維持し、pitchとrollはアプリケーションのchin-up／image-clockwise規約へ符号変換する。neutralはworker起動直後から収集し、camera frame未到達とNoFaceを診断値で分離した。

2026-08-12 Windows C922 guided physical proof passed with:

```powershell
cargo run -p xtask -- mediapipe-pose-probe --camera 0 --guided --json 2>&1
```

MediaPipe 0.10.35、native library `verified cache`、`signs_pass=true`。neutral 96 samples、image_right 90、image_left 89、chin_up 90、chin_down 89、image_clockwise 89、image_counter_clockwise 89。Observed semantic signs: yaw `+0.600952/-0.752999`, pitch `+0.237842/-0.299808`, roll `+0.572944/-0.560954` for the tested positive/negative directions. This proves matrix/relative-pose sign behavior only; GUI, VRM application, bounded performance, and macOS acceptance remain pending.

Commit: `feat(mocap): derive neutral-relative pose from MediaPipe transforms`

#### M1-08-015-006: Replace calibration with auto-neutral and instant Recenter

状態: `DONE`

30-frame collector gateをproduction pathから除去し、first-valid auto-neutral、300 ms／15 sampleのrobust recent window、one-sample fallback、instant Recenter、waiting-for-face、filter resetを実装する。expression/head motionをreject理由にしない。

Commit: `feat(mocap): replace blocking calibration with instant recenter`

#### M1-08-015-007: Add tracking hysteresis, SO(3) filter, and recovery

状態: `DONE`

3-hit acquire、5-miss/300 ms loss、hold/neutral return/reacquire、SO(3) biquad、limits/outlier quarantine、deterministic replayを実装する。

Commit: `feat(mocap): add stable head tracking and loss recovery`

#### M1-08-015-008: Map MediaPipe blendshapes to VRM control

状態: `DONE`

exact typed 52-category parser、blink、A/I/U、gaze、missing/duplicate diagnostics、capability fallbackを実装する。invalid Peppa expression mappingと`BasicExpressionFallback`のproduction reachabilityを削除する。

Commit: `feat(mocap): drive VRM expressions from MediaPipe blendshapes`

#### M1-08-015-009: Integrate app UI, diagnostics, and avatar bridge

状態: `DONE`

MediaPipe outcomeからtracking、`AvatarControlFrame`、generation-safe avatar bridge、real/synthetic source exclusivity、diagnostics UIまで接続する。

Commit: `feat(mocap): integrate MediaPipe tracking with desktop runtime`

#### M1-08-015-010: Remove the legacy production path

状態: `DONE`

UltraFace、PeppaPig、custom crop、planar pose、placeholder expression、old calibration collectorのdefault production reachabilityを削除する。research commandとして残す場合だけ理由を記録する。

`vtuber-inference`のlegacy detector/crop/ONNX stackはdefault featureから外し、`legacy-face-stack`を明示したresearch/evaluation commandだけが有効化する。desktop appはMediaPipe task bundleのみを起動し、旧manifest/catalogは歴史的replayとartifact検証のためにresearch moduleとして残す。

Commit: `refactor(mocap): remove legacy custom face pipeline`

#### M1-08-015-011: Windows functional and performance acceptance

状態: `DONE`

section 16 protocolをC922とapproved VRMで実施し、10/10 Recenter、physical signs、real VRM head/blink/mouth/gaze、15 Hz以上、capture-to-apply p95 180 ms以下、Stop/Start 3回を測定する。thresholdを下げて合格扱いにしない。

2026-08-12のlive release GUI runでC922 symbolic-link明示選択、VRM 1.0 import/Ready、auto-neutral、real preview、6方向head、両目blink、`aa` mouth、左右gaze、face loss/reacquire、3回のStop/Start、avatar replace/unload/reload、camera unplug/replug後のStop→Start復旧、約405 msでの再取得、capture-to-apply、clean shutdownを確認した。M1-08-019の既存1,800秒bounded runはrender 56.257–60.770 FPS、tracking 30–60 Hz、capture-to-apply p95 29.393–31.213 ms、capture slot overwrite 0、RSS 866.2–885.6 MiB、thread 125–138、crash/hang 0を記録した。

Commit: `test(mocap): record live Windows acceptance evidence`

#### M1-08-015-012: Final documentation and task state

状態: `DONE`

acceptance report、manifest、native/runtime provenance、license notices、task statesを更新する。全acceptanceが通った場合のみM1-08-015を`DONE`とし、016／017はnew backendのrevalidation state、018／019はblocked、M1-09はdeferredを維持する。

Acceptance report、MediaPipe task manifest、ADR-009、015-011/016/017/018/019の状態を更新した。その後のrepairと再受入でfunctional/performance gateが通過したため、親M1-08-015は`DONE`へ移行した。

Commit: `docs(mocap): record MediaPipe rewrite acceptance`


#### M1-08-016: 既存avatar bridgeをreal tracking sourceで閉じる

状態: `DONE`
備考: MediaPipe rewrite後のcanonical outcome、generation／stale frame排他、synthetic source排他、binding／expression／gazeを再検証する。旧real-source evidenceは新backendの完了証拠として扱わない。
依存: `M1-08-015`
親参照: DESIGN.md §15、§16、§20.1、ADR-004

**現状の扱い**

`crates/vtuber-app/src/avatar_bridge.rs`、`ActiveControlFrame`、generation tagging、head／neck apply、expression／gaze fallbackは既に実装されている。新しいVRM制御経路を作らず、real tracking sourceをexisting bridgeへ接続し、synthetic sourceとの排他と実機挙動を検証する。

**変更候補**

- `crates/vtuber-app/src/avatar_bridge.rs`
- `crates/vtuber-app/src/synthetic_tracking.rs`
- `crates/vtuber-avatar/src/plugin.rs`
- `crates/vtuber-avatar/src/unload.rs`
- `crates/vtuber-avatar/src/expression.rs`
- `crates/vtuber-avatar/src/gaze.rs`
- `crates/vtuber-avatar/tests/vertical_control.rs`
- `crates/vtuber-app/tests/avatar_bridge.rs`

**実装指示**

- existing tracking control slotのlatest `AvatarControlFrame`をcurrent `AvatarLifecycle` generationへtagし、`ActiveControlFrame`へpublishする。
- lifecycle `Ready`以外ではframeを適用せず、stale generation／bindingをdropしてmetric化する。
- real tracking sourceとdev synthetic sourceを同時有効にしない。production defaultはreal sourceとする。
- M1-05のhead／neck pose、M1-06のexpression／gaze systemが`VtuberAvatarPlugin`へ正しいschedule順で登録済みか監査し、不足登録だけを補う。
- one-frame expression commandを一回にcoalesceし、`bevy_vrm1::ModifyExpressions` pathを維持する。
- gazeはexisting expression／eye-bone fallbackを使い、`bevy_vrm1::LookAt`／`BodyTracking`をproduct pathへ導入しない。
- avatar replace／unload、camera stop、pipeline failure時にactive frame、binding、capability cacheをclearする。
- face lossではTrackingPipelineが生成したhold／neutral-return frameを適用し、last poseを永久freezeしない。
- existing synthetic modeを診断用controlとして残し、同じVRMでsynthetic success／real failureを切り分けられるようにする。
- integration testではold generation frame、replace、unload、missing expression、head-only avatarを確認する。

**このsubtaskで行わないこと**

- app crateからbone transformへ直接writeしない。
- 独自VRM loader／expression resolverを作らない。
- SpringBone／MToonへtracking値を混ぜない。
- detector／tracking failureをsynthetic sourceで自動fallbackして隠さない。

**完了条件**

- Web cameraの顔動作でactive VRMのhead／neckが動く。
- blink、mouth、gazeがmodel capabilityに応じて動く。
- model replace後は新avatarだけが動く。
- face lossでneutralへ戻り、face returnで再追従する。
- stale frame／bindingでpanicしない。
- synthetic modeとreal modeが明示的に排他である。

**検証**

```powershell
cargo test -p vtuber-avatar
cargo test -p vtuber-app avatar_bridge
cargo test -p vtuber-app synthetic_tracking
cargo clippy -p vtuber-avatar -p vtuber-app -p vtuber-desktop --all-targets -- -D warnings
cargo build -p vtuber-desktop --release
```

#### M1-08-017: 既存Diagnostics／error recovery／shutdownを実pipelineで監査する

状態: `DONE`
備考: MediaPipe backend identity、478/52/matrix contract diagnostics、worker exit／panic recovery、逆順shutdown、retry、GUI Diagnostics表示を再検証する。旧composite evidenceは新backendの完了証拠として扱わない。

MediaPipe task identity/hash、worker failure stage、no-face通常状態、reverse-order stop/join、retry、diagnostics contract displayをautomated gateで再確認した。GUIはC922起動時に30 Hz capture/inferenceとRunning stateを観測できたが、face motionが無かったため、functional avatar acceptanceは015-011/018に残す。
依存: `M1-08-016`
親参照: DESIGN.md §17、§20.2〜§20.4、§21、§24

**現状の扱い**

Diagnostics、error presenter、worker status、clean shutdownの基盤は既にある。このsubtaskではstubやsingle-model前提を除去し、detector／landmark composite pipelineの実stateを表示・回復できることを監査する。

**変更候補**

- `crates/vtuber-app/src/inference_runtime.rs`
- `crates/vtuber-app/src/tracking_runtime.rs`
- `crates/vtuber-app/src/diagnostics.rs`
- `crates/vtuber-app/src/error_presenter.rs`
- `crates/vtuber-app/src/orchestrator.rs`
- `crates/vtuber-app/src/ui/diagnostics.rs`
- `crates/vtuber-app/tests/runtime_recovery.rs`

**実装指示**

- `DiagnosticsSnapshot`をcapture、detector、crop、landmark、tracking、avatar applyの実metricsから更新し、stub値を残さない。
- 少なくとも次を表示する。
  - capture／detector／landmark／tracking rate
  - detector confidence、ROI state、no-face count
  - detector／crop／landmark／tracking／apply stage timing
  - input／output slot overwrite
  - pipeline IDと短いmodel hashes
  - worker stateとlast stable error code
- Startの遷移をIdle→Starting→Running、StopをRunning／Failed→Stopping→Idleとして、各workerの実stateと一致させる。
- Start途中failureでは、起動済みworkerをinference→captureの逆順に停止／joinし、partial runtimeを残さない。
- camera disconnect、detector model load、detector infer、landmark model load、landmark infer、malformed output、avatar load、worker panicをstable code付きrecoverable errorへmapする。
- `NoFace`／Searching／Lostは通常stateであり、error bannerを出さない。
- unexpected worker exitを定期監視し、UIへ通知する。
- Retryはerror stageに応じて必要なruntimeを新規構築し、closed slot／stopped controllerを不正再利用しない。
- app exit時にinference→captureの順でexplicit shutdownし、slot waiterをwakeしてjoinする。
- raw pixel、crop image、全landmark、full local pathをnormal logへ出さない。
- Start／Stop／Retry／panic／disconnectをmockで反復し、thread leak／hangを検知する。

**このsubtaskで行わないこと**

- performance tuning、settings永続化、release packagingを先行しない。
- errorを一種類のStringへ潰さない。
- Dropだけを唯一のshutdown pathにしない。
- no-faceをretry対象errorにしない。

**完了条件**

- GUI Diagnosticsがcomposite pipelineの実rate／state／timingを表示する。
- Start／Stop／Retryを複数回行ってthread leak／hangがない。
- detector／landmarkのどちらが失敗したかUIで判別できる。
- worker panic testがprocess crashではなくFailed stateとUI errorになる。
- app close時にcamera handle、detector、landmark runtime、workerが解放される。
- normal no-faceでerror bannerが出ない。

**検証**

```powershell
cargo test -p vtuber-app runtime_recovery
cargo test -p vtuber-camera -p vtuber-inference -p vtuber-tracking -p vtuber-avatar --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
```


#### M1-08-018: Windows functional／recovery acceptanceを実pipelineで実施する

状態: `DONE`
備考: release build、MSMF symbolic-linkで明示したC922実preview、approved VRM Ready／auto-neutral、yaw／pitch／roll各2方向、両目blink、mouth-open、左右gaze、capture-to-apply p50 29.90 ms／p95 48.02 ms、avatar apply 12,262／skip 0を確認した。同一processでStop／Start 3回後もC922 trackingへ復帰した。既存runのface loss／reacquire、avatar replace／unload／reload、camera unplug/replug後のStop→Start復旧、約405 ms再取得、clean shutdownと統合し、functional／recovery correctness blockerを閉じた。
依存: `M1-08-022`
親参照: DESIGN.md §6、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/acceptance/windows-m1.md`
- `docs/acceptance/artifacts/`
- `AI_AGENT_TASKS.md`の該当status

**実装指示**

- release build、commit SHA、binary hash、model hash、camera descriptor、driver等をreportへ固定する。
- GUI import、Ready、calibration、yaw／pitch／roll、blink、mouth、gazeをVRM matrixで実施する。
- face loss／return、Stop／Start、camera抜去／再接続（可能な場合）、avatar replace、unload、invalid model、model runtime failureを実施する。
- synthetic modeを診断用に一度実行し、avatar apply pathとcamera／inference pathの問題を切り分ける。
- capability limitationとcorrectness failureを区別する。
- FAILは再現手順、log、artifact、source候補を記録し、必要なら新repair subtaskを`M1-08-XXX`末尾に提案する。既存IDを変更しない。
- correctness blockerが一つでもあれば`BLOCKED`とし、M1-08-019へ進まない。

**このsubtaskで行わないこと**

- acceptance中に無関係なUI polish／refactorを行わない。
- process再起動を通常のrecovery成功に含めない。
- 未実施項目をPASSにしない。

**完了条件**

- GUIから選んだVRMが実camera trackingで動く。
- Start／Stop／replace／loss recoveryがprocess再起動なしで成立する。
- fatal render issue、panic、worker leakがない。
- functional／recovery項目がPASS／FAIL／NOT RUNで記録される。

**検証**

```powershell
cargo build -p vtuber-desktop --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
# 上記に加え、docs/acceptance/windows-m1.mdの手動protocolを実施する。
```

#### M1-08-019: latency、bounded実行、Windows final gateを閉じる

状態: `DONE`
備考: release appで10秒warm-up後、60秒cadence、0〜1,800秒の31点をCSVへbounded exportした。render 56.257–60.770 FPS、tracking 30–60 Hz、capture-to-apply p95 29.393–31.213 ms、capture slot overwrite 0、RSS 866.2–885.6 MiB、thread 125–138、全resource sample responding=True、crash/hang 0、Stop→Idle→clean shutdownを確認しWindows final gateをPASSした。MediaPipe Tasksの単一call内部にあるdetector／landmark個別cadenceはruntimeから露出されず0として記録するが、canonical inferenceは29–30 Hzでありcorrectness blockerではない。
依存: `M1-08-018`
親参照: DESIGN.md §6、§20.2〜§20.3、§21.5、§24、docs/PERFORMANCE_TEST_PLAN.md

**現状の扱い**

process-wide monotonic timing、latest-slot metrics、tracking／avatar apply timingの基盤は既に実装済みである。このsubtaskでは再実装せず、全stageが同じepochを使うことを監査し、実camera／release buildで測定する。correctnessを変える修正が必要になった場合は本subtaskを止め、repair leafを追加する。

**変更候補**

- `crates/vtuber-core/src/time.rsまたはtiming.rs`
- `crates/vtuber-camera/`
- `crates/vtuber-inference/`
- `crates/vtuber-tracking/`
- `crates/vtuber-avatar/`
- `crates/vtuber-app/src/diagnostics.rs`
- `tools/xtask/src/acceptance.rs`
- `docs/acceptance/windows-m1.md`
- `docs/acceptance/artifacts/`

**実装指示**

- capture、detector start／finish、crop、landmark start／finish、tracking produce、avatar applyがprocess-wide同一monotonic epochの`MonoTimeNs`を使うことをsourceとtestで監査する。
- frameごとのlocal `Instant::now().elapsed()`等、別epoch timestampが残っていれば同じclock abstractionへ統一する。
- source sequenceとtimestampを全stageで維持し、sequence mismatch／missing stageをmetric上で区別する。
- warm-up 10秒、measurement window 60秒以上、minimum sample count 300を固定する。
- render FPS、capture Hz、detector Hz、landmark Hz、tracking Hz、p50／p95 capture-to-apply、各stage p50／p95、overwrite、queue depthをbounded exportする。
- detector cadenceによりdetector Hzがlandmark Hzより低くても、landmark／tracking Hzが目標を満たすことを確認する。
- MVP基準を判定する。
  - render 30fps以上
  - tracking 15Hz以上
  - capture-to-apply p95 180ms以下
  - queue depth 1以下
- 基準未達でもcorrectnessが成立している場合、stage別blockerを数値でQ2-03へ送る。本subtask内で根拠のないpool／unsafe最適化を行わない。
- 固定model／cameraでwarm-up後に60秒以上、300 sample以上の測定窓を設け、latency、rates、queue depth、overwrite、worker statusをbounded exportする。人の連続待機は要求しない。
- 測定前後に短いface loss／returnとStop／Startを行い、app exit時に全worker／camera／model runtimeの終了を確認する。
- `docs/acceptance/windows-m1.md`をPASS／FAIL／CONDITIONAL／NOT RUNで完成させ、binary、detector、landmark、VRM、config、metrics artifactのhashを記録する。
- no correctness blockerかつ最低性能基準を満たした場合だけWindows Q2をunlockする。
- M1-09は引き続き`DEFERRED`とする。

**このsubtaskで行わないこと**

- 異なるclock domainを単純減算しない。
- 平均値だけでlatencyを判定しない。
- detector Hzをtracking Hzと誤表示しない。
- 未達性能を測定なしの仕様変更で正当化しない。
- 本格最適化をQ2-03より先に行わない。

**完了条件**

- capture-to-apply p50／p95が同一monotonic domainから算出される。
- detector／landmark／tracking各rateが区別される。
- MVP最低基準を数値で判定できる。
- 測定窓でprocess crashがなく、queue depthが1以下である。
- Stop／Start後にtrackingへ復帰し、終了時にthread leakがない。
- Windows gate判断とQ2進行可否がreportに明記される。
- `M1-09`を実行せずWindows Q2だけを開始できる依存状態になる。

**検証**

```powershell
cargo fmt --all -- --check
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- acceptance verify assets/models/manifest.toml
cargo build -p vtuber-desktop --release
# 上記に加え、warm-up後の60秒以上のmeasurementを実施しartifactを保存する。
```


#### M1-08-020: Live preview登録とavatar viewport framingを修復する

状態: `DONE`
依存: `M1-08-017`
親参照: DESIGN.md §19.2、§21.5、docs/acceptance/windows-m1.md

**実装指示**

- camera previewの再利用Imageをmain worldに保持し、`bevy_egui` user textureへ明示登録する。
- preview表示だけを変更し、inferenceへ渡す非mirror frameを変更しない。
- avatar Ready時にhead／hips boneのworld位置から上半身framingを計算し、generationごとに一度だけviewport cameraへ適用する。
- bone位置が非有限または不正な場合は既存固定cameraをfallbackとして維持する。
- 実camera／approved VRMでpreviewとhead表示を再確認する前に、functional gateをPASSへ変更しない。

**完了条件**

- preview image handleがegui texture IDへ解決され、動的Imageが継続更新可能である。
- synthetic bone配置でheadが画面上部、hipsが画面下部に入るframingを自動検証する。
- model replacement時に新generationでframingが再計算される。
- workspace test／clippyが成功する。

**検証**

```powershell
cargo fmt --all -- --check
cargo test -p vtuber-app -p vtuber-avatar --no-fail-fast
cargo clippy -p vtuber-app -p vtuber-avatar --all-targets -- -D warnings
cargo build -p vtuber-desktop --release
```

2026-08-12にpreviewのdynamic `Image`をmain/render両worldへ保持し、
`EguiUserTextures`へ明示登録する経路を実装した。viewport cameraはReady時の
head／hips world位置からgenerationごとに一度だけ上半身framingを計算する。
workspace test／clippyとrelease buildが成功し、release GUIでapproved
`inore-vrm1.vrm`の顔／上半身がviewportへ表示されることを確認した。
このGUI確認時はcamera列挙が`None`だったため、実camera previewとfunctional
motionは015-011／018の再受入で確認する。

#### M1-08-021: production rest-orientation cacheとreal pose applyを修復する

状態: `DONE`
依存: `M1-08-020`
親参照: DESIGN.md §15、§20.2〜§20.3、§21.5、ADR-004

**実装指示**

- humanoid binding成功時にavatar generationと一致する`RestOrientationCache`を構築し、cache挿入前にlifecycleを`Ready`へ遷移しない。
- `GlobalTransform`が未伝播ならbinding deadlineまでretryし、external model／component欠落でpanicしない。
- model replacement後は新generationのcacheだけを使用し、stale cacheを再利用しない。
- real sourceでpose apply sampleが増加し、capture-to-applyが数値化されることをrelease GUIで確認する。

**完了条件**

- binding統合testが`AvatarBinding`と同じgenerationの`RestOrientationCache`挿入を証明する。
- head `GlobalTransform`未生成時は`Binding`で待機し、生成後に`Ready`へ進む。
- workspace test／clippy／release buildが成功する。
- 実camera／approved VRMでhead motionとcapture-to-apply sampleを確認する。

**検証**

```powershell
cargo fmt --all -- --check
cargo test -p vtuber-avatar -p vtuber-app --no-fail-fast
cargo clippy -p vtuber-avatar -p vtuber-app --all-targets -- -D warnings
cargo build -p vtuber-desktop --release
```

2026-08-12にproduction binding成功時の`RestOrientationCache`構築を接続し、
generation一致のbinding／cacheを同時挿入してから`Ready`へ遷移するよう修正した。
ELECOM 2MP Webcamとapproved `inore-vrm1.vrm`のrelease GUI runでは、実preview
とhead、両目blink、mouth-open、eye gazeを直接確認した。Diagnosticsはavatar
apply 5,111 frame、skip 0、capture-to-apply p50 30.82 ms／p95 48.23 ms、
tracking 30.0 Hz、slot overwrite 0を示した。C922固有のfinal rerunは
M1-08-015-011／018に残す。

#### M1-08-022: Windows複数camera列挙／選択を修復する

状態: `DONE`
依存: `M1-08-021`
親参照: DESIGN.md §13.2、§17.4、§21.1、§21.5

**実装指示**

- C922とELECOM cameraが同時接続された環境で、MSMF backendの列挙結果が欠落する位置を実測して修正する。
- backend固有のstable identityを保持し、UI list indexだけをcamera identityとして永続化しない。
- setup UIへ列挙結果を全件渡し、選択したdescriptorとcapture workerが開く物理cameraを一致させる。
- enumeration失敗を空listへ黙って変換せず、既存typed error／user-facing error境界を維持する。

**完了条件**

- 2台以上のdescriptorが順序変更しても選択identityを保持する自動testがある。
- C922とELECOMの同時接続状態でsetup UIが両方を表示する。
- C922を明示選択してpreview frameがC922由来であることを実機確認する。
- workspace test／clippy／release buildが成功する。

**検証**

```powershell
cargo fmt --all -- --check
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p vtuber-desktop --release
```

2026-08-12にpinned `nokhwa`のMSMF列挙を直接probeし、C922
（VID_046D／PID_085C）とELECOM 2MP Webcam（VID_056E／PID_701E）の2台を
確認した。descriptorとworker openを列挙順indexからMSMF symbolic linkへ変更し、
起動時自動列挙とRefresh時の選択identity追従を追加した。C922を指定した5秒の
MediaPipe smokeはcapture 87 frame、face 79件、contract failure 0で完了した。
release GUIも手動Refreshなしで2台を表示し、C922を明示選択した実previewを表示した。


## M1-09: macOS vertical acceptance（保留）
状態: `DEFERRED`
実行単位: `M1-09-NNN`
重点参照: DESIGN.md §4.1、§13.4、§22.2、§24、ADR-007


依存: M1-08-019のWindows gate PASS

### 保留方針

- 現在のWindows開発中は全subtaskを`DEFERRED`とし、Windows agentへ委嘱しない。
- macOS実機と開発環境へ移った時点で`M1-09-001`から再開する。
- sourceのcross-platform性は維持するが、未実行のmacOS build／permission／performanceを成功と報告しない。
- M1-09未完でもWindows-only Q2は進めてよい。ただしmacOS固有・両OS比較のQ2 subtaskは開始しない。

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
- M1-08と同じ10秒warm-up＋60秒以上・300 sample以上のbounded測定を通す。
- Stop／Start後に復帰し、終了時に全workerをjoinできる。

---

# Quality 2 — 完成度向上

### 実行subtask

#### M1-09-001: minimal macOS `.app` bundle layoutを作る

状態: `DEFERRED`
依存: `M1-08`
親参照: DESIGN.md §4.1、§13.4、§22.2、§24、ADR-007

**変更候補**

- `tools/xtask/src/package_macos.rs`
- `packaging/macos/`


**実装指示**

- `Contents/MacOS`、`Contents/Resources`、`Info.plist`の最小layoutを生成する。
- binary、VRM fixture／model assets、licensesの配置をresource locator方針に合わせる。
- bundle ID、display name、versionを一箇所のmetadataから生成する。
- signing／notarizationはこの段階では必須化しない。


**このsubtaskで行わないこと**

- DMG／installerを作らない。
- absolute developer pathをbundleへ埋め込まない。


**完了条件**

- `xtask`で再現可能に`.app`が生成される。
- working directoryに依存せずresourcesを見つけられる。
- bundle外temporary filesを誤参照しない。


**検証**

```bash
cargo run -p xtask -- package-macos --help
cargo build -p vtuber-desktop --release
```

#### M1-09-002: `Info.plist`とresource locatorを完成させる

状態: `DEFERRED`
依存: `M1-09-001`
親参照: DESIGN.md §4.1、§13.4、§22.2、§24、ADR-007

**変更候補**

- `packaging/macos/Info.plist.template`
- `crates/vtuber-app/src/resources.rs`


**実装指示**

- `NSCameraUsageDescription`、bundle identifier、executable、version keysを設定する。
- runtimeでexecutable／bundle resource directoryを解決するcross-platform APIを用意する。
- development runとbundle runのresource rootを明確に分ける。
- missing resourceをtyped startup errorにする。


**このsubtaskで行わないこと**

- resource fallbackでsource treeを探索しない。
- permission説明を空にしない。


**完了条件**

- plist validatorまたは`plutil`でvalidである。
- `.app`外current directoryから起動してもassetsを見つける。
- camera permission文字列がbundleへ含まれる。


**検証**

```bash
plutil -lint <generated-app>/Contents/Info.plist
cargo test -p vtuber-app resource_locator
```

#### M1-09-003: macOS camera permission flowを`.app`で検証する

状態: `DEFERRED`
依存: `M1-09-002`
親参照: DESIGN.md §4.1、§13.4、§22.2、§24、ADR-007

**変更候補**

- `docs/acceptance/macos-m1.md`
- `macOS app bundle`


**実装指示**

- fresh permission状態または明示reset後に`.app`を起動する。
- permission prompt、grant、deny、再起動後statusを記録する。
- app自身のbundle identityにpermissionが付くことを確認する。
- deny後にUIがrecoverable errorを表示する。


**このsubtaskで行わないこと**

- ユーザー同意なしにpermission DBを自動変更しない。
- Terminalへのpermissionをapp permissionと誤認しない。


**完了条件**

- grant後にAVFoundation captureが動く。
- denyでpanic／hangしない。
- raw binary試験と`.app`試験を区別して記録する。


**検証**

```bash
tccutil reset Camera <bundle-id>（必要な場合。実行内容を記録）
手動permission protocol
```

#### M1-09-004: Apple Silicon release buildとbasic smokeを実施する

状態: `DEFERRED`
依存: `M1-09-003`
親参照: DESIGN.md §4.1、§13.4、§22.2、§24、ADR-007

**変更候補**

- `docs/acceptance/macos-m1.md`
- `generated `.app``


**実装指示**

- aarch64-apple-darwin release buildを生成し`.app`へ配置する。
- 起動、VRM load、MToon、SpringBone、camera list、Start／Stopを確認する。
- CPU／GPU／OS／build hashを記録する。
- Intel MacはTier 2のcompile／smoke可否だけを別欄にする。


**このsubtaskで行わないこと**

- Universal binaryをこのtaskで必須化しない。
- 未所有Intel Macの成功を推測しない。


**完了条件**

- Apple Siliconでappが起動する。
- AVFoundation backendが選択される。
- MToon／SpringBoneに致命的描画差がない。


**検証**

```bash
cargo build -p vtuber-desktop --release --target aarch64-apple-darwin
cargo run -p xtask -- package-macos
```

#### M1-09-005: M1-08と同じfunctional／recovery protocolをmacOSで実施する

状態: `DEFERRED`
依存: `M1-09-004`
親参照: DESIGN.md §4.1、§13.4、§22.2、§24、ADR-007

**変更候補**

- `docs/acceptance/macos-m1.md`
- `run logs`


**実装指示**

- 同じVRM matrix、calibration、pose、blink、mouth、gazeを実施する。
- face loss、camera stop／restart、avatar replaceを同じ順序で実施する。
- platform差がある場合はprotocolを変えず差分として記録する。
- camera permission状態を各run headerへ含める。


**このsubtaskで行わないこと**

- Mac向けに別機能仕様を作らない。
- unsupportedを無断fallbackで隠さない。


**完了条件**

- Windows reportと同じfieldで比較できる。
- functional recoveryがprocess restartなしで成立する。
- model compatibility結果が更新される。


**検証**

```bash
手動protocol。docs/acceptance/macos-m1.mdへ結果記録。
```

#### M1-09-006: macOS latency／rateとbounded実行を検証する

状態: `DEFERRED`
依存: `M1-09-005`
親参照: DESIGN.md §4.1、§13.4、§22.2、§24、ADR-007

**変更候補**

- `docs/acceptance/macos-m1.md`
- `metrics artifact`


**実装指示**

- M1-08と同じ10秒warm-up、60秒以上・300 sample以上の測定窓、metric計算を使う。測定中に人の連続操作は要求しない。
- render FPS、tracking Hz、p95 latency、queue depth、overwrite、worker statusを記録する。
- 測定前後にface loss／returnとStop／Startを各1回行い、permission／sleep等の外乱があれば記録する。
- 終了時にclean Stop／app exitと全workerのjoinを確認する。


**このsubtaskで行わないこと**

- Windows結果を流用しない。
- RSSの短期変動だけを合否根拠にしない。


**完了条件**

- render 30fps以上、tracking 15Hz以上、capture-to-apply p95 180ms以下を数値判定できる。
- queue depthが1以下で、overwriteを記録できる。
- Stop／Start後にtrackingへ復帰し、終了時にworker threadが残らない。
- 測定窓でprocess crashなし。


**検証**

```bash
実機run（10秒warm-up＋60秒以上、300 sample以上）＋Stop／Start＋metrics export
```

#### M1-09-007: Windows／macOS差分を分類する

状態: `DEFERRED`
依存: `M1-09-006`
親参照: DESIGN.md §4.1、§13.4、§22.2、§24、ADR-007

**変更候補**

- `docs/acceptance/platform-comparison-m1.md`


**実装指示**

- camera permission、capture format、latency、render、MToon、SpringBone、errorsを比較表にする。
- 差分をexpected platform behavior／bug／performance gapへ分類する。
- Mac固有fixが共通architectureを壊さないか確認する。
- Q2へ送る改善項目をparent task IDへ紐付ける。


**このsubtaskで行わないこと**

- OSごとにcore pipelineをforkしない。
- 差を主観だけで評価しない。


**完了条件**

- 両platformの同一metricが比較できる。
- 致命的差が明示される。
- platform-specific workaroundが局所化される。


**検証**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

#### M1-09-008: macOS vertical acceptance reportとMilestone 1 gateを完成させる

状態: `DEFERRED`
依存: `M1-09-007`
親参照: DESIGN.md §4.1、§13.4、§22.2、§24、ADR-007

**変更候補**

- `docs/acceptance/macos-m1.md`
- `docs/acceptance/platform-comparison-m1.md`
- `artifact manifest`


**実装指示**

- permission、functional、recovery、latency、bounded実行をPASS／FAIL／NOT RUNで統合する。
- `.app`、binary、Info.plist、model、config、metricsのhashを記録する。
- M1-09受入条件とMilestone 1全体のacceptanceを判定する。
- failがあればrepair提案を既存parent IDへ紐付け、番号を変更しない。


**このsubtaskで行わないこと**

- Quality 2を同時実装しない。
- 未実行項目をpassにしない。


**完了条件**

- M1-09の全受入条件を満たすか明示FAILになる。
- 同じcompatibility report formatを使用する。
- Milestone 1完了判断に根拠がある。


**検証**

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Q2-01: 5母音と表情品質
状態: `PENDING`
実行単位: `Q2-01-NNN`
重点参照: DESIGN.md §14、§15.4、§16.8、§25 R5


依存: M1-08-019（Windows実装・評価）／M1-09（macOS検証追補）


### platform gate

- WindowsではM1-08-019 PASS後に全subtaskを実装・評価できる。
- macOS結果はM1-09後に同じreportへ追補し、Windows完了を巻き戻さない。

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

### 実行subtask

#### Q2-01-001: blendshape output contractをmanifestへ追加する

状態: `PENDING`
依存: `M1-08-019`
親参照: DESIGN.md §14、§15.4、§16.8、§25 R5

**変更候補**

- `assets/models/manifest.toml`
- `crates/vtuber-inference/src/model_manifest.rs`


**実装指示**

- backendが提供するblendshape tensorのname／index、shape、label order、value rangeを記録する。
- label listをartifact version／hashへ束縛する。
- 未提供backendを明示できるoptional contractにする。
- source codeにlabel indexをhard-codeしない。


**このsubtaskで行わないこと**

- 別modelのlabel orderを流用しない。
- 5母音を推測indexで読むことを禁止する。


**完了条件**

- manifestと実outputのlabel数が一致する。
- hash変更時にcontract mismatchを検出する。
- unsupported backendでもMVP decodeが維持される。


**検証**

```bash
cargo test -p vtuber-inference blendshape_manifest
cargo run -p xtask -- model verify
```

#### Q2-01-002: blendshape tensor decodeを実装する

状態: `PENDING`
依存: `Q2-01-001`
親参照: DESIGN.md §14、§15.4、§16.8、§25 R5

**変更候補**

- `crates/vtuber-inference/src/decode/blendshape.rs`
- `crates/vtuber-inference/tests/blendshape.rs`


**実装指示**

- manifest label mappingからname→raw coefficientを構築する。
- shape、dtype、NaN／Inf、範囲を検証する。
- 必要labelだけをcompact domain structへ変換し、raw全vectorを長期保持しない。
- missing labelをNoneとして扱う。


**このsubtaskで行わないこと**

- tracking normalizationを同時実装しない。
- unknown labelをpanicにしない。


**完了条件**

- golden outputから期待label値を取得できる。
- index mismatchがtyped errorになる。
- MVP fallback pathが壊れない。


**検証**

```bash
cargo test -p vtuber-inference blendshape_decode
```

#### Q2-01-003: neutral baseline calibrationをblendshapeへ拡張する

状態: `PENDING`
依存: `Q2-01-002`
親参照: DESIGN.md §14、§15.4、§16.8、§25 R5

**変更候補**

- `crates/vtuber-tracking/src/calibration/neutral.rs`
- `crates/vtuber-tracking/tests/blendshape_calibration.rs`


**実装指示**

- 5母音、blink、emotion／gaze候補のneutral baseline統計をprofileへ追加する。
- channelごとのnoise floorとavailabilityを算出する。
- model hash／label contract versionが違うprofileを拒否する。
- MVP landmark fallback baselineを残す。


**このsubtaskで行わないこと**

- 全channelを必須にしない。
- profile schemaを無versionで変更しない。


**完了条件**

- neutralでfalse mouth／blink activationが抑えられる。
- missing channelでprofile creationがpanicしない。
- fallback profileとの互換方針が明示される。


**検証**

```bash
cargo test -p vtuber-tracking blendshape_calibration
```

#### Q2-01-004: 5母音normalize／competition policyを実装する

状態: `PENDING`
依存: `Q2-01-003`
親参照: DESIGN.md §14、§15.4、§16.8、§25 R5

**変更候補**

- `crates/vtuber-tracking/src/expression/vowels.rs`
- `crates/vtuber-tracking/tests/vowels.rs`


**実装指示**

- aa／ih／ou／ee／ohをbaseline補正し0..1へnormalizeする。
- simultaneous activation時のsum clamp／softmax／max blend方針を一つ選び文書化する。
- mouth opennessとvowel係数の関係をgateする。
- missing vowelは0またはNoneとして扱い、available channelsだけでnormalizeする。


**このsubtaskで行わないこと**

- model indexを直接参照しない。
- 音声認識を追加しない。


**完了条件**

- 全weightがfiniteかつrange内である。
- neutral、単一母音、複数母音fixtureが期待結果を返す。
- weight合計policyがtestされる。


**検証**

```bash
cargo test -p vtuber-tracking vowel_normalization
```

#### Q2-01-005: coarticulation smoothingを実装する

状態: `PENDING`
依存: `Q2-01-004`
親参照: DESIGN.md §14、§15.4、§16.8、§25 R5

**変更候補**

- `crates/vtuber-tracking/src/filter/vowels.rs`
- `crates/vtuber-tracking/tests/vowel_filter.rs`


**実装指示**

- 母音ごとのattack／releaseまたはvector-space smoothingを実装する。
- 短いtransitionで全母音が一瞬0になるgapを抑える。
- timestamp基準でframe rate非依存にする。
- reset／lost face時のneutral returnを既存state machineへ接続する。


**このsubtaskで行わないこと**

- Research 3の複数filter比較を先行しない。
- 固定frame alphaだけを使わない。


**完了条件**

- step inputで定義rise time内に追従する。
- 母音切替時に大きなovershootがない。
- lost時にneutralへ戻る。


**検証**

```bash
cargo test -p vtuber-tracking vowel_filter
```

#### Q2-01-006: blink／gaze品質をblendshapeで改善する

状態: `PENDING`
依存: `Q2-01-005`
親参照: DESIGN.md §14、§15.4、§16.8、§25 R5

**変更候補**

- `crates/vtuber-tracking/src/expression/blink.rs`
- `crates/vtuber-tracking/src/expression/gaze.rs`


**実装指示**

- blendshape blink／eye look channelが信頼できる場合にlandmark fallbackより優先する。
- 左右blink asymmetry、noise floor、confidence gatingを適用する。
- gaze raw channelとhead poseの二重反映を避けるpolicyを実装する。
- channel unavailable時はMVP pathへ戻す。


**このsubtaskで行わないこと**

- bevy_vrm1 LookAtを再導入しない。
- quality改善でcapability fallbackを削除しない。


**完了条件**

- backendあり／なし双方のtestが通る。
- neutral jitterがMVPより悪化しない。
- 片目channel欠落でpanicしない。


**検証**

```bash
cargo test -p vtuber-tracking blendshape_blink_gaze
```

#### Q2-01-007: Expression conflict／override integration testを追加する

状態: `PENDING`
依存: `Q2-01-006`
親参照: DESIGN.md §14、§15.4、§16.8、§25 R5

**変更候補**

- `crates/vtuber-avatar/tests/expression_conflict.rs`
- `crates/vtuber-tracking/tests/`


**実装指示**

- mouth＋blink＋gaze＋emotion係数が同一frameに存在するfixtureを作る。
- `ModifyExpressions`一回で送信され、bevy_vrm1 override設定に委譲されることを確認する。
- binary expression、block／blend overrideの対象modelまたはsynthetic setupをtestする。
- unsupported expressionは送信しない。


**このsubtaskで行わないこと**

- 独自VRM expression resolverを作らない。
- material bindsを勝手に実装しない。


**完了条件**

- 同一frame event数が一つ。
- mouth／blink conflictがNaNやweight累積を起こさない。
- MVP fallback testが維持される。


**検証**

```bash
cargo test -p vtuber-avatar expression_conflict
```

#### Q2-01-008: UI calibration値とQ2-01総合検証を追加する

状態: `PENDING`
依存: `Q2-01-007`
親参照: DESIGN.md §14、§15.4、§16.8、§25 R5

**変更候補**

- `crates/vtuber-app/src/ui/calibration.rs`
- `docs/quality/expression-quality.md`


**実装指示**

- availability、baseline、noise floor、vowel activationをDiagnostics／calibration UIへ表示する。
- 編集可能値は必要最小限にし、invalid rangeをdomain validationで拒否する。
- 同じrecorded streamでMVPとblendshape quality pathを比較する。
- Q2-01受入条件とfallback維持をreportする。


**このsubtaskで行わないこと**

- UI preset editorを作らない。
- 実測なしで品質向上を断言しない。


**完了条件**

- modelごとのindexがUI／codeへhard-codeされない。
- unsupported backendでMVPが継続する。
- 5母音／blink／gazeのtestとmanual resultが揃う。


**検証**

```bash
cargo test -p vtuber-inference -p vtuber-tracking -p vtuber-avatar -p vtuber-app
cargo clippy --workspace --all-targets -- -D warnings
```

## Q2-02: settings、recent avatar、import UX
状態: `PENDING`
実行単位: `Q2-02-NNN`
重点参照: DESIGN.md §17.4、§18.3〜§18.5


依存: M1-08-019（Windows実装・評価）／M1-09（macOS検証追補）


### platform gate

- Windows settings／recent avatar／import UXはM1-08-019後に開始可能。
- macOS AppData、dialog、camera identityの実機検証はM1-09後に追補する。

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

### 実行subtask

#### Q2-02-001: versioned config schemaを定義する

状態: `PENDING`
依存: `M1-08-019`
親参照: DESIGN.md §17.4、§18.3〜§18.5

**変更候補**

- `crates/vtuber-app/src/config/schema.rs`


**実装指示**

- `ConfigV1`とtop-level schema versionを定義する。
- camera preference、calibration references、recent avatar IDs、UI settingsを必要最小限含める。
- pathはmanaged asset ID／hashを優先し、raw absolute pathを必須にしない。
- default値と値域validationを実装する。


**このsubtaskで行わないこと**

- Bevy Reflectだけを永続formatにしない。
- secret／camera frameを保存しない。


**完了条件**

- empty／partial configからvalid defaultsを作れる。
- unknown future versionをsilent parseしない。
- camera index単独を保存しない。


**検証**

```bash
cargo test -p vtuber-app config_schema
```

#### Q2-02-002: config loadとvalidation fallbackを実装する

状態: `PENDING`
依存: `Q2-02-001`
親参照: DESIGN.md §17.4、§18.3〜§18.5

**変更候補**

- `crates/vtuber-app/src/config/load.rs`


**実装指示**

- AppData内`config.toml`を読み、parse、version dispatch、validationを行う。
- missing fileはdefault、invalid fileはtyped warningとsafe defaultへfallbackする。
- I/O errorとparse errorを区別する。
- load時にfilesystem外のside effectを起こさない。


**このsubtaskで行わないこと**

- invalid configを無言で上書きしない。
- UI resourceから直接file readしない。


**完了条件**

- missing／valid／invalid／future version testが通る。
- broken configでappが起動できる。
- invalid値がdomainへ流れない。


**検証**

```bash
cargo test -p vtuber-app config_load
```

#### Q2-02-003: atomic config writeとbroken backupを実装する

状態: `PENDING`
依存: `Q2-02-002`
親参照: DESIGN.md §17.4、§18.3〜§18.5

**変更候補**

- `crates/vtuber-app/src/config/save.rs`


**実装指示**

- temp write、flush、atomic renameで保存する。
- 既存invalid configをtimestampまたはhash付き`.broken`へ一度backupする。
- write coalescing／debounceをapp layerで行い、毎frame保存しない。
- permission／disk fullをUI向けtyped errorにする。


**このsubtaskで行わないこと**

- bundle内へ書かない。
- synchronous saveをrender loop毎に呼ばない。


**完了条件**

- partial writeで本configを壊さない。
- broken backupが無限増殖しないpolicyがある。
- save failure後もin-memory configが使える。


**検証**

```bash
cargo test -p vtuber-app config_save
```

#### Q2-02-004: config migration frameworkとV1 migration testを作る

状態: `PENDING`
依存: `Q2-02-003`
親参照: DESIGN.md §17.4、§18.3〜§18.5

**変更候補**

- `crates/vtuber-app/src/config/migrate.rs`
- `crates/vtuber-app/tests/config_migration.rs`


**実装指示**

- raw versionを読み、隣接versionのpure migration関数を連鎖させる構造を作る。
- 現時点ではV1 identityまたはlegacy placeholderからV1への最小testを置く。
- migration後に通常validationを必ず通す。
- unknown newer versionをdowngradeしない。


**このsubtaskで行わないこと**

- field renameをad hocにload関数へ散らさない。
- future versionを破壊的に上書きしない。


**完了条件**

- migrationがdeterministicである。
- invalid legacy dataがtyped errorになる。
- schema migration testが将来追加しやすい。


**検証**

```bash
cargo test -p vtuber-app config_migration
```

#### Q2-02-005: per-camera calibration identityを保存／解決する

状態: `PENDING`
依存: `Q2-02-004`
親参照: DESIGN.md §17.4、§18.3〜§18.5

**変更候補**

- `crates/vtuber-app/src/config/camera.rs`
- `crates/vtuber-tracking/src/calibration/profile.rs`


**実装指示**

- backend、device name／metadata、selected format、model hashをcamera profile keyにする。
- 再列挙後にbest matchし、曖昧ならuser selectionを要求する。
- profile schema versionとquality metadataを保存する。
- camera index変化だけで別profile／誤profileを選ばない。


**このsubtaskで行わないこと**

- hardware serialを必須としない。
- 一つのcalibrationを全cameraへ共有しない。


**完了条件**

- index入替fixtureでも同device profileを解決できる。
- ambiguous matchをsilent決定しない。
- model hash違いcalibrationを使わない。


**検証**

```bash
cargo test -p vtuber-app camera_profile_resolution
cargo test -p vtuber-tracking calibration_profile
```

#### Q2-02-006: recent avatar listとmissing cleanupを実装する

状態: `PENDING`
依存: `Q2-02-005`
親参照: DESIGN.md §17.4、§18.3〜§18.5

**変更候補**

- `crates/vtuber-app/src/recent_avatar.rs`
- `crates/vtuber-app/src/config/schema.rs`


**実装指示**

- managed asset hash、display name、last used timestampをbounded listで保存する。
- entry利用時にmodel／manifest存在とpreflight schemaを確認する。
- missing／corrupt entryをUIへ示し、明示cleanupまたは自動prune policyを実装する。
- list上限とstable orderingを定義する。


**このsubtaskで行わないこと**

- import cache全体を勝手に削除しない。
- recent listをfilesystem scanだけで再構築しない。


**完了条件**

- listがunboundedに増えない。
- missing fileでstartupが失敗しない。
- raw source pathなしでmanaged modelを再loadできる。


**検証**

```bash
cargo test -p vtuber-app recent_avatar
```

#### Q2-02-007: optional native file dialogをadapterとして追加する

状態: `PENDING`
依存: `Q2-02-006`
親参照: DESIGN.md §17.4、§18.3〜§18.5

**変更候補**

- `crates/vtuber-app/src/file_dialog.rs`
- `apps/desktop/Cargo.toml`


**実装指示**

- 依存を追加する場合はWindows／macOS対応とlicenseを確認する。
- dialog結果を単なるpathとしてImportAvatar actionへ渡し、import処理をdialog adapterへ入れない。
- cancelをerrorにしない。
- file-drop pathを引き続き利用可能にする。


**このsubtaskで行わないこと**

- dialog dependencyにcamera／web runtimeを持ち込まない。
- absolute pathをAssetServerへ直接渡さない。


**完了条件**

- dialogなしbuildまたはfallbackが可能である。
- cancel後にUI stateが壊れない。
- 選択pathがG0-03 validationを必ず通る。


**検証**

```bash
cargo check -p vtuber-desktop
cargo test -p vtuber-app file_dialog
```

#### Q2-02-008: settings／import UXを総合検証する

状態: `PENDING`
依存: `Q2-02-007`
親参照: DESIGN.md §17.4、§18.3〜§18.5

**変更候補**

- `crates/vtuber-app/`
- `docs/quality/settings.md`


**実装指示**

- config load／save／broken backup／migration、camera profile、recent avatar、dialog／dropを通す。
- AppData以外へwriteしていないかtemp-root testで確認する。
- broken config、missing avatar、camera index変更をreplayする。
- Q2-02受入条件をreportする。


**このsubtaskで行わないこと**

- cloud syncを追加しない。
- configを手動編集必須にしない。


**完了条件**

- user dataをbundleへ書かない。
- schema migration testが通る。
- error後にUIから回復できる。


**検証**

```bash
cargo test -p vtuber-app --no-fail-fast
cargo clippy -p vtuber-app -p vtuber-desktop --all-targets -- -D warnings
```

## Q2-03: performance tuningとlatency budget
状態: `PENDING`
実行単位: `Q2-03-NNN`
重点参照: DESIGN.md §6.1、§14.5〜§14.6、§20.2〜§20.3、§21.5


依存: M1-08-019（Windows performance）／M1-09（macOS比較）


### platform gate

- `Q2-03-001`〜`Q2-03-007`をWindowsで先行できる。
- `Q2-03-008`は両OS比較taskのためM1-09完了まで`DEFERRED`とする。

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

### 実行subtask

#### Q2-03-001: fixed-size latency histogramを実装する

状態: `PENDING`
依存: `M1-08-019`
親参照: DESIGN.md §6.1、§14.5〜§14.6、§20.2〜§20.3、§21.5

**変更候補**

- `crates/vtuber-core/src/metrics/histogram.rs`


**実装指示**

- bounded bucketまたはring-backed histogramでduration sampleを集計する。
- p50／p95／count／min／maxを取得できる。
- sample window／reset semanticsを明示する。
- allocation-free update pathを目標にする。


**このsubtaskで行わないこと**

- 全durationをVecへ保存しない。
- 浮動小数percentileの曖昧仕様を放置しない。


**完了条件**

- known sample setでpercentileが期待値になる。
- sample数が増えてもmemoryが一定である。
- empty histogramを安全に扱う。


**検証**

```bash
cargo test -p vtuber-core latency_histogram
```

#### Q2-03-002: capture→apply全stage instrumentationを統一する

状態: `PENDING`
依存: `Q2-03-001`
親参照: DESIGN.md §6.1、§14.5〜§14.6、§20.2〜§20.3、§21.5

**変更候補**

- `crates/vtuber-core/src/timing.rs`
- `crates/vtuber-camera/`
- `crates/vtuber-inference/`
- `crates/vtuber-tracking/`
- `crates/vtuber-avatar/`


**実装指示**

- capture、decode、inference start/end、tracking output、avatar applyのtimestamp fieldsを一つのtrace ID／sequenceへ紐付ける。
- 同一monotonic clock abstractionを共有する。
- stage missing時のmetricを無効値として区別する。
- production logへraw per-frame traceを常時出さない。


**このsubtaskで行わないこと**

- SystemTimeを混ぜない。
- string log parseをmetric sourceにしない。


**完了条件**

- capture-to-applyを正しいtimestampから算出できる。
- sequence mismatchを検出する。
- instrumentationなしfeatureとの差が不要ならfeature分岐を増やさない。


**検証**

```bash
cargo test -p vtuber-core timing
cargo test -p vtuber-app diagnostics_snapshot
```

#### Q2-03-003: recorded replay benchmark harnessを作る

状態: `PENDING`
依存: `Q2-03-002`
親参照: DESIGN.md §6.1、§14.5〜§14.6、§20.2〜§20.3、§21.5

**変更候補**

- `tools/xtask/src/bench.rs`
- `crates/vtuber-inference/benches/またはtests/perf_replay.rs`


**実装指示**

- privacy-safe fixed RGB frames／synthetic observationsを一定cadenceでreplayする。
- preprocess、inference、tracking、avatar mathを個別／統合で計測できる。
- warm-up、iteration、thread count、CPU affinity非依存条件を記録する。
- benchmark結果をJSON／CSVへ出力する。


**このsubtaskで行わないこと**

- criterion等を必要なく追加しない。
- 個人camera recordingをbundleしない。


**完了条件**

- 同じartifact／commandで再実行できる。
- hardware cameraなしで主要stageを測定できる。
- benchmarkがunit test pass条件を不安定にしない。


**検証**

```bash
cargo run -p xtask -- bench --help
```

#### Q2-03-004: allocation profileを取得しhot allocationを特定する

状態: `PENDING`
依存: `Q2-03-003`
親参照: DESIGN.md §6.1、§14.5〜§14.6、§20.2〜§20.3、§21.5

**変更候補**

- `docs/performance/allocation-baseline.md`
- `profiling scripts／commands（必要最小限）`


**実装指示**

- 現在はWindowsで利用可能なprofilerまたはallocator countersを使い、capture、preprocess、preview、expression pathを計測する。macOS allocation profileはQ2-03-008で追補する。
- steady-state frameあたりallocation件数／bytesを記録する。
- 上位allocation siteをsource location付きで列挙する。
- 修正前baselineを保存してからcode変更する。


**このsubtaskで行わないこと**

- このsubtaskで大規模rewriteしない。
- unsafe poolを先に追加しない。


**完了条件**

- 少なくとも上位3 allocation sourceが特定される。
- 測定commandとenvironmentが記録される。
- 推測だけで最適化対象を決めない。


**検証**

```bash
platform profiler command。docsへ正確に記録。
```

#### Q2-03-005: frame／tensor／preview buffer reuseを最適化する

状態: `PENDING`
依存: `Q2-03-004`
親参照: DESIGN.md §6.1、§14.5〜§14.6、§20.2〜§20.3、§21.5

**変更候補**

- `crates/vtuber-camera/`
- `crates/vtuber-inference/`
- `crates/vtuber-app/src/preview.rs`


**実装指示**

- allocation baselineで確認したhot pathだけを修正する。
- RGB frame poolまたはreuse ownershipを導入する場合、LatestSlot overwrite時の返却を明確にする。
- preprocess tensorとpreview image capacityをsteady-state reuseする。
- before／after allocationとlatencyを同じharnessで測る。


**このsubtaskで行わないこと**

- 根拠なくlock-free poolを作らない。
- unsafeを追加しない。


**完了条件**

- steady-state allocationが数値で減る。
- buffer alias／use-after-returnがtestで防がれる。
- correctness goldenが変わらない。


**検証**

```bash
cargo test -p vtuber-camera -p vtuber-inference -p vtuber-app
cargo run -p xtask -- bench --help
```

#### Q2-03-006: preview throttleとdetector cadenceを計測調整する

状態: `PENDING`
依存: `Q2-03-005`
親参照: DESIGN.md §6.1、§14.5〜§14.6、§20.2〜§20.3、§21.5

**変更候補**

- `crates/vtuber-app/src/preview.rs`
- `crates/vtuber-inference/src/pipeline.rs`
- `docs/performance/tuning.md`


**実装指示**

- preview update上限とdetector再実行cadenceを設定候補ごとにbenchmarkする。
- tracking loss率、latency、CPU／GPU usageのtrade-offを記録する。
- Windowsでdefault候補を固定し、platform固有値をhard-codeせず設定可能に保つ。macOS妥当性はQ2-03-008で検証する。
- 極端なdeviceでfallback可能な範囲を設定する。


**このsubtaskで行わないこと**

- FPSだけを見てqualityを犠牲にしない。
- OSごとに無関係な分岐を増やさない。


**完了条件**

- 採用defaultに数値根拠がある。
- detector cadence変更でrecoveryが著しく悪化しない。
- preview OFFでtracking性能が維持される。


**検証**

```bash
cargo run -p xtask -- bench --help
manual loss/reacquire protocol
```

#### Q2-03-007: tract thread behaviorを計測し設定を固定する

状態: `PENDING`
依存: `Q2-03-006`
親参照: DESIGN.md §6.1、§14.5〜§14.6、§20.2〜§20.3、§21.5

**変更候補**

- `crates/vtuber-inference/src/backend/tract.rs`
- `docs/performance/tract-threading.md`


**実装指示**

- runtime／operatorが使用するthread数と環境変数影響をまずWindowsで測る。macOS測定はQ2-03-008へ送る。
- 1 worker thread＋tract内部parallelismのoversubscriptionを確認する。
- 候補設定ごとにlatency p50／p95、throughput、CPU usageを記録する。
- 安定した設定だけを明示的defaultへする。


**このsubtaskで行わないこと**

- 未確認環境変数を強制しない。
- 独自thread poolを重ねない。


**完了条件**

- thread設定の採用理由が数値化される。
- model output goldenが変わらない。
- machine core数依存の暴走がない。


**検証**

```bash
cargo run -p xtask -- bench --help
cargo test -p vtuber-inference golden
```

#### Q2-03-008: Windows／macOS最適化reportと目標判定を完成させる

状態: `DEFERRED`
依存: `Q2-03-007、M1-09`
親参照: DESIGN.md §6.1、§14.5〜§14.6、§20.2〜§20.3、§21.5

**変更候補**

- `docs/performance/optimization-report.md`
- `docs/adr/ADR-001-*.mdまたは性能ADR amendment`


**実装指示**

- M1-09完了後、Windowsで確定済みのbefore／after結果へmacOSのstage timing、allocation、tracking Hz、p95を追補して両OS表にする。
- Windows 25Hz／110ms、macOS 25Hz／120msのtargetを判定する。
- 未達ならstage別blockerと測定誤差を記載する。
- 必要な場合だけADR-001または性能目標をamendする。


**このsubtaskで行わないこと**

- 目標未達を測定なしで仕様変更しない。
- 平均値だけでp95目標を判定しない。


**完了条件**

- Q2-03の全受入条件を満たす。
- profile根拠のないpool／unsafeがない。
- 未達理由が数値で示される。


**検証**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- bench --help
```

## Q2-04: release packaging
状態: `PENDING`
実行単位: `Q2-04-NNN`
重点参照: DESIGN.md §22、ADR-007


依存: M1-08-019（Windows package）／M1-09（macOS package）


### platform gate

- `Q2-04-001`〜`Q2-04-003`でWindows portable packageを先行できる。
- `Q2-04-004`〜`Q2-04-008`はmacOSまたは両OS統合を含むためM1-09完了まで`DEFERRED`とする。

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

### 実行subtask

#### Q2-04-001: release input／license manifestを定義する

状態: `PENDING`
依存: `M1-08-019`
親参照: DESIGN.md §22、ADR-007

**変更候補**

- `packaging/manifest.toml`
- `LICENSES/`
- `tools/xtask/src/package_common.rs`


**実装指示**

- binary、models、VRM samples、shaders、licenses、config defaultsをpackage inputとして列挙する。
- 各inputにsource path、destination、required、hash policyを持たせる。
- third-party license一覧をCargo metadataとmodel manifestsから生成する。
- private／development assetをrelease manifestへ含めない。


**このsubtaskで行わないこと**

- source tree全体をcopyしない。
- 権利不明assetを同梱しない。


**完了条件**

- missing required inputでpackageが失敗する。
- license sourceが追跡できる。
- release内容がmanifest外のglobに依存しない。


**検証**

```bash
cargo test -p xtask package_manifest
cargo deny check licenses
```

#### Q2-04-002: `xtask package-windows`を実装する

状態: `PENDING`
依存: `Q2-04-001`
親参照: DESIGN.md §22、ADR-007

**変更候補**

- `tools/xtask/src/package_windows.rs`
- `packaging/windows/`


**実装指示**

- release build、staging directory作成、manifest input copyを一commandで行う。
- exe version、commit SHA、model hashをmetadata fileへ出す。
- runtime DLLが必要な場合だけ明示列挙し、開発toolchainを同梱しない。
- stagingはclean作成し古いfileを混ぜない。


**このsubtaskで行わないこと**

- installer／registry変更を実装しない。
- debug symbolsを無条件同梱しない。


**完了条件**

- commandを二度実行して同じ内容になる。
- missing resourceでnon-zero終了する。
- working directoryに依存しない。


**検証**

```bash
cargo run -p xtask -- package-windows --help
cargo build -p vtuber-desktop --release
```

#### Q2-04-003: Windows portable zipとintegrity checkを実装する

状態: `PENDING`
依存: `Q2-04-002`
親参照: DESIGN.md §22、ADR-007

**変更候補**

- `tools/xtask/src/package_windows.rs`
- `release artifact output`


**実装指示**

- stagingをportable zipへarchiveする。
- zip内path separator／root directoryをstableにする。
- archive作成後に展開検査とrequired file検査を行う。
- zip SHA-256とcontents manifestを出力する。


**このsubtaskで行わないこと**

- 自己解凍exeを作らない。
- source model cacheを丸ごと含めない。


**完了条件**

- zipを別directoryへ展開して起動可能である。
- hash manifestと実fileが一致する。
- absolute path entryがない。


**検証**

```bash
cargo run -p xtask -- package-windows
unzip -t <artifact.zip>
```

#### Q2-04-004: `xtask package-macos`をrelease用に完成させる

状態: `DEFERRED`
依存: `M1-09、Q2-04-001`
親参照: DESIGN.md §22、ADR-007

**変更候補**

- `tools/xtask/src/package_macos.rs`
- `packaging/macos/`


**実装指示**

- M1-09 minimal bundleをrelease manifest駆動に変更する。
- Info.plist version、bundle ID、camera usage、resourcesを生成する。
- aarch64 buildを既定にし、追加targetは明示optionにする。
- bundle内file permissionとexecutable bitを検証する。


**このsubtaskで行わないこと**

- notarization credentialをrepositoryへ入れない。
- Universal buildを暗黙に作らない。


**完了条件**

- clean checkoutから`.app`が生成される。
- `plutil`とbundle structure checkが通る。
- resources／licensesが正しい場所にある。


**検証**

```bash
cargo run -p xtask -- package-macos
plutil -lint <app>/Contents/Info.plist
```

#### Q2-04-005: macOS ad-hoc signing instructions／verificationを追加する

状態: `DEFERRED`
依存: `Q2-04-004`
親参照: DESIGN.md §22、ADR-007

**変更候補**

- `docs/release/macos-signing.md`
- `tools/xtask/src/package_macos.rs（verifyのみ）`


**実装指示**

- ad-hoc signing command、verification、quarantine注意、camera permission再検証手順を書く。
- Developer ID／notarizationはoptional future stepとして分離する。
- sign後にbundle hashが変わるためhash生成順を固定する。
- credential／team IDをhard-codeしない。


**このsubtaskで行わないこと**

- 自動upload／notarizationを実装しない。
- 秘密鍵pathを文書へ固定しない。


**完了条件**

- instructionsをfresh `.app`で再現できる。
- `codesign --verify --deep --strict`結果を確認できる。
- permission文字列がsign後も存在する。


**検証**

```bash
codesign --verify --deep --strict <app>
plutil -p <app>/Contents/Info.plist
```

#### Q2-04-006: version／hash manifestを全packageへ統一する

状態: `DEFERRED`
依存: `Q2-04-003、Q2-04-005`
親参照: DESIGN.md §22、ADR-007

**変更候補**

- `tools/xtask/src/package_common.rs`
- `release manifest output`


**実装指示**

- app version、git commit、rustc、Bevy、bevy_vrm1 revision、model hashes、file SHA-256をmanifestへ出す。
- manifest自身のhashまたはdetached checksumを出す。
- timestampがreproducibilityを壊す場合はbuild metadataとcontent hashを分離する。
- Windows／macOS同一schemaを使う。


**このsubtaskで行わないこと**

- hashを手書きしない。
- build machine absolute pathをmanifestへ入れない。


**完了条件**

- package contentをmanifestから検証できる。
- model hash検査がruntime／packageで一致する。
- 両OS artifactを同じtoolでverifyできる。


**検証**

```bash
cargo run -p xtask -- package verify --help
sha256sum <artifact> 2>/dev/null || shasum -a 256 <artifact>
```

#### Q2-04-007: clean-location resource／startup testを実施する

状態: `DEFERRED`
依存: `Q2-04-006`
親参照: DESIGN.md §22、ADR-007

**変更候補**

- `docs/release/package-smoke.md`
- `generated artifacts`


**実装指示**

- source tree外へartifactを移し、異なるcurrent directoryから起動する。
- model verify、VRM load、camera permission／capture、licenses閲覧を確認する。
- Windows zip展開、macOS `.app`の双方で同じchecklistを使う。
- missing resourceを意図的に削除しstartup errorを確認する。


**このsubtaskで行わないこと**

- developer checkoutでしか動かない状態を許容しない。
- system-wide installを要求しない。


**完了条件**

- install directory以外から起動してresourceを見つける。
- missing fileで明確に失敗する。
- source treeへのfallbackがない。


**検証**

```bash
manual package smoke。artifact hashと実行場所を記録。
```

#### Q2-04-008: ADR-007とrelease packaging reportを完成させる

状態: `DEFERRED`
依存: `Q2-04-007`
親参照: DESIGN.md §22、ADR-007

**変更候補**

- `docs/adr/ADR-007-desktop-packaging.md`
- `docs/release/release-checklist.md`


**実装指示**

- 実測したpackage方式、resource layout、permission、signing制約でADRをAccepted／Amended／Supersededにする。
- Windows／macOS command、artifact、hash、licenses、known limitationsをrelease checklistへまとめる。
- Q2-04受入条件を一対一確認する。
- 未対応installer／notarizationを明示的非scopeとして残す。


**このsubtaskで行わないこと**

- 未実施signingを完成済みと書かない。
- package差分を手作業だけにしない。


**完了条件**

- Q2-04の全受入条件を満たす。
- release artifactがverify可能である。
- ADRが実装と一致する。


**検証**

```bash
cargo test -p xtask
cargo deny check
cargo run -p xtask -- package verify --help
```

## Q2-05: release hardeningとprivacy audit
状態: `PENDING`
実行単位: `Q2-05-NNN`
重点参照: DESIGN.md §6.2〜§6.3、§20.4、§21、AGENTS.md §Error handling／§Model provenance


依存: M1-08-019（Windows hardening）／M1-09（macOS verification追補）


### platform gate

- privacy、worker failure、reconnect、release hardeningはWindowsで先行する。
- macOS固有挙動はM1-09後に同じauditへ追補する。

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

### 実行subtask

#### Q2-05-001: privacy／debug data flow inventoryを作る

状態: `PENDING`
依存: `M1-08-019`
親参照: DESIGN.md §6.2〜§6.3、§20.4、§21、AGENTS.md §Error handling／§Model provenance

**変更候補**

- `docs/security/privacy-audit.md`


**実装指示**

- camera pixels、landmarks、model paths、device metadata、metrics、logsの生成／保持／出力箇所を列挙する。
- release／debug featureごとのdata retentionを記録する。
- disk、network、clipboard等のegressをsource searchで確認する。
- 既定で外部送信なしを検証する。


**このsubtaskで行わないこと**

- プライバシー方針を宣言だけで終わらせない。
- raw user pathをreportへ掲載しない。


**完了条件**

- 全sensitive data flowにownerとretentionがある。
- 未承認disk write／network dependencyがない。
- audit scopeがsource locationへ紐づく。


**検証**

```bash
cargo tree --workspace
rg -n "write\(|File::create|reqwest|hyper|TcpStream|UdpSocket|clipboard" crates apps tools || true
```

#### Q2-05-002: debug frame／pixel保存をreleaseで無効化する

状態: `PENDING`
依存: `Q2-05-001`
親参照: DESIGN.md §6.2〜§6.3、§20.4、§21、AGENTS.md §Error handling／§Model provenance

**変更候補**

- `crates/vtuber-camera/`
- `crates/vtuber-app/`
- `Cargo.toml features`


**実装指示**

- pixel dump／screenshot debug pathが存在する場合は明示debug featureとuser actionの両方を要求する。
- release buildではcode pathがcompileされないかruntime無効であることをtestする。
- temporary file cleanupとpermissionを検証する。
- 通常error handlingでframeを自動保存しない。


**このsubtaskで行わないこと**

- panic時に自動raw frame dumpしない。
- debug featureをdefaultにしない。


**完了条件**

- release buildでpixel保存pathが無効である。
- debug保存は明示opt-inである。
- 保存先がAppData内でbounded cleanupされる。


**検証**

```bash
cargo test --workspace --release
rg -n "frame_dump|pixel_dump|save_frame|screenshot" crates apps || true
```

#### Q2-05-003: log redactionとstructured error auditを実装する

状態: `PENDING`
依存: `Q2-05-002`
親参照: DESIGN.md §6.2〜§6.3、§20.4、§21、AGENTS.md §Error handling／§Model provenance

**変更候補**

- `crates/*/src/error.rs`
- `crates/vtuber-app/src/error_presenter.rs`
- `docs/security/privacy-audit.md`


**実装指示**

- full path、camera frame、landmark array、model tensorを通常logへ出す箇所を除去／redactする。
- hashは短縮表示、device nameは必要最小限にする。
- error chainはDiagnosticsで取得できるがuser messageへ秘密情報を出さない。
- test loggerで禁止patternが出ないことを検証する。


**このsubtaskで行わないこと**

- 全error detailを削って診断不能にしない。
- hashそのものをsecret扱いして追跡不能にしない。


**完了条件**

- camera image／landmarkが通常logへ出ない。
- path redaction testが通る。
- debuggingに必要なerror code／stageは残る。


**検証**

```bash
cargo test --workspace log_redaction
rg -n "\{.*landmark|pixels|full_path" crates apps || true
```

#### Q2-05-004: path／size parserへfuzz-like property testsを追加する

状態: `PENDING`
依存: `Q2-05-003`
親参照: DESIGN.md §6.2〜§6.3、§20.4、§21、AGENTS.md §Error handling／§Model provenance

**変更候補**

- `crates/vtuber-avatar/tests/import_adversarial.rs`
- `crates/vtuber-app/tests/config_adversarial.rs`


**実装指示**

- random／table-driven bytes、truncated GLB、巨大length header、deep JSON、odd path componentを生成する。
- memory／time上限を持つtestにし、実巨大allocationを避ける。
- 全inputでpanicしないこととtyped error categoryを検証する。
- seedを固定しreproductionを出力する。


**このsubtaskで行わないこと**

- 外部fuzzer導入を必須にしない。
- unbounded recursive generatorを使わない。


**完了条件**

- path／size／preflightでpanicがない。
- hard capを迂回できない。
- failure seedから再現できる。


**検証**

```bash
cargo test -p vtuber-avatar import_adversarial
cargo test -p vtuber-app config_adversarial
```

#### Q2-05-005: worker panic／unexpected exit handlingをhardeningする

状態: `PENDING`
依存: `Q2-05-004`
親参照: DESIGN.md §6.2〜§6.3、§20.4、§21、AGENTS.md §Error handling／§Model provenance

**変更候補**

- `crates/vtuber-core/src/worker.rs`
- `crates/vtuber-camera/`
- `crates/vtuber-inference/`
- `crates/vtuber-app/`


**実装指示**

- camera／inference worker panic、error return、control disconnectをapp lifecycleへ伝播する。
- UIにunexpected exitとrecover actionを表示する。
- 一方のworker failureで他方を安全にstop／joinする。
- panic payloadを安全なdiagnosticへ変換する。


**このsubtaskで行わないこと**

- panicを再panicしてprocessを落とさない。
- 自動無限restartしない。


**完了条件**

- unexpected worker exitがUIへ出る。
- thread leak／zombie Running stateがない。
- fault injection testが通る。


**検証**

```bash
cargo test -p vtuber-core -p vtuber-camera -p vtuber-inference -p vtuber-app worker_failure
```

#### Q2-05-006: camera reconnect edge casesを追加検証する

状態: `PENDING`
依存: `Q2-05-005`
親参照: DESIGN.md §6.2〜§6.3、§20.4、§21、AGENTS.md §Error handling／§Model provenance

**変更候補**

- `crates/vtuber-camera/tests/reconnect_edges.rs`


**実装指示**

- stop中error、device switch中remove、permission revoked、rapid failure、format changeをfake sequenceでtestする。
- retry budgetとbackoff cancelを検証する。
- reconnect後sequence／timestampが一貫する。
- controller stateがstuckしないことを確認する。


**このsubtaskで行わないこと**

- 実cameraだけでedge caseを検証しない。
- retry limitをtestだけ緩和しすぎない。


**完了条件**

- 全edge caseがbounded timeで終了する。
- workerが二重spawnしない。
- failure原因がstatusへ残る。


**検証**

```bash
cargo test -p vtuber-camera reconnect_edges -- --nocapture
```

#### Q2-05-007: release profile／crash-free shutdownを検証する

状態: `PENDING`
依存: `Q2-05-006`
親参照: DESIGN.md §6.2〜§6.3、§20.4、§21、AGENTS.md §Error handling／§Model provenance

**変更候補**

- `Cargo.toml`
- `crates/vtuber-app/tests/shutdown.rs`
- `docs/security/privacy-audit.md`


**実装指示**

- release profileでpanic strategy、debug info、strip、LTOがdiagnostic／packaging要件と整合するか確認する。
- 起動各段階、Running、Failed、permission deniedからapp exitをtestする。
- window close時にworkers、LatestSlot、asset lifecycleを順序立てて停止する。
- shutdown timeoutをreportする。


**このsubtaskで行わないこと**

- panic abortを安易に採用しない。
- Drop順序だけに依存しない。


**完了条件**

- release buildでclean shutdownする。
- exit時panic／thread leakがない。
- profile変更に根拠がある。


**検証**

```bash
cargo test --workspace --release
cargo build -p vtuber-desktop --release
```

#### Q2-05-008: dependency／license／privacy release auditを完成させる

状態: `PENDING`
依存: `Q2-05-007`
親参照: DESIGN.md §6.2〜§6.3、§20.4、§21、AGENTS.md §Error handling／§Model provenance

**変更候補**

- `docs/security/privacy-audit.md`
- `docs/release/release-checklist.md`
- `deny.toml`


**実装指示**

- `cargo deny`、duplicate／forbidden dependency、license bundle、network dependencyを確認する。
- release artifact内fileをmanifestと照合する。
- privacy inventoryの各項目をPASS／FAIL／NOT APPLICABLEで閉じる。
- Q2-05受入条件と残存riskを文書化する。


**このsubtaskで行わないこと**

- 未解決privacy issueを既知制約だけで閉じない。
- release artifact確認をsource tree確認で代用しない。


**完了条件**

- Q2-05の全受入条件を満たす。
- `cargo deny`とlicense reviewが成功する。
- 通常log／artifactにraw sensitive dataがない。


**検証**

```bash
cargo deny check
cargo test --workspace --release
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Q2-06: BodyTracking上半身追従とhead-relative gaze

状態: `IN_PROGRESS`
実行単位: `Q2-06-001`、`Q2-06-002`、repair `Q2-06-002-001`〜`Q2-06-002-004`、`Q2-06-003`
重点参照: DESIGN.md §7.3、§11.8、§15.4、§16.5〜§16.9、ADR-002、ADR-004、ADR-010

### 目的

calibrationとsemantic座標変換済みのface yaw／pitch／rollをdirect-pose `BodyTracking`へ渡し、webcam eye gazeは別入力として推定／filterした後、現在のhead姿勢へhead-relative LookAt deltaとして階層合成する。head writerとeye writerを競合させず、VRM作者のLookAt backendとrange mapを尊重する。

### 実行subtask

#### Q2-06-001: direct-pose BodyTrackingと上半身追従を統合する

状態: `DONE`
依存: `M1-08-019`
親参照: DESIGN.md §7.3、§16.5〜§16.7、ADR-002、ADR-004

**変更候補**

- `AGENTS.md`
- `DESIGN.md`
- `docs/adr/ADR-002-bevy-vrm1-runtime.md`
- `docs/adr/ADR-004-avatar-control-order.md`
- `Cargo.toml`／`Cargo.lock`
- sourceとlicense／base revisionを記録した最小vendored `bevy_vrm1` patch
- `crates/vtuber-avatar/src/pose/`
- `crates/vtuber-avatar/src/plugin.rs`
- `crates/vtuber-avatar/tests/`

**実装指示**

- 固定revision `f9593fd78136fb9e0507bcae111e09291ec9b82a`の`body_tracking.rs`、`look_at.rs`、`system_set.rs`、plugin登録、rest transformとhumanoid bone holderを基準にする。
- `BodyTrackingPoseInput`相当のradian yaw／pitch／roll、confidence weight、active flagを追加し、直接入力では`LookAt`を要求しない。
- small／large yaw、pitch、rollのnamed weights、12°〜45°smoothstep engagement、optional bone再正規化、bone別half-lifeとrotation limitを設定可能にする。
- bone順は`spine -> chest -> upperChest -> neck -> head`とし、実際の`ChildOf`経路を使って中間nodeを含む`GlobalTransform`を更新する。
- `RestTransform`／`RestGlobalTransform`を正本とし、animation baseへ`base * (rest.inverse() * tracking_target)`で加算する。tracking deltaを累積させず、loss時はbone別half-lifeでneutralへ戻す。
- direct入力rootをlegacy `LookAt + BodyTracking` pathから除外し、direct pathを`AnimationSystems`後かつ`Constraints`前へ置く。legacy pathの順序と挙動は維持する。
- `vtuber-avatar`の旧`apply_tracked_head_pose`をTransform writerから入力bridgeへ置き換え、同じboneへのwriterを一つにする。
- 顔姿勢用synthetic `LookAt`を作らず、既存eye expression／eye-bone gazeを維持する。
- Bevy、`bevy_vrm1`のbase revision、その他依存を無関係に更新しない。unsafe、warning suppression、新規IK dependencyを追加しない。

**完了条件**

- 指定された軸別weight、engagement、optional bone再正規化、bone別half-life、shortest-angle smoothingがpure unit testで固定される。
- upperChest／chest／spine／neckの欠落でpanicせず、元weightが0のboneを勝手に参加させない。
- animation baseへの加算、非累積、base更新検出、tracking loss／復帰、finite guardが自動検証される。
- direct入力があるrootはdirect pathだけ、ないrootはlegacy pathだけで処理される。
- eye gazeが独立し、headの最新`GlobalTransform`を参照できる。
- workspaceとvendored dependencyのfmt、check、clippy、testが成功し、`cargo deny check`が成功する。
- Windows実機の上半身追従はPASS／FAIL／NOT RUNを正直に記録し、macOSとVRMA playbackは未実施ならPASSにしない。

**検証**

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo deny check
# vendored bevy_vrm1でもfmt/check/clippy/testを実行する。
```

**このsubtaskで行わないこと**

- synthetic `LookAt` target、独自FBIK、FABRIK、CCD、物理spine solverを追加しない。
- head poseとeye gazeを同じ入力へ統合しない。
- 毎frameのbone名検索、固定lerp、rest orientationの重複cache、tracking deltaの累積を導入しない。
- VRMA playbackまたは未実施hardware acceptanceを対応済みと表現しない。

**完了記録（2026-08-13）**

- upstream revision `f9593fd78136fb9e0507bcae111e09291ec9b82a`をbaseとして、licenseとprovenanceを保持したvendored patchへdirect yaw／pitch／roll入力、axis別weight、yaw engagement、optional bone再正規化、bone別half-life／limitを追加した。
- `vtuber-avatar`はbone Transformを直接書かず、generation一致の`BodyTrackingPoseInput`だけを更新する。旧`HeadNeckWeights`とrest orientation cacheは永続化対象ではなかったため、設定migrationは不要だった。
- direct rootはlegacy `LookAt + BodyTracking` queryから除外し、両writerを同じscheduleで実行する排他testを追加した。eye expression／eye-bone gazeは独立経路のまま維持した。
- root workspaceで`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace --no-fail-fast`、`cargo deny check`、`git diff --check`を実行し、すべて成功した。
- `vendor/bevy_vrm1`で`cargo fmt --all -- --check`、`cargo check --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo test`を実行し、67 unit testsと10 doctestsを含めて成功した。
- Windows実機での新しい上半身追従確認、macOS実機確認、VRMA playback確認は`NOT RUN`。既存M1のhead tracking実機証拠を本変更の上半身受入証拠として再利用しない。
- commits: `docs(avatar): approve direct body tracking path`、`feat(vrm): extend body tracking for direct pose`、`refactor(avatar): route pose through body tracking`、`test(avatar): close body tracking compatibility gaps`。

#### Q2-06-002: head-relative gaze coordinationとVRM LookAt統合

状態: `DONE`
依存: `Q2-06-001`
親参照: DESIGN.md §7.3、§11.8、§15.4、§16.5〜§16.9、ADR-002、ADR-004、ADR-010

**実装指示**

- valid centered gazeとUnavailableを区別する正規化`GazeSignal`契約をcoreへ追加し、MediaPipe typed 52係数から左右眼観測、blink weight、agreement confidence、共通eye-in-head gazeを直接生成する。
- neutralへ左右眼horizontal／vertical baselineを追加し、専用指数filter（tracked 0.055秒、return 0.150秒、hold 0.080秒）とloss／reacquisition連続化を実装する。
- `bevy_vrm1`へworld target不要のdirect head-relative LookAt入力を追加し、`LookAtProperties`、左右inner／outer、up／down、non-identity rest、additive animation baseを再利用する。
- モデル作者の`LookAtType`を尊重してBone／Expressionを排他的に選択し、Expression weightは既存の1frame 1回`ModifyExpressions`経路へcoalesceする。
- Animation、BodyTracking、LookAt、Expression、Constraint、SpringBoneのVRM 1.0意味順序を実行testで固定し、legacy cursor／target pathを維持する。
- reset、recalibration、avatar replacement、despawnでgaze stateを消去し、eye translation／scale／GlobalTransformとhead／body bonesをgaze systemから変更しない。

**完了条件**

- centerとUnavailable、左右符号／融合／blink、baseline、30／60／120fps、hold／neutral return／reacquisitionが自動testされる。
- head-only、gaze-only、head＋counter-rotation、non-identity rest、range map、backend fallback／排他、animation base／非累積、state cleanupが自動testされる。
- synthetic coefficient round-trip、旧`Option<GazePose>`正本、旧独自eye writer、`ExpressionAndEyeBones`実行modeが残らない。
- workspaceとvendored dependencyのfmt、check、clippy、test、`cargo deny check`が成功する。
- Windows visual acceptanceとmacOS確認はPASS／FAIL／NOT RUNを正直に記録する。

**検証**

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo deny check
cargo fmt --manifest-path vendor/bevy_vrm1/Cargo.toml -- --check
cargo check --manifest-path vendor/bevy_vrm1/Cargo.toml --all-targets
cargo clippy --manifest-path vendor/bevy_vrm1/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path vendor/bevy_vrm1/Cargo.toml
```

**このsubtaskで行わないこと**

- webcam gaze用world target、gaze由来の追加head rotation、独立eye world transform、Eye translation変更を追加しない。
- BoneとExpressionを同時適用せず、Bevy／MediaPipe／無関係なdependencyを更新しない。
- random saccade、IK、未実施hardware acceptanceを対応済みと表現しない。

**完了記録（2026-08-12）**

- `AvatarControlFrame`の正本をfinite／boundedな`GazeSignal`へ変更し、valid centered gazeと`Unavailable`を型で区別した。MediaPipe typed eye channelから左右眼観測、blink weight、agreement confidence、共通gazeを直接生成し、synthetic coefficient round-tripを削除した。
- neutral profileをversion 2へ更新して左右眼horizontal／vertical baselineを追加し、旧値はzero baseline defaultで安全に扱う。専用filterはtracked half-life 0.055秒、neutral return 0.150秒、unavailable hold 0.080秒とし、loss／reacquisitionもheadと同時に連続補間する。
- vendored `bevy_vrm1`へworld target不要の`DirectLookAtInput`を追加した。Boneは既存rest local／globalと左右inner／outer・up／down range mapを再利用し、animation baseへ非累積加算する。Expressionはrange-map済みweightを既存の1frame 1回`ModifyExpressions`経路へ渡す。
- モデル作者のLookAt typeを優先してBone／Expressionを排他的に選び、破損宣言またはmetadata欠損はcapability snapshotとwarningへfallback理由を残す。旧`apply_tracked_eye_gaze`、独自eye range設定、重複`GazeMode`／`ExpressionAndEyeBones`実行modeは削除した。
- 実行testでAnimation→body input→direct BodyTracking→gaze input→GazeControl→Expression→伝播→Constraint→伝播→SpringBoneを固定した。head階層追従、counter-rotation、translation／scale不変、non-identity rest、center復帰、animation base、非累積、backend排他、replacement／despawn cleanupを自動検証した。
- root workspaceでfmt、check、clippy、`cargo test --workspace --no-fail-fast`、`cargo deny check`、対象gaze／loss recovery／schedule test、`git diff --check`が成功した。`deny.toml`はlock済み依存の`Ubuntu-font-1.0`、`CDLA-Permissive-2.0`、固定MediaPipe Git sourceだけを監査済み許可へ追加し、dependency versionは変更していない。
- vendored `bevy_vrm1`でfmt、all-target check、all-target clippy、76 unit tests、10 doctestsが成功した。base revisionとlicenseは不変である。
- Windowsではlicensed test VRM 1.0のimport、描画、lifecycle `Ready`まで実画面で確認した。C922 MSMFは5秒で150 framesを取得しstage errorなしだったが、顔がframe内になく`face_count=0`だったため、head／eye visual acceptanceは`NOT RUN`。macOS実機確認も`NOT RUN`。
- commits: `docs(gaze): define head-relative coordination`、`refactor(core): make gaze availability explicit`、`feat(tracking): filter calibrated binocular gaze`、`feat(vrm): add direct head-relative look at`、`feat(avatar): coordinate gaze through VRM LookAt`、`test(vrm): verify additive direct eye gaze`、`test(gaze): cover composition and schedule regressions`、`chore(policy): audit locked runtime licenses`、`docs(gaze): record Q2-06-002 completion`。

#### Q2-06-002-001: backend fallbackのrange-map単位を修正する

状態: `DONE`
依存: `Q2-06-002`

- 宣言backendと選択backendが一致する場合だけモデルのrange mapをそのまま使う。
- backend変更時は`inputMaxValue`を保持し、`outputScale`をBoneの度またはExpression weightへ変換する。
- Expression→Bone、Bone→Expression、metadataなし→Bone／Expressionをbinding後の実効値で検証する。

#### Q2-06-002-002: auto-neutral窓とgaze baseline品質を修正する

状態: `DONE`
依存: `Q2-06-002-001`

- 15Hzでも15 sampleのrobust windowへ到達できる期間へ変更する。
- blink／低weightの眼をgaze baseline集計から除外する。
- pose referenceとgaze baselineの変更通知を分離し、該当filterをresetする。

#### Q2-06-002-003: VRM zero-rangeとQuaternion signを修正する

状態: `DONE`
依存: `Q2-06-002-002`

- `inputMaxValue == 0`のVRM 1.0推奨挙動をBone／Expressionの両mappingで実装する。
- 同一回転を表す`q`／`-q`でdirect eye deltaが二重適用されないようにする。

#### Q2-06-002-004: CI・実schedule test・visual gate状態を修正する

状態: `DONE`
依存: `Q2-06-002-003`

- `percentile_ms`を全platformでtest compile可能にする。
- 実際の`VtuberAvatarPlugin`／`VrmPlugin`登録を使うschedule回帰testを追加する。
- Windows実カメラ＋実VRM visual acceptanceとmacOS実機確認は、証拠がなければ`PENDING`／`NOT RUN`のままにする。

**完了記録（2026-08-12）**

- `percentile_ms`のWindows限定compile guardを外し、macOS test moduleでも同じpure functionをcompileできるようにした。
- 手組みのset順テストを、実際の`VtuberAvatarPlugin`が追加する`VrmPlugin`とavatar bridge systemへtrace systemを加えた解決済み実行順検査へ置き換えた。
- Windows C922＋実VRM visual acceptance 6項目は`PENDING`、macOS実機確認は`NOT RUN`。過去のM1眼球確認をhead-relative gaze修正後の証拠として再利用しない。

#### Q2-06-003: アバターとpreviewのmirrorを既定にする

状態: `DONE`
依存: `Q2-06-002-004`
親参照: DESIGN.md §11.6、§13.6、§18.2〜§18.3、ADR-004、ADR-010

**実装指示**

- previewの表示UV mirrorを既定ONにし、capture／inferenceへ渡すframeは非mirrorのまま保持する。
- `vtuber-avatar`にadapter-localなavatar motion mirrorを追加し、既定ONかつLive UIから個別に切替可能にする。
- mirror ONではVRM入力直前にyaw／rollとhorizontal gazeを反転し、pitch／vertical gazeは維持する。per-eye blinkは左右をswapする。
- core／inference／trackingのcanonical unmirrored座標、calibration、filter、`AvatarControlFrame`を変更しない。preview toggleをtracking mathへ入れない。
- direct `BodyTracking`、direct head-relative LookAt、Expression backendの既存排他とscheduleを維持する。

**完了条件**

- previewとavatar motionが新規sessionでmirror ONになるunit testがある。
- mirror ON／OFFのpose、gaze、per-eye blinkの左右変換と、pitch／vertical gaze不変が自動検証される。
- UI actionがavatar motion mirrorを一度だけ切替え、view-modelへ同期する。
- workspace fmt、check、clippy、test、`cargo deny check`、`git diff --check`が成功する。
- Windows／macOSの実camera＋実VRM visual acceptanceは、実施しなければ`NOT RUN`と記録する。

**このsubtaskで行わないこと**

- camera frame、MediaPipe入力、landmark、tracking座標、calibration値をmirrorしない。
- synthetic world-space LookAt target、eye world transform、VRM runtime改変を追加しない。
- 未実施の実camera visual acceptanceをPASSと記録しない。

**完了記録（2026-08-13）**

- preview UV mirrorとadapter-localな`AvatarMotionMirror`を既定ONにした。Live UIの`Mirror Preview`と`Mirror Avatar Motion`は独立して切替可能であり、後者はview-modelにも同期する。
- avatar motion mirrorはVRM適用境界だけでyaw／roll、horizontal gaze、per-eye blinkの左右を反転する。pitch、vertical gaze、mouth、emotionは維持し、camera frame、MediaPipe入力、landmark、calibration、tracking filter、canonical `AvatarControlFrame`は変更しない。
- pose、gaze、blink、default、UI actionのunit／integration testを追加・更新した。root workspaceで`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace --no-fail-fast`、`cargo deny check`、`git diff --check`はすべて成功した。
- Windows実camera＋実VRMのmirror visual acceptance、macOS実機確認は`NOT RUN`。この変更はhardware PASSを主張しない。

#### Q2-06-004: T-poseを避けるrelaxed-arm defaultを適用する

状態: `DONE`
依存: `Q2-06-003`
親参照: DESIGN.md §16.4.1、ADR-004

**実装指示**

- binding成功時に存在する`leftUpperArm`／`rightUpperArm`を解決し、model-authored `RestTransform`から左右対称に55°下げた表示用local transformを一度だけ設定する。
- `RestTransform`／`RestGlobalTransform`を変更せず、`BodyTracking`のhead〜spine writer所有権、eye gaze、Node Constraint、SpringBoneを変更しない。
- upper armがないVRMを拒否せず、lower arm、hand、world transformに直接書き込まない。
- replacementで新しいavatarだけへ再適用し、frameごとのdelta累積を導入しない。

**完了条件**

- left／rightの回転符号と55°のoffset、missing upper armのno-op、rest pose不変が自動testで固定される。
- workspace fmt、check、clippy、対象test、`git diff --check`が成功する。
- Windows／macOS実VRMでのvisual確認は、実施しなければ`NOT RUN`と明記する。

**完了記録（2026-08-13）**

- bindingがoptional `leftUpperArm`／`rightUpperArm`を一度だけ解決し、model-authored local rest rotationへleft `-55°`、right `+55°`のZ軸offsetを加えてからavatarを表示する。`RestTransform`／`RestGlobalTransform`は不変である。
- head、neck、upperChest、chest、spineのdirect `BodyTracking`、head-relative gaze、Node Constraint、SpringBone、lower arm、handのwriter所有権は変更していない。上腕なしmodelは従来どおり`Ready`になる。
- binding integration testでnon-identity rest rotationへの加算、左右が下方向を向く符号、rest pose不変、upper armなしのno-op、Ready後にoffsetを再適用しないことを固定した。
- root workspaceで`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace --no-fail-fast`、`cargo deny check`、`git diff --check`が成功した。対象の`cargo test -p vtuber-avatar --test binding`と`cargo clippy -p vtuber-avatar --all-targets -- -D warnings`も成功した。
- Windows／macOSでの実VRM visual確認は`NOT RUN`。この自動検証は実画面の見た目をPASSと主張しない。

---

# Research 3 — 自由研究としての評価

## R3-01: smoothingとlatencyの比較実験
状態: `PENDING`
実行単位: `R3-01-NNN`
重点参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md


依存: Q2-01-008、Q2-03-007（Windows実験。macOS比較は後日追補）

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

### 実行subtask

#### R3-01-001: experiment dataset／replay contractを固定する

状態: `PENDING`
依存: `Q2-01-008、Q2-03-007`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/experiments/filter-comparison.md`
- `crates/vtuber-tracking/src/experiment/replay.rs`
- `test data manifest`


**実装指示**

- recorded observation format、timestamp、pose／expression fields、license／privacyを定義する。
- 同じstreamを全filterへ入力するdeterministic replay APIを作る。
- raw camera imageではなくlandmark／observationまたはsynthetic streamを使用する。
- dataset hashとgenerator versionを記録する。


**このsubtaskで行わないこと**

- filterごとに別datasetを使わない。
- manual motionだけをdatasetにしない。


**完了条件**

- 全filterが同一input sequenceを受ける。
- datasetを再生成／verifyできる。
- 個人識別可能な画像を含まない。


**検証**

```bash
cargo test -p vtuber-tracking experiment_replay
```

#### R3-01-002: filter strategy interfaceとmeasurement hooksを定義する

状態: `PENDING`
依存: `R3-01-001`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `crates/vtuber-tracking/src/experiment/filter.rs`


**実装指示**

- reset、update(timestamp, quaternion)、name／parametersを持つ小さなtraitまたはenum dispatchを定義する。
- production filterとexperiment filterのcode reuse境界を明確にする。
- per-update outputとinternal latency以外のside effectを持たせない。
- parameter setをresult metadataへserializeできるようにする。


**このsubtaskで行わないこと**

- dynamic plugin systemを作らない。
- GUI tuning toolを先行実装しない。


**完了条件**

- 同じharnessで複数filterを切り替えられる。
- filter stateがrun間でresetされる。
- production dependencyへexperiment codeが漏れない。


**検証**

```bash
cargo test -p vtuber-tracking filter_strategy
```

#### R3-01-003: fixed exponential smoothing baselineをadapter化する

状態: `PENDING`
依存: `R3-01-002`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `crates/vtuber-tracking/src/experiment/filters/exponential.rs`


**実装指示**

- M1-03 production filterと同じformula／parameterをstrategy interfaceへ接続する。
- duplicate実装を避け、production関数をreuseする。
- step／constant inputのsanity testを追加する。
- baseline parameterをreportへ固定する。


**このsubtaskで行わないこと**

- formulaを別copyとして保守しない。
- baselineだけparameter探索しない。


**完了条件**

- production outputとexperiment adapterが同一になる。
- reset後に初期条件が一致する。
- baseline resultが再現可能である。


**検証**

```bash
cargo test -p vtuber-tracking experiment_exponential
```

#### R3-01-004: One Euro filterをquaternion入力向けに実装する

状態: `PENDING`
依存: `R3-01-003`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `crates/vtuber-tracking/src/experiment/filters/one_euro.rs`


**実装指示**

- quaternion deltaをlog mapまたはangular velocityへ変換してOne Euro cutoffを計算する。
- min cutoff、beta、derivative cutoff、dt clampをparameter化する。
- shortest arcとsign continuityを扱う。
- 論文／一次資料に対応する式をcomment／referenceへ記載する。


**このsubtaskで行わないこと**

- Euler軸独立One Euroだけで済ませない。
- production defaultへまだ採用しない。


**完了条件**

- constant inputでjitterを増やさない。
- step inputでfinite outputを返す。
- zero／large dtでpanicしない。


**検証**

```bash
cargo test -p vtuber-tracking experiment_one_euro
```

#### R3-01-005: quaternion log-space adaptive filterを実装する

状態: `PENDING`
依存: `R3-01-004`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `crates/vtuber-tracking/src/experiment/filters/log_adaptive.rs`


**実装指示**

- previous outputからtargetへのrelative quaternionをlog vectorへ変換する。
- angular speed／noise estimateからadaptive smoothing factorを決める。
- exp mapでoutputへ戻しnormalizeする。
- small-angle numerical stabilityをtestする。


**このsubtaskで行わないこと**

- unsafe math optimizationを入れない。
- 未定義noise modelを隠さない。


**完了条件**

- identity近傍でNaNが出ない。
- fast turnでslow motionより追従率が上がる。
- shortest arcを維持する。


**検証**

```bash
cargo test -p vtuber-tracking experiment_log_adaptive
```

#### R3-01-006: output-only slerp filterを実装する

状態: `PENDING`
依存: `R3-01-005`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `crates/vtuber-tracking/src/experiment/filters/output_slerp.rs`


**実装指示**

- 入力pose estimationは変更せず、最終outputだけをfixed／dt-adjusted slerpする。
- factor、dt clamp、resetをparameter化する。
- production avatar-side smoothingと混同しないようtracking experiment内に置く。
- step response testを追加する。


**このsubtaskで行わないこと**

- avatar bone systemへ追加smoothingしない。
- baselineと同名parameterを曖昧にしない。


**完了条件**

- outputがnormalized quaternionである。
- fixed FPS差をdt調整で抑える。
- reset時に初期lagを定義どおり扱う。


**検証**

```bash
cargo test -p vtuber-tracking experiment_output_slerp
```

#### R3-01-007: 標準scenario generatorを実装する

状態: `PENDING`
依存: `R3-01-006`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `crates/vtuber-tracking/src/experiment/scenarios.rs`
- `test data manifest`


**実装指示**

- slow sine motion、step rotation、fast turn、noisy neutral、blink pulse、face loss／returnを固定seedで生成する。
- ground truth pose／expressionとsample timestampを出力する。
- noise分布とamplitudeをmetadataへ記録する。
- scenario duration／rateを全filterで共通にする。


**このsubtaskで行わないこと**

- 手入力CSVだけにしない。
- filter outputをscenarioへfeedbackしない。


**完了条件**

- 同じseedで同じCSV／streamになる。
- ground truthが各metric計算に利用できる。
- face loss区間がstate signalとして表現される。


**検証**

```bash
cargo test -p vtuber-tracking experiment_scenarios
```

#### R3-01-008: jitter／rise time／overshoot／phase lag metricsを実装する

状態: `PENDING`
依存: `R3-01-007`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `crates/vtuber-tracking/src/experiment/metrics.rs`
- `crates/vtuber-tracking/tests/experiment_metrics.rs`


**実装指示**

- noisy neutralのRMS angular jitterを計算する。
- stepの10–90% rise time、overshootを計算する。
- sineのphase lagをcross-correlationまたは明示methodで計算する。
- capture-to-apply p50／p95はpipeline metricsと区別して入力できるようにする。


**このsubtaskで行わないこと**

- 単純Euler差でwrap errorを出さない。
- 主観評価を数値metricに混ぜない。


**完了条件**

- known synthetic signalで期待metricになる。
- insufficient sampleをtyped N/Aにする。
- 角度wrapを正しく扱う。


**検証**

```bash
cargo test -p vtuber-tracking experiment_metrics
```

#### R3-01-009: filter×scenario matrixを実行しraw CSVを生成する

状態: `PENDING`
依存: `R3-01-008`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `tools/xtask/src/filter_experiment.rs`
- `docs/experiments/data/`


**実装指示**

- 全filter／parameter setを全scenarioへ同じ順序で実行する。
- run metadata、input hash、filter parameters、per-sample output、summary metricsをCSV／JSONへ出す。
- output file名をcontent／run IDで一意にする。
- 失敗runもerror rowとして残す。


**このsubtaskで行わないこと**

- 都合の悪いrunを削除しない。
- generated CSVをsource codeへ埋め込まない。


**完了条件**

- 一commandで全matrixを再生成できる。
- raw dataからsummaryを再計算できる。
- 結果がmanual copyに依存しない。


**検証**

```bash
cargo run -p xtask -- filter-experiment --help
cargo run -p xtask -- filter-experiment
```

#### R3-01-010: 比較reportとrecommended defaultsを完成させる

状態: `PENDING`
依存: `R3-01-009`
親参照: DESIGN.md §15.6、§23、docs/PERFORMANCE_TEST_PLAN.md

**変更候補**

- `docs/experiments/filter-comparison.md`
- `docs/experiments/data/`
- `設定defaults／ADR amendment（採用時のみ）`


**実装指示**

- 各scenarioのmetricsを表／必要最小限のplotで比較する。
- 滑らかさと反応速度のtrade-offを数値とsubjective visual noteで分けて記述する。
- recommended filter／parametersと不採用理由を示す。
- production default変更は別差分としてtest／acceptanceを通し、reportだけで変更しない。


**このsubtaskで行わないこと**

- 主観だけでwinnerを決めない。
- 長期未解決を理由に結論を放棄しない。


**完了条件**

- R3-01の全成果物が揃う。
- raw CSV、repro command、recommended defaultsがある。
- 結論が測定結果に対応する。


**検証**

```bash
cargo test -p vtuber-tracking experiment
cargo run -p xtask -- filter-experiment
```
