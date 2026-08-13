# ADR-004: tracking結果のVRM適用順と座標変換

Status: Accepted  
Date: 2026-08-04
Amended: 2026-08-14

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
 -> DirectPoseBodyTracking
 -> VrmSystemSets::Constraints
 -> PropagateAfterConstraints
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

### Gaze

この節の旧MVP判断はADR-010で置換された。Webcam gazeはhead poseとは別の計測／フィルタ入力だが、適用時はhead-relativeなLookAt deltaである。モデル作者の`LookAtType`を尊重してBoneまたはExpressionを排他的に選び、tracked body pose後、Expression／Constraint／SpringBone前のVRM 1.0順序で解決する。

### Expressions

procedural expressionは`ModifyExpressions`へ統合し、1アバター・1フレーム最大1回triggerする。blink、mouth、gaze用に制御する全presetを0を含めて明示し、同一frameで`SetExpressions`と混在させない。

## Consequences

tracking coreはBevy／VRM座標を知らない。`vtuber-avatar`は既にcalibrationとsemantic座標変換が済んだyaw／pitch／rollを、既定のmirror-control adapterを通して`BodyTrackingPoseInput`へ渡し、bone Transformへの適用、rest-pose変換、animation base検出は`bevy_vrm1`の`BodyTracking`へ限定される。gazeは入力チャネルとして分離し、同じmirror-control adapterを通してADR-010のhead-relative直接LookAtへ渡す。
