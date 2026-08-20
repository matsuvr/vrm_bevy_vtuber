# ADR-016: MediaPipe to GNM sparse projection contract

Status: Accepted for Issue #50 child #53

## Decision

Keep the MediaPipe 478 to GNM sparse correspondence in one typed table inside
`vtuber-gnm`. The first cut uses a repository-owned iBUG/FAN-like 68 semantic
order and an explicit MediaPipe index for every row. It is a provisional
observation contract, not an upstream Google GNM correspondence claim; the
upstream request for an official correspondence remains open.

The fitting objective is weak perspective: GNM sparse 3D points are rotated,
scaled, and translated into normalized image `xy`, then compared to the
selected MediaPipe points. MediaPipe `z` is intentionally not treated as a
metric GNM coordinate. Positive static weights and zero-weight omission are
handled by a pure Rust helper; non-finite and out-of-range observations fail
with typed errors.

The camera convention is fixed in code: normalized image `x` points right,
normalized image `y` points down, and the displayed preview mirror is outside
this contract. The weak-perspective solver uses deterministic finite-difference
Gauss-Newton with a bounded 32-iteration budget and reports RMS residual and
valid point count.

## Provenance

- MediaPipe source contract: the repository's approved Face Landmarker 478
  output contract and the official MediaPipe Face Mesh connectivity/landmark
  semantics.
- GNM source contract: `head_sparse_68.txt` from Google GNM revision
  `970092e4b25be85adb1278ba015598342d43ef64`, recorded in ADR-015.
- Semantic ordering: iBUG/FAN-like naming only; no external landmark data file
  is copied and no official one-to-one MediaPipe correspondence is asserted.
- The mapping is deliberately replaceable as one table when stronger
  upstream-backed evidence becomes available.

## Scope boundary

This ADR does not estimate GNM identity or expression coefficients, does not
start camera/inference workers, and does not wire the result into avatar ECS.
Those responsibilities remain in later Issue #50 leaves.
