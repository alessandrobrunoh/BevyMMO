//! Plugin per la sidebar NPC (pannello informativo su click NPC amichevole).
//!
//! Al click sinistro, raycast dalla Camera3d e seleziona il NPC
//! (`EntityKind::Friendly`) più vicino, mostrando una Card UI con nome e info.

pub mod components;
pub mod systems;

use bevy::prelude::*;

use crate::ui::systems::in_gameplay;
use crate::ui::npc_sidebar::systems::npc_sidebar_on_click;

/// Plugin che registra il sistema di click-to-inspect per gli NPC.
pub struct NpcSidebarPlugin;

impl Plugin for NpcSidebarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, npc_sidebar_on_click.run_if(in_gameplay));
    }
}
