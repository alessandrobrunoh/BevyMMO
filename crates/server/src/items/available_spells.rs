//! Authoritative recomputation of [`AvailableSpellChoices`] and enforcement
//! against [`SpellHotbar`].
//!
//! The type and the pure union logic live in `bevymmo_shared::items` (shared
//! with the client, which recomputes the identical value locally for its
//! spell-selection UI — see `bevymmo_presentation::spells::available_choices`).
//! This module only owns the *authoritative* half: running the recompute on
//! `Changed<Equipment>` and clearing a `SpellHotbar` selection that the new
//! equipment no longer offers. Mirrors
//! `crate::items::bonuses::recompute_equipment_bonuses` almost exactly.

use bevy::prelude::*;

use bevymmo_shared::items::components::Equipment;
use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::items::{compute_available_choices, AvailableSpellChoices};
use bevymmo_shared::spells::components::{HotbarSlot, SpellHotbar};

/// Recomputes `AvailableSpellChoices` whenever `Equipment` changes, and
/// clears any `SpellHotbar` selection the new equipment no longer offers.
///
/// Runs on `Changed<Equipment>`, which also fires right after spawn (same
/// trick as `recompute_equipment_bonuses`), so a freshly joined player's
/// persisted hotbar selection is validated against their persisted
/// equipment exactly once.
pub fn recompute_available_spells(
    mut players: Query<(&Equipment, &mut AvailableSpellChoices, &mut SpellHotbar), Changed<Equipment>>,
    registry: Res<ItemRegistry>,
) {
    for (equipment, mut choices, mut hotbar) in &mut players {
        let recomputed = compute_available_choices(equipment, &registry);
        if *choices != recomputed {
            *choices = recomputed;
        }

        for slot in [HotbarSlot::Q, HotbarSlot::W, HotbarSlot::E] {
            if let Some(selected) = hotbar.spell_for_slot(slot) {
                if !choices.contains(slot, selected) {
                    bevy::log::info!(
                        "clearing {:?} selection {} — no longer offered by equipped items",
                        slot,
                        selected.as_str()
                    );
                    hotbar.assign(slot, None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::App;
    use bevymmo_shared::items::instance::ItemInstance;
    use bevymmo_shared::items::registry::ItemId;
    use bevymmo_shared::items_impl::iron_sword::IronSword;
    use bevymmo_shared::spells::registry::SpellId;
    use std::sync::Arc;

    #[test]
    fn unequipping_clears_a_selection_no_longer_offered() {
        let mut app = App::new();
        app.init_resource::<ItemRegistry>();
        app.world_mut()
            .resource_mut::<ItemRegistry>()
            .register(Arc::new(IronSword::new()));
        app.add_systems(Update, recompute_available_spells);

        let equipment = Equipment {
            weapon: Some(ItemInstance::new(ItemId::new(IronSword::ID))),
            ..Default::default()
        };
        let mut hotbar = SpellHotbar::default();
        hotbar.assign(HotbarSlot::Q, Some(SpellId::new("attack")));

        let entity = app
            .world_mut()
            .spawn((equipment, AvailableSpellChoices::default(), hotbar))
            .id();

        app.update();
        let choices = app.world().get::<AvailableSpellChoices>(entity).unwrap();
        assert_eq!(choices.q.len(), 2);
        let hotbar = app.world().get::<SpellHotbar>(entity).unwrap();
        assert_eq!(hotbar.q_spell, Some(SpellId::new("attack")));

        // Unequip: Q selection is no longer offered by anything equipped.
        app.world_mut()
            .get_mut::<Equipment>(entity)
            .unwrap()
            .weapon = None;
        app.update();

        let choices = app.world().get::<AvailableSpellChoices>(entity).unwrap();
        assert!(choices.q.is_empty());
        let hotbar = app.world().get::<SpellHotbar>(entity).unwrap();
        assert_eq!(hotbar.q_spell, None, "stale Q selection must be cleared on unequip");
    }
}
