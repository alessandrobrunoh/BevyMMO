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
use bevymmo_shared::movement::{move_towards_target, effective_movement_speed, should_block_movement_for_cast, MoveTarget};
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
            receive_move_commands.run_if(mode::has_server),
        );
        app.add_systems(FixedUpdate, server_move_to_target.run_if(mode::has_server));
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
            info!(
                "Received authoritative move target for {player_entity:?}: {:?}",
                command.target
            );
            commands
                .entity(player_entity)
                .insert(PlayerMoveTarget(command.target));
        }
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
    for (entity, position, input, move_target, stats, vital, look_direction, mut state, modifiers, cast, cc_state) in
        &mut players
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

        if let Inputs::MoveTo(target) = &authoritative_input {
            let offset = *target - position.0;
            let distance = offset.length();
            if distance > 0.001 {
                let step = effective_speed.min(distance);
                let candidate = position.0 + offset / distance * step;
                if world_map.collision.is_blocked([candidate.x, candidate.y, candidate.z], 0.45) {
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
