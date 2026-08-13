//! "Storm Hammer" — a second reference spell-granting weapon.
//!
//! Distinct kit from `flame_staff.rs` and `iron_sword.rs` on purpose, to show
//! that spell kits are per-item data, not a fixed template: a heavy strike
//! or a meteor smash on Q, a mobility charge on W, a holy beam finisher on E.

use bevymmo_props_macro::item;

use crate::spells_impl::attack::AttackSpell;
use crate::spells_impl::meteorite::MeteoriteSpell;
use crate::spells_impl::ray_of_light::RayOfLightSpell;
use crate::spells_impl::swift::SwiftSpell;

#[item(
    id = "storm_hammer",
    name = "Storm Hammer",
    description = "Un maglio pesante intriso di energia tempestosa: ogni colpo può scatenare una furia devastante.",
    category = Weapon,
    rarity = Epic,
    slot = Weapon,
    effects = [stat_bonus(field = AttackPower, op = Add, value = 40.0)],
    spells(
        q = [AttackSpell, MeteoriteSpell],
        w = [SwiftSpell],
        e = RayOfLightSpell,
    ),
)]
pub struct StormHammer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::components::EquipSlot;
    use crate::items::definition::Item;
    use crate::spells::components::HotbarSlot;
    use crate::spells::registry::SpellId;

    #[test]
    fn id_and_slot_match_design() {
        let hammer = StormHammer;
        assert_eq!(hammer.id().as_str(), "storm_hammer");
        assert_eq!(hammer.config().equippable_into, Some(EquipSlot::Weapon));
    }

    #[test]
    fn grants_two_q_options_one_w_one_e() {
        let hammer = StormHammer;
        let kit = hammer.spell_kit().expect("storm_hammer must grant a spell kit");

        assert_eq!(
            kit.candidates_for(HotbarSlot::Q),
            &[SpellId::new(AttackSpell::ID), SpellId::new(MeteoriteSpell::ID)]
        );
        assert_eq!(kit.candidates_for(HotbarSlot::W), &[SpellId::new(SwiftSpell::ID)]);
        assert_eq!(kit.candidates_for(HotbarSlot::E), &[SpellId::new(RayOfLightSpell::ID)]);
    }

    #[test]
    fn still_carries_a_stat_bonus_like_any_other_item() {
        let hammer = StormHammer;
        assert_eq!(hammer.effects().len(), 1);
    }
}
