# ADR-001: 顔推論runtimeとmodel artifact

Status: Accepted with deviation; G0-05で確定済み
Date: 2026-08-04 (updated 2026-08-05)

## Context

本アプリはWindowsとmacOSで同じ顔追跡処理を実行し、配布時の推論runtimeをRustだけで構成する。顔追跡はmodel fileだけでは成立せず、detector、ROI、landmark、必要に応じたblendshape、前処理、後処理、tensor契約を一体として固定する必要がある。

TFLiteまたはONNXを読めることと、採用候補modelが実際に動くことは別である。unsupported operator、dynamic shape、quantization、custom postprocessのいずれかで失敗し得るため、runtime名を先に確定して実装を進めない。

## Fixed decision

- production runtimeはpure Rustとする。
- Python subprocess、MediaPipe native runtime、TensorFlow Lite C API、ONNX Runtime、OpenCV DNNをproduction pathへ含めない。
- WindowsとmacOSで同一artifact、同一前後処理、同一golden contractを使う。
- TFLite backendとONNX backendをproduction buildで同時に有効化しない。
- compatibility failure時にnative runtimeへ無断で切り替えない。

## Candidate order

1. `tract-tflite = 0.23.4` (ADR原文: 0.23.0; crates.ioに 0.23.0 が存在しないため 0.23.4 に修正)
2. `tract-onnx = 0.23.4`

## Resolution

- `tract-tflite` 0.23.4 で MediaPipe Face Landmarker `.task` 内の TFLite モデル (`face_detector.tflite`, `face_landmarks_detector.tflite`, `face_blendshapes.tflite`) および従来の `face_landmark.tflite` を読み込もうとすると、`F16 ADD` operator (builtin_code ADD, version 2) が unsupported として失敗する。
- 第二候補 `tract-onnx` 0.23.4 では、PINTO_model_zoo 経由で提供されている `peppapig_student_1x3x256x256.onnx` (Peppa_Pig_Face_Landmark, Apache-2.0) が正常に load / optimize でき、入力 `[1,3,256,256]` F32、出力 `[1,98,3]` F32 を確認した。
- 本 ADR の採用 runtime は `tract-onnx = 0.23.4` とし、production feature は `onnx` とする。TFLite パスは fallback として Cargo feature `tflite` にはせず、G0-05 時点では非採用と記録する。

## TFLite blocker record

- operator: `ADD` (builtin_code 6, version 2) with F16 tensor
- affected artifacts:
  - `face_landmarker.task` → extracted `face_detector.tflite`, `face_landmarks_detector.tflite`, `face_blendshapes.tflite`
  - `face_landmark.tflite` (legacy MediaPipe Face Mesh, float32)
- SHA-256: see `assets/models/manifest.toml`
- reproduction: `cargo test -p vtuber-inference --features onnx probe_face_detector -- --ignored --nocapture` (TFLite load fails)
- error pattern: `Unsupported: OperatorCode { deprecated_builtin_code: 6, custom_code: None, version: 2, builtin_code: ADD }, inputs: [16,3,3,3,F16 ...]`

## ONNX model provenance

- model: Peppa_Pig_Face_Landmark student 256x256 ONNX
- upstream: https://github.com/610265158/Peppa_Pig_Face_Landmark (Apache-2.0)
- converted/distributed by: PINTO0309/PINTO_model_zoo (conversion scripts MIT; model follows upstream license)
- download: https://s3.ap-northeast-2.wasabisys.com/pinto-model-zoo/436_Peppa_Pig_Face_Landmark/resources.tar.gz
- local file: `assets/models/peppapig_student_1x3x256x256.onnx`
- SHA-256: `73EDF90954F05EBEF4639E7FA8620C5F83CCA09D2476DE66AB100F26C2B25E7A`
- input: `[1,3,256,256]` F32 NCHW, ImageNet normalization (mean=[0.485,0.456,0.406], std=[0.229,0.224,0.225])
- output: `[1,98,3]` F32, 98 facial landmarks with confidence/visibility in third channel
- validation: `cargo test -p vtuber-inference --features onnx probe_peppapig_student_256 -- --ignored --nocapture` passes on Windows x86_64

第二候補へ進むのは、第一候補のblockerをoperator名、model SHA-256、入力shape、再現commandとともに記録した場合に限る。ONNX変換を行う場合はconverterのversionとcommandを固定し、original artifactとのgolden比較を必須とする。今回採用した ONNX artifact は PINTO_model_zoo が既に変換済みのものであり、upstream と同じ Apache-2.0 license の下で再配布可能である。

## G0-05 acceptance evidence

- detector／landmark／optional blendshape artifactの一次取得元
- licenseとredistribution条件
- SHA-256
- input dtype、shape、layout、normalization
- output indexまたはname、shape、意味
- ROI変換とlandmark座標の定義
- operator inventory
- fixed inputに対するgolden outputとtolerance
- WindowsとmacOSのload／optimize／run結果
- release buildのp50／p95 inference time
- model manifestとlicense file

## Consequences

model/runtime選定はblocking gateとなる。Gateを通るまで、UI、tracking core、VRM adapterを特定modelのtensor indexへ直接結合しない。`FaceInference`と`LandmarkSchemaId`を境界とし、model固有indexはschema adapterへ閉じ込める。
