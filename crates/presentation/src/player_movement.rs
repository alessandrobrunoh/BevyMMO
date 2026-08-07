//! Client-side movement prediction.
//!
//! Mirrors the server-authoritative movement stepping on the locally predicted
//! `Player` so the client reacts immediately to clicks. Lives in the
//! presentation crate because it consumes `ObservedCasts` (the local mirror of
//! authoritative cast/channel state) to keep the Swift speed boost responsive
//! during prediction without rubber-banding.

use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::prelude::input::native::ActionState;

use bevymmo_shared::crowd_control::CrowdControlState;
use bevymmo_shared::entity::components::EntityState;
use bevymmo_shared::entity::player::components::Player;
use bevymmo_shared::movement::{move_towards_target, effective_movement_speed};
use bevymmo_shared::network::mode;
use bevymmo_shared::network::protocol::{Inputs, LookDirection, NetworkEntityId, Position};
use bevymmo_shared::spells_impl::swift::SwiftSpell;
use bevymmo_shared::stats::components::{MovementStats, VitalStats};
use bevymmo_shared::stats::modifiers::ActiveStatModifiers;

use crate::spells::cast_bar::ObservedCasts;
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

    for (position, input, stats, vital, network_id, look_direction, mut state, modifiers, cc_state) in
        &mut players
    {
        if vital.is_dead() || state.is_dead() {
            *state = EntityState::Dead;
            continue;
        }

        if cc_state.map(|c| c.has_blocking_cc()).unwrap_or(false) {
            *state = EntityState::Idle;
            continue;
        }

        let effective_speed = predicted_effective_speed(
            stats.speed,
            modifiers,
            network_id,
            observed_casts.as_deref(),
        );

        if let Inputs::MoveTo(target) = &input.0 {
            let offset = *target - position.0;
            let distance = offset.length();
            if distance > 0.001 {
                let step = effective_speed.min(distance);
                let candidate = position.0 + offset / distance * step;
                if world_map
                    .collision
                    .as_ref()
                    .is_some_and(|grid| grid.is_blocked([candidate.x, candidate.y, candidate.z], 0.45))
                {
                    *state = EntityState::Idle;
                    continue;
                }
            }
        }

        move_towards_target(position, look_direction, &input.0, effective_speed, state);
    }
}

/// Mirrors the server-authoritative Swift speed boost for predicted movement.
///
/// The canonical buff still lives on the server as a stat modifier. The client
/// uses observed channel progress only to keep local prediction responsive while
/// holding `F`, avoiding visible rubber-banding.
fn predicted_effective_speed(
    base_speed: f32,
    modifiers: Option<&ActiveStatModifiers>,
    network_id: &NetworkEntityId,
    observed_casts: Option<&ObservedCasts>,
) -> f32 {
    let server_speed = effective_movement_speed(base_speed, modifiers);
    let Some(observed_casts) = observed_casts else {
        return server_speed;
    };
    let Some(cast) = observed_casts.0.get(&network_id.0) else {
        return server_speed;
    };
    if cast.spell_id != SwiftSpell::ID {
        return server_speed;
    }

    server_speed * SwiftSpell::SPEED_MULTIPLIER
}
