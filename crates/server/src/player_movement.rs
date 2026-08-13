//! Server-authoritative point-and-click movement.
//!
//! Holds the systems that read buffered client inputs and advance each
//! `Player` towards its target on the authoritative simulation. The shared
//! stepping math lives in `bevymmo_shared::movement`.

use bevy::prelude::*;
use lightyear::prelude::client::input::InputSystems;
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::*;
use std::collections::HashMap;

use bevymmo_shared::crowd_control::CrowdControlState;
use bevymmo_shared::entity::components::EntityState;
use bevymmo_shared::entity::player::components::Player;
use bevymmo_shared::movement::{
    effective_movement_speed, move_towards_target, should_block_movement_for_cast, snap_to_ground,
    step_on_terrain, MoveTarget, TerrainStep,
};
use bevymmo_shared::network::mode;
use bevymmo_shared::network::protocol::{Inputs, LookDirection, MoveCommand, PlayerId, Position};
use bevymmo_shared::spells::CastProgress;
use bevymmo_shared::stats::components::{MovementStats, VitalStats};
use bevymmo_shared::stats::modifiers::ActiveStatModifiers;

use crate::world::ServerWorldMap;

pub struct PlayerMovementPlugin;

#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerMoveTarget(pub Vec3);

#[derive(Resource, Default)]
struct LastLoggedMoveInputs(HashMap<Entity, Inputs>);

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MoveTarget>();
        app.init_resource::<LastLoggedMoveInputs>();
        app.add_systems(
            FixedPreUpdate,
            buffer_move_input
                .in_set(InputSystems::WriteClientInputs)
                .run_if(mode::has_client),
        );
        app.add_systems(
            Update,
            (receive_move_commands, forget_disconnected_players).run_if(mode::has_server),
        );
        app.add_systems(
            FixedUpdate,
            (server_move_to_target, log_authoritative_player_positions)
                .chain()
                .run_if(mode::has_server),
        );
    }
}

/// Writes the same input for each tick, so the server retains the command until arrival.
fn buffer_move_input(
    move_target: Res<MoveTarget>,
    mut players: Query<&mut ActionState<Inputs>, With<InputMarker<Inputs>>>,
) {
    let input = move_target.0.map(Inputs::MoveTo).unwrap_or(Inputs::Stop);

    for mut player in &mut players {
        player.0 = input.clone();
    }
}

fn receive_move_commands(
    mut commands: Commands,
    mut receivers: Query<(&mut MessageReceiver<MoveCommand>, &RemoteId)>,
    players: Query<(Entity, &PlayerId), With<Player>>,
) {
    for (mut receiver, remote_id) in &mut receivers {
        for command in receiver.receive() {
            let Some((player_entity, _)) = players
                .iter()
                .find(|(_, player_id)| player_id.0 == remote_id.0)
            else {
                continue;
            };
            // `trace!`, not `info!`: this fires once per inbound `MoveCommand`
            // (up to 20/s per player, unthrottled), so at `info` it turned any
            // client that spams the channel into log amplification.
            trace!(
                "Received authoritative move target for {player_entity:?}: {:?}",
                command.target
            );
            commands
                .entity(player_entity)
                .insert(PlayerMoveTarget(command.target));
        }
    }
}

/// Drops the per-entity log state of players that left.
///
/// [`LastLoggedMoveInputs`] is keyed by `Entity` and was never pruned, so a
/// long-running server accumulated one entry for every player that had ever
/// connected. Runs in `Update` rather than `FixedUpdate` because removal
/// detection is cleared per frame, and `FixedUpdate` may not run on a given
/// frame.
fn forget_disconnected_players(
    mut last_inputs: ResMut<LastLoggedMoveInputs>,
    mut removed: RemovedComponents<Player>,
) {
    for entity in removed.read() {
        last_inputs.0.remove(&entity);
    }
}

