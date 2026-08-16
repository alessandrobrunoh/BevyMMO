//! Client-side movement prediction.
//!
//! Mirrors the server-authoritative movement stepping on the locally predicted
//! `Player` so the client reacts immediately to clicks. Lives in the
//! presentation crate because it consumes `ObservedCasts` (the local mirror of
//! authoritative cast/channel state) to prevent movement during casts without
//! rubber-banding.

use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::*;

use bevymmo_shared::crowd_control::CrowdControlState;
use bevymmo_shared::entity::components::EntityState;
use bevymmo_shared::entity::player::components::Player;
use bevymmo_shared::movement::{
    effective_movement_speed, move_towards_target, snap_to_ground, step_on_terrain, TerrainStep,
};
use bevymmo_shared::network::mode;
use bevymmo_shared::network::protocol::{Inputs, LookDirection, NetworkEntityId, Position};
use bevymmo_shared::spells::{ChannelMovementPolicy, SpellId, SpellRegistry};

use bevymmo_shared::stats::components::{MovementStats, VitalStats};
use bevymmo_shared::stats::modifiers::ActiveStatModifiers;

use crate::spells::cast_bar::{ObservedCast, ObservedCasts};
use crate::world::ClientWorldMap;

pub struct PlayerMovementPredictionPlugin;

impl Plugin for PlayerMovementPredictionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, predict_move_to_target.run_if(mode::has_client));
    }
}

/// Same calculation on predicted Player for immediate click response.
fn predict_move_to_target(
    synced_client: Query<(), (With<Client>, With<IsSynced<()>>)>,
    observed_casts: Option<Res<ObservedCasts>>,
    spell_registry: Res<SpellRegistry>,
    world_map: Res<ClientWorldMap>,
    mut players: Query<
        (
            &mut Position,
            &ActionState<Inputs>,
            &MovementStats,
            &VitalStats,
            &NetworkEntityId,
            &mut LookDirection,
            &mut EntityState,
            Option<&ActiveStatModifiers>,
            Option<&CrowdControlState>,
        ),
        (With<Player>, With<Predicted>),
    >,
) {
    if synced_client.is_empty() {
        return;
    }

    for (
        mut position,
        input,
        stats,
        vital,
        network_id,
        mut look_direction,
        mut state,
        modifiers,
        cc_state,
    ) in &mut players
    {
        if vital.is_dead() || state.is_dead() {
            *state = EntityState::Dead;
            continue;
        }

        if cc_state.map(|c| c.has_blocking_cc()).unwrap_or(false) {
            *state = EntityState::Idle;
            continue;
        }

        // Mirror the server's cast-blocks-movement rule on the predicted
        // entity. Without this, the client keeps stepping toward the move
        // target during a cast and the server snaps it back every tick,
        // producing a visible rubber-band.
        if cast_blocks_movement(network_id.0, observed_casts.as_deref(), &spell_registry) {
            *state = EntityState::Idle;
            continue;
        }

        let effective_speed = effective_movement_speed(stats.speed, modifiers);

        if let Some(surface_query) = world_map.surface_query.as_ref() {
            if !surface_query.is_empty() {
                let max_step_height = world_map
                    .manifest
                    .as_ref()
                    .map(|m| m.get_world_metrics().max_step_height)
                    .unwrap_or_default();
                // Recovery: mirror the server-side terrain snap before
                // reachability checks so prediction starts from the same
                // height as the authoritative simulation.
                snap_to_ground(&mut position.0, surface_query, max_step_height);

                let Inputs::MoveTo(target) = &input.0 else {
                    *state = EntityState::Idle;
                    continue;
                };

                let offset_xz = Vec3::new(target.x - position.0.x, 0.0, target.z - position.0.z);
                let distance = offset_xz.length();
                if distance > 0.001 {
                    look_direction.0 = offset_xz.normalize_or_zero();
                }

                let Some(collision) = world_map.collision.as_ref() else {
                    *state = EntityState::Idle;
                    continue;
                };

                match step_on_terrain(
                    position.0,
                    target.x,
                    target.z,
                    effective_speed,
                    surface_query,
                    collision,
                    max_step_height,
                ) {
                    TerrainStep::Arrived(p) => {
                        position.0 = p;
                        *state = EntityState::Idle;
                    }
                    TerrainStep::Moved(p) => {
                        position.0 = p;
                        *state = EntityState::Moving;
                    }
                    TerrainStep::Blocked | TerrainStep::NoSurface => {
                        snap_to_ground(&mut position.0, surface_query, max_step_height);
                        *state = EntityState::Idle;
                    }
                }
                continue;
            }
        }

        if let Inputs::MoveTo(target) = &input.0 {
            let offset = *target - position.0;
            let distance = offset.length();
            if distance > 0.001 {
                let step = effective_speed.min(distance);
                let candidate = position.0 + offset / distance * step;
                if world_map.collision.as_ref().is_some_and(|grid| {
                    grid.is_blocked([candidate.x, candidate.y, candidate.z], 0.45)
                }) {
                    *state = EntityState::Idle;
                    continue;
                }
            }
        }

        move_towards_target(position, look_direction, &input.0, effective_speed, state);
    }
}

/// Client-side mirror of `bevymmo_shared::movement::should_block_movement_for_cast`,
/// driven by the locally observed cast snapshot instead of the authoritative
/// `CastProgress` component (which is server-only).
///
/// Matches the server rule: CastTime always freezes; Channeling only when the
/// spell's policy is `InterruptOnMove` (Swift allows simultaneous movement).
fn cast_blocks_movement(
    network_id: u64,
    observed_casts: Option<&ObservedCasts>,
    registry: &SpellRegistry,
) -> bool {
    let Some(observed_casts) = observed_casts else {
        return false;
    };
    let Some(cast) = observed_casts.0.get(&network_id) else {
        return false;
    };
    observed_cast_blocks_movement(cast, registry)
}

fn observed_cast_blocks_movement(cast: &ObservedCast, registry: &SpellRegistry) -> bool {
    // kind == 1 -> Channeling; anything else here is CastTime (Instant casts
    // never produce an `ObservedCast` entry, so we don't need to special-case them).
    if cast.kind != 1 {
        return true;
    }
    let spell_id = SpellId::new(cast.spell_id.clone());
    let Some(spell) = registry.get(&spell_id) else {
        return false;
    };
    spell.config().channel_movement == ChannelMovementPolicy::InterruptOnMove
}
