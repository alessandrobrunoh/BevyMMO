use bevy::prelude::*;
use bevymmo_shared::network::protocol::{
    EntityColor, Inputs, PlayerId, PlayerMessage, SpellVisualEffect,
};
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::*;

use crate::network::types::ConnectedClient;

/// Validated player name waiting to be sent to the server.
///
/// Populated when the user requests a connection and consumed as soon as the
/// Lightyear `Client` enters the `Connected` state.
#[derive(Resource, Default, Debug)]
pub struct PendingJoinRequest(pub Option<String>);

/// The user explicitly requested a disconnect while the link was active.
#[derive(Component)]
pub struct DisconnectRequested;

/// Deferred cleanup: avoids despawning the link in the same frame that
/// Lightyear applies its own teardown commands.
#[derive(Component)]
pub struct PendingClientCleanup;

/// Reduces saturation on predicted entities so the local player is visually distinct.
pub fn handle_predicted_spawn(
    trigger: On<Add, (PlayerId, Predicted)>,
    mut predicted: Query<&mut EntityColor, Without<Controlled>>,
) {
    if let Ok(mut color) = predicted.get_mut(trigger.entity) {
        let hsva = Hsva {
            saturation: 0.4,
            ..Hsva::from(color.0)
        };
        color.0 = bevy::color::Color::from(hsva);
    }
}

/// Reduces saturation of the controlled (local) player even further.
pub fn lower_controlled_saturation(mut controlled: Query<&mut EntityColor, Added<Controlled>>) {
    for mut color in controlled.iter_mut() {
        let hsva = Hsva {
            saturation: 0.2,
            ..Hsva::from(color.0)
        };
        color.0 = bevy::color::Color::from(hsva);
    }
}

/// Adds the local action state used by client-side movement prediction.
///
/// Movement is sent explicitly through `MoveCommand`, so this entity must not
/// receive Lightyear's `InputMarker<Inputs>` as well. Registering both paths
/// makes the native input plugin build redundant tick sequences and can cause
/// an unbounded allocation when its tick range wraps.
pub fn handle_controlled_spawn(
    trigger: On<Add, Controlled>,
    mut commands: Commands,
    players: Query<&PlayerId, Without<ActionState<Inputs>>>,
) {
    let entity = trigger.entity;
    let Ok(player_id) = players.get(entity) else {
        return;
    };
    info!("Adding local ActionState to controlled player {entity:?} {player_id:?}");
    commands
        .entity(entity)
        .insert(ActionState::<Inputs>::default());
}

/// Reduces saturation on interpolated entities (other players / other entities).
pub fn handle_interpolated_spawn(
    trigger: On<Add, Interpolated>,
    mut interpolated: Query<&mut EntityColor>,
) {
    if let Ok(mut color) = interpolated.get_mut(trigger.entity) {
        let hsva = Hsva {
            saturation: 0.1,
            ..Hsva::from(color.0)
        };
        color.0 = bevy::color::Color::from(hsva);
    }
}

/// Receives simple debug messages from the server.
pub fn receive_messages(mut receiver: Single<&mut MessageReceiver<PlayerMessage>>) {
    for message in receiver.receive() {
        info!("Received message: {:?}", message);
    }
}

/// Converts server -> client visual messages into local Bevy messages consumed
/// by presentation systems.
pub fn receive_spell_visual_effects(
    mut receivers: Query<&mut MessageReceiver<SpellVisualEffect>, With<ConnectedClient>>,
    mut local_effects: MessageWriter<SpellVisualEffect>,
) {
    for mut receiver in receivers.iter_mut() {
        for effect in receiver.receive() {
            local_effects.write(effect.clone());
        }
    }
}
