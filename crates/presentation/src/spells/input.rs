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
//!   at that moment so the impact follows the hold, not the press.  A tap that
//!   beats replication still sends the release — waiting for [`ObservedCasts`]
//!   first is how a fast Q/W/E left the charge bar stuck at 0.0s.
//! - **Channeling**: press **immediately** starts the server-side channel via
//!   `eidolon_cast`; release calls `release_cast` with the resolved ability id
//!   to end it early.  An optimistic HUD cooldown starts at press time (the
//!   server does the same) so the key cannot be spammed while the channel runs.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::movement::{ClientSurfaceQuery, LocalMovementFreeze};
use bevymmo_client::stdb::{commands as stdb_commands, StdbConnection};
use bevymmo_client::targeting::CurrentTarget;
use bevymmo_client::user_settings::{GameSettingsResource, KeyAction};
use bevymmo_gameplay::abilities::{
    flush_queued_release, movement_lock_for_ability, queue_release_until_observed,
    resolve_active_ability, weapon_cast_intent, AbilityAim, AbilityId, AbilitySlot, ArcBaseAbility,
    BaseAbilityRegistry, BlueprintExecution,
};
use bevymmo_gameplay::items::components::Equipment;
use bevymmo_gameplay::items::registry::ItemRegistry;
use bevymmo_gameplay::movement::movement_intent_allowed;
use bevymmo_gameplay::stats::components::VitalStats;
use bevymmo_gameplay::stats::formulas::can_afford_mana;
use bevymmo_network::network::protocol::{LookDirection, NetworkEntityId, Position};

use crate::game_state::{in_gameplay, Screen};
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

/// Charge/Channel key-up that left the client before the replicated snapshot.
///
/// The first `release_cast` still goes out immediately (same-client reducers
/// stay ordered). This retry covers the case where that send raced ahead of
/// `eidolon_cast` and no-op'd, which would otherwise leave the charge bar up
/// until the player held the key again.
#[derive(Resource, Default)]
pub struct PendingCastRelease(Option<QueuedCastRelease>);

#[derive(Clone)]
struct QueuedCastRelease {
    ability_id: AbilityId,
    action: KeyAction,
    target_id: Option<u64>,
    target_position: Option<Vec3>,
    stop_movement: bool,
    /// `Some` for Charge (HUD countdown starts on fire). Channeling already
    /// started its countdown on press.
    hud_cooldown_seconds: Option<f32>,
}

impl PendingCastRelease {
    fn clear(&mut self) {
        self.0 = None;
    }
}

/// Raycast and surface query parameters bundled for aiming.
#[derive(SystemParam)]
pub struct AimRaycastParams<'w, 's> {
    pub windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
    pub cameras: Query<'w, 's, (&'static Camera, &'static Transform), With<Camera3d>>,
    pub surface_query: Option<Res<'w, ClientSurfaceQuery>>,
}

/// Local movement side-effects of a weapon cast. Bundled so
/// [`cast_abilities_on_key`] stays within Bevy's 16-argument system limit.
#[derive(SystemParam)]
pub struct CastMovementParams<'w> {
    move_target: ResMut<'w, bevymmo_client::movement::MoveTarget>,
    movement_freeze: ResMut<'w, LocalMovementFreeze>,
    time: Res<'w, Time>,
}

