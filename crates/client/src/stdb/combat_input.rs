//! Bevy input for authoritative weapon and armor casts.
//!
//! This system deliberately sends only slot/source and the currently selected
//! target. Ability resolution, inscriptions, cast timing and cooldowns remain
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
        (KeyAction::CastPrimary, AbilitySlot::Primary),
        (KeyAction::CastSecondary, AbilitySlot::Secondary),
        (KeyAction::CastUltimate, AbilitySlot::Ultimate),
    ] {
        if settings.just_pressed(action, &keyboard) {
            let _ = commands::eidolon_cast(&connection, slot, target_entity, target_position);
        }
    }

    for (action, slot, ability_slot) in [
        (KeyAction::CastHelmet, EquipSlot::Helmet, AbilitySlot::Primary),
        (KeyAction::CastChestplate, EquipSlot::Armor, AbilitySlot::Primary),
        (KeyAction::CastBoots, EquipSlot::Shoes, AbilitySlot::Primary),
        (KeyAction::CastHelmetSecondary, EquipSlot::Helmet, AbilitySlot::Secondary),
        (KeyAction::CastChestplateSecondary, EquipSlot::Armor, AbilitySlot::Secondary),
        (KeyAction::CastBootsSecondary, EquipSlot::Shoes, AbilitySlot::Secondary),
    ] {
        if settings.just_pressed(action, &keyboard) {
            let _ = commands::armor_cast(
                &connection,
                slot,
                ability_slot,
                target_entity,
                target_position,
            );
        }
    }
}
