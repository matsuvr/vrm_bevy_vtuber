# ADR-015: GNM Head v3 Rust model boundary

Status: Accepted for Issue #50 child #52

## Decision

Add `crates/vtuber-gnm` as the engine-neutral Rust boundary for the official
GNM Head v3 data contract. The crate validates the required NPZ arrays, rejects
unsupported versions/variants, non-finite values, malformed shapes and invalid
indices, and evaluates only the requested sparse landmarks into a reusable
output buffer. It does not render dense meshes, depend on Bevy, or decide how
GNM state is retargeted to ARKit52.

The official sparse 68-point landmark table is embedded as source data. The
model NPZ remains an external runtime asset: this change deliberately does not
commit or download the approximately 53 MB GNM model binary.

## Provenance and licensing

- Source repository: [google/GNM](https://github.com/google/GNM)
- Source revision: `970092e4b25be85adb1278ba015598342d43ef64`
- Schema source: `gnm/shape/gnm_data_schema.py`
- Model source path: `gnm/shape/data/versions/v3_0/gnm_head.npz`
- Sparse source path: `gnm/shape/data/landmarks/head_sparse_68.txt`
- Upstream license: Apache-2.0
- Sparse table SHA-256: must be recorded when the release asset manifest is
  added; no release asset is included in this change.
- GNM model NPZ SHA-256: `PENDING` because the binary is not downloaded or
  redistributed by this change.

The exact NPZ hash, redistribution status, and model manifest entry are a
release-packaging responsibility. A caller must supply the archive path to
`load_gnm_head_v3`; missing or changed arrays fail before evaluation.

## Dependency

The crate uses `zip = 6.0.0` with the `deflate-flate2-zlib-rs` feature to read
NPZ containers without a Python process or a native inference runtime. The
crate remains `forbid(unsafe_code)` and has no Bevy or MediaPipe dependency.
