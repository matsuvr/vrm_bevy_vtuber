//! Live screen rendering.

use bevy_egui::egui::Ui;

use crate::actions::UiAction;
use crate::preview::PreviewState;
use crate::ui_model::UiViewModel;

fn preview_uv(mirrored: bool) -> bevy_egui::egui::Rect {
    if mirrored {
        bevy_egui::egui::Rect::from_min_max(
            bevy_egui::egui::pos2(1.0, 0.0),
            bevy_egui::egui::pos2(0.0, 1.0),
        )
    } else {
        bevy_egui::egui::Rect::from_min_max(
            bevy_egui::egui::pos2(0.0, 0.0),
            bevy_egui::egui::pos2(1.0, 1.0),
        )
    }
}

/// Render the Live screen.
pub fn render_live_screen(
    ui: &mut Ui,
    vm: &UiViewModel,
    ui_state: &mut super::UiState,
    preview: &PreviewState,
    preview_texture: Option<bevy_egui::egui::TextureId>,
) {
    ui.heading("Live");
    ui.separator();

    // Lifecycle and tracking status.
    ui.heading("Status");
    ui.label(format!("Lifecycle: {:?}", vm.lifecycle));
    ui.label(format!("Tracking: {:?}", vm.tracking.state));
    ui.label(format!("Confidence: {:.2}", vm.tracking.confidence));
    ui.label(format!(
        "Face detected: {}",
        if vm.tracking.face_detected {
            "yes"
        } else {
            "no"
        }
    ));
    ui.separator();

    // Calibration section.
    ui.heading("Calibration");
    if vm.calibration.is_calibrating {
        ui.label(format!(
            "Samples: {}/{}",
            vm.calibration.samples_collected, vm.calibration.samples_target
        ));
        if let Some(score) = vm.calibration.quality_score {
            ui.label(format!("Quality: {:.2}", score));
        }
        if let Some(reason) = &vm.calibration.last_reject_reason {
            ui.colored_label(
                bevy_egui::egui::Color32::LIGHT_RED,
                format!("Rejected: {reason}"),
            );
        }
        if ui.button("Cancel").clicked() {
            ui_state.emit(UiAction::CancelCalibration);
        }
    } else if vm.calibration.is_complete {
        ui.colored_label(
            bevy_egui::egui::Color32::LIGHT_GREEN,
            "Calibration complete.",
        );
        if ui.button("Retry").clicked() {
            ui_state.emit(UiAction::RetryCalibration);
        }
    } else if vm.can_calibrate() {
        if ui.button("Begin Calibration").clicked() {
            ui_state.emit(UiAction::BeginCalibration);
        }
    } else {
        ui.add_enabled(false, bevy_egui::egui::Button::new("Begin Calibration"));
        ui.label("Start tracking to calibrate.");
    }
    ui.separator();

    // Preview section.
    ui.heading("Preview");
    let mut preview_visible = preview.visible;
    if ui.checkbox(&mut preview_visible, "Show Preview").changed() {
        ui_state.emit(UiAction::TogglePreview);
    }
    let mut mirror = preview.mirrored;
    if ui.checkbox(&mut mirror, "Mirror Preview").changed() {
        ui_state.emit(UiAction::ToggleMirror);
    }

    if preview.visible {
        if let Some(texture) = preview_texture {
            let available = ui.available_width().max(160.0);
            let size = bevy_egui::egui::vec2(available, available * 9.0 / 16.0);
            ui.add(
                bevy_egui::egui::Image::from_texture((texture, size))
                    .uv(preview_uv(preview.mirrored)),
            );
        } else {
            ui.label("Waiting for camera frames…");
        }
    }
    ui.separator();

    // Start/Stop buttons.
    if vm.can_stop() && ui.button("Stop").clicked() {
        ui_state.emit(UiAction::Stop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_preview_reverses_only_the_horizontal_uv_axis() {
        let normal = preview_uv(false);
        let mirrored = preview_uv(true);

        assert_eq!(normal.min, bevy_egui::egui::pos2(0.0, 0.0));
        assert_eq!(normal.max, bevy_egui::egui::pos2(1.0, 1.0));
        assert_eq!(mirrored.min, bevy_egui::egui::pos2(1.0, 0.0));
        assert_eq!(mirrored.max, bevy_egui::egui::pos2(0.0, 1.0));
    }
}
