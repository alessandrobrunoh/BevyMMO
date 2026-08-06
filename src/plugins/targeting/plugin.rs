//! Plugin per il sistema di targeting.

use bevy::prelude::*;

use crate::network::mode;
use crate::plugins::targeting::resources::CurrentTarget;
use crate::plugins::targeting::systems::{
    cleanup_invalid_target, clear_target_with_escape, select_target_with_right_click,
};

/// Plugin per il sistema di targeting.
///
/// Aggiunge:
/// - [`CurrentTarget`] resource
/// - Sistema di selezione con tasto destro
/// - Sistema di pulizia con Escape
/// - Sistema di pulizia automatica
pub struct TargetingPlugin;

impl Plugin for TargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTarget>();
        app.add_systems(
            Update,
            (
                select_target_with_right_click,
                clear_target_with_escape,
                cleanup_invalid_target,
            )
                .run_if(mode::has_client),
        );
    }
}
