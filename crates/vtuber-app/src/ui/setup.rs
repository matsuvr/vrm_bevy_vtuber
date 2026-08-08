//! Setup screen rendering.

use bevy_egui::egui::Ui;

use crate::actions::UiAction;
use crate::ui_model::{AvatarLifecycleState, UiViewModel};

use super::file_dialog::FileDialogState;

/// Render the Setup screen.
pub fn render_setup_screen(
    ui: &mut Ui,
    vm: &UiViewModel,
    ui_state: &mut super::UiState,
    file_dialog: &mut FileDialogState,
) {
    ui.heading("Setup");
    ui.separator();

    // Camera section.
    ui.heading("Camera");
    if vm.camera.available_cameras.is_empty() {
        ui.label("No cameras detected.");
        if ui.button("Refresh Cameras").clicked() {
            ui_state.emit(UiAction::RefreshCameras);
        }
    } else {
        let selected = vm.camera.selected_index.unwrap_or(0);
        let mut new_selected = selected;
        bevy_egui::egui::ComboBox::from_label("Select camera")
            .selected_text(
                vm.camera
                    .available_cameras
                    .get(selected)
                    .map(|c| c.name.as_str())
                    .unwrap_or("None"),
            )
            .show_ui(ui, |ui| {
                for (i, cam) in vm.camera.available_cameras.iter().enumerate() {
                    ui.selectable_value(&mut new_selected, i, &cam.name);
                }
            });
        if new_selected != selected {
            ui_state.emit(UiAction::SelectCamera {
                index: new_selected,
            });
        }
    }
    ui.separator();

    // Avatar section.
    ui.heading("Avatar");
    if let Some(model) = &vm.avatar.imported_model {
        ui.label(format!("Model: {}", model.name));
        ui.label(format!("ID: {}", &model.id[..8.min(model.id.len())]));
        ui.label(format!(
            "Required bones: {}",
            if model.has_required_bones {
                "yes"
            } else {
                "no"
            }
        ));
        ui.label(format!("Expressions: {}", model.expression_count));

        // Lifecycle status for the avatar.
        match vm.avatar.lifecycle {
            AvatarLifecycleState::None => {
                ui.label("Status: Waiting to load…");
            }
            AvatarLifecycleState::Loading => {
                ui.colored_label(
                    bevy_egui::egui::Color32::LIGHT_BLUE,
                    "Status: Loading model…",
                );
            }
            AvatarLifecycleState::Binding => {
                ui.colored_label(
                    bevy_egui::egui::Color32::LIGHT_YELLOW,
                    "Status: Binding bones…",
                );
            }
            AvatarLifecycleState::Ready => {
                ui.colored_label(bevy_egui::egui::Color32::LIGHT_GREEN, "Status: Ready");
            }
            AvatarLifecycleState::Unloading => {
                ui.label("Status: Unloading…");
            }
            AvatarLifecycleState::Failed => {
                ui.colored_label(bevy_egui::egui::Color32::LIGHT_RED, "Status: Load failed");
                if ui.button("Retry Load").clicked() {
                    ui_state.emit(UiAction::RetryAfterError);
                }
            }
        }

        if ui.button("Unload Avatar").clicked() {
            ui_state.emit(UiAction::UnloadAvatar);
        }
    } else {
        ui.label("No avatar loaded.");
    }

    if ui.button("Import VRM...").clicked() && !file_dialog.is_active() {
        file_dialog.start();
    }

    ui.separator();

    // Lifecycle display.
    ui.heading("Status");
    ui.label(format!("App lifecycle: {:?}", vm.lifecycle));
    ui.label(format!("Avatar lifecycle: {:?}", vm.avatar.lifecycle));

    ui.separator();

    // Start/Stop buttons.
    if vm.can_start() {
        if ui.button("Start").clicked() {
            ui_state.emit(UiAction::Start);
        }
    } else {
        ui.add_enabled(false, bevy_egui::egui::Button::new("Start"));
        if vm.camera.selected_index.is_none() {
            ui.label("Select a camera to start.");
        }
        if !vm.avatar.is_ready {
            ui.label("Import an avatar to start.");
        }
    }

    if vm.can_stop() && ui.button("Stop").clicked() {
        ui_state.emit(UiAction::Stop);
    }
}
