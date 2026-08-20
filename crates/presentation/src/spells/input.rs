//! Player ability input for the Eidolon cast pipeline.
//!
//! Routes the weapon HUD keys (default 1/2/3) to the press/aim/release path
//! for every equipped weapon that exposes `Item::ability_loadout()`. There is
//! no second, legacy `SpellHotbar` input path.
//!
//! # Cast behavior by [`AbilityCastMode`]
//!
//! - **Instant / CastTime**: press-to-aim, release-to-confirm.  The press opens
//!   an aim window ([`AbilityAim`]) during which
//!   [`crate::spells::aim_preview`] draws the exact impact area on the ground;
//!   the release closes it and sends the command.  A quick tap behaves like an
//!   instant cast because press and arrive a few frames apart.
//! - **Charge**: press starts the server-side charge via `eidolon_cast` and
//!   keeps the aim preview open; release calls `release_cast` with the cursor
//!   at that moment so the impact follows the hold, not the press.
//! - **Channeling**: press **immediately** starts the server-side channel via
//!   `eidolon_cast`; release calls `release_cast` with the resolved ability id
//!   to end it early.  An optimistic HUD cooldown starts at press time (the
//!   server does the same) so the key cannot be spammed while the channel runs.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::movement::ClientSurfaceQuery;
use bevymmo_client::stdb::{commands as stdb_commands, StdbConnection};
use bevymmo_client::targeting::CurrentTarget;
use bevymmo_client::user_settings::{GameSettingsResource, KeyAction};
use bevymmo_gameplay::abilities::{
    resolve_active_ability, weapon_cast_intent, AbilityAim, AbilityId, AbilitySlot, ArcBaseAbility,
    BaseAbilityRegistry,
};
use bevymmo_gameplay::items::components::Equipment;
use bevymmo_gameplay::items::registry::ItemRegistry;
use bevymmo_network::network::protocol::{LookDirection, NetworkEntityId, Position};

use crate::game_state::{GameScreen, Screen};
use crate::spells::cast_bar::ObservedCasts;
use crate::spells::cursor::{cursor_ground_point, flat_direction_towards};
use crate::spells::ui::{HudCooldownKey, SpellHudCooldownStarted, SpellHudState};

/// Canonical HUD + input mapping for the three weapon slots.
///
/// The hotbar labels these actions, and [`cast_abilities_on_key`] reads them
/// so a Charge bound to the printed key starts on press and fires on release.
pub const WEAPON_HUD_BINDINGS: [(KeyAction, AbilitySlot); 3] = [
    (KeyAction::CastPrimary, AbilitySlot::Primary),
    (KeyAction::CastSecondary, AbilitySlot::Secondary),
    (KeyAction::CastUltimate, AbilitySlot::Ultimate),
];

/// Every key that drives a weapon slot through the aim / charge / release path.
pub fn weapon_slot_bindings() -> impl Iterator<Item = (KeyAction, AbilitySlot)> {
    WEAPON_HUD_BINDINGS.into_iter()
}

/// Raycast and surface query parameters bundled for aiming.
#[derive(SystemParam)]
pub struct AimRaycastParams<'w, 's> {
    pub windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
    pub cameras: Query<'w, 's, (&'static Camera, &'static Transform), With<Camera3d>>,
    pub surface_query: Option<Res<'w, ClientSurfaceQuery>>,
}

