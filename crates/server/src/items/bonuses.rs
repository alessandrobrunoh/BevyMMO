//! Derived-stats recomputation for equipped items.
//!
//! Base stats (from the DB) are the source of truth. When `Equipment` changes,
//! we revert the previously applied bonus and re-apply the bonus derived from
//! the current equipment, storing the base snapshot transiently so we can
//! revert again on the next change without reloading from the DB.

use bevy::prelude::*;

use bevymmo_shared::items::components::{EquipSlot, Equipment};
use bevymmo_shared::items::effects::ItemEffect;
use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::stats::components::{CombatStats, MovementStats, StatsBundleData, VitalStats};
use bevymmo_shared::stats::events::{ModifierOp, StatField};

/// Single stat bonus to apply (or revert) on top of base stats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatBonusEffect {
    pub field: StatField,
    pub op: ModifierOp,
    pub value: f32,
}

/// Transient record of the base stats used for the currently applied bonus.
///
/// NOT replicated: the client sees the post-bonus stats through normal
/// replication and never needs to know the delta.
#[derive(Component, Debug, Clone, Default)]
pub struct AppliedEquipmentBonus {
    /// Base stats (without equipment bonus) at the time of the last apply.
    ///
    /// `None` means "never applied yet": on first run we treat the current
    /// stats as base (they were just loaded from the DB at spawn).
    pub base: Option<StatsBundleData>,
}

/// Collects the passive stat bonuses of every equipped item.
fn compute_bonus(equipment: &Equipment, registry: &ItemRegistry) -> Vec<StatBonusEffect> {
    let mut effects: Vec<StatBonusEffect> = Vec::new();

    let mut collect = |item_id: &bevymmo_shared::items::registry::ItemId| {
        let Some(item) = registry.get(item_id) else {
            bevy::log::warn!("equipped item {} not in registry", item_id.as_str());
            return;
        };
        for effect in item.effects() {
            if !effect.is_passive_while_equipped() {
                continue;
            }
            if let ItemEffect::StatBonus { field, op, value } = effect {
                effects.push(StatBonusEffect {
                    field: *field,
                    op: *op,
                    value: *value,
                });
            }
        }
    };

    for slot in EquipSlot::ALL {
        if let Some(item_id) = equipment.get(slot) {
            collect(item_id);
        }
    }

    effects
}

/// Applies a stat bonus to the runtime stats.
fn apply_effect(stats: &mut (MovementStats, CombatStats, VitalStats), effect: &StatBonusEffect) {
    let (movement, combat, vital) = stats;
    let slot: &mut f32 = match effect.field {
        StatField::Speed => &mut movement.speed,
        StatField::Armor => &mut combat.armor,
        StatField::AttackPower => &mut combat.attack_power,
        StatField::MaxHealth => &mut vital.max_health,
        StatField::ManaRegeneration => &mut vital.mana_regeneration,
    };
    match effect.op {
        ModifierOp::Add => *slot += effect.value,
        ModifierOp::Multiply => *slot *= effect.value,
        ModifierOp::Override => *slot = effect.value,
    }
}

/// Returns the base stats (without equipment bonus) for `applied`.
///
/// Used by the disconnect path so the DB always stores base stats, avoiding
/// double-counting the bonus on the next join.
pub fn base_stats_without_equipment(
    movement: &MovementStats,
    combat: &CombatStats,
    vital: &VitalStats,
    applied: &AppliedEquipmentBonus,
) -> StatsBundleData {
    applied
        .base
        .unwrap_or_else(|| StatsBundleData::from_components(movement, combat, vital))
}

