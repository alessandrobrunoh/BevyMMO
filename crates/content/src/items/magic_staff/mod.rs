//! "Magic Staff" — arma di riferimento del sistema Eidolon.
//!
//! Arcane Orb è l'unico gesto disponibile per ogni slot; il giocatore può
//! inciderci sopra i Glifi disponibili. Ha 8 di Capacità Runica e Affinità
//! Fuoco.

use bevymmo_props_macro::item;

use crate::ability_definitions::arcane_orb::ArcaneOrb;
use crate::items::ItemRegistry;

#[item(
    id = "magic_staff",
    name = "Magic Staff",
    description = "Un bastone da mago: la forma dei gesti è sua, cosa manifestano lo decidi tu incidendola.",
    category = Weapon,
    rarity = Rare,
    slot = Weapon,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 25.0)],
    abilities(
        primary = [ArcaneOrb],
        secondary = [ArcaneOrb],
        ultimate = ArcaneOrb,
    ),
    rune_profile(capacity = 8, stability = 0.96, affinity = fuoco),
)]
pub struct MagicStaff;

/// Adds this content package to the item registry.
pub fn register(registry: &mut ItemRegistry) {
    MagicStaff::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::AbilitySlot;
    use crate::items::components::EquipSlot;
    use crate::items::definition::Item;

    #[test]
    fn id_and_slot_match_design() {
        let staff = MagicStaff;
        assert_eq!(staff.id().as_str(), "magic_staff");
        assert_eq!(staff.config().equippable_into, Some(EquipSlot::Weapon));
    }

    #[test]
    fn no_longer_grants_a_spell_kit_menu() {
        // Modello Eidolon: niente più menu di spell pronte.
        assert!(MagicStaff.spell_kit().is_none());
    }

    #[test]
    fn offers_arcane_orb_for_every_slot() {
        let staff = MagicStaff;
        let abilities = staff.weapon_abilities().expect("magic_staff must grant weapon abilities");
        let expected = [ArcaneOrb::ID.into()];

        assert_eq!(abilities.options_for(AbilitySlot::Primary), expected);
        assert_eq!(abilities.options_for(AbilitySlot::Secondary), expected);
        assert_eq!(abilities.options_for(AbilitySlot::Ultimate), expected);
    }

    #[test]
    fn has_a_rune_profile_with_fire_affinity() {
        let staff = MagicStaff;
        let profile = staff.rune_profile().expect("magic_staff must grant a rune profile");
        assert_eq!(profile.capacity, 8);
        assert_eq!(profile.affinity.as_ref().map(|id| id.as_str()), Some("fuoco"));
    }

    #[test]
    fn still_carries_a_stat_bonus_like_any_other_item() {
        let staff = MagicStaff;
        assert_eq!(staff.effects().len(), 1);
    }
}