#[allow(clippy::too_many_arguments)]
pub fn cast_abilities_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    settings: Res<GameSettingsResource>,
    screen: Res<GameScreen>,
    current_target: Res<CurrentTarget>,
    mut aim: ResMut<AbilityAim>,
    target_ids: Query<&NetworkEntityId>,
    aim_ray: AimRaycastParams,
    mut controlled_players: Query<
        (&Equipment, &Position, &NetworkEntityId, &mut LookDirection),
        With<LocalPlayer>,
    >,
    observed_casts: Res<ObservedCasts>,
    mut move_target: ResMut<bevymmo_client::movement::MoveTarget>,
    conn: Option<Res<StdbConnection>>,
    item_registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    hud_state: Res<SpellHudState>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
) {
    // Any condition that invalidates the aiming context must close the aim window,
    // otherwise a stale aim would fire on the next unrelated key-release.
    let Some(keys) = keys else {
        aim.clear();
        return;
    };
    if !matches!(screen.0, Screen::InGame | Screen::Paused) {
        aim.clear();
        return;
    }

    let Ok((equipment, player_position, local_network_id, mut look_direction)) =
        controlled_players.single_mut()
    else {
        aim.clear();
        return;
    };

    // Only weapons with Eidolon gestures drive Q/W/E.
    let Some(weapon) = &equipment.weapon else {
        aim.clear();
        return;
    };
    let Some(item) = item_registry.get(&weapon.item_id) else {
        aim.clear();
        return;
    };
    let Some(weapon_abilities) = item.ability_loadout() else {
        aim.clear();
        return;
    };

    let aiming = aim.slot.is_some();
    let needs_ground = aiming
        || weapon_slot_bindings().any(|(action, _)| {
            settings.just_pressed(action, &keys) || settings.just_released(action, &keys)
        });
    let target_position = if needs_ground {
        cursor_ground_point(
            &aim_ray.windows,
            &aim_ray.cameras,
            aim_ray.surface_query.as_deref(),
        )
    } else {
        None
    };

    let target_id = current_target
        .entity
        .and_then(|entity| target_ids.get(entity).ok())
        .map(|net_id| net_id.0);

    // ── Press handling ────────────────────────────────────────────────
    // Instant/CastTime open aim; Charge starts the server-side charge and
    // keeps the preview open; Channeling starts immediately.
    for (action, slot) in weapon_slot_bindings() {
        if !settings.just_pressed(action, &keys) {
            continue;
        }

        let Some((ability_id, ability)) =
            active_ability(slot, weapon_abilities, weapon, &ability_registry)
        else {
            continue;
        };

        let blueprint = item.ability_blueprint(ability.as_ref());
        let intent = weapon_cast_intent(true, false, blueprint.execution, ability.cast_mode());

        if intent.open_aim {
            aim.begin(slot);
        }
        if !intent.start_cast {
            continue;
        }
        if hud_state.ability_on_cooldown(&ability_id) {
            continue;
        }
        if let Some(conn) = conn.as_deref() {
            if let Err(err) = stdb_commands::eidolon_cast(conn, slot, target_id, target_position) {
                error!("could not start Eidolon cast: {err}");
            }
        }
        // Channel cooldown starts on press (server does the same). Charge
        // waits for release_cast.
        if ability.cast_mode().is_channeling() {
            hud_cooldowns.write(SpellHudCooldownStarted {
                key: HudCooldownKey::Ability(ability_id.clone()),
                cooldown_seconds: ability.base_params().cooldown,
            });
        }
    }

    // ── Per-frame aim tracking ──────────────────────────────────────
    // While aiming, face the cursor every frame so the preview (which reads
    // LookDirection) stays in sync with mouse movement.
    if aim.slot.is_some() {
        aim.ground_point = target_position;
        if let Some(direction) =
            target_position.and_then(|target| flat_direction_towards(player_position.0, target))
        {
            look_direction.0 = direction;
        }
    }

    // ── Release handling ─────────────────────────────────────────────
    // Instant/CastTime confirm the aimed `eidolon_cast`. Charge and
    // Channeling call `release_cast`. Channeling is not gated on `aim.slot`
    // because it never opens an aim window.
    for (action, slot) in weapon_slot_bindings() {
        if !settings.just_released(action, &keys) {
            continue;
        }

        let Some((ability_id, ability)) =
            active_ability(slot, weapon_abilities, weapon, &ability_registry)
        else {
            continue;
        };

        let blueprint = item.ability_blueprint(ability.as_ref());
        let intent = weapon_cast_intent(false, true, blueprint.execution, ability.cast_mode());

        if intent.start_cast {
            if aim.slot != Some(slot) {
                continue;
            }
            let cancelled = aim.cancelled;
            aim.clear();
            if cancelled {
                continue;
            }
            if hud_state.ability_on_cooldown(&ability_id) {
                continue;
            }

            let face_direction = target_position
                .and_then(|target| flat_direction_towards(player_position.0, target));
            if let Some(direction) = face_direction {
                look_direction.0 = direction;
            }
            if stops_movement_for_ability(ability.cast_mode()) {
                move_target.0 = None;
            }

            if let Some(conn) = conn.as_deref() {
                if let Err(err) =
                    stdb_commands::eidolon_cast(conn, slot, target_id, target_position)
                {
                    error!("could not cast Eidolon ability: {err}");
                }
            }

            if ability.cast_mode().is_instant() {
                hud_cooldowns.write(SpellHudCooldownStarted {
                    key: HudCooldownKey::Ability(ability_id.clone()),
                    cooldown_seconds: ability.base_params().cooldown,
                });
            }
        }

        if intent.release_cast {
            let is_this_cast = observed_casts
                .0
                .get(&local_network_id.0)
                .is_some_and(|cast| cast.spell_id == ability_id.as_str());
            aim.clear();

            if is_this_cast {
                if let Some(direction) = target_position
                    .and_then(|target| flat_direction_towards(player_position.0, target))
                {
                    look_direction.0 = direction;
                }
                if let Some(conn) = conn.as_deref() {
                    if let Err(err) = stdb_commands::release_cast(
                        conn,
                        ability_id.as_str().to_owned(),
                        target_id,
                        target_position,
                    ) {
                        error!("could not release cast: {err}");
                    }
                }
                if !ability.cast_mode().is_channeling() {
                    hud_cooldowns.write(SpellHudCooldownStarted {
                        key: HudCooldownKey::Ability(ability_id.clone()),
                        cooldown_seconds: ability.base_params().cooldown,
                    });
                }
            }
        }
    }
}

