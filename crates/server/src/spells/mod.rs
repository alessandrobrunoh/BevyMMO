//! Server-authoritative spell runtime.
//!
//! Owns the cast pipeline, persistent AoE lifecycle, homing projectile updates,
//! and other authoritative spell execution systems.

pub mod aoe;
pub mod projectile;
pub mod systems;

use bevy::prelude::*;

use bevymmo_shared::network::mode::has_server;
use bevymmo_shared::network::protocol::{SpellCastEnded, SpellCastProgress, SpellVisualEffect};
use bevymmo_shared::spells::events::{SpellCastRequest, SpellReleaseRequest};
use bevymmo_shared::spells::registry::SpellRegistry;


use crate::spells::projectile::update_homing_projectiles;
use crate::spells::systems::*;

/// Server-side umbrella plugin for the spells domain.
///
/// Sets up the authoritative cast pipeline only: registry initialization,
/// spell-related message channels, built-in spell registration at startup, and
/// the FixedUpdate chain that processes casts, advances progress, replicates
/// state, and ticks cooldowns. All systems are gated on `has_server` so this
/// plugin is safe to add in host-client builds where both roles coexist.
pub struct SpellsServerPlugin;

impl Plugin for SpellsServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpellRegistry>()
            .add_message::<SpellCastRequest>()
            .add_message::<SpellReleaseRequest>()
            .add_message::<SpellVisualEffect>()
            .add_message::<SpellCastProgress>()
            .add_message::<SpellCastEnded>()
            .add_systems(Startup, register_builtin_spells)
            .add_systems(
                FixedUpdate,
                (
                    process_cast_requests,
                    handle_cast_release,
                    advance_cast_progress,
                    replicate_cast_progress,
                    update_homing_projectiles,
                    aoe::update_aoe_regions,
                    tick_spell_cooldowns,
                )
                    .chain()
                    .run_if(has_server),
            );
    }
}

pub fn register_builtin_spells(_registry: ResMut<SpellRegistry>) {}
