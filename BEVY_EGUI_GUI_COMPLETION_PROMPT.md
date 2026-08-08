# Coding-agent prompt: complete the desktop GUI with `bevy_egui`

Implement the missing production GUI in `https://github.com/matsuvr/vrm_bevy_vtuber`.

## Operating context

- Inspect the current repository before editing. The repository, not this prompt, is authoritative when names or paths differ.
- The implementation has progressed through **M1-08**. Preserve all existing work and tests.
- Do not renumber, reopen, or reimplement the existing `G0-*` / `M1-*` tasks.
- Treat this as a focused corrective completion of the GUI that was left as a stub in M1-07.
- Windows is the only acceptance platform for this task.
- Do not perform macOS acceptance, packaging, permission work, or CI changes. Preserve existing macOS-compatible source where that requires no extra work, but do not block completion on macOS.
- Do not add Android, Linux-product, WebAssembly, or mobile code.
- Do not commit, push, or open a PR unless explicitly instructed.

The current repository is expected to contain:

- Bevy `=0.19.0`.
- `bevy_vrm1` pinned to commit `f9593fd78136fb9e0507bcae111e09291ec9b82a`.
- `UiAction`, `UiViewModel`, `UiState`, `Orchestrator`, `DiagnosticsSnapshot`, `PreviewState`, and `ErrorPresenter` scaffolding.
- `crates/vtuber-app/src/ui/shell.rs` with an `ui_stub_system`.
- A commented-out `bevy_egui` dependency whose comment incorrectly says a Bevy 0.19-compatible version is unavailable.
- `apps/desktop/src/main.rs` that currently does not install `EguiPlugin` or `UiShellPlugin`.

Do not discard or replace those boundaries.

## Decision already made

Use exactly:

```toml
bevy_egui = { version = "=0.41.1", default-features = false, features = ["default_fonts", "render"] }
rfd = { version = "=0.17.2", default-features = false }
```

Rationale and constraints:

- `bevy_egui 0.41.1` explicitly targets Bevy 0.19.
- Do not use `bevy_egui` default features. They additionally enable `picking`, `bevy_ui`, clipboard management, and URL opening. This application does not need them for the MVP, and `picking` can interfere with pointer input over a 3D viewport.
- Do not add `bevy-inspector-egui`, `bevy_feathers`, `bevy_ui` widget frameworks, `bevy_lunex`, `bevy_hui`, a theme framework, or a custom renderer.
- Do not depend directly on `egui`; use the `egui` re-export from `bevy_egui` so the versions cannot diverge.
- Put the exact dependency declarations in `[workspace.dependencies]`, then use `workspace = true` from crates that directly import them.

Before editing, inspect the locally downloaded source or official 0.41.1 examples for:

- `bevy_egui/examples/simple.rs`
- `bevy_egui/examples/side_panel.rs`
- `bevy_egui/examples/absorb_input.rs`
- `bevy_egui/examples/file_browse.rs`

Do not copy APIs from an older `bevy_egui` release.

## Architectural rules

1. `apps/desktop` owns installation of `EguiPlugin::default()`.
2. `EguiPlugin::default()` must be added before `UiShellPlugin`.
3. `UiShellPlugin` must not install `EguiPlugin` itself. It may assert with a clear message that `EguiPlugin` was already installed.
4. All egui drawing systems must run in `EguiPrimaryContextPass`, not in the ordinary `Update` schedule.
5. Keep `process_ui_actions_system` in `Update`. `bevy_egui` runs the primary context pass before `Update`, so actions emitted by the GUI can be processed through the existing action boundary without calling domain services from the drawing system.
6. The drawing system may read immutable UI snapshots and append `UiAction` values to `UiState`. It must not call camera, import, tracking, filesystem-read, worker, or VRM services directly.
7. Native file selection is a UI concern. It may use `rfd::AsyncFileDialog`, but after selecting a path it must emit `UiAction::ImportAvatar`; it must not parse or copy the model itself.
8. Never use the blocking `rfd::FileDialog::pick_file()` on the Bevy main thread. Follow the official `bevy_egui` file-browse example: start an async dialog task and poll it without blocking the render loop.
9. Do not globally enable `EguiGlobalSettings::enable_absorb_bevy_input_system`. The global event-clearing mode is intentionally avoided. Since this task disables the `picking` feature and the current application has no interactive world controls, no further input interception is required. If existing world input systems are found, gate only those systems with `egui_wants_any_pointer_input` / `egui_wants_any_keyboard_input`.
10. Do not add a second renderer, a second window, docking, charts, or a custom texture pipeline.

