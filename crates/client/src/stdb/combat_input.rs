//! Bevy input for authoritative armor casts.
//!
//! Weapon Q/W/E and 1/2/3 go through `cast_abilities_on_key` so Charge
//! abilities get a press (`eidolon_cast`) and a release (`release_cast`).
//! This system only sends armor slot/source plus the selected target.
//! Ability resolution, inscriptions, cast timing and cooldowns remain
//! server-authoritative in SpacetimeDB.

use bevy::prelude::*;
use bevymmo_gameplay::abilities::AbilitySlot;
use bevymmo_gameplay::items::EquipSlot;
use bevymmo_network::world_components::{NetworkEntityId, Position};

use crate::targeting::CurrentTarget;
use crate::user_settings::{GameSettingsResource, KeyAction};

use super::commands;
use super::plugin::StdbConnection;

/// Sends one cast request for each combat action pressed during this frame.
///
/// A target is optional: the server resolves range and targeting from the
/// ability blueprint, while the selected entity/position only supplies the
/// player's intent.
pub fn send_combat_inputs(
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    connection: Res<StdbConnection>,
    current_target: Res<CurrentTarget>,
    target_entities: Query<(&NetworkEntityId, &Position)>,
) {
    let target = current_target
        .entity
        .and_then(|entity| target_entities.get(entity).ok())
        .map(|(network_id, position)| (Some(network_id.0), Some(position.0)));
    let (target_entity, target_position) = target.unwrap_or((None, None));

    for (action, slot) in [
        (KeyAction::CastHelmet, EquipSlot::Helmet),
        (KeyAction::CastChestplate, EquipSlot::Armor),
        (KeyAction::CastBoots, EquipSlot::Shoes),
    ] {
        if settings.just_pressed(action, &keyboard) {
            let _ = commands::armor_cast(
                &connection,
                slot,
                AbilitySlot::Primary,
                target_entity,
                target_position,
            );
        }
    }
}
