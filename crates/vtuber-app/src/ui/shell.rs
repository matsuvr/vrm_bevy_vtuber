//! UI shell — the bevy_egui integration layer.
//!
//! Provides the [`UiShellPlugin`] which sets up egui and renders the
//! three main screens: Setup, Live, and Diagnostics.
//!
//! NOTE: This module requires bevy_egui compatible with Bevy 0.19.
//! The egui rendering is stubbed until version compatibility is resolved.

use bevy::prelude::*;

use crate::actions::UiAction;
use crate::ui_model::UiViewModel;

/// Plugin that sets up the egui-based UI shell.
///
/// Currently a stub — the actual egui integration requires a bevy_egui
/// version compatible with Bevy 0.19. The UI model and action types
/// are fully functional (see [`crate::ui_model`] and [`crate::actions`]).
pub struct UiShellPlugin;

impl Plugin for UiShellPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .init_resource::<UiViewModel>()
            .add_systems(Update, ui_stub_system);
    }
}

/// Resource holding the current UI state and pending actions.
#[derive(Resource, Debug, Default)]
pub struct UiState {
    /// Actions emitted by the UI this frame.
    pub pending_actions: Vec<UiAction>,
}

impl UiState {
    /// Emit a UI action.
    pub fn emit(&mut self, action: UiAction) {
        self.pending_actions.push(action);
    }

    /// Take all pending actions, clearing the internal list.
    pub fn take_actions(&mut self) -> Vec<UiAction> {
        std::mem::take(&mut self.pending_actions)
    }
}

/// Stub system — will be replaced with actual egui rendering.
fn ui_stub_system(_view_model: Res<UiViewModel>, _ui_state: ResMut<UiState>) {
    // TODO: Integrate bevy_egui rendering when version compatibility is resolved.
    // The UI model and action types are ready for use.
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
}
