use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::movement::MoveTarget;
use bevymmo_shared::network::protocol::{
    Channel2, LookDirection, NetworkEntityId, Position, SpellCastCommand, SpellCastRelease,
};
use bevymmo_shared::spells::{
    CastKind, ChannelMovementPolicy, HotbarSlot, SpellHotbar, SpellId, SpellRegistry,
};
use bevymmo_shared::targeting::CurrentTarget;
use bevymmo_shared::user_settings::{GameSettingsResource, KeyAction};
use lightyear::prelude::Controlled;
use lightyear::prelude::MessageSender;

use crate::game_state::{GameScreen, Screen};
use crate::spells::cast_bar::ObservedCasts;
use crate::spells::ui::{SpellHudCooldownStarted, SpellHudState};

#[allow(clippy::too_many_arguments)]
pub fn cast_spells_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    settings: Res<GameSettingsResource>,
    screen: Res<GameScreen>,
    hud_state: Res<SpellHudState>,
    current_target: Res<CurrentTarget>,
    target_ids: Query<&NetworkEntityId>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut controlled_players: Query<
        (
            &SpellHotbar,
            &NetworkEntityId,
            &Position,
            &mut LookDirection,
        ),
        With<Controlled>,
    >,
    observed_casts: Res<ObservedCasts>,
    mut move_target: ResMut<MoveTarget>,
    mut cast_senders: Query<&mut MessageSender<SpellCastCommand>, With<ConnectedClient>>,
    mut release_senders: Query<&mut MessageSender<SpellCastRelease>, With<ConnectedClient>>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
    registry: Res<SpellRegistry>,
) {
    if !matches!(screen.0, Screen::InGame | Screen::Paused) {
        return;
    }
    let Some(keys) = keys else {
        return;
    };

    let Ok((hotbar, local_network_id, player_position, mut look_direction)) =
        controlled_players.single_mut()
    else {
        return;
    };

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

    // Pre-compute the desired facing once per frame: all hotbar keys that fire
    // a cast in this system share the same cursor ground point, so reusing the
    // value keeps the code DRY.
    let cast_face_direction = target_position.and_then(|target| {
        let offset = target - player_position.0;
        let length = offset.length();
        if length > 0.001 {
            Some(offset / length)
        } else {
            None
        }
    });

    let check_slot = |action: KeyAction, slot: HotbarSlot| {
        if settings.just_pressed(action, &keys) {
            hotbar.spell_for_slot(slot).cloned()
        } else {
            None
        }
    };

    for (action, slot) in [
        (KeyAction::CastSpellQ, HotbarSlot::Q),
        (KeyAction::CastSpellW, HotbarSlot::W),
        (KeyAction::CastSpellE, HotbarSlot::E),
    ] {
        let Some(spell_id) = check_slot(action, slot) else {
            continue;
        };

        let Some(spell_def) = registry.get(&spell_id) else {
            continue;
        };

        // Apply immediate client-side feedback the first time this spell is
        // actually cast this frame: snap the player's facing toward the cursor
        // and stop movement for any cast that the server will also freeze.
        // Doing this on the predicted entity avoids the ~100ms replication lag
        // of SpellCastProgress and the rubber-band that would otherwise occur
        // because the local LookDirection (a predicted component) keeps being
        // recomputed by `move_towards_target` toward the old move target.
        if let Some(direction) = cast_face_direction {
            look_direction.0 = direction;
        }
        if stops_movement_for_cast(spell_def.cast_kind(), spell_def.config().channel_movement) {
            move_target.0 = None;
        }

        if spell_def.cast_kind() == CastKind::Channeling {
            let is_channeling_this_spell = observed_casts
                .0
                .get(&local_network_id.0)
                .is_some_and(|cast| cast.spell_id == spell_id.as_str());

            if is_channeling_this_spell {
                for mut sender in release_senders.iter_mut() {
                    sender.send::<Channel2>(SpellCastRelease {
                        spell_id: spell_id.as_str().to_owned(),
                    });
                }
                continue;
            }

            if hud_state.is_on_cooldown(&spell_id) {
                continue;
            }

            for mut sender in cast_senders.iter_mut() {
                sender.send::<Channel2>(SpellCastCommand {
                    spell_id: spell_id.as_str().to_owned(),
                    target_position,
                    target_id,
                });
            }
            hud_cooldowns.write(SpellHudCooldownStarted {
                spell_id: spell_id.clone(),
                cooldown_seconds: spell_def.config().cooldown_seconds,
            });
            continue;
        }

        if hud_state.is_on_cooldown(&spell_id) {
            continue;
        }
        for mut sender in cast_senders.iter_mut() {
            sender.send::<Channel2>(SpellCastCommand {
                spell_id: spell_id.as_str().to_owned(),
                target_position,
                target_id,
            });
        }
        if matches!(spell_def.cast_kind(), CastKind::Instant) {
            hud_cooldowns.write(SpellHudCooldownStarted {
                spell_id: spell_id.clone(),
                cooldown_seconds: spell_def.config().cooldown_seconds,
            });
        }
    }
}

/// Mirrors `bevymmo_shared::movement::should_block_movement_for_cast` for
/// the locally predicted entity so feedback is instant instead of waiting for
/// the next `SpellCastProgress` replication (~100ms + RTT).
///
/// Rules:
/// - Instant: keep moving.
/// - CastTime: stop.
/// - Channeling: stop only when the spell's policy is `InterruptOnMove`.
///   `AllowMovement` (Swift) keeps the player running.
pub(crate) fn stops_movement_for_cast(
    cast_kind: CastKind,
    channel_movement: ChannelMovementPolicy,
) -> bool {
    match cast_kind {
        CastKind::Instant => false,
        CastKind::CastTime => true,
        CastKind::Channeling => channel_movement == ChannelMovementPolicy::InterruptOnMove,
    }
}