## Camera/context policy

Use the existing first window-targeting `Camera3d` as the primary egui context through `bevy_egui`'s normal automatic context creation.

- Add `EguiPlugin::default()` before `VtuberAvatarPlugin` and `UiShellPlugin` in `apps/desktop/src/main.rs`.
- Do not add another `Camera2d` or overlay camera in this patch unless the automatic primary context demonstrably fails with the current repository.
- Do not change the VRM camera, projection, render layers, MToon setup, or `bevy_vrm1` systems merely to make the GUI appear.
- If automatic context creation fails, document the exact reproduction first, then use the official 0.41.1 `side_panel.rs` pattern with an explicitly marked UI camera. Do not improvise a render graph.

## Required implementation

### 1. Dependency and plugin wiring

Update only the necessary manifests and entry point.

- Add exact workspace dependencies for `bevy_egui` and `rfd`.
- Remove the obsolete commented dependency and comment.
- Make `vtuber-app` depend on both workspace dependencies.
- Make `vtuber-desktop` depend on `bevy_egui` because it installs `EguiPlugin` directly.
- In `apps/desktop/src/main.rs`, install plugins in this logical order:

```rust
DefaultPlugins
EguiPlugin::default()
VtuberAvatarPlugin
UiShellPlugin
```

Equivalent tuple syntax is acceptable only if the actual initialization order remains explicit and reliable.

### 2. Replace the UI stub

Replace `ui_stub_system` with an actual egui drawing system in `EguiPrimaryContextPass`.

Retain the current `UiAction` / `UiViewModel` / `UiState` boundary. Do not expose `bevy_vrm1` types to the UI crate.

Use a compact desktop layout:

- A top status/navigation bar containing Setup, Live, and Diagnostics tabs.
- A fixed or resizable left control panel, approximately 320–380 logical pixels wide.
- The remaining window area stays available for the 3D VRM scene. Do not paint an opaque full-window `CentralPanel` over the model.
- Use a simple functional dark style. No design system or theme dependency.

Split rendering into small pure or mostly pure functions, for example:

```text
ui/mod.rs
ui/shell.rs
ui/setup.rs
ui/live.rs
ui/diagnostics.rs
ui/error.rs
```

Reuse existing files when present; do not create duplicate modules with the same responsibility.

### 3. Navigation and UI-local state

Make navigation actually work through `UiAction::SwitchScreen`.

- Do not mutate `UiViewModel` directly from egui.
- Add the smallest necessary orchestrator state so that `SwitchScreen` is processed and `UiViewModel.screen` is refreshed from the orchestrator.
- Preserve the existing `UiViewModel` predicates such as `can_start`, `can_stop`, and `can_calibrate`.
- Disable unavailable controls rather than allowing invalid operations and relying only on an error afterward.
- Show a brief reason beside or below disabled Start and calibration controls.

Because egui may perform more than one UI pass, make identical one-shot actions idempotent within the pending batch. The minimal acceptable implementation is for `UiState::emit` to avoid inserting the exact same `UiAction` twice before `take_actions()` drains the batch. Add a unit test.

Do not build a general command bus or event-sourcing layer.

### 4. Setup screen

Render from the existing view model and emit the existing actions.

Required controls:

- Refresh camera list.
- Camera combo box using the enumerated descriptors.
- Import VRM 1.0 button.
- Imported avatar summary: name, short ID, required-bone status, expression count, and original file name only. Do not show the full local path by default.
- Start button, enabled only when `UiViewModel::can_start()` is true.
- Stop button when the pipeline can stop.
- Unload avatar button when an avatar exists.
- Current application lifecycle and avatar lifecycle.

