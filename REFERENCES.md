# References

基準日: 2026-08-04

実装時には検索結果や古いtutorialではなく、固定version／revisionの一次資料とsourceを確認する。URLの`latest`は探索用に限定し、実装判断には記載したtagまたはcommitを使う。

## Bevy 0.19.0

- Release notes: https://bevy.org/news/bevy-0-19/
- Crate documentation: https://docs.rs/bevy/0.19.0/bevy/
- Source tag: https://github.com/bevyengine/bevy/tree/v0.19.0
- Main schedule order: https://github.com/bevyengine/bevy/blob/v0.19.0/crates/bevy_app/src/main_schedule.rs
- Animation API: https://github.com/bevyengine/bevy/blob/v0.19.0/crates/bevy_animation/src/lib.rs
- glTF loader: https://github.com/bevyengine/bevy/blob/v0.19.0/crates/bevy_gltf/src/loader/mod.rs
- glTF extension handlers: https://github.com/bevyengine/bevy/blob/v0.19.0/crates/bevy_gltf/src/loader/extensions/mod.rs
- glTF coordinate conversion: https://github.com/bevyengine/bevy/blob/v0.19.0/crates/bevy_gltf/src/convert_coordinates.rs
- Asset application APIs: https://docs.rs/bevy/0.19.0/bevy/asset/trait.AssetApp.html
- Asset path: https://docs.rs/bevy/0.19.0/bevy/asset/struct.AssetPath.html
- Unapproved path policy: https://docs.rs/bevy/0.19.0/bevy/asset/enum.UnapprovedPathMode.html

## bevy_vrm1

Design baseline commit:

```text
f9593fd78136fb9e0507bcae111e09291ec9b82a
```

- Repository: https://github.com/not-elm/bevy_vrm1
- Pinned README: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/README.md
- Pinned Cargo.toml: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/Cargo.toml
- Main VRM plugin: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/vrm.rs
- `.vrm` loader wrapping Bevy `GltfLoader`: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/vrm/loader.rs
- Initialization lifecycle: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/vrm/initialize.rs
- Humanoid bone binding: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/vrm/humanoid_bone.rs
- Humanoid public bone components: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/vrm/humanoid_bone/bones.rs
- Expression control: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/vrm/expressions.rs
- Expression example: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/examples/expressions.rs
- LookAt implementation: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/vrm/look_at.rs
- BodyTracking implementation: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/vrm/body_tracking.rs
- Runtime system sets: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/system_set.rs
- MToon material: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/src/vrm/mtoon/material.rs
- CI matrix: https://github.com/not-elm/bevy_vrm1/blob/f9593fd78136fb9e0507bcae111e09291ec9b82a/.github/workflows/ci.yml
- Spec-compliance and loader discussion: https://github.com/not-elm/bevy_vrm1/issues/64

## VRM 1.0 specification

- VRM 1.0 overview: https://vrm.dev/en/vrm1/
- Developer characteristics and T-pose direction: https://vrm.dev/en/vrm/vrm_development/
- Coordinate conversion notes: https://vrm.dev/api/coordinate/
- VRM 1.0 glTF storage details: https://vrm.dev/vrm/gltf/vrm10_details/
- VRMC_vrm 1.0: https://github.com/vrm-c/vrm-specification/tree/master/specification/VRMC_vrm-1.0
- Humanoid: https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/humanoid.md
- Expressions: https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/expressions.md
- LookAt: https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/lookAt.md
- First Person: https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/firstPerson.md
- MToon 1.0: https://github.com/vrm-c/vrm-specification/tree/master/specification/VRMC_materials_mtoon-1.0
- SpringBone 1.0: https://github.com/vrm-c/vrm-specification/tree/master/specification/VRMC_springBone-1.0
- Node Constraint 1.0: https://github.com/vrm-c/vrm-specification/tree/master/specification/VRMC_node_constraint-1.0
- Runtime update order: https://vrm.dev/en/api/api_update/
- Official samples: https://github.com/vrm-c/vrm-specification/tree/master/samples

## Camera

- `nokhwa` 0.10.11 source tag: https://github.com/l1npengtul/nokhwa/tree/0.10.11
- `nokhwa` public API: https://github.com/l1npengtul/nokhwa/blob/0.10.11/src/lib.rs
- macOS initialization contract: https://github.com/l1npengtul/nokhwa/blob/0.10.11/src/init.rs
- Feature list: https://docs.rs/crate/nokhwa/0.10.11/features
- Apple `NSCameraUsageDescription`: https://developer.apple.com/documentation/bundleresources/information-property-list/nscamerausagedescription
- Microsoft Media Foundation overview: https://learn.microsoft.com/windows/win32/medfound/microsoft-media-foundation-sdk

## Face tracking and pure-Rust inference

- MediaPipe Face Landmarker guide: https://ai.google.dev/edge/mediapipe/solutions/vision/face_landmarker
- MediaPipe source: https://github.com/google-ai-edge/mediapipe
- `tract` source: https://github.com/sonos/tract
- `tract-tflite` v0.23.0 source: https://github.com/sonos/tract/tree/v0.23.0/tflite
- `tract-onnx` 0.23.4 docs: https://docs.rs/tract-onnx/0.23.4/tract_onnx/

## Mathematics and filtering

- Kabsch, W. “A solution for the best rotation to relate two sets of vectors.” Acta Crystallographica A, 1976／1978.
- Schönemann, P. “A generalized solution of the orthogonal Procrustes problem.” Psychometrika, 1966.
- Casiez, G.; Roussel, N.; Vogel, D. “1€ Filter: A Simple Speed-based Low-pass Filter for Noisy Input in Interactive Systems.” CHI 2012.
- `nalgebra` SVD documentation: https://docs.rs/nalgebra/latest/nalgebra/linalg/struct.SVD.html

## Packaging

- Apple bundle resources: https://developer.apple.com/documentation/bundleresources
- Apple code signing: https://developer.apple.com/support/code-signing/
- Rust platform support: https://doc.rust-lang.org/rustc/platform-support.html