/// When `Equipment` changes, revert the previous bonus and re-apply the bonus
/// derived from the current equipment.
///
/// Runs on `Changed<Equipment>`, which also fires right after spawn, so a
/// freshly joined player gets their persisted equipment bonus applied exactly
/// once.
pub fn recompute_equipment_bonuses(
    mut players: Query<
        (
            &Equipment,
            &mut CombatStats,
            &mut VitalStats,
            &mut MovementStats,
            &mut AppliedEquipmentBonus,
        ),
        Changed<Equipment>,
    >,
    registry: Res<ItemRegistry>,
) {
    for (equipment, mut combat, mut vital, mut movement, mut applied) in &mut players {
        // 1. Recover the base stats snapshot (current stats at first run).
        let base = match applied.base.take() {
            Some(base) => base,
            None => StatsBundleData::from_components(&movement, &combat, &vital),
        };

        // 2. Revert: bring the runtime stats back to base.
        let (base_movement, base_combat, base_vital) = base.into_components();
        *movement = base_movement;
        *combat = base_combat;
        *vital = base_vital;

        // 3. Remember the base for the next change.
        applied.base = Some(StatsBundleData::from_components(&movement, &combat, &vital));

        // 4. Re-apply the bonus derived from the current equipment.
        let bonus = compute_bonus(equipment, &registry);
        let mut stats = (*movement, *combat, *vital);
        for effect in &bonus {
            apply_effect(&mut stats, effect);
        }
        *movement = stats.0;
        *combat = stats.1;
        *vital = stats.2;

        // 5. Clamp health after a possible max_health shrink.
        vital.clamp_health();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_shared::items::registry::ItemId;

    #[test]
    fn base_stats_falls_back_to_current_stats_when_no_snapshot() {
        let applied = AppliedEquipmentBonus { base: None };
        let movement = MovementStats { speed: 0.15 };
        let combat = CombatStats {
            attack_power: 10.0,
            armor: 25.0,
        };
        let vital = VitalStats {
            current_health: 80.0,
            max_health: 100.0,
            max_mana: 100.0,
            mana_regeneration: 5.0,
        };

        let base = base_stats_without_equipment(&movement, &combat, &vital, &applied);
        assert_eq!(base.movement.speed, 0.15);
        assert_eq!(base.vital.max_health, 100.0);
    }

    #[test]
    fn base_stats_uses_snapshot_when_present() {
        let snapshot = StatsBundleData {
            movement: MovementStats { speed: 0.15 },
            combat: CombatStats {
                attack_power: 10.0,
                armor: 25.0,
            },
            vital: VitalStats {
                current_health: 50.0,
                max_health: 60.0,
                max_mana: 100.0,
                mana_regeneration: 5.0,
            },
        };
        let applied = AppliedEquipmentBonus {
            base: Some(snapshot.clone()),
        };

        // Runtime stats are different from the snapshot: the base must win.
        let movement = MovementStats { speed: 0.9 };
        let combat = CombatStats {
            attack_power: 999.0,
            armor: 999.0,
        };
        let vital = VitalStats {
            current_health: 999.0,
            max_health: 999.0,
            max_mana: 999.0,
            mana_regeneration: 999.0,
        };

        let base = base_stats_without_equipment(&movement, &combat, &vital, &applied);
        assert_eq!(base, snapshot);
    }

    #[test]
    fn apply_effect_adds_to_max_health() {
        let mut stats = (
            MovementStats { speed: 0.15 },
            CombatStats {
                attack_power: 10.0,
                armor: 25.0,
            },
            VitalStats {
                current_health: 100.0,
                max_health: 100.0,
                max_mana: 100.0,
                mana_regeneration: 5.0,
            },
        );

        apply_effect(
            &mut stats,
            &StatBonusEffect {
                field: StatField::MaxHealth,
                op: ModifierOp::Add,
                value: 1000.0,
            },
        );

        assert_eq!(stats.2.max_health, 1100.0);
        assert_eq!(stats.2.current_health, 100.0);
    }

    #[test]
    fn compute_bonus_ignores_unknown_equipped_item() {
        let equipment = Equipment {
            weapon: Some(ItemId::new("does_not_exist")),
            ..Default::default()
        };
        let registry = ItemRegistry::default();

        let bonus = compute_bonus(&equipment, &registry);
        assert!(bonus.is_empty());
    }
}
