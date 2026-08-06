//! Plugin delle statistiche: registra componenti, eventi e sistemi
//! server-authoritative.

use bevy::prelude::*;

use crate::network::mode;
use crate::stats::components::{CombatStats, MovementStats, VitalStats};
use crate::stats::events::{ApplyStatModifierEvent, DamageEvent, HealEvent};

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MovementStats>();
        app.register_type::<CombatStats>();
        app.register_type::<VitalStats>();

        app.add_message::<DamageEvent>();
        app.add_message::<HealEvent>();
        app.add_message::<ApplyStatModifierEvent>();

        app.add_systems(
            FixedUpdate,
            (
                crate::stats::systems::apply_damage,
                crate::stats::systems::apply_healing,
                crate::stats::systems::apply_stat_modifiers,
                crate::stats::systems::tick_stat_modifiers,
            )
                .run_if(mode::has_server),
        );
    }
}
