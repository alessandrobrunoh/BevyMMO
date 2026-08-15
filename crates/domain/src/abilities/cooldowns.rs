//! Cooldown tracking for Eidolon abilities — mirrors
//! `crate::spells::components::SpellCooldowns`, but keyed by `AbilityId`
//! (the gesto) instead of `SpellId`: cooldown belongs to the weapon slot's
//! gesture, not to whatever Essence happens to be inscribed on it.

use crate::spells::components::Cooldown;
use std::collections::HashMap;

use super::base_ability::AbilityId;

#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Default)]
pub struct AbilityCooldowns {
    pub timers: HashMap<AbilityId, Cooldown>,
}

impl AbilityCooldowns {
    pub fn is_on_cooldown(&self, id: &AbilityId) -> bool {
        self.timers.get(id).is_some_and(|timer| !timer.is_finished())
    }

    pub fn start_cooldown(&mut self, id: AbilityId, duration_seconds: f32) {
        self.timers.insert(id, Cooldown::new(duration_seconds));
    }

    pub fn tick(&mut self, delta_seconds: f32) {
        for timer in self.timers.values_mut() {
            timer.tick(delta_seconds);
        }
    }

    pub fn cleanup_finished(&mut self) {
        self.timers.retain(|_, timer| !timer.is_finished());
    }
}
