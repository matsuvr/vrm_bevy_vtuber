//! UI shell — the bevy_egui integration layer.
//!
//! Provides the [`UiShellPlugin`] which sets up egui and renders the
//! three main screens: Setup, Live, and Diagnostics.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use crate::actions::UiAction;
#[cfg(not(feature = "dev-synthetic-input"))]
use crate::avatar_bridge::publish_control_frame_system;
use crate::avatar_bridge::sync_avatar_diagnostics;
use crate::capture_runtime::{
    CaptureRuntime, LatestVideoFrame, capture_bridge_system, read_latest_frame,
    register_preview_texture_system, sync_capture_diagnostics, update_preview_texture_system,
};
use crate::diagnostics::{DiagnosticsSnapshot, sync_engine_diagnostics};
use crate::error_presenter::ErrorPresenter;
use crate::inference_runtime::{
    InferenceProjectRoot, InferenceRuntime, inference_bridge_system, read_inference_output_system,
};
use crate::metrics_export::{MetricsExportState, export_diagnostics_system};
use crate::orchestrator::{Orchestrator, process_ui_actions_system, sync_avatar_lifecycle_system};
use crate::preview::PreviewState;
use crate::preview_landmarks::{PreviewLandmarkState, sync_preview_landmark_system};
use crate::settings::{ArmPoseSettings, restore_arm_pose_settings_system};
use crate::tracking_runtime::{TrackingRuntime, tracking_bridge_system};
use crate::ui_model::{Screen, UiViewModel};
use vtuber_avatar::{AvatarMotionMirror, apply_arm_pose_profile_changes};

use super::diagnostics::render_diagnostics_screen;
use super::live::render_live_screen;
use super::setup::render_setup_screen;

const CONTROL_WINDOW_WIDTH: f32 = 350.0;
const CONTROL_WINDOW_HEIGHT: f32 = 600.0;

fn control_window() -> bevy_egui::egui::Window<'static> {
    // `default_width`/`default_height` only apply before egui has a Resize
    // state for this Id. The state survives hiding and showing the window, so
    // a resizable window would make the Controls panel depend on an earlier
    // drag or on the previous screen's content measurement.
    bevy_egui::egui::Window::new("Controls")
        .id(bevy_egui::egui::Id::new("control_window"))
        .fixed_size([CONTROL_WINDOW_WIDTH, CONTROL_WINDOW_HEIGHT])
        .collapsible(false)
        .movable(true)
}

/// Plugin that sets up the egui-based UI shell.
///
/// Requires `EguiPlugin` to be installed before this plugin.
pub struct UiShellPlugin;

