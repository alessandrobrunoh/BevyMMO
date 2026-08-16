//! Derived ability blueprint shared by preview and execution.
//!
//! A blueprint is not persisted. It is rebuilt from the base ability, the item
//! and later the item's Root Words/Ancient Words. Persisted state contains only
//! stable ids and selections.

use super::base_ability::{AbilityCastMode, AbilityGeometry, AbilityId, AbilityParams, AbilityTag, BaseAbility};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintExecution {
    Base,
    Charge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityBlueprint {
    pub ability_id: AbilityId,
    pub tags: Vec<AbilityTag>,
    pub geometry: AbilityGeometry,
    pub cast_mode: AbilityCastMode,
    pub execution: BlueprintExecution,
    pub params: AbilityParams,
    pub animation: &'static str,
    pub impact_vfx: &'static str,
    pub impact_delay: f32,
    pub stun_seconds: f32,
}

impl AbilityBlueprint {
    pub fn from_base_ability<T: BaseAbility + ?Sized>(ability: &T) -> Self {
        Self {
            ability_id: ability.id(),
            tags: ability.tags().to_vec(),
            geometry: ability.geometry(),
            cast_mode: ability.cast_mode(),
            execution: BlueprintExecution::Base,
            params: ability.base_params(),
            animation: ability.animation(),
            impact_vfx: ability.impact_vfx(),
            impact_delay: ability.impact_delay(),
            stun_seconds: ability.stun_seconds(),
        }
    }

    pub fn has_tag(&self, tag: AbilityTag) -> bool {
        self.tags.contains(&tag)
    }
}
