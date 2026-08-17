//! Unified player ability input for the Eidolon cast pipeline.
//!
//! Routes Q/W/E to `EidolonCastCommand` for **all** equipped weapons that
//! expose `Item::ability_loadout()`. The legacy `SpellHotbar` path is no
//! longer used for player input — it remains only for NPC/boss entities.
//!
//! # Cast behavior by [`AbilityCastMode`]
//!
//! - **Instant / CastTime**: press-to-aim, release-to-confirm.  The press opens
//!   an aim window ([`AbilityAim`]) during which
//!   [`crate::spells::aim_preview`] draws the exact impact area on the ground;
//!   the release closes it and sends the command.  A quick tap behaves like an
//!   instant cast because press and arrive a few frames apart.
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
    resolve_active_ability, AbilityAim, AbilityId, AbilitySlot, ArcBaseAbility,
    BaseAbilityRegistry, BlueprintExecution,
};
use bevymmo_gameplay::items::components::Equipment;
use bevymmo_gameplay::items::registry::ItemRegistry;
use bevymmo_network::network::protocol::{LookDirection, NetworkEntityId, Position};

use crate::game_state::{GameScreen, Screen};
use crate::spells::cast_bar::ObservedCasts;
use crate::spells::cursor::{cursor_ground_point, flat_direction_towards};
use crate::spells::ui::{HudCooldownKey, SpellHudCooldownStarted, SpellHudState};

/// Key ↔ slot mapping.  Lives here (not on `AbilitySlot`) because the link
/// between a physical key and its gameplay role is an input-layer concern.
pub const SLOT_BINDINGS: [(KeyAction, AbilitySlot); 3] = [
    (KeyAction::CastSpellQ, AbilitySlot::Primary),
    (KeyAction::CastSpellW, AbilitySlot::Secondary),
    (KeyAction::CastSpellE, AbilitySlot::Ultimate),
];

/// Raycast and surface query parameters bundled for aiming.
#[derive(SystemParam)]
pub struct AimRaycastParams<'w, 's> {
    pub windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
    pub cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<Camera3d>>,
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

    let target_position = cursor_ground_point(
        &aim_ray.windows,
        &aim_ray.cameras,
        aim_ray.surface_query.as_deref(),
    );

    let target_id = current_target
        .entity
        .and_then(|entity| target_ids.get(entity).ok())
        .map(|net_id| net_id.0);

