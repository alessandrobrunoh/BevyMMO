//! Server-authoritative items domain.
//!
//! Owns the inventory/equipment command pipeline (equip, unequip, move) and
//! the derived-stats recomputation triggered by equipment changes. All systems
//! are gated on `has_server`, so the plugin is safe in host-client builds.

pub mod available_spells;
pub mod bonuses;
pub mod systems;

use bevy::prelude::*;

use bevymmo_shared::abilities::{AncientWordRegistry, BaseAbilityRegistry, EssenceRegistry, ModifierRegistry};
use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::network::mode::has_server;

/// Server-side umbrella plugin for the items domain.
///
/// Registers the shared `ItemRegistry` (client and server share the same item
/// definitions), populates it at startup, and wires the command handlers plus
/// the equipment-bonus recomputation into `FixedUpdate` behind `has_server`.
///
/// Also owns the "Eidolon" ability catalogs (`BaseAbilityRegistry` +
/// `EssenceRegistry`/`ModifierRegistry`/`AncientWordRegistry`): same
/// shared-catalog pattern as `ItemRegistry`/`SpellRegistry`, populated
/// identically on client and server.
pub struct ItemsServerPlugin;

impl Plugin for ItemsServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemRegistry>()
            .init_resource::<BaseAbilityRegistry>()
            .init_resource::<EssenceRegistry>()
            .init_resource::<ModifierRegistry>()
            .init_resource::<AncientWordRegistry>()
            .add_systems(
                Startup,
                (
                    bevymmo_shared::items_impl::register_default_items,
                    bevymmo_shared::base_abilities_impl::register_default_base_abilities,
                    bevymmo_shared::essences_impl::register_default_essences,
                    bevymmo_shared::modifiers_impl::register_default_modifiers,
                    bevymmo_shared::ancient_words_impl::register_default_ancient_words,
                ),
            )
            .add_systems(
                FixedUpdate,
                (
                    systems::handle_equip_item_commands,
                    systems::handle_unequip_item_commands,
                    systems::handle_move_item_commands,
                    bonuses::recompute_equipment_bonuses,
                    available_spells::recompute_available_spells,
                )
                    .chain()
                    .run_if(has_server),
            );
    }
}
