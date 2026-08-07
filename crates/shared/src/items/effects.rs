//! Effects an item can apply.
//!
//! To avoid duplicating the existing stat vocabulary, [`ItemEffect::StatBonus`]
//! reuses [`StatField`] / [`ModifierOp`] from `crate::stats::events`. That way
//! "Spada 1 grants +1000 MaxHealth" is expressed with the same types already
//! understood by the stat modifier pipeline.

use serde::{Deserialize, Serialize};

use crate::stats::events::{ModifierOp, StatField};

/// Effect applied by an item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemEffect {
    /// Permanent stat bonus applied while the item is equipped.
    ///
    /// Example: `StatBonus { field: StatField::MaxHealth, op: Add, value: 1000.0 }`
    /// for "Spada 1 grants +1000 MaxHealth".
    StatBonus {
        field: StatField,
        op: ModifierOp,
        value: f32,
    },

    /// Instant heal applied when a consumable is used. Reserved for future
    /// consumables; the server applies it on `UseItemCommand`.
    InstantHeal { amount: f32 },
    // Future extensions: ProcOnHit, Aura, OnUse, ...
}

impl ItemEffect {
    /// True if the effect must be active for as long as the item is equipped
    /// (as opposed to one-shot effects like `InstantHeal`).
    pub fn is_passive_while_equipped(&self) -> bool {
        matches!(self, ItemEffect::StatBonus { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_bonus_is_passive_while_equipped() {
        let effect = ItemEffect::StatBonus {
            field: StatField::MaxHealth,
            op: ModifierOp::Add,
            value: 1000.0,
        };
        assert!(effect.is_passive_while_equipped());
    }

    #[test]
    fn instant_heal_is_not_passive() {
        let effect = ItemEffect::InstantHeal { amount: 50.0 };
        assert!(!effect.is_passive_while_equipped());
    }

    #[test]
    fn stat_bonus_roundtrips_through_serde() {
        let effect = ItemEffect::StatBonus {
            field: StatField::Armor,
            op: ModifierOp::Multiply,
            value: 1.5,
        };
        let json = serde_json::to_string(&effect).expect("serialize");
        let back: ItemEffect = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(effect, back);
    }
}
