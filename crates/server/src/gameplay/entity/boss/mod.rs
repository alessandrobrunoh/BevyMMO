//! Server-authoritative boss systems, target selection helpers, and plugin.

use bevy::prelude::*;

use bevymmo_shared::entity::boss::components::{BossArena, BossPhase};
use bevymmo_shared::network::mode::has_server;

use crate::gameplay::entity::boss::systems::{
    accrue_threat, boss_aggro_check, boss_chase, run_boss_rotation, update_boss_phase,
};

pub mod systems;
pub mod target_select;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        // Replicated components must be registered in the Bevy type registry
        // for lightyear to (de)serialize them.
        app.register_type::<BossPhase>();
        app.register_type::<BossArena>();

        // Server-authoritative encounter control. `accrue_threat` runs before
        // the rotation driver (Phase 2) so a fresh damage tick can influence
        // the next target pick within the same fixed step.
        app.add_systems(
            FixedUpdate,
            (
                boss_aggro_check,
                accrue_threat,
                update_boss_phase,
                boss_chase,
                run_boss_rotation,
            )
                .chain()
                .run_if(has_server),
        );
    }
}
