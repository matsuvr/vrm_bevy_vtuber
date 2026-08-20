# ADR-019: GNM expression から ARKit52 への regularized decoder

- Status: Accepted for Issue #50 / #55
- Date: 2026-08-21

## Context

GNM expression state から Perfect Sync 用 ARKit52 coefficient を作るには、
MediaPipe teacher score を runtime でそのまま返す shortcut を禁止し、同期済み
numeric pair から検証済み decoder を学習する必要がある。MediaPipe teacher は
共通 51 channel の学習信号であり、TongueOut の教師は存在しない。

## Decision

`vtuber-gnm` に `GnmDecoderTrainer` と frozen な
`GnmToArkit52Decoder` を追加する。trainer は source sequence、active GNM
expression、teacher coefficient、GNM fit confidence、reprojection residual
だけを受け取る。品質 gate を通った sample のみで ridge-regularized linear
solve を行い、active subspace dimension、rank、conditioning、per-channel
variance/reliability、train residual を diagnostics に保存する。

runtime の `decode` API は `GnmFaceState` だけを引数に取り、MediaPipe
coefficient や画像にアクセスできない。出力は有限かつ `[0, 1]` に clamp し、
TongueOut は常に 0 で `Unobserved` と記録する。neutral-heavy coverage は
decoder object の作成と `ready` 判定を分離し、reliable channel が不足する
場合に Ready と誤判定しない。unsupported GNM version、active subspace
mismatch、invalid runtime state は typed error とする。

## Consequences

- GNM state が runtime の output authority になり、teacher passthrough を API
  構造で防止できる。
- decoder は first-cut の active subspace に限定され、全 383 成分や TongueOut
  を観測済みとは主張しない。
- persistent model serialization と application mode switching は後続 #57
  の責務として残す。
- 実カメラ teacher capture、実 GNM binary、VRM 視覚確認、Windows/macOS 実機
  確認は未実施である。