#[allow(clippy::too_many_arguments)]
pub fn cast_abilities_on_key(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    settings: Res<GameSettingsResource>,
    screen: Res<State<Screen>>,
    current_target: Res<CurrentTarget>,
    mut aim: ResMut<AbilityAim>,
    target_ids: Query<&NetworkEntityId>,
    aim_ray: AimRaycastParams,
    mut controlled_players: Query<
        (
            &Equipment,
            &Position,
            &NetworkEntityId,
            &mut LookDirection,
            &VitalStats,
        ),
        With<LocalPlayer>,
    >,
    observed_casts: Res<ObservedCasts>,
    mut pending_release: ResMut<PendingCastRelease>,
    movement: CastMovementParams,
    conn: Option<Res<StdbConnection>>,
    item_registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    hud_state: Res<SpellHudState>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
) {
    let CastMovementParams {
        mut move_target,
        mut movement_freeze,
        time,
    } = movement;
    // Any condition that invalidates the aiming context must close the aim window,
    // otherwise a stale aim would fire on the next unrelated key-release.
    let Some(keys) = keys else {
        aim.clear();
        pending_release.clear();
        return;
    };
    if !in_gameplay(screen) {
        aim.clear();
        pending_release.clear();
        return;
    }

    let Ok((equipment, player_position, local_network_id, mut look_direction, vitals)) =
        controlled_players.single_mut()
    else {
        aim.clear();
        pending_release.clear();
        return;
    };

    // Only weapons with Eidolon gestures drive Q/W/E.
    let Some(weapon) = &equipment.weapon else {
        aim.clear();
        pending_release.clear();
        return;
    };
    let Some(item) = item_registry.get(&weapon.item_id) else {
        aim.clear();
        pending_release.clear();
        return;
    };
    let Some(weapon_abilities) = item.ability_loadout() else {
        aim.clear();
        pending_release.clear();
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

    let selected_id = current_target
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

        // A new press of this slot is a new hold, not a retry of the last tap.
        if pending_release
            .0
            .as_ref()
            .is_some_and(|queued| queued.action == action)
        {
            pending_release.clear();
        }

        let Some((ability_id, ability)) =
            active_ability(slot, weapon_abilities, weapon, &ability_registry)
        else {
            continue;
        };
        if !can_afford_mana(vitals.current_mana, ability.base_params().energy_cost) {
            continue;
        }

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
            if let Err(err) = stdb_commands::eidolon_cast(
                conn,
                slot,
                ability.geometry().selected_entity_payload(selected_id),
                target_position,
            ) {
                error!("could not start Eidolon cast: {err}");
            }
        }
        root_local_movement(
            &mut move_target,
            &mut movement_freeze,
            time.elapsed_secs(),
            movement_lock_for_ability(
                ability.cast_mode(),
                blueprint.execution == BlueprintExecution::Charge,
            ),
            ability.cast_mode(),
        );
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
        if intent.start_cast
            && !can_afford_mana(vitals.current_mana, ability.base_params().energy_cost)
        {
            aim.clear();
            continue;
        }

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
            root_local_movement(
                &mut move_target,
                &mut movement_freeze,
                time.elapsed_secs(),
                movement_lock_for_ability(
                    ability.cast_mode(),
                    blueprint.execution == BlueprintExecution::Charge,
                ),
                ability.cast_mode(),
            );

            if let Some(conn) = conn.as_deref() {
                if let Err(err) = stdb_commands::eidolon_cast(
                    conn,
                    slot,
                    ability.geometry().selected_entity_payload(selected_id),
                    target_position,
                ) {
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
            let observed_matches =
                observed_cast_is(&observed_casts, local_network_id.0, &ability_id);
            aim.clear();

            let stop_movement = blueprint.execution == BlueprintExecution::Charge;
            // Optimistic HUD cooldown only once we know this snapshot is ours.
            // Starting it on a no-op release would lock the slot while the
            // charge bar stayed up.
            let hud_cooldown_seconds =
                (!ability.cast_mode().is_channeling()).then_some(ability.base_params().cooldown);

            let hud_cooldown = if observed_matches {
                hud_cooldown_seconds
            } else {
                None
            };
            send_release_cast(
                conn.as_deref(),
                &ability_id,
                ability.geometry().selected_entity_payload(selected_id),
                target_position,
                player_position.0,
                &mut look_direction,
                &mut move_target,
                &mut movement_freeze,
                time.elapsed_secs(),
                stop_movement,
                hud_cooldown.map(|seconds| (&mut hud_cooldowns, seconds)),
            );

            if queue_release_until_observed(observed_matches) {
                pending_release.0 = Some(QueuedCastRelease {
                    ability_id,
                    action,
                    target_id: ability.geometry().selected_entity_payload(selected_id),
                    target_position,
                    stop_movement,
                    hud_cooldown_seconds,
                });
            } else {
                pending_release.clear();
            }
        }
    }

    // A tap that beat replication queued a retry; fire it once the charge
    // row is visible and the key is still up.
    let should_flush = pending_release.0.as_ref().is_some_and(|queued| {
        let observed_matches =
            observed_cast_is(&observed_casts, local_network_id.0, &queued.ability_id);
        let slot_held = settings.pressed(queued.action, &keys);
        flush_queued_release(observed_matches, slot_held)
    });
    if should_flush {
        if let Some(queued) = pending_release.0.take() {
            send_release_cast(
                conn.as_deref(),
                &queued.ability_id,
                queued.target_id,
                queued.target_position,
                player_position.0,
                &mut look_direction,
                &mut move_target,
                &mut movement_freeze,
                time.elapsed_secs(),
                queued.stop_movement,
                queued
                    .hud_cooldown_seconds
                    .map(|seconds| (&mut hud_cooldowns, seconds)),
            );
        }
    }
}

