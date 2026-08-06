use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::network::protocol::{
    Channel2, NetworkEntityId, SpellCastCommand, SpellCastRelease,
};
use bevymmo_shared::spells::{CastKind, HotbarSlot, SpellHotbar, SpellRegistry};
use bevymmo_shared::targeting::CurrentTarget;
use lightyear::prelude::Controlled;
use lightyear::prelude::MessageSender;

use crate::game_state::{GameScreen, Screen};
use crate::spells::cast_bar::ObservedCasts;
use crate::spells::ui::{SpellHudCooldownStarted, SpellHudState};

#[allow(clippy::too_many_arguments)]
pub fn cast_spells_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    screen: Res<GameScreen>,
    hud_state: Res<SpellHudState>,
    current_target: Res<CurrentTarget>,
    target_ids: Query<&NetworkEntityId>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    controlled_players: Query<(&SpellHotbar, &NetworkEntityId), With<Controlled>>,
    observed_casts: Res<ObservedCasts>,
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

    let Ok((hotbar, local_network_id)) = controlled_players.single() else {
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

    let check_slot = |key: KeyCode, slot: HotbarSlot| {
        if keys.just_pressed(key) {
            hotbar.spell_for_slot(slot).cloned()
        } else {
            None
        }
    };

    for (key, slot) in [
        (KeyCode::KeyQ, HotbarSlot::Q),
        (KeyCode::KeyW, HotbarSlot::W),
        (KeyCode::KeyE, HotbarSlot::E),
    ] {
        let Some(spell_id) = check_slot(key, slot) else {
            continue;
        };

        let Some(spell_def) = registry.get(&spell_id) else {
            continue;
        };

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
