# ADR-018: GNM neutral identity と bounded expression fitting

- Status: Accepted for Issue #50 / #54
- Date: 2026-08-21

## Context

GNM Head v3 は 253 identity 成分と 383 expression 成分を持つが、first-cut
入力は 68 点の 2D sparse observation である。この観測だけで全成分を
無正則化で解くことは under-determined で、identity が毎フレーム表情を
吸収する退行も起こり得る。

## Decision

`vtuber-gnm` に engine-neutral な `GnmFaceFitter` を追加する。fitter は
validated `GnmModel` と sparse landmark set を受け取り、MediaPipe の image
や Bevy/VRM 型を保持しない。

first cut の active subspace は、モデルの先頭から選ぶ明示的な有限次元
（default: identity 8、expression 16）とする。inactive component は状態の
full-size vector では 0 のままにし、全 253/383 成分が観測済みであるとは
主張しない。各 solve は ridge regularization、bounded alternating iteration、
係数 bound、normal-equation の condition estimate を持つ。ill-conditioned
solve は typed error として成功扱いしない。

neutral calibration は expression を zero に固定し、複数 sample から
identity を一度だけ求めて固定する。runtime expression fitting はその固定
identity 上で、前フレーム expression を temporal prior として用いる。
source sequence の regression は拒否し、gap は expression/camera の prior
を neutral に reset する。camera convention と 2D projection は #53 の
`project_weak_perspective` / `fit_weak_perspective` を再利用する。

出力は `GnmIdentityCalibration` と `GnmFaceState` で、residual、active
dimension、rank/condition、confidence、`Uncalibrated`/`Tracking`/`Degraded`
status を保持する。VRM/ARKit52 への変換は後続 #55 に残す。

## Consequences

- neutral identity が frame ごとに動いて expression を吸収する実装を防ぐ。
- active dimension、regularization、solver iteration が machine-runnable な
  synthetic tests で確認できる。
- first-cut は leading component policy であり、official GNM basis の全成分
  を解くものではない。region-aware/SVD の active subspace 置換は別途、根拠と
  conditioning report を伴う変更にする。
- 実 GNM binary、実カメラ、VRM 視覚確認、Windows/macOS 実機確認は未実施で、
  この ADR の automated result からは推論しない。
