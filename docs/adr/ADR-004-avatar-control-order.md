# ADR-004: tracking結果のVRM適用順と座標変換

Status: Accepted  
Date: 2026-08-04
Amended: 2026-08-14 (Issue #20: always-on idle breathing)

## Context

Bevy animation、VRM Node Constraint、gaze、Expression、SpringBoneは同じframe内でTransformまたはmorphへ作用する。適用順が曖昧だと、上書き、one-frame lag、jitterが発生する。

固定revisionの`bevy_vrm1` loaderは`GltfConvertCoordinates::default()`を使う。したがってVRM nodeはVRM 1.0／glTFのmodel軸、すなわちforward `+Z`、up `+Y`、avatar right `-X`として扱う。

## Decision

### Semantic pose

tracking coreは次を出力する。

- unmirrored imageで顔が画像右へ向く: `yaw > 0`
- 顎が上がる: `pitch > 0`
- 観察者から見て時計回り: `roll > 0`

adapterの初期model-space対応:

```text
yaw   -> +Y rotation
pitch -> -X rotation
roll  -> -Z rotation
```

実装候補は`Quat::from_euler(EulerRot::YXZ, yaw, -pitch, -roll)`。人工点群と公式VRM 1.0 sampleで符号を固定し、目視だけで反転しない。

### Mirror-control default

canonical trackingは常にunmirrored camera座標のまま保持する。一方、ユーザーがアバターを鏡のように操作できるよう、adapter-localの`AvatarMotionMirror`は既定で有効にする。これはpreviewのUV mirrorとは独立した設定であり、camera frame、inference、calibration、tracking filter、`AvatarControlFrame`を書き換えない。

VRM入力の直前に水平反射を一度だけ適用する。

```text
yaw              -> -yaw
pitch            ->  pitch
roll             -> -roll
gaze horizontal  -> -horizontal
blink left/right -> swap
```

水平反射はpitch、vertical gaze、口・感情など左右を持たないexpressionを変えない。OFF時は従来のcanonical-to-model対応を用いる。この分離により、preview toggleがtracking mathへ入らない既存の不変条件を維持しつつ、avatar表示だけをユーザー向けに反射できる。

### Direct-pose BodyTracking

`PostUpdate`で次の順序に置く。

```text
AnimationSystems
 -> ApplyBreathingHipsTranslation
 -> DirectPoseBodyTracking
 -> ModelAdaptiveDefaultArmPose
 -> DirectHeadRelativeLookAt / GazeControl
 -> Expressions
 -> VrmSystemSets::Constraints
 -> PropagateAfterConstraints
 -> SpringBone
```

VRM 1.0はbone local rotationがidentityとは限らない。model-space deltaを各boneのrest orientationへ共役変換する。

```text
R_bone_rest_model = inverse(R_root_rest_global) * R_bone_rest_global
R_delta_local     = inverse(R_bone_rest_model) * R_delta_model * R_bone_rest_model
R_output_local    = R_bone_rest_local * R_delta_local
```

直接入力は`spine -> chest -> upperChest -> neck -> head`の順に適用する。各boneのtracking targetからrest-relative deltaを求め、animation systemが書いたbaseへ`base * delta`で加算する。前frameの出力とlast deltaを保持してanimationによるbase更新を識別し、deltaを累積させない。bone間の非Humanoid中間nodeを含む実際の`ChildOf`経路へ最新`GlobalTransform`を伝播する。

tracking喪失時はtarget yaw／pitch／rollを0へ戻し、bone別half-lifeでanimated baseへ復帰する。汎用Bevy Animationへの加算合成はこのADRの対象だが、VRMA playbackの製品サポートを追加するものではない。

### Model-adaptive default arm pose

binding成功時は`Transform`へ固定角度を書き込まず、completeなupper arm／lower arm／hand chainのrest-space geometryをcacheする。shoulderとfingerはoptional capabilityとして保持し、欠損または退化したchainはそのsideのenhanced poseだけを無効にする。純粋なanalytic two-bone IKでtyped `DefaultArmPose`をgeneration付きで解決し、既定値はarm drop 70°、reach 0.99、forward hand offset 0.081 total（VRM model-space `+Z`）、rearward elbow pole offset 0.05 total（`-Z`）とする。

`DefaultArmPose`は`AnimationSystems`とdirect-pose `BodyTracking`の後、direct head-relative gaze／`VrmSystemSets::GazeControl`および`VrmSystemSets::Constraints`の前に毎frame適用する。animation baseへrest-relative upper／lower deltaを`base * delta`で加算し、前frameのcomposed outputを基準にして累積を防ぐ。実際の`ChildOf`経路を通じて中間nodeを含む影響subtreeの`GlobalTransform`を更新する。`RestTransform`／`RestGlobalTransform`は不変で、generation不一致、replacement、欠損geometryはno-opとする。

このcompositorはhead、neck、upperChest、chest、spineを追跡するdirect `BodyTracking`のwriter所有権を変更せず、eye gaze、Node Constraint、SpringBoneのwriter競合も導入しない。

Issue #17の二次補正は、解決upper displacementの18%を肩へ追従させ、肩回転を5°以内にclampする。finger jointには各jointのrest-global axisを用いた10°の弱いcurlを適用する。wrist／handへ固定角度を直接書かず、lower armの解決回転を実階層へ伝播してauthored wrist orientationを保持する。shoulder／fingerがないモデルでは該当補正だけを無効にする。

Issue #18のmodel-specific tuningは、import content hashを`AvatarAssetId`としてversion 1のbounded `ArmPoseProfileOverride`へ対応付ける。bindingは`ArmPoseOverrideStore`から検証済み値だけを読み、未知version、非finite値、範囲外値は既定profileへfallbackする。storeの`entries`／`import_entries`がアプリ設定層との保存境界であり、同一session内のmodel unload／reloadではresourceが値を保持する。明示的なresetはgeometry-derived defaultへ戻す。

初回default適用は0.25秒、defaultへ戻す操作は0.6秒の左右独立blendとする。blendはdelta quaternionをshortest arcでslerpし、経過時間を`Time::delta_secs`で進めるため固定FPS依存にならない。invalid timeは状態を進めず、generationごとに新規stateを作ることでreplacement間のpose transition漏れを防ぐ。

### Always-on procedural idle breathing (Issue #20)

`Ready`状態のアバターには常時、subtleなprocedural breathingを適用する。カメラ・control frame・`BodyTrackingPoseInput.active`・confidenceには依存せず、tracking loss／hold中も継続する。

- **所有する値**: additive `hips.translation` のみ。head〜spineのrotation writerであるdirect-pose `BodyTracking`、arm、eye、scale、scene root、cameraは一切書かない。
- **波形**: 真の周期を使う `phase_01 = (elapsed_seconds / period_seconds) mod 1`、`breath_01 = sin(PI * phase_01)^2`。位相はbinding直後の最初のframeでphase `0`（完全にneutral、popなし）になるよう評価してから進める。phase accumulatorは`f64`で、最終的なbounded値だけを`f32`へ変換する。frame-rate非依存で、同じphaseの再評価は同じ出力になる。
- **プロファイル既定値**: `period_seconds = 5.0`。amplitudeはimmutable rest空間のhips高さ（model-space `+Y`の正値）から `vertical = clamp(0.010 * rest_hips_height, 0.006 m, 0.0125 m)`、`forward = clamp(0.008 * rest_hips_height, 0.004 m, 0.010 m)`。VMagicMirror（pinned `malaybaku/VMagicMirror@8c97982`）のsmall body offset（prefab 0.01 m）以下に抑えた。
- **座標変換**: ピーク吸入時のsemantic model-space offsetは `+Y * vertical + +Z * forward`。`RestGlobalTransform(hips) = parent_rest ∘ RestTransform(hips)` のaffine逆から `parent_linear⁻¹` をbinding時に一度だけ導出し、non-humanoid intermediate nodeを含む実際の`ChildOf`経路でもmodel軸の意味が保たれる。runtimeはcached ancestor pathでhips自身の`GlobalTransform`だけを更新し、full hierarchyを毎frame走査しない。
- **base合成と非累積**: `output = base + current_delta`。animationがhips translationを書いたら新しい値をbaseとして捕捉し、自前の前回出力を累積しない。cycle境界でauthored/animated baseへ正確に復帰する。
- **lifecycle**: `Ready`の間だけ書き、unload／replacementで全状態を破棄する（componentはroot entityと共にdespawn）。replacementはneutral phase `0`から開始する。

このcompositorはhead、neck、upperChest、chest、spineを追跡するdirect `BodyTracking`のwriter所有権を変更せず、eye gaze、Node Constraint、SpringBoneのwriter競合も導入しない。

### Final control-order contract (Issue #19, amended by Issue #20)

Issue #19で実装が所有するwriterと制御順を確定し、Issue #20でbreathingのhips-translation writerを追記した。

| 順序 | writer / system | 所有する値 | 禁止事項 |
|---|---|---|---|
| 1 | Bevy `AnimationSystems` | animation base `Transform` | — |
| 2 | `apply_breathing_hips_translation` | additive `hips.translation`（idle breathing） | torso/arm/eye回転、scale、scene root、cameraの書き込み |
| 3 | `bevy_vrm1` direct-pose `BodyTracking` | spine〜headのtracked body rotation | arm、eye、world targetの書き込み |
| 4 | `apply_default_arm_pose` | upper/lower arm、optional shoulder/fingerのrest-relative local delta | handの直接world transform、head〜spineの上書き |
| 5 | direct head-relative LookAt / `VrmSystemSets::GazeControl` | eye-in-head gaze delta | synthetic world-space cursor/target |
| 6 | `ModifyExpressions` / `VrmSystemSets::Expressions` | supported expression weights | unsupported presetへの書き込み |
| 7 | `VrmSystemSets::Constraints` → propagation → SpringBone | VRM runtime constraints and physics | constraints／SpringBoneの無効化 |

Default-arm pose is a resolved generation-scoped state. It reads immutable
`RestTransform`／`RestGlobalTransform`, composes onto the current animation base,
and refreshes only the affected `ChildOf` subtrees. A missing or degenerate arm
side is a safe no-op; the avatar remains eligible for required head binding.
The local automated and manual-validation status for this contract is recorded
in `docs/DEFAULT_ARM_POSE_VALIDATION_2026-08-14.md`.

### Gaze

この節の旧MVP判断はADR-010で置換された。Webcam gazeはhead poseとは別の計測／フィルタ入力だが、適用時はhead-relativeなLookAt deltaである。モデル作者の`LookAtType`を尊重してBoneまたはExpressionを排他的に選び、tracked body pose後、Expression／Constraint／SpringBone前のVRM 1.0順序で解決する。

### Expressions

procedural expressionは`ModifyExpressions`へ統合し、1アバター・1フレーム最大1回triggerする。blink、mouth、gaze用に制御する全presetを0を含めて明示し、同一frameで`SetExpressions`と混在させない。

## Consequences

tracking coreはBevy／VRM座標を知らない。`vtuber-avatar`は既にcalibrationとsemantic座標変換が済んだyaw／pitch／rollを、既定のmirror-control adapterを通して`BodyTrackingPoseInput`へ渡し、bone Transformへの適用、rest-pose変換、animation base検出は`bevy_vrm1`の`BodyTracking`へ限定される。gazeは入力チャネルとして分離し、同じmirror-control adapterを通してADR-010のhead-relative直接LookAtへ渡す。