    // ── Press handling ────────────────────────────────────────────────
    // Press opens the aim window (Instant/CastTime) or starts the channel
    // immediately (Channeling).
    for (action, slot) in SLOT_BINDINGS {
        if !settings.just_pressed(action, &keys) {
            continue;
        }

        let Some((ability_id, ability)) =
            active_ability(slot, weapon_abilities, weapon, &ability_registry)
        else {
            continue;
        };

        // Determine if this is a Charge execution (hold-to-charge, fires on release).
        let is_charge =
            item.ability_blueprint(ability.as_ref()).execution == BlueprintExecution::Charge;

        match (ability.cast_mode(), is_charge) {
            (
                bevymmo_gameplay::abilities::AbilityCastMode::Instant
                | bevymmo_gameplay::abilities::AbilityCastMode::CastTime,
                false,
            ) => {
                // Open aim; the release below will confirm and send.
                aim.begin(slot);
            }
            (_, true) => {
                // Charge: start charging immediately on press (like Channeling).
                // The server opens a Charge CastState that waits for release.
                if hud_state.ability_on_cooldown(&ability_id) {
                    continue;
                }
                if let Some(conn) = conn.as_deref() {
                    if let Err(err) =
                        stdb_commands::eidolon_cast(conn, slot, target_id, target_position)
                    {
                        error!("could not start Eidolon charge: {err}");
                    }
                }
                // Don't start cooldown yet — cooldown starts on release when the
                // ability actually fires (server handles this in release_cast).
            }
            (bevymmo_gameplay::abilities::AbilityCastMode::Channeling { .. }, false) => {
                // Channeling starts immediately on press — no aim window.
                if hud_state.ability_on_cooldown(&ability_id) {
                    continue;
                }
                if let Some(conn) = conn.as_deref() {
                    if let Err(err) =
                        stdb_commands::eidolon_cast(conn, slot, target_id, target_position)
                    {
                        error!("could not start Eidolon channel: {err}");
                    }
                }
                // Optimistic cooldown: server starts its cooldown at channel-start too.
                hud_cooldowns.write(SpellHudCooldownStarted {
                    key: HudCooldownKey::Ability(ability_id.clone()),
                    cooldown_seconds: ability.base_params().cooldown,
                });
            }
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
    // Release confirms an aimed cast (Instant/CastTime) or ends a channel.
    //
    // Only Instant/CastTime open an aim window on press (see above), so
    // gating this whole loop on `aim.slot == Some(slot)` made the Channeling
    // arm below unreachable: its release never had an aim to match, and a
    // channel could only end by moving or by the server's own timeout.
    for (action, slot) in SLOT_BINDINGS {
        if !settings.just_released(action, &keys) {
            continue;
        }

        let Some((ability_id, ability)) =
            active_ability(slot, weapon_abilities, weapon, &ability_registry)
        else {
            continue;
        };

        // Determine if this is a Charge execution.
        let is_charge =
            item.ability_blueprint(ability.as_ref()).execution == BlueprintExecution::Charge;

        match (ability.cast_mode(), is_charge) {
            (
                bevymmo_gameplay::abilities::AbilityCastMode::Instant
                | bevymmo_gameplay::abilities::AbilityCastMode::CastTime,
                false,
            ) => {
                if aim.slot != Some(slot) {
                    continue;
                }
                let cancelled = aim.cancelled;
                aim.clear();
                if cancelled {
                    continue;
                }

                // Confirm the aimed cast.
                if hud_state.ability_on_cooldown(&ability_id) {
                    continue;
                }

                // Immediate local feedback: snap facing and stop movement so the
                // player feels the cast instantly instead of waiting ~100ms for
                // SpellCastProgress replication.
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

                // Instant abilities start cooldown locally; CastTime waits for
                // server completion (cast_bar.rs handles that).
                if ability.cast_mode().is_instant() {
                    hud_cooldowns.write(SpellHudCooldownStarted {
                        key: HudCooldownKey::Ability(ability_id.clone()),
                        cooldown_seconds: ability.base_params().cooldown,
                    });
                }
            }
            (_, true) => {
                // Release fires the charged ability (server resolves in release_cast).
                let is_charging_this = observed_casts
                    .0
                    .get(&local_network_id.0)
                    .is_some_and(|cast| cast.spell_id == ability_id.as_str());

                if is_charging_this {
                    if let Some(conn) = conn.as_deref() {
                        if let Err(err) =
                            stdb_commands::release_cast(conn, ability_id.as_str().to_owned())
                        {
                            error!("could not release charge cast: {err}");
                        }
                    }
                    // Cooldown starts now — server starts it in release_cast too.
                    hud_cooldowns.write(SpellHudCooldownStarted {
                        key: HudCooldownKey::Ability(ability_id.clone()),
                        cooldown_seconds: ability.base_params().cooldown,
                    });
                }
            }
            (bevymmo_gameplay::abilities::AbilityCastMode::Channeling { .. }, false) => {
                // Release ends the channel early.
                let is_channeling_this = observed_casts
                    .0
                    .get(&local_network_id.0)
                    .is_some_and(|cast| cast.spell_id == ability_id.as_str());

                if is_channeling_this {
                    if let Some(conn) = conn.as_deref() {
                        if let Err(err) =
                            stdb_commands::release_cast(conn, ability_id.as_str().to_owned())
                        {
                            error!("could not release channel: {err}");
                        }
                    }
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
