//! Diagnostics screen rendering.

use bevy_egui::egui::Ui;

use crate::diagnostics::DiagnosticsSnapshot;
use crate::ui_model::UiViewModel;

/// Render the Diagnostics screen.
pub fn render_diagnostics_screen(ui: &mut Ui, vm: &UiViewModel, diagnostics: &DiagnosticsSnapshot) {
    ui.heading("Diagnostics");
    ui.separator();

    // Performance metrics.
    ui.heading("Performance");
    bevy_egui::egui::Grid::new("perf_grid")
        .num_columns(2)
        .spacing([40.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Render FPS");
            ui.label(format!("{:.1}", diagnostics.render_fps));
            ui.end_row();

            ui.label("Capture rate");
            ui.label(format!("{:.1} Hz", diagnostics.capture_rate));
            ui.end_row();

            ui.label("Inference rate");
            ui.label(format!("{:.1} Hz", diagnostics.inference_rate));
            ui.end_row();

            ui.label("Slot overwrites");
            ui.label(format!("{}", diagnostics.slot_overwrites));
            ui.end_row();
        });
    ui.separator();

    // Stage timings.
    if !diagnostics.stage_timings.is_empty() {
        ui.heading("Stage Timings");
        bevy_egui::egui::Grid::new("timing_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                for (name, duration) in &diagnostics.stage_timings {
                    ui.label(name);
                    ui.label(format!("{:.2} ms", duration));
                    ui.end_row();
                }
            });
        ui.separator();
    }

    // Model and camera info.
    ui.heading("Model & Camera");
    bevy_egui::egui::Grid::new("info_grid")
        .num_columns(2)
        .spacing([40.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Model hash");
            ui.label(diagnostics.model_hash.as_deref().unwrap_or("(none)"));
            ui.end_row();

            ui.label("Camera backend");
            ui.label(diagnostics.camera_backend.as_deref().unwrap_or("(none)"));
            ui.end_row();

            ui.label("Avatar capabilities");
            ui.label(
                diagnostics
                    .avatar_capabilities
                    .as_deref()
                    .unwrap_or("(none)"),
            );
            ui.end_row();
        });
    ui.separator();

    // Tracking state.
    ui.heading("Tracking");
    ui.label(format!("State: {}", diagnostics.tracking_state));
    ui.separator();

    // Last error.
    if let Some(error) = &diagnostics.last_error {
        ui.heading("Last Error");
        ui.colored_label(bevy_egui::egui::Color32::LIGHT_RED, error);
    }

    // Current app state summary.
    ui.separator();
    ui.heading("Current State");
    ui.label(format!("Screen: {:?}", vm.screen));
    ui.label(format!("Lifecycle: {:?}", vm.lifecycle));
    ui.label(format!(
        "Camera: {}",
        vm.camera
            .selected_index
            .map(|i| vm
                .camera
                .available_cameras
                .get(i)
                .map(|c| c.name.as_str())
                .unwrap_or("?"))
            .unwrap_or("none")
    ));
    ui.label(format!(
        "Avatar: {}",
        vm.avatar
            .imported_model
            .as_ref()
            .map(|m| m.name.as_str())
            .unwrap_or("none")
    ));
}
