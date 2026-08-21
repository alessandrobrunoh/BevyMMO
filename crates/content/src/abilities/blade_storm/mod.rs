//! Blade Storm — Sword ultimate ability (E).
//!
//! Becomes a whirlwind of steel, dealing rapid damage to all nearby enemies.
//! The spinning blades leave deep, burning cuts on anyone caught within.

use bevymmo_props_macro::base_ability;

use crate::abilities::BaseAbilityRegistry;

#[base_ability(
    id = "blade_storm",
    name = "Blade Storm",
    tags = [Melee, Area],
    range = 5.5,
    geometry = circle(radius = 5.5),
    potency = 235.0,
    cast_time = 0.7,
    cooldown = 22.0,
    mana_cost = 40.0,
    channeling = (tick_interval = 0.2, movement = InterruptOnMove),
    animation = "sword_ultimate",
    impact_vfx = "blade_storm_impact",
    icon = "abilities/icons/blade_storm.png",
)]
pub struct BladeStorm;

/// Adds this content package to the base-ability registry.
pub fn register(registry: &mut BaseAbilityRegistry) {
    BladeStorm::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::abilities::{AbilityCastMode, BaseAbility, ChannelMovementPolicy};

    #[test]
    fn blade_storm_is_channeling() {
        match BladeStorm.cast_mode() {
            AbilityCastMode::Channeling {
                tick_interval_seconds,
                movement_policy,
            } => {
                assert!((tick_interval_seconds - 0.2).abs() < f32::EPSILON);
                assert_eq!(movement_policy, ChannelMovementPolicy::InterruptOnMove);
            }
            other => panic!("expected Channeling, got {other:?}"),
        }
    }

    #[test]
    fn selects_its_icon_asset() {
        assert_eq!(BladeStorm.icon(), "abilities/icons/blade_storm.png");
    }
}