impl Plugin for UiShellPlugin {
    fn build(&self, app: &mut App) {
        // Assert that EguiPlugin is already installed.
        assert!(
            app.is_plugin_added::<EguiPlugin>(),
            "UiShellPlugin requires EguiPlugin to be installed first"
        );

        app.init_resource::<UiState>()
            .init_resource::<UiViewModel>()
            .init_resource::<Orchestrator>()
            .init_resource::<ArmPoseSettings>()
            .init_resource::<PreviewState>()
            .init_resource::<PreviewLandmarkState>()
            .init_resource::<AvatarMotionMirror>()
            .init_resource::<DiagnosticsSnapshot>()
            .init_resource::<MetricsExportState>()
            .init_resource::<ErrorPresenter>()
            .init_resource::<super::file_dialog::FileDialogState>()
            .init_resource::<CaptureRuntime>()
            .init_resource::<LatestVideoFrame>();

        let frame_slot = app.world().resource::<CaptureRuntime>().frame_slot();
        let project_root = app
            .world()
            .get_resource::<InferenceProjectRoot>()
            .map(|root| root.0.clone())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        app.insert_resource(InferenceRuntime::new(frame_slot, project_root))
            .init_resource::<TrackingRuntime>()
            .add_systems(Startup, restore_arm_pose_settings_system)
            // Action processing, avatar pose re-resolution, and lifecycle
            // sync are chained so a settings action reaches the compositor in
            // the same Update frame.
            .add_systems(
                Update,
                (
                    process_ui_actions_system,
                    apply_arm_pose_profile_changes,
                    sync_avatar_lifecycle_system,
                )
                    .chain(),
            )
            .add_systems(Update, sync_error_presenter.before(inference_bridge_system))
            // Capture bridge: connects orchestrator intent to real camera.
            .add_systems(
                Update,
                (
                    capture_bridge_system,
                    read_latest_frame,
                    update_preview_texture_system,
                    register_preview_texture_system,
                    sync_capture_diagnostics,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (inference_bridge_system, read_inference_output_system)
                    .chain()
                    // Inference owns the first half of shutdown. Capture is
                    // started by the following bridge once its state is
                    // visible, but is stopped only after inference has joined.
                    .before(capture_bridge_system),
            )
            .add_systems(Update, sync_preview_landmark_system.after(read_inference_output_system))
            .add_systems(Update, tracking_bridge_system.after(read_inference_output_system))
            .add_systems(
                Last,
                (sync_engine_diagnostics, export_diagnostics_system)
                    .chain()
                    .before(shutdown_workers_on_exit),
            )
            .add_systems(Last, shutdown_workers_on_exit)
            // egui rendering in EguiPrimaryContextPass.
            .add_systems(EguiPrimaryContextPass, ui_render_system);

        // The synthetic source is an explicit diagnostic build mode. It
        // replaces the real bridge rather than running beside it, so two
        // producers can never race on ActiveControlFrame.
        #[cfg(not(feature = "dev-synthetic-input"))]
        app.add_systems(
            Update,
            publish_control_frame_system.after(tracking_bridge_system),
        )
        .add_systems(
            Update,
            sync_avatar_diagnostics.after(publish_control_frame_system),
        );

        #[cfg(feature = "dev-synthetic-input")]
        app.insert_resource(crate::synthetic_tracking::SyntheticTrackingSource::default())
            .add_systems(
                Update,
                crate::synthetic_tracking::synthetic_tracking_system.after(tracking_bridge_system),
            )
            .add_systems(
                Update,
                sync_avatar_diagnostics.after(crate::synthetic_tracking::synthetic_tracking_system),
            );
    }
}

/// Performs the explicit reverse-order shutdown required by the worker
/// ownership contract when Bevy is closing the application.
fn shutdown_workers_on_exit(
    mut exit_messages: MessageReader<AppExit>,
    mut inference: ResMut<InferenceRuntime>,
    mut capture: ResMut<CaptureRuntime>,
) {
    if exit_messages.read().next().is_some() {
        inference.stop_model();
        capture.shutdown();
    }
}

/// Resource holding the current UI state and pending actions.
#[derive(Resource, Debug, Default)]
pub struct UiState {
    /// Actions emitted by the UI this frame.
    pub pending_actions: Vec<UiAction>,
    /// Session-local visibility state for the main Controls window.
    control_window: ControlWindowState,
}

/// Session-local visibility state for the main Controls window.
///
/// The window starts visible so first-run setup remains discoverable. Its
/// visibility is deliberately not persisted until the settings task owns the
/// configuration schema.
#[derive(Resource, Debug)]
struct ControlWindowState {
    visible: bool,
}

impl Default for ControlWindowState {
    fn default() -> Self {
        Self { visible: true }
    }
}

impl ControlWindowState {
    fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

impl UiState {
    /// Emit a UI action, deduplicating one-shot actions within the same batch.
    pub fn emit(&mut self, action: UiAction) {
        // Deduplicate navigation and toggle actions within the same batch.
        if is_deduplicatable(&action) && self.pending_actions.contains(&action) {
            return;
        }
        self.pending_actions.push(action);
    }

