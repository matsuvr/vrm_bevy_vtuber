# ADR-004: tracking結果のVRM適用順と座標変換

Status: Accepted  
Date: 2026-08-04

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

### Gaze

`bevy_vrm1::LookAt`は使用しない。優先順位:

1. `lookLeft / lookRight / lookUp / lookDown` Expression
2. left／right eye bone
3. disabled

eye bone systemは`VrmSystemSets::GazeControl`へ置き、`PropagateAfterConstraints`後、Expression前に実行する。

### Expressions

procedural expressionは`ModifyExpressions`へ統合し、1アバター・1フレーム最大1回triggerする。blink、mouth、gaze用に制御する全presetを0を含めて明示し、同一frameで`SetExpressions`と混在させない。

## Consequences

tracking coreはBevy／VRM座標を知らない。`vtuber-avatar`は既にcalibrationとsemantic座標変換が済んだyaw／pitch／rollを`BodyTrackingPoseInput`へ渡し、bone Transformへの適用、rest-pose変換、animation base検出は`bevy_vrm1`の`BodyTracking`へ限定される。eye gazeは独立したGazeControl経路を維持する。
