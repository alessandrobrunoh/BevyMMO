//! Server-authoritative point-and-click movement.
//!
//! Holds the systems that read buffered client inputs and advance each
//! `Player` towards its target on the authoritative simulation. The shared
//! stepping math lives in `bevymmo_shared::movement`.

use bevy::prelude::*;
use lightyear::prelude::client::input::InputSystems;
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::*;

use bevymmo_shared::crowd_control::CrowdControlState;
use bevymmo_shared::entity::components::EntityState;
use bevymmo_shared::entity::player::components::Player;
use bevymmo_shared::movement::{move_towards_target, effective_movement_speed, should_block_movement_for_cast, MoveTarget};
use bevymmo_shared::network::mode;
use bevymmo_shared::network::protocol::{Inputs, LookDirection, Position};
use bevymmo_shared::spells::CastProgress;
use bevymmo_shared::stats::components::MovementStats;
use bevymmo_shared::stats::modifiers::ActiveStatModifiers;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MoveTarget>();
        app.add_systems(
            FixedPreUpdate,
            buffer_move_input
                .in_set(InputSystems::WriteClientInputs)
                .run_if(mode::has_client),
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

/// Authoritative movement: the server updates only Players towards the received target.
fn server_move_to_target(
    mut players: Query<
        (
            &mut Position,
            &ActionState<Inputs>,
            &MovementStats,
            &mut LookDirection,
            &mut EntityState,
            Option<&ActiveStatModifiers>,
            Option<&CastProgress>,
            Option<&CrowdControlState>,
        ),
        With<Player>,
    >,
) {
    for (position, input, stats, look_direction, mut state, modifiers, cast, cc_state) in
        &mut players
    {
        if cc_state.map(|c| c.has_blocking_cc()).unwrap_or(false) {
            *state = EntityState::Idle;
            continue;
        }

        if should_block_movement_for_cast(cast) {
            *state = EntityState::Idle;
            continue;
        }

        let effective_speed = effective_movement_speed(stats.speed, modifiers);
        move_towards_target(position, look_direction, &input.0, effective_speed, state);
    }
}