    /// Take all pending actions, clearing the internal list.
    pub fn take_actions(&mut self) -> Vec<UiAction> {
        std::mem::take(&mut self.pending_actions)
    }
}

/// Check if an action should be deduplicated within a batch.
fn is_deduplicatable(action: &UiAction) -> bool {
    if matches!(action, UiAction::ToggleAvatarMotionMirror) {
        return true;
    }
    matches!(
        action,
        UiAction::SwitchScreen(_)
            | UiAction::ToggleMirror
            | UiAction::TogglePreview
            | UiAction::DismissError
    )
}

/// System that synchronizes the error presenter from the orchestrator.
fn sync_error_presenter(
    orchestrator: Res<Orchestrator>,
    mut error_presenter: ResMut<ErrorPresenter>,
    mut diagnostics: ResMut<DiagnosticsSnapshot>,
) {
    error_presenter.update(orchestrator.last_error());
    diagnostics.last_error = orchestrator.last_error().map(ToString::to_string);
    diagnostics.last_error_code = orchestrator
        .last_error()
        .map(crate::error_presenter::present_error)
        .map(|presentation| presentation.code.to_string());
}

/// System that renders the UI using egui.
///
/// Reads [`UiViewModel`] and emits [`UiAction`] via [`UiState`].
#[allow(clippy::too_many_arguments)]
fn ui_render_system(
    mut contexts: EguiContexts,
    view_model: Res<UiViewModel>,
    mut ui_state: ResMut<UiState>,
    diagnostics: Res<DiagnosticsSnapshot>,
    preview: Res<PreviewState>,
    landmarks: Res<PreviewLandmarkState>,
    avatar_motion_mirror: Res<AvatarMotionMirror>,
    mut file_dialog: ResMut<super::file_dialog::FileDialogState>,
) -> Result {
    let preview_texture = preview
        .image_handle
        .as_ref()
        .and_then(|handle| contexts.image_id(handle.id()));
    let ctx = contexts.ctx_mut()?;

    // Poll file dialog.
    super::file_dialog::poll_file_dialog(&mut file_dialog, &mut ui_state);

    if ctx.input(|input| input.key_pressed(bevy_egui::egui::Key::F1)) {
        ui_state.control_window.toggle();
    }

    if ui_state.control_window.visible {
        let mut visible = true;
        let mut hide_requested = false;

        // Main control window. Its size is part of the session UI contract;
        // the scroll area below must absorb content growth instead of
        // feeding it back into the outer window geometry.
        control_window().open(&mut visible).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Hide Controls").clicked() {
                    hide_requested = true;
                }
                ui.small("F1 toggles controls");
            });
            ui.separator();