File dialog behavior:

- Use `rfd::AsyncFileDialog`.
- Filter for `vrm`.
- Keep the async task in a local/resource state and poll it without blocking.
- After a result arrives, emit exactly one `UiAction::ImportAvatar { path }` and clear the task.
- Ignore cancellation without reporting an error.
- While the dialog is active, prevent opening a duplicate dialog.
- Do not read the selected file from the drawing system.

Also accept drag-and-drop when `bevy_egui` exposes dropped files through the egui context:

- Accept only the first regular path whose extension is `.vrm`, case-insensitively.
- Emit `UiAction::ImportAvatar` once.
- Do not import arbitrary bytes or URLs.

### 5. Live screen

Required display and controls:

- Lifecycle state.
- Tracking state, face-detected state, and confidence.
- Calibration progress, target sample count, quality, and last rejection reason.
- Begin, Cancel, and Retry calibration buttons using the existing actions and predicates.
- Preview visibility and mirror toggles using `TogglePreview` and `ToggleMirror`.
- Start/Stop as appropriate.

Preview handling:

- Reuse the existing `PreviewState.image_handle` and Bevy `Image` asset.
- Do not create an `Image` asset per frame.
- If a preview handle exists, register/show it through `EguiContexts` / `EguiUserTextures` using the 0.41.1 API.
- Apply mirroring only through image UV/display coordinates. Do not alter inference or tracking data.
- If no image is currently available, show a fixed-size unobtrusive placeholder saying that the preview is unavailable. Do not synthesize fake camera frames.
- Turning preview off must hide only the widget and must not stop workers.

Process `TogglePreview` and `ToggleMirror` at the existing application action boundary. Keep `PreviewState`, `UiViewModel.preview_visible`, and `UiViewModel.mirror_preview` consistent. Do not put camera frames in `UiViewModel`.

### 6. Diagnostics screen

Read the existing `DiagnosticsSnapshot` resource without retaining unbounded history.

Show at least:

- Render FPS.
- Capture rate.
- Inference rate.
- Tracking state.
- Slot overwrite count.
- Stage timings.
- Model hash short value.
- Camera backend.
- Avatar capability summary.
- Last technical error summary.

Use labels, a two-column `egui::Grid`, and separators. Do not add a chart dependency.

### 7. Recoverable errors

Initialize and use the existing `ErrorPresenter`.

- Synchronize it from `Orchestrator::last_error()` in an ordinary Bevy system, not inside widget rendering.
- Render the current safe user message and stable code in a compact error panel/banner.
- Render suggested recovery actions as buttons that emit the existing `UiAction` values.
- Do not display full file paths, raw frames, landmarks, or an unbounded error stack.
- Dismissal must emit `UiAction::DismissError`; the egui renderer must not clear domain error state directly.

### 8. Resource/plugin registration

`UiShellPlugin` should initialize only the resources it owns or consumes when not already initialized:

- `UiState`
- `UiViewModel`
- `Orchestrator`
- `PreviewState`
- `DiagnosticsSnapshot`
- `ErrorPresenter`

Do not silently initialize a second instance of a resource that another plugin already owns. Inspect the current app wiring and use `init_resource` only where appropriate.

Register:

- egui rendering in `EguiPrimaryContextPass`.
- action processing / view-model synchronization in `Update`.
- error-presenter synchronization in `Update`, ordered after action processing.

Do not move domain simulation, VRM pose application, SpringBone, or worker updates into the egui schedule.

## Tests

Add focused tests without requiring a physical camera or VRM asset.

Required automated coverage:

