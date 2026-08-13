//! Client input for the Eidolon cast pipeline.
//!
//! Routes Q/W/E to `EidolonCastCommand` instead of `SpellCastCommand`
//! whenever the equipped weapon has Eidolon gestures
//! (`Item::weapon_abilities()`), so the two pipelines never both fire for
//! the same key press — see the matching early-bail in
//! `crate::spells::input::cast_spells_on_key`.
//!
//! Instant-only for now: no CastTime/Channeling equivalent exists yet for
//! Eidolon abilities, so there is no movement-freeze/cast-bar handling here.

use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::abilities::AbilitySlot;
use bevymmo_shared::items::components::Equipment;
use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::network::protocol::{
    Channel2, EidolonCastCommand, LookDirection, NetworkEntityId, Position,
};
use bevymmo_shared::targeting::CurrentTarget;
use bevymmo_shared::user_settings::{GameSettingsResource, KeyAction};
use lightyear::prelude::Controlled;
use lightyear::prelude::MessageSender;

use crate::game_state::{GameScreen, Screen};

#[allow(clippy::too_many_arguments)]
pub fn cast_eidolon_abilities_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    settings: Res<GameSettingsResource>,
    screen: Res<GameScreen>,
    current_target: Res<CurrentTarget>,
    target_ids: Query<&NetworkEntityId>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut controlled_players: Query<(&Equipment, &Position, &mut LookDirection), With<Controlled>>,
    mut cast_senders: Query<&mut MessageSender<EidolonCastCommand>, With<ConnectedClient>>,
    registry: Res<ItemRegistry>,
) {
    if !matches!(screen.0, Screen::InGame | Screen::Paused) {
        return;
    }
    let Some(keys) = keys else {
        return;
    };

    let Ok((equipment, player_position, mut look_direction)) = controlled_players.single_mut() else {
        return;
    };

    // Only these keys when the equipped weapon actually has Eidolon
    // gestures — `cast_spells_on_key` owns them otherwise.
    let Some(weapon) = &equipment.weapon else {
        return;
    };
    let Some(item) = registry.get(&weapon.item_id) else {
        return;
    };
    if item.weapon_abilities().is_none() {
        return;
    }

    let mut target_position = None;
    if let Ok(window) = windows.single() {
        if let Some(cursor_position) = window.cursor_position() {
            if let Some((camera, camera_transform)) = cameras.iter().next() {
                if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                    if let Some(target) = ray.plane_intersection_point(
                        Vec3::ZERO,
                        bevy::math::primitives::InfinitePlane3d::new(Vec3::Y),
                    ) {
                        target_position = Some(Vec3::new(target.x, 0.0, target.z));
                    }
                }
            }
        }
    }

    let mut target_id = None;
    if let Some(target_entity) = current_target.entity {
        if let Ok(net_id) = target_ids.get(target_entity) {
            target_id = Some(net_id.0);
        }
    }

    let cast_face_direction = target_position.and_then(|target| {
        let offset = target - player_position.0;
        let length = offset.length();
        if length > 0.001 {
            Some(offset / length)
        } else {
            None
        }
    });

    for (action, slot) in [
        (KeyAction::CastSpellQ, AbilitySlot::Primary),
        (KeyAction::CastSpellW, AbilitySlot::Secondary),
        (KeyAction::CastSpellE, AbilitySlot::Ultimate),
    ] {
        if !settings.just_pressed(action, &keys) {
            continue;
        }

        if let Some(direction) = cast_face_direction {
            look_direction.0 = direction;
        }

        for mut sender in cast_senders.iter_mut() {
            sender.send::<Channel2>(EidolonCastCommand {
                slot,
                target_position,
                target_id,
            });
        }
    }
}
