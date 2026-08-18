//! Plugin per il sistema di targeting.

use bevy::prelude::*;

use crate::pointer::PointerOnHud;
use crate::targeting::CurrentTarget;
use bevymmo_network::network::mode;

use crate::targeting::systems::{
    cleanup_invalid_target, clear_target_with_escape, select_target_with_left_click,
};

/// Plugin per il sistema di targeting.
///
/// Aggiunge:
/// - [`CurrentTarget`] resource
/// - Sistema di selezione con tasto sinistro
/// - Sistema di pulizia con Escape
/// - Sistema di pulizia automatica
pub struct TargetingPlugin;

impl Plugin for TargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTarget>();
        app.init_resource::<PointerOnHud>();
        app.add_systems(
            Update,
            (
                select_target_with_left_click,
                // Escape is a keybind, unlike the world clicks the other two
                // systems here react to — must not fire while a text field
                // has focus, or clearing target and typing "e" both happen.
                clear_target_with_escape.run_if(crate::app_state::not_typing),
                cleanup_invalid_target,
            )
                .run_if(mode::has_client),
        );
    }
}
