//! "Magic Staff" — arma di riferimento del sistema Eidolon.
//!
//! Due opzioni di gesto per Primary (Sfera/Sigillo) e per Secondary
//! (Vincolante/Raffica), una sola per Ultimate (Meteorite): il giocatore
//! sceglie una fra le opzioni di ogni slot, poi incide Glifi sopra il gesto
//! scelto (vedi `crate::abilities`). 8 di Capacità Runica, Affinità Fuoco.
//!
//! `iron_sword.rs`/`storm_hammer.rs` restano sul vecchio modello
//! (`spells(...)`, menu di spell pronte) come riferimento di quel pattern.

use bevymmo_props_macro::item;

use crate::base_abilities_impl::arcane_gale::ArcaneGale;
use crate::base_abilities_impl::arcane_orb::ArcaneOrb;
use crate::base_abilities_impl::arcane_seal::ArcaneSeal;
use crate::base_abilities_impl::binding_seal::BindingSeal;
use crate::base_abilities_impl::meteor_strike::MeteorStrike;

#[item(
    id = "magic_staff",
    name = "Magic Staff",
    description = "Un bastone da mago: la forma dei gesti è sua, cosa manifestano lo decidi tu incidendola.",
    category = Weapon,
    rarity = Rare,
    slot = Weapon,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 25.0)],
    abilities(
        primary = [ArcaneOrb, ArcaneSeal],
        secondary = [BindingSeal, ArcaneGale],
        ultimate = MeteorStrike,
    ),
    rune_profile(capacity = 8, stability = 0.96, affinity = fuoco),
)]
pub struct MagicStaff;

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
    fn offers_two_options_for_primary_and_secondary_one_for_ultimate() {
        let staff = MagicStaff;
        let abilities = staff.weapon_abilities().expect("magic_staff must grant weapon abilities");

        assert_eq!(
            abilities.options_for(AbilitySlot::Primary),
            &[ArcaneOrb::ID.into(), ArcaneSeal::ID.into()]
        );
        assert_eq!(
            abilities.options_for(AbilitySlot::Secondary),
            &[BindingSeal::ID.into(), ArcaneGale::ID.into()]
        );
        assert_eq!(abilities.options_for(AbilitySlot::Ultimate), &[MeteorStrike::ID.into()]);
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
