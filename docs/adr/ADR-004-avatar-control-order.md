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

### Head／neck

`PostUpdate`で次の順序に置く。

```text
AnimationSystems
 -> ApplyTrackedHumanoidPose
 -> VrmSystemSets::Constraints
 -> PropagateAfterConstraints
```

VRM 1.0はbone local rotationがidentityとは限らない。model-space deltaを各boneのrest orientationへ共役変換する。

```text
R_bone_rest_model = inverse(R_root_rest_global) * R_bone_rest_global
R_delta_local     = inverse(R_bone_rest_model) * R_delta_model * R_bone_rest_model
R_output_local    = R_bone_rest_local * R_delta_local
```

MVPではVRMAを再生せず、毎frame rest poseから再計算して蓄積を防ぐ。animationとの合成は別ADRなしに追加しない。

### Gaze

`bevy_vrm1::LookAt`は使用しない。優先順位:

1. `lookLeft / lookRight / lookUp / lookDown` Expression
2. left／right eye bone
3. disabled

eye bone systemは`VrmSystemSets::GazeControl`へ置き、`PropagateAfterConstraints`後、Expression前に実行する。

### Expressions

procedural expressionは`ModifyExpressions`へ統合し、1アバター・1フレーム最大1回triggerする。blink、mouth、gaze用に制御する全presetを0を含めて明示し、同一frameで`SetExpressions`と混在させない。

## Consequences

tracking coreはBevy／VRM座標を知らず、符号変換とrest-pose変換は`vtuber-avatar`へ限定される。MVPでVRMAを使わないことにより、animation base検出という不確実性を持ち込まない。
