# UltraFace RFB-320 operator inventory blocker

Status: BLOCKED

## Exact artifact identity

- model ID: ultraface-rfb-320
- file: assets/models/version-RFB-320.onnx
- expected byte size: 1270727
- expected SHA-256: 34CD7E60AEFF28744C657DE7A3DC64E872D506741DE66987F3426F2B79F88017
- source: https://huggingface.co/onnxmodelzoo/version-RFB-320/resolve/main/version-RFB-320.onnx
- runtime under test: tract-onnx = 0.23.4

## Reproduction

From the repository root on 2026-08-11:

    Test-Path assets/models/version-RFB-320.onnx
    # False

    cargo run -p xtask -- acceptance verify assets/models/manifest.toml
    # verify failed: model artifact 'ultraface-rfb-320' is missing: assets/models\version-RFB-320.onnx

    cargo test -p vtuber-inference ultraface_probe --features onnx -- --nocapture
    # EXACT_ULTRAFACE_PROBE_BLOCKED:
    # model_id=ultraface-rfb-320 sha256=unknown stage=artifact_read ...

The official model page exposes the exact SHA-256 and file size, but the
browser download was not materialized into this repository workspace. No
operator inventory or load/optimize/run result is claimed until the exact
bytes are present locally.

The next repair leaf must supply this exact file, rerun the probe, and replace
this blocker record with the stable operator inventory. Do not substitute an
int8 variant, a different UltraFace variant, or another runtime in this leaf.
