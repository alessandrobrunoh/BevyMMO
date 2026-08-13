//! "Flame Staff" — arma di riferimento del sistema Eidolon: 2 opzioni di
//! gesto per Primary (Getto/Scintilla) e per Secondary (Onda/Nova), 1 sola
//! per Ultimate (Convergenza) — il giocatore sceglie una fra le opzioni di
//! ogni slot, poi incide Glifi sopra il gesto scelto (vedi `crate::abilities`).
//! 8 di Capacità Runica, Affinità Fuoco.
//!
//! Prima versione di questo item (menu di spell pronte via `spell_kit()`)
//! sostituita da questa: `abilities(...)`/`rune_profile(...)` al posto di
//! `spells(...)`. `iron_sword.rs`/`storm_hammer.rs` restano sul vecchio
//! modello come riferimento di quel pattern.

use bevymmo_props_macro::item;

use crate::base_abilities_impl::staff_bolt::StaffBolt;
use crate::base_abilities_impl::staff_convergence::StaffConvergence;
use crate::base_abilities_impl::staff_nova::StaffNova;
use crate::base_abilities_impl::staff_spark::StaffSpark;
use crate::base_abilities_impl::staff_wave::StaffWave;

#[item(
    id = "flame_staff",
    name = "Flame Staff",
    description = "Forgiato nel cuore di un vulcano dormiente, sprigiona fiamme a ogni colpo.",
    category = Weapon,
    rarity = Rare,
    slot = Weapon,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 25.0)],
    abilities(
        primary = [StaffBolt, StaffSpark],
        secondary = [StaffWave, StaffNova],
        ultimate = StaffConvergence,
    ),
    rune_profile(capacity = 8, stability = 0.96, affinity = fuoco),
)]
pub struct FlameStaff;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::AbilitySlot;
    use crate::items::components::EquipSlot;
    use crate::items::definition::Item;

    #[test]
    fn id_and_slot_match_design() {
        let staff = FlameStaff;
        assert_eq!(staff.id().as_str(), "flame_staff");
        assert_eq!(staff.config().equippable_into, Some(EquipSlot::Weapon));
    }

    #[test]
    fn no_longer_grants_a_spell_kit_menu() {
        // Migrata al modello Eidolon: niente più menu di spell pronte.
        assert!(FlameStaff.spell_kit().is_none());
    }

    #[test]
    fn offers_two_options_for_primary_and_secondary_one_for_ultimate() {
        let staff = FlameStaff;
        let abilities = staff.weapon_abilities().expect("flame_staff must grant weapon abilities");

        assert_eq!(
            abilities.options_for(AbilitySlot::Primary),
            &[StaffBolt::ID.into(), StaffSpark::ID.into()]
        );
        assert_eq!(
            abilities.options_for(AbilitySlot::Secondary),
            &[StaffWave::ID.into(), StaffNova::ID.into()]
        );
        assert_eq!(abilities.options_for(AbilitySlot::Ultimate), &[StaffConvergence::ID.into()]);
    }

    #[test]
    fn has_a_rune_profile_with_fire_affinity() {
        let staff = FlameStaff;
        let profile = staff.rune_profile().expect("flame_staff must grant a rune profile");
        assert_eq!(profile.capacity, 8);
        assert_eq!(profile.affinity.as_ref().map(|id| id.as_str()), Some("fuoco"));
    }

    #[test]
    fn still_carries_a_stat_bonus_like_any_other_item() {
        let staff = FlameStaff;
        assert_eq!(staff.effects().len(), 1);
    }
}
