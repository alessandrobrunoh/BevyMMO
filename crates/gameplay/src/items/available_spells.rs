//! Shared computation of which spells equipped items currently offer.
//!
//! The module uses this to validate `set_hotbar_spell` and to clear a
//! `SpellHotbar` slot an unequip just invalidated. `Equipment` is already
//! replicated and `ItemRegistry` is the same catalogue on both sides, so the
//! derived [`AvailableSpellChoices`] is not itself a table.

use super::components::{EquipSlot, Equipment};
use super::registry::ItemRegistry;
use crate::abilities::AbilitySlot;
use crate::spells::registry::SpellId;

/// Spells currently selectable for each ability slot, derived from equipped
/// items. Nothing should write to this except the recompute systems.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AvailableSpellChoices {
    pub primary: Vec<SpellId>,
    pub secondary: Vec<SpellId>,
    pub ultimate: Vec<SpellId>,
}

impl AvailableSpellChoices {
    /// Candidates currently offered for `slot`.
    pub fn for_slot(&self, slot: AbilitySlot) -> &[SpellId] {
        match slot {
            AbilitySlot::Primary => &self.primary,
            AbilitySlot::Secondary => &self.secondary,
            AbilitySlot::Ultimate => &self.ultimate,
        }
    }

    /// `true` if `spell_id` is currently a legal pick for `slot`.
    pub fn contains(&self, slot: AbilitySlot, spell_id: &SpellId) -> bool {
        self.for_slot(slot).contains(spell_id)
    }
}

/// Unions the [`crate::items::SpellKit`] of every equipped item into one
/// [`AvailableSpellChoices`]. Pure and deterministic: same `equipment` +
/// same `registry` always yields the same result, which is exactly what lets
/// the client recompute it locally instead of receiving it over the network.
pub fn compute_available_choices(
    equipment: &Equipment,
    registry: &ItemRegistry,
) -> AvailableSpellChoices {
    let mut choices = AvailableSpellChoices::default();

    for slot in EquipSlot::ALL {
        let Some(item_instance) = equipment.get(slot) else {
            continue;
        };
        let Some(item) = registry.get(&item_instance.item_id) else {
            log::warn!(
                "equipped item {} not in registry",
                item_instance.item_id.as_str()
            );
            continue;
        };
        let Some(kit) = item.spell_kit() else {
            continue;
        };

        for id in &kit.primary {
            if !choices.primary.contains(id) {
                choices.primary.push(id.clone());
            }
        }
        for id in &kit.secondary {
            if !choices.secondary.contains(id) {
                choices.secondary.push(id.clone());
            }
        }
        if !choices.ultimate.contains(&kit.ultimate) {
            choices.ultimate.push(kit.ultimate.clone());
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
            armor: Some(ItemInstance::new(ItemId::new("unknown_item"))),
            ..Default::default()
        };
        let registry = ItemRegistry::default(); // item not even registered — must not panic
        let choices = compute_available_choices(&equipment, &registry);
        assert!(choices.primary.is_empty());
        assert!(choices.secondary.is_empty());
        assert!(choices.ultimate.is_empty());
    }

    #[test]
    fn contains_checks_the_right_slot_only() {
        let choices = AvailableSpellChoices {
            primary: vec![SpellId::new("fireball")],
            secondary: vec![],
            ultimate: vec![],
        };
        assert!(choices.contains(AbilitySlot::Primary, &SpellId::new("fireball")));
        assert!(!choices.contains(AbilitySlot::Secondary, &SpellId::new("fireball")));
    }
}
