//! Unified effect vocabulary shared by the client and authoritative module.

mod spec;
mod status;

pub use spec::{
    ApplyStatusEffect, CleanseEffect, DamageEffect, EffectBundle, EffectContext, EffectSpec,
    HealEffect, PurgeEffect, StatusFilter, StatusSelection,
};
pub use status::{
    ArcStatus, ControlSpec, DispelPolicy, PeriodicEffect, PeriodicSpec, RefreshPolicy,
    ActiveStatusSnapshot, ActiveStatuses, StackPolicy, StackScope, StatModifierSpec, Status,
    StatusCategory, StatusDefinition, StatusId, StatusPresentation, StatusRegistry,
};

use crate::EntityId;

/// One effect in the bounded, deterministic resolution queue.
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedEffect {
    /// Sequence of the originating action within the current simulation step.
    pub action_sequence: u64,
    /// Position in the bundle emitted by that action.
    pub bundle_index: u32,
    pub context: EffectContext,
    pub spec: EffectSpec,
}

impl QueuedEffect {
    pub fn sort_key(&self) -> (u64, u32, EntityId, EntityId) {
        (
            self.action_sequence,
            self.bundle_index,
            self.context.target,
            self.context.source.unwrap_or(EntityId::new(0)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_order_has_stable_tie_breakers() {
        let mut effects = [
            QueuedEffect {
                action_sequence: 2,
                bundle_index: 0,
                context: EffectContext::new(EntityId::new(2)),
                spec: EffectSpec::Heal(HealEffect { amount: 1.0 }),
            },
            QueuedEffect {
                action_sequence: 1,
                bundle_index: 1,
                context: EffectContext::new(EntityId::new(1)),
                spec: EffectSpec::Heal(HealEffect { amount: 1.0 }),
            },
        ];

        effects.sort_by_key(QueuedEffect::sort_key);

        assert_eq!(effects[0].action_sequence, 1);
        assert_eq!(effects[1].action_sequence, 2);
    }
}
