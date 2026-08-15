//! Shared computation of which spells a player's equipped items currently
//! offer for Q/W/E.
//!
//! The type and the pure function live here (not in `bevymmo_server`)
//! because both sides need the identical result:
//! - the server uses it to validate `UpdateHotbarSlotRequest` and to clear a
//!   `SpellHotbar` selection an unequip just invalidated (see
//!   `bevymmo_server::items::available_spells::recompute_available_spells`);
//! - the client uses it to render the spell-selection UI (see
//!   `bevymmo_presentation::spells::available_choices`).
//!
//! `Equipment` is already replicated and `ItemRegistry` is populated
//! identically on both sides (`register_default_items`, run at `Startup` on
//! client and server alike), so `AvailableSpellChoices` itself is **not**
//! replicated — both sides derive the same value locally instead of paying
//! network cost for it.

use bevy::prelude::*;

use super::components::{EquipSlot, Equipment};
use super::registry::ItemRegistry;
use crate::spells::components::HotbarSlot;
use crate::spells::registry::SpellId;

/// Spells currently selectable for each hotbar key, derived from equipped
/// items. Nothing should write to this except the recompute systems.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct AvailableSpellChoices {
    pub q: Vec<SpellId>,
    pub w: Vec<SpellId>,
    pub e: Vec<SpellId>,
}

impl AvailableSpellChoices {
    /// Candidates currently offered for `slot`.
    pub fn for_slot(&self, slot: HotbarSlot) -> &[SpellId] {
        match slot {
            HotbarSlot::Q => &self.q,
            HotbarSlot::W => &self.w,
            HotbarSlot::E => &self.e,
        }
    }

    /// `true` if `spell_id` is currently a legal pick for `slot`.
    pub fn contains(&self, slot: HotbarSlot, spell_id: &SpellId) -> bool {
        self.for_slot(slot).contains(spell_id)
    }
}

/// Unions the [`crate::items::SpellKit`] of every equipped item into one
/// [`AvailableSpellChoices`]. Pure and deterministic: same `equipment` +
/// same `registry` always yields the same result, which is exactly what lets
/// the client recompute it locally instead of receiving it over the network.
pub fn compute_available_choices(equipment: &Equipment, registry: &ItemRegistry) -> AvailableSpellChoices {
    let mut choices = AvailableSpellChoices::default();

    for slot in EquipSlot::ALL {
        let Some(item_instance) = equipment.get(slot) else {
            continue;
        };
        let Some(item) = registry.get(&item_instance.item_id) else {
            bevy::log::warn!("equipped item {} not in registry", item_instance.item_id.as_str());
            continue;
        };
        let Some(kit) = item.spell_kit() else {
            continue;
        };

        for id in &kit.q {
            if !choices.q.contains(id) {
                choices.q.push(id.clone());
            }
        }
        for id in &kit.w {
            if !choices.w.contains(id) {
                choices.w.push(id.clone());
            }
        }
        if !choices.e.contains(&kit.e) {
            choices.e.push(kit.e.clone());
        }
    }

    choices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::instance::ItemInstance;
    use crate::items::registry::ItemId;

    #[test]
    fn compute_available_choices_ignores_items_without_a_spell_kit() {
        let equipment = Equipment {
            armor: Some(ItemInstance::new(ItemId::new("iron_plate_armor"))),
            ..Default::default()
        };
        let registry = ItemRegistry::default(); // item not even registered — must not panic
        let choices = compute_available_choices(&equipment, &registry);
        assert!(choices.q.is_empty());
        assert!(choices.w.is_empty());
        assert!(choices.e.is_empty());
    }

    #[test]
    fn contains_checks_the_right_slot_only() {
        let choices = AvailableSpellChoices {
            q: vec![SpellId::new("attack")],
            w: vec![SpellId::new("stun_field")],
            e: vec![SpellId::new("meteorite")],
        };
        assert!(choices.contains(HotbarSlot::Q, &SpellId::new("attack")));
        assert!(!choices.contains(HotbarSlot::W, &SpellId::new("attack")));
    }

    #[test]
    fn unions_kits_from_two_equipped_items_without_duplicates() {
        use crate::items_impl::iron_sword::IronSword;

        let mut registry = ItemRegistry::default();
        registry.register(std::sync::Arc::new(IronSword::new()));

        let equipment = Equipment {
            weapon: Some(ItemInstance::new(ItemId::new(IronSword::ID))),
            ..Default::default()
        };

        let choices = compute_available_choices(&equipment, &registry);
        assert_eq!(choices.q.len(), 2);
        assert_eq!(choices.w.len(), 1);
        assert_eq!(choices.e.len(), 1);
    }
}