1. `UiState::emit` does not queue the exact same one-shot action twice before drain.
2. `take_actions()` still drains and allows the same action in a later frame/batch.
3. Screen navigation updates `UiViewModel.screen` only through action processing.
4. Rendering Setup, Live, and Diagnostics with empty/default snapshots does not panic. Use a standalone `egui::Context` test or small pure rendering functions; do not initialize a GPU just for this test.
5. Disabled-state predicates remain authoritative for Start/Stop/calibration.
6. Preview mirror/visibility actions update UI-facing state without modifying tracking data.
7. File-dialog completion-to-`ImportAvatar` conversion is isolated enough to test without opening a real dialog; factor the path-to-action conversion into a small pure helper.
8. Existing M1-08 tests remain unchanged and passing.

Do not test egui itself.

## Windows manual smoke protocol

Run only on Windows for this task. Record what was actually executed.

1. Launch `vtuber-desktop` without a model.
2. Confirm the 3D scene remains visible and the GUI appears.
3. Switch among Setup, Live, and Diagnostics.
4. Refresh cameras and select an available entry.
5. Open and cancel the VRM dialog; confirm no error.
6. Select a valid `.vrm`; confirm one import action and a visible model summary.
7. Confirm Start is disabled until camera and avatar preconditions are met.
8. Exercise Start/Stop using the current orchestrator implementation.
9. Toggle preview and mirror; confirm no pipeline stop and no crash when no preview texture exists.
10. Open Diagnostics and confirm current snapshot values render.
11. Trigger a recoverable validation error and confirm Dismiss/Retry-related actions remain usable.
12. Resize the window and verify the panel remains usable and the VRM scene is not covered by an opaque full-window panel.

Do not claim hardware or visual checks that were not actually performed.

## Required validation commands

Run from the workspace root:

```powershell
cargo fmt --all -- --check
cargo check -p vtuber-app -p vtuber-desktop
cargo clippy -p vtuber-app -p vtuber-desktop --all-targets -- -D warnings
cargo test -p vtuber-app
cargo test -p vtuber-desktop
cargo tree -p vtuber-desktop -d
```

Inspect `cargo tree -p vtuber-desktop -d` and confirm there is no duplicate Bevy 0.19/egui family caused by an incorrect integration version. Do not mechanically fail because unrelated transitive crates have multiple versions; report only duplicates relevant to Bevy, egui, winit, or wgpu integration.

Run the Windows desktop application when the environment permits:

```powershell
cargo run -p vtuber-desktop
```

Do not add macOS validation commands to this task.

## Scope exclusions

Do not:

- Rewrite the existing action/view-model architecture.
- Replace `bevy_vrm1` or modify its pinned revision.
- Modify face inference, calibration math, head-pose math, expression mapping, SpringBone, or MToon.
- Complete unrelated camera/worker TODOs merely because the GUI exposes them.
- Fabricate domain success states for unfinished services.
- Add settings persistence, localization, docking, custom fonts, icons, animations, charts, recording, streaming, or OBS integration.
- Add a native menu bar or system tray.
- Perform macOS work.

## Completion criteria

The task is complete only when:

- `bevy_egui = 0.41.1` builds with Bevy 0.19.0.
- The obsolete compatibility TODO and `ui_stub_system` are gone.
- The desktop executable installs `EguiPlugin` and `UiShellPlugin` correctly.
- All egui drawing runs in `EguiPrimaryContextPass`.
- Setup, Live, and Diagnostics are visible and switchable.
- UI widgets only read snapshots and emit actions.
- Native VRM selection is asynchronous and non-blocking.
- The VRM 3D scene remains visible.
- Default/no-camera/no-avatar/no-preview/error states do not panic.
- Minimal feature selection avoids `picking` and `bevy_ui` integration.
- The required automated commands pass.
- The final report distinguishes automated validation from Windows manual validation.

## Final report format

End with exactly these sections:

1. `Status`
2. `Repository HEAD before / after`
3. `Changed files`
4. `Dependency decision`
5. `Implementation summary`
6. `Automated commands and exact results`
7. `Windows manual smoke results`
8. `Acceptance checklist`
9. `Assumptions`
10. `Remaining blockers or follow-ups`

Do not continue to M1-09 or any quality task after completing this patch.