            // Navigation tabs at the top.
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(view_model.screen == Screen::Setup, "Setup")
                    .clicked()
                {
                    ui_state.emit(UiAction::SwitchScreen(Screen::Setup));
                }
                if ui
                    .selectable_label(view_model.screen == Screen::Live, "Live")
                    .clicked()
                {
                    ui_state.emit(UiAction::SwitchScreen(Screen::Live));
                }
                if ui
                    .selectable_label(view_model.screen == Screen::Diagnostics, "Diagnostics")
                    .clicked()
                {
                    ui_state.emit(UiAction::SwitchScreen(Screen::Diagnostics));
                }
            });
            ui.separator();

            // Screen content in a scroll area.
            bevy_egui::egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| match view_model.screen {
                    Screen::Setup => {
                        render_setup_screen(ui, &view_model, &mut ui_state, &mut file_dialog)
                    }
                    Screen::Live => render_live_screen(
                        ui,
                        &view_model,
                        &mut ui_state,
                        &preview,
                        &landmarks,
                        *avatar_motion_mirror,
                        preview_texture,
                    ),
                    Screen::Diagnostics => render_diagnostics_screen(ui, &view_model, &diagnostics),
                });
        });
        ui_state.control_window.visible = visible && !hide_requested;
    }

    if !ui_state.control_window.visible {
        bevy_egui::egui::Area::new(bevy_egui::egui::Id::new("show_control_window"))
            .anchor(
                bevy_egui::egui::Align2::LEFT_TOP,
                bevy_egui::egui::vec2(12.0, 12.0),
            )
            .show(ctx, |ui| {
                if ui.button("Show Controls (F1)").clicked() {
                    ui_state.control_window.visible = true;
                }
            });
    }

    // Handle drag-and-drop for VRM files.
    super::file_dialog::handle_dropped_files(ctx, &mut ui_state);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_state_emit_and_take() {
        let mut state = UiState::default();
        assert!(state.pending_actions.is_empty());

        state.emit(UiAction::Start);
        state.emit(UiAction::Stop);
        assert_eq!(state.pending_actions.len(), 2);

        let actions = state.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(state.pending_actions.is_empty());
    }

    #[test]
    fn ui_state_default_is_empty() {
        let state = UiState::default();
        assert!(state.pending_actions.is_empty());
    }

    #[test]
    fn ui_state_emit_deduplicates_navigation() {
        let mut state = UiState::default();
        state.emit(UiAction::SwitchScreen(Screen::Live));
        state.emit(UiAction::SwitchScreen(Screen::Live)); // duplicate
        assert_eq!(state.pending_actions.len(), 1);

        state.emit(UiAction::SwitchScreen(Screen::Setup)); // different
        assert_eq!(state.pending_actions.len(), 2);
    }

    #[test]
    fn ui_state_emit_deduplicates_toggle() {
        let mut state = UiState::default();
        state.emit(UiAction::ToggleMirror);
        state.emit(UiAction::ToggleMirror); // duplicate
        assert_eq!(state.pending_actions.len(), 1);

        state.emit(UiAction::ToggleAvatarMotionMirror);
        state.emit(UiAction::ToggleAvatarMotionMirror); // duplicate
        assert_eq!(state.pending_actions.len(), 2);
    }

    #[test]
    fn ui_state_take_allows_same_action_next_batch() {
        let mut state = UiState::default();
        state.emit(UiAction::SwitchScreen(Screen::Live));
        let _ = state.take_actions();

        // Same action in next batch should work.
        state.emit(UiAction::SwitchScreen(Screen::Live));
        assert_eq!(state.pending_actions.len(), 1);
    }

    #[test]
    fn ui_state_emit_does_not_deduplicate_non_deduplicatable() {
        let mut state = UiState::default();
        state.emit(UiAction::Start);
        state.emit(UiAction::Start); // not deduplicated
        assert_eq!(state.pending_actions.len(), 2);
    }

    #[test]
    fn control_window_is_visible_by_default_and_toggles() {
        let mut state = ControlWindowState::default();
        assert!(state.visible);

        state.toggle();
        assert!(!state.visible);

        state.toggle();
        assert!(state.visible);
    }

    #[test]
    fn control_window_replaces_a_large_resizable_state_with_fixed_geometry() {
        let ctx = bevy_egui::egui::Context::default();
        let screen_rect = bevy_egui::egui::Rect::from_min_size(
            bevy_egui::egui::Pos2::ZERO,
            bevy_egui::egui::vec2(1920.0, 1080.0),
        );

        // Reproduce the old implementation after a user/content-driven
        // expansion. Resize state is persisted by egui for this Id.
        for _ in 0..2 {
            let _ = ctx.run_ui(
                bevy_egui::egui::RawInput {
                    screen_rect: Some(screen_rect),
                    ..Default::default()
                },
                |ui| {
                    bevy_egui::egui::Window::new("Controls")
                        .id(bevy_egui::egui::Id::new("control_window"))
                        .default_size([CONTROL_WINDOW_WIDTH, CONTROL_WINDOW_HEIGHT])
                        .resizable(true)
                        .show(ui.ctx(), |window_ui| {
                            window_ui.allocate_space(bevy_egui::egui::vec2(1500.0, 700.0));
                        });
                },
            );
        }

        let legacy_rect = ctx
            .memory(|memory| memory.area_rect(bevy_egui::egui::Id::new("control_window")))
            .expect("legacy control window should have a remembered rect");
        assert!(legacy_rect.width() > CONTROL_WINDOW_WIDTH);

        // The current builder must clamp that stale in-session state and keep
        // oversized content inside the scrollable fixed viewport.
        let _ = ctx.run_ui(
            bevy_egui::egui::RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ui| {
                control_window().show(ui.ctx(), |window_ui| {
                    bevy_egui::egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(window_ui, |window_ui| {
                            window_ui.allocate_space(bevy_egui::egui::vec2(1500.0, 700.0));
                        });
                });
            },
        );

        let fixed_rect = ctx
            .memory(|memory| memory.area_rect(bevy_egui::egui::Id::new("control_window")))
            .expect("fixed control window should remain visible");
        assert_eq!(
            fixed_rect.size(),
            bevy_egui::egui::vec2(CONTROL_WINDOW_WIDTH, CONTROL_WINDOW_HEIGHT)
        );

        // Hiding the window must not make its next appearance depend on the
        // previous content pass either.
        let _ = ctx.run_ui(
            bevy_egui::egui::RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |_ui| {},
        );
        let _ = ctx.run_ui(
            bevy_egui::egui::RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ui| {
                control_window().show(ui.ctx(), |window_ui| {
                    bevy_egui::egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(window_ui, |window_ui| {
                            window_ui.label("reopened");
                        });
                });
            },
        );

        let reopened_rect = ctx
            .memory(|memory| memory.area_rect(bevy_egui::egui::Id::new("control_window")))
            .expect("reopened control window should remain visible");
        assert_eq!(reopened_rect.size(), fixed_rect.size());
    }
}