fn observed_cast_is(observed: &ObservedCasts, caster: u64, ability_id: &AbilityId) -> bool {
    observed
        .0
        .get(&caster)
        .is_some_and(|cast| cast.spell_id == ability_id.as_str())
}

#[allow(clippy::too_many_arguments)]
fn send_release_cast(
    conn: Option<&StdbConnection>,
    ability_id: &AbilityId,
    target_id: Option<u64>,
    target_position: Option<Vec3>,
    player_position: Vec3,
    look_direction: &mut LookDirection,
    move_target: &mut bevymmo_client::movement::MoveTarget,
    freeze: &mut LocalMovementFreeze,
    now: f32,
    stop_movement: bool,
    hud_cooldown: Option<(&mut MessageWriter<SpellHudCooldownStarted>, f32)>,
) {
    if let Some(direction) =
        target_position.and_then(|target| flat_direction_towards(player_position, target))
    {
        look_direction.0 = direction;
    }
    if stop_movement {
        move_target.0 = None;
        freeze.arm(now);
    }
    if let Some(conn) = conn {
        if let Err(err) = stdb_commands::release_cast(
            conn,
            ability_id.as_str().to_owned(),
            target_id,
            target_position,
        ) {
            error!("could not release cast: {err}");
        }
    }
    if let Some((hud_cooldowns, cooldown_seconds)) = hud_cooldown {
        hud_cooldowns.write(SpellHudCooldownStarted {
            key: HudCooldownKey::Ability(ability_id.clone()),
            cooldown_seconds,
        });
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

/// Drop the local dest immediately, and root prediction when the ability
/// will freeze the character. A channel that interrupts on move only
/// clears the dest — walking is still allowed so a held click can cancel it.
fn root_local_movement(
    move_target: &mut bevymmo_client::movement::MoveTarget,
    freeze: &mut LocalMovementFreeze,
    now: f32,
    lock: bevymmo_gameplay::movement::MovementLock,
    cast_mode: bevymmo_gameplay::abilities::AbilityCastMode,
) {
    if !movement_intent_allowed(lock, false) {
        move_target.0 = None;
        freeze.arm(now);
    } else if stops_movement_for_ability(cast_mode) {
        move_target.0 = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unaffordable_weapon_casts_are_skipped() {
        assert!(!can_afford_mana(5.0, 12.0));
        assert!(can_afford_mana(12.0, 12.0));
    }

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

    #[test]
    fn observed_cast_matches_the_named_ability_only() {
        let mut observed = ObservedCasts::default();
        observed.0.insert(
            1,
            crate::spells::cast_bar::ObservedCast {
                spell_id: "arcane_bolt".into(),
                kind: 2,
                elapsed_seconds: 0.0,
                required_seconds: 0.15,
                since_update_seconds: 0.0,
                stale_after_seconds: 1.0,
            },
        );
        assert!(observed_cast_is(
            &observed,
            1,
            &AbilityId::new("arcane_bolt")
        ));
        assert!(!observed_cast_is(
            &observed,
            1,
            &AbilityId::new("arcane_wave")
        ));
        assert!(!observed_cast_is(
            &observed,
            2,
            &AbilityId::new("arcane_bolt")
        ));
    }

    #[test]
    fn rooted_cast_arms_the_optimistic_freeze() {
        let mut dest = bevymmo_client::movement::MoveTarget(Some(Vec3::X));
        let mut freeze = LocalMovementFreeze::default();
        root_local_movement(
            &mut dest,
            &mut freeze,
            1.0,
            bevymmo_gameplay::movement::MovementLock::CastTime,
            bevymmo_gameplay::abilities::AbilityCastMode::CastTime,
        );
        assert!(dest.0.is_none());
        assert!(freeze.is_active(1.0));
        assert!(!freeze.is_active(1.0 + LocalMovementFreeze::DURATION));
    }

    #[test]
    fn instant_cast_does_not_root_prediction() {
        let mut dest = bevymmo_client::movement::MoveTarget(Some(Vec3::X));
        let mut freeze = LocalMovementFreeze::default();
        root_local_movement(
            &mut dest,
            &mut freeze,
            1.0,
            bevymmo_gameplay::movement::MovementLock::None,
            bevymmo_gameplay::abilities::AbilityCastMode::Instant,
        );
        assert_eq!(dest.0, Some(Vec3::X));
        assert!(!freeze.is_active(1.0));
    }
}
