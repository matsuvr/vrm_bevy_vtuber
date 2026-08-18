# NDI release and OBS interoperability

Status as of 2026-08-18: the source/build boundary and guarded Windows x64
package staging command are implemented. A distributable release is still
NOT RELEASE READY until the exact SDK package, license agreement, runtime
hash, clean-machine run, and receiver roundtrip are recorded.

NDI® is a registered trademark of Vizrt NDI AB. The application uses NDI only
to identify compatibility; it is not an NDI product and this repository does
not claim sponsorship. The official link is present in the Live UI and in
THIRD_PARTY_NOTICES.md.

## Fixed release boundary

The first release target is Windows x86_64. The normal workspace build remains
SDK-free. Only the explicit ndi-output feature requires the locally installed
NDI SDK headers and bindgen environment:

~~~~text
cargo build -p vtuber-desktop --release --features ndi-output
~~~~

The runtime is not committed to Git and must not be copied to System32, PATH,
or an NDI Tools directory. The package contains the application, the
explicitly supplied Standard SDK x64 runtime DLL, the exact license/notice
file supplied by that SDK package, the project notices, the approved MediaPipe
task bundle with its manifest and license, and a generated hash manifest.

## Exact SDK license gate

The license is a release gate, not an assumption made by the packaging tool.
The exact license agreement shipped with the SDK package used for the build
must be checked again before each release. In particular, the agreement and
SDK documentation determine which object-code files may be distributed and
which end-user restrictions, NDI notices, trademark wording, export
conditions, and SDK freshness requirements apply.

The current official developer page identifies the NDI SDK as version 6.3.2
on this date. The local package command still requires the actual package
version and package SHA-256 from the build machine; it never infers these from
the web page.

References:

- [NDI for Developers](https://ndi.video/for-developers/)
- [NDI SDK license agreement](https://downloads.ndi.tv/SDK/NDI_SDK/NDI%20SDK%20License%20Agreement.pdf)
- [NDI SDK dynamic loading and application-local runtime guidance](https://docs.ndi.video/all/developing-with-ndi/sdk/dynamic-loading-of-ndi-libraries)
- [NDI SDK documentation](https://docs.ndi.video/all/developing-with-ndi/sdk)

If the exact agreement does not permit this application's intended
application-local distribution, do not bundle the DLL. Record the exact
blocker against Issue #45 and stop the release path.

## Reproducible package staging

Obtain the exact Standard SDK x64 runtime DLL and license agreement through the
approved SDK package channel. Do not use NDI Tools as a substitute. Record the
SDK package archive SHA-256, then run:

~~~~powershell
$sdkPackageSha256 = '<64 hexadecimal characters>'
cargo run -p xtask -- ndi package --output target/ndi-package --runtime-dll 'C:\path\to\Processing.NDI.Lib.x64.dll' --sdk-license 'C:\path\to\NDI SDK License Agreement.pdf' --sdk-version '6.3.2' --sdk-package-sha256 $sdkPackageSha256 --force
cargo run -p xtask -- ndi verify-package target/ndi-package
~~~~

The staging command rejects another DLL name, requires the project
attribution/link, records runtime/license/face-task hashes, and rejects extra
files outside the allow-listed model resource directory. It does not download
or delete the SDK, and it does not make a legal determination.

Expected top-level package:

~~~~text
vtuber-desktop.exe
Processing.NDI.Lib.x64.dll
NDI_SDK_LICENSE_AGREEMENT.pdf
THIRD_PARTY_NOTICES.md
NDI_RUNTIME_MANIFEST.txt
assets/
  models/
    face_landmarker.task
    manifest.toml
    LICENSE.mediapipe.txt
~~~~

The generated manifest must say application_local=true,
system_path_install=false, and must explicitly say that NDI Tools,
Advanced/HX, and audio components are absent.

## OBS and DistroAV receiver procedure

The receiver is not redistributed by this repository. Install OBS and
DistroAV separately on the receiver machine, then add an NDI Source and
select vrm-bevy-vtuber. As of this report, the DistroAV README states OBS
31.1.1 or newer and NDI Runtime 6.3 or newer as requirements; re-check the
current receiver release before a release report is signed.

- [DistroAV repository and installation requirements](https://github.com/DistroAV/DistroAV)
- [DistroAV NDI source mapping](https://github.com/DistroAV/DistroAV/blob/master/src/ndi-source.cpp)

No firewall rule is installed by this application. If discovery fails, check
that the two hosts are on the intended LAN and that local firewall policy
permits the receiver workflow.

## Machine-readable roundtrip evidence

The receiver-side harness must write a UTF-8 key=value capture manifest after
it has discovered the source, captured frames, and observed sender shutdown.
The repository validates the normative assertions without pretending that a
Rust-only unit test received an NDI packet:

~~~~powershell
cargo run -p xtask -- ndi verify-roundtrip path\to\ndi-roundtrip.txt
~~~~

The manifest must contain source_name=vrm-bevy-vtuber, four_cc=BGRA,
width=1920, height=1080, fps=60, connection_count at least one, frame_count
at least two, distinct_frame_hashes at least two, positive counts for alpha
zero/opaque/partial pixels, transparent_rgb_zero=true, sender_stopped=true,
stop_source_absent=true, render_blocked=false, and queue_depth_max no greater
than one. This is the machine-runnable assertion boundary; an NDI
SDK/runtime receiver harness must supply the manifest on a validation
machine. No such receiver harness was available for this local run.

## Acceptance evidence

| Gate | Result on 2026-08-18 | Evidence |
|---|---|---|
| SDK-free default workspace build | PASS | cargo check --workspace |
| SDK-free unit/workspace tests | PASS | cargo test --workspace --no-fail-fast --quiet |
| package layout/hash validator | PASS | cargo test -p xtask, plus a staged-package run when SDK input exists |
| exact SDK license/package hash | NOT RUN | NDI SDK is not installed on this development machine |
| NDI-enabled release build | NOT RUN | SDK headers are absent |
| NDI sender/receiver numeric alpha roundtrip | NOT RUN | no SDK/runtime receiver environment |
| clean Windows machine without SDK/Tools/toolchain | NOT RUN | no clean machine run available |
| OBS + DistroAV smoke | NOT RUN | no OBS/DistroAV GUI environment available |

The following are required before calling the release self-contained:

1. Capture multiple frames through an NDI receiver and assert configured
   dimensions/FPS, BGRA/RGBA alpha, transparent/opaque/semitransparent pixels,
   changing frame hashes, and clean sender stop.
2. Repeat with a stopped or intentionally slow receiver and record that the
   application render loop remains responsive and the latest-value mailbox
   stays bounded.
3. Run the package on a clean Windows x64 machine with no SDK, NDI Tools,
   system-wide NDI Runtime, Visual Studio, LLVM, bindgen, or PATH edits.
4. Run the OBS/DistroAV smoke and attach the machine, package, SDK, receiver,
   commands, and results to the release report.

These physical/network checks are intentionally not represented as automated
PASS results.
