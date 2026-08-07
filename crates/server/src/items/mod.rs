//! Server-authoritative items domain.
//!
//! Owns the inventory/equipment command pipeline (equip, unequip, move) and
//! the derived-stats recomputation triggered by equipment changes. All systems
//! are gated on `has_server`, so the plugin is safe in host-client builds.

pub mod bonuses;
pub mod systems;

use bevy::prelude::*;

use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::network::mode::has_server;

/// Server-side umbrella plugin for the items domain.
///
/// Registers the shared `ItemRegistry` (client and server share the same item
/// definitions), populates it at startup, and wires the command handlers plus
/// the equipment-bonus recomputation into `FixedUpdate` behind `has_server`.
pub struct ItemsServerPlugin;

impl Plugin for ItemsServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemRegistry>()
            .add_systems(Startup, bevymmo_shared::items_impl::register_default_items)
            .add_systems(
                FixedUpdate,
                (
                    systems::handle_equip_item_commands,
                    systems::handle_unequip_item_commands,
                    systems::handle_move_item_commands,
                    bonuses::recompute_equipment_bonuses,
                )
                    .chain()
                    .run_if(has_server),
            );
    }
}
