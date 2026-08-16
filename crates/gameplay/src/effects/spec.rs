//! Shared descriptions of effects emitted by spells, items and statuses.
//!
//! These types describe work to be resolved. They do not mutate state and do not
//! depend on Bevy or SpacetimeDB.

use crate::abilities::{AbilityId, AncientWordId};
use crate::EntityId;

use super::status::StatusId;

/// Context retained for attribution while an effect moves through the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectContext {
    /// The entity or runtime object that currently generated the effect.
    pub source: Option<EntityId>,
    /// The player/entity that originally caused the action, when different from
    /// `source` (for example a projectile, trap or summon).
    pub original_caster: Option<EntityId>,
    pub target: EntityId,
    pub ability_id: Option<AbilityId>,
    pub ancient_word_id: Option<AncientWordId>,
}

impl EffectContext {
    pub fn new(target: EntityId) -> Self {
        Self {
            source: None,
            original_caster: None,
            target,
            ability_id: None,
            ancient_word_id: None,
        }
    }
}

/// An instantaneous damage request, before the target's mitigation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageEffect {
    pub amount: f32,
}

/// An instantaneous healing request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealEffect {
    pub amount: f32,
}

/// Requests application of a static status definition to the target.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyStatusEffect {
    pub status_id: StatusId,
    pub duration_override_seconds: Option<f32>,
    pub potency: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    Buffs,
    Debuffs,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSelection {
    Oldest,
    Newest,
    ShortestRemaining,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanseEffect {
    pub filter: StatusFilter,
    pub max_statuses: Option<u16>,
    pub selection: StatusSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PurgeEffect {
    pub filter: StatusFilter,
    pub max_statuses: Option<u16>,
    pub selection: StatusSelection,
}

/// The common semantic vocabulary for effects. More variants should be added
/// only when a concrete gameplay path needs them.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectSpec {
    Damage(DamageEffect),
    Heal(HealEffect),
    ApplyStatus(ApplyStatusEffect),
    Cleanse(CleanseEffect),
    Purge(PurgeEffect),
}

/// A group of effects emitted by one action for one target.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectBundle {
    pub context: EffectContext,
    pub effects: Vec<EffectSpec>,
}

impl EffectBundle {
    pub fn new(context: EffectContext, effects: Vec<EffectSpec>) -> Self {
        Self { context, effects }
    }

    pub fn single(context: EffectContext, effect: EffectSpec) -> Self {
        Self::new(context, vec![effect])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_preserves_attribution_and_target() {
        let mut context = EffectContext::new(EntityId::new(42));
        context.source = Some(EntityId::new(7));
        context.original_caster = Some(EntityId::new(3));
        context.ability_id = Some(AbilityId::new("fireball"));

        assert_eq!(context.target, EntityId::new(42));
        assert_eq!(context.source, Some(EntityId::new(7)));
        assert_eq!(context.original_caster, Some(EntityId::new(3)));
        assert_eq!(context.ability_id.as_ref().map(AbilityId::as_str), Some("fireball"));
    }

    #[test]
    fn bundle_can_contain_multiple_effects_for_one_target() {
        let context = EffectContext::new(EntityId::new(42));
        let bundle = EffectBundle::new(
            context,
            vec![
                EffectSpec::Damage(DamageEffect { amount: 10.0 }),
                EffectSpec::Heal(HealEffect { amount: 2.0 }),
            ],
        );

        assert_eq!(bundle.effects.len(), 2);
        assert!(matches!(bundle.effects[0], EffectSpec::Damage(_)));
        assert!(matches!(bundle.effects[1], EffectSpec::Heal(_)));
    }

    #[test]
    fn single_bundle_is_convenient_for_one_effect() {
        let bundle = EffectBundle::single(
            EffectContext::new(EntityId::new(1)),
            EffectSpec::Heal(HealEffect { amount: 5.0 }),
        );

        assert_eq!(bundle.effects, vec![EffectSpec::Heal(HealEffect { amount: 5.0 })]);
    }
}
