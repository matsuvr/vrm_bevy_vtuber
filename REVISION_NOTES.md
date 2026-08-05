# Revision Notes — 設計版2.0

基準日: 2026-08-04

## 変更の中心

VRM処理を独自実装する方針を廃止し、VRM 1.0専用の`bevy_vrm1`へ集約した。設計対象はWindowsとmacOSに限定し、顔追跡と低遅延pipelineを自由研究の主題として残した。

## 削除した実装範囲

- 独自`.vrm` AssetLoader
- 独自Humanoid registry／runtime
- 独自Expression resolver／morph accumulation
- 独自MToon shader／outline pass
- 独自SpringBone／Node Constraint
- VRM 1.0以外の互換layer
- desktop二系統以外のapplication／camera／package task

## 追加・強化した設計

- `bevy_vrm1` revision `f9593fd78136fb9e0507bcae111e09291ec9b82a`固定
- `vtuber-avatar`へVRM依存を隔離
- `Initialized`、Humanoid bone entity holder、`ExpressionEntityMap`、`ModifyExpressions`の利用
- Bevy Animation後、VRM Constraint前へのhead／neck適用
- VRM 1.0のrest rotationを考慮するmodel-space-to-local変換
- `LookAt`と`BodyTracking`を使わないgaze／head制御
- application-managed asset sourceとhash-based VRM import cache
- target-model compatibility gateとminimal upstream fork policy
- Windows MSMF／macOS AVFoundationを使うcamera設計
- macOS `.app` bundleでのcamera permission試験
- capacity-one latest-value pipeline

## 維持した研究テーマ

- pure-Rust face inference runtimeの成立性
- model provenance、license、SHA-256、tensor contract
- weighted Kabschによるneutral-relative head pose
- blink／mouth geometryまたはblendshape mapping
- One Euro Filter等の比較
- capture-to-apply latencyとjitterの定量評価
- tracking lossとneutral return