/// Active gesture on the given slot, with its base cooldown.  `None` when the
/// weapon offers nothing for this slot or the inscribed ability id is missing
/// from the registry.
fn active_ability(
    slot: AbilitySlot,
    weapon_abilities: &bevymmo_gameplay::abilities::WeaponAbilities,
    weapon: &bevymmo_gameplay::items::instance::ItemInstance,
    ability_registry: &BaseAbilityRegistry,
) -> Option<(AbilityId, ArcBaseAbility)> {
    let ability_id = resolve_active_ability(slot, weapon_abilities, &weapon.ability_selection)?;
    let ability = ability_registry.get(ability_id)?;
    Some((ability_id.clone(), ability))
}

/// Whether the given cast mode stops local movement prediction.
///
/// Mirrors the server's logic so the client freezes instantly instead of
/// waiting for the next `SpellCastProgress` (~100ms + RTT).
fn stops_movement_for_ability(cast_mode: bevymmo_gameplay::abilities::AbilityCastMode) -> bool {
    use bevymmo_gameplay::abilities::{AbilityCastMode, ChannelMovementPolicy};
    match cast_mode {
        AbilityCastMode::Instant => false,
        AbilityCastMode::CastTime => true,
        AbilityCastMode::Channeling {
            movement_policy, ..
        } => movement_policy == ChannelMovementPolicy::InterruptOnMove,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_keys_drive_each_slot_once() {
        assert_eq!(WEAPON_HUD_BINDINGS[0].1, AbilitySlot::Primary);
        assert_eq!(WEAPON_HUD_BINDINGS[1].1, AbilitySlot::Secondary);
        assert_eq!(WEAPON_HUD_BINDINGS[2].1, AbilitySlot::Ultimate);
        assert_eq!(WEAPON_HUD_BINDINGS[0].0, KeyAction::CastPrimary);

        let slots: Vec<AbilitySlot> = weapon_slot_bindings().map(|(_, slot)| slot).collect();
        assert_eq!(
            slots,
            vec![
                AbilitySlot::Primary,
                AbilitySlot::Secondary,
                AbilitySlot::Ultimate,
            ]
        );
    }
}