/// Authoritative movement: the server updates only Players towards the received target.
fn server_move_to_target(
    mut last_inputs: ResMut<LastLoggedMoveInputs>,
    world_map: Res<ServerWorldMap>,
    mut players: Query<
        (
            Entity,
            &mut Position,
            &ActionState<Inputs>,
            Option<&PlayerMoveTarget>,
            &MovementStats,
            &VitalStats,
            &mut LookDirection,
            &mut EntityState,
            Option<&ActiveStatModifiers>,
            Option<&CastProgress>,
            Option<&CrowdControlState>,
        ),
        With<Player>,
    >,
) {
    for (
        entity,
        mut position,
        input,
        move_target,
        stats,
        vital,
        mut look_direction,
        mut state,
        modifiers,
        cast,
        cc_state,
    ) in &mut players
    {
        if vital.is_dead() || state.is_dead() {
            *state = EntityState::Dead;
            continue;
        }
        if last_inputs.0.get(&entity) != Some(&input.0) {
            info!("Server movement input for {entity:?}: {:?}", input.0);
            last_inputs.0.insert(entity, input.0.clone());
        }

        if cc_state.map(|c| c.has_blocking_cc()).unwrap_or(false) {
            *state = EntityState::Idle;
            continue;
        }

        if should_block_movement_for_cast(cast) {
            *state = EntityState::Idle;
            continue;
        }

        let effective_speed = effective_movement_speed(stats.speed, modifiers);
        let authoritative_input = move_target
            .map(|target| Inputs::MoveTo(target.0))
            .unwrap_or_else(|| input.0.clone());

        if !world_map.surface_query.is_empty() {
            let max_step_height = world_map.manifest.get_world_metrics().max_step_height;
            // Recovery: align persisted/spawned/teleported positions with the
            // authoritative terrain before reachability checks. A player loaded
            // at y=0 on a raised hill must not be treated as too low to climb
            // the surface already below their feet.
            snap_to_ground(&mut position.0, &world_map.surface_query, max_step_height);

            let Inputs::MoveTo(target) = authoritative_input else {
                *state = EntityState::Idle;
                continue;
            };

            let offset_xz = Vec3::new(target.x - position.0.x, 0.0, target.z - position.0.z);
            let distance = offset_xz.length();
            if distance > 0.001 {
                look_direction.0 = offset_xz.normalize_or_zero();
            }

            match step_on_terrain(
                position.0,
                target.x,
                target.z,
                effective_speed,
                &world_map.surface_query,
                &world_map.collision,
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
                    snap_to_ground(&mut position.0, &world_map.surface_query, max_step_height);
                    *state = EntityState::Idle;
                }
            }
            continue;
        }

        if let Inputs::MoveTo(target) = &authoritative_input {
            let offset = *target - position.0;
            let distance = offset.length();
            if distance > 0.001 {
                let step = effective_speed.min(distance);
                let candidate = position.0 + offset / distance * step;
                if world_map
                    .collision
                    .is_blocked([candidate.x, candidate.y, candidate.z], 0.45)
                {
                    *state = EntityState::Idle;
                    continue;
                }
            }
        }

        move_towards_target(
            position,
            look_direction,
            &authoritative_input,
            effective_speed,
            state,
        );
    }
}

/// Emits the authoritative player position often enough to inspect terrain climbing.
///
/// Server-side logging is intentionally used instead of client UI/chat because the
/// server owns the replicated `Position`. If `pos.y` follows `ground_y` here, any
/// remaining visual mismatch is presentation-side rather than gameplay-side.
///
/// # Example
/// ```text
/// SERVER PLAYER XYZ Entity(42): pos=(1.250, 3.410, -0.750) ground_y=Some(3.410) target=Some(...) state=Moving
/// ```
fn log_authoritative_player_positions(
    time: Res<Time>,
    mut elapsed_seconds: Local<f32>,
    world_map: Res<ServerWorldMap>,
    players: Query<(Entity, &Position, Option<&PlayerMoveTarget>, &EntityState), With<Player>>,
) {
    *elapsed_seconds += time.delta_secs();
    if *elapsed_seconds < 0.25 {
        return;
    }
    *elapsed_seconds = 0.0;

    for (entity, position, move_target, state) in &players {
        let ground_y = world_map
            .surface_query
            .ground_at(position.x, position.z)
            .map(|contact| contact.height);
        let target = move_target.map(|target| target.0);
        info!(
            "SERVER PLAYER XYZ {entity:?}: pos=({:.3}, {:.3}, {:.3}) ground_y={:?} target={:?} state={:?}",
            position.x, position.y, position.z, ground_y, target, state
        );
    }
}
