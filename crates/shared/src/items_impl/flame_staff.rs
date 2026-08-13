//! "Flame Staff" — arma di riferimento del sistema Eidolon: 3 gesti fissi
//! (Getto/Onda/Convergenza), 8 di Capacità Runica, Affinità Fuoco. Il
//! giocatore incide Glifi sopra questi gesti — vedi `crate::abilities`.
//!
//! Prima versione di questo item (menu di spell pronte via `spell_kit()`)
//! sostituita da questa: `abilities(...)`/`rune_profile(...)` al posto di
//! `spells(...)`. `iron_sword.rs`/`storm_hammer.rs` restano sul vecchio
//! modello come riferimento di quel pattern.

use bevymmo_props_macro::item;

use crate::base_abilities_impl::staff_bolt::StaffBolt;
use crate::base_abilities_impl::staff_convergence::StaffConvergence;
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
        primary = StaffBolt,
        secondary = StaffWave,
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
    fn grants_the_three_staff_gestures() {
        let staff = FlameStaff;
        let abilities = staff.weapon_abilities().expect("flame_staff must grant weapon abilities");
        assert_eq!(abilities.get(AbilitySlot::Primary).as_str(), StaffBolt::ID);
        assert_eq!(abilities.get(AbilitySlot::Secondary).as_str(), StaffWave::ID);
        assert_eq!(abilities.get(AbilitySlot::Ultimate).as_str(), StaffConvergence::ID);
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
