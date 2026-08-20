//! Pure press/release policy for weapon (Eidolon) casts.
//!
//! The HUD and the keyboard system must agree on this: a Charge starts on
//! press (`eidolon_cast`) and fires on release (`release_cast`). Instant opens
//! an aim window on press and sends `eidolon_cast` on release. CastTime starts
//! the wind-up on press and auto-fires when it fills — release does nothing.
//! Channeling starts on press and ends on release.

use super::base_ability::AbilityCastMode;
use super::blueprint::BlueprintExecution;
use crate::movement::MovementLock;

/// What one frame of a slot key should do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WeaponCastIntent {
    /// Open (or keep) the client aim preview for this slot.
    pub open_aim: bool,
    /// Call `eidolon_cast` this frame.
    pub start_cast: bool,
    /// Call `release_cast` this frame.
    pub release_cast: bool,
}

/// Aim used when a Charge fires on key-release.
///
/// The preview follows the cursor during the hold, so the fire must too:
/// prefer the point/entity from the release frame, and fall back to the
/// aim captured when the charge started if the release omitted one.
pub fn charge_release_aim<P: Copy>(
    stored_position: Option<P>,
    stored_entity: Option<u64>,
    release_position: Option<P>,
    release_entity: Option<u64>,
) -> (Option<P>, Option<u64>) {
    (
        release_position.or(stored_position),
        release_entity.or(stored_entity),
    )
}

/// Movement lock this ability will apply once the reducer accepts it.
///
/// Charge roots even when the base mode is CastTime. Instant, CastTime, and
/// Channeling do not: Instant must not steal facing from the path, and
/// CastTime / Channeling accept a later click so movement can interrupt.
pub fn movement_lock_for_ability(cast_mode: AbilityCastMode, is_charge: bool) -> MovementLock {
    if is_charge {
        return MovementLock::Charge;
    }
    match cast_mode {
        AbilityCastMode::Instant => MovementLock::None,
        AbilityCastMode::CastTime => MovementLock::CastTime,
        AbilityCastMode::Channeling { .. } => MovementLock::Channel,
    }
}

/// Maps a key edge + ability shape onto the reducer calls the client must make.
///
/// `just_pressed` and `just_released` can both be true on a same-frame tap;
/// Charge then both starts and releases, which is what a click-cast wants.
pub fn weapon_cast_intent(
    just_pressed: bool,
    just_released: bool,
    execution: BlueprintExecution,
    cast_mode: AbilityCastMode,
) -> WeaponCastIntent {
    let is_charge = execution == BlueprintExecution::Charge;
    let mut intent = WeaponCastIntent::default();

    if just_pressed {
        match (cast_mode, is_charge) {
            (AbilityCastMode::Instant, false) => {
                intent.open_aim = true;
            }
            (AbilityCastMode::CastTime, false) => {
                intent.start_cast = true;
            }
            (_, true) => {
                intent.open_aim = true;
                intent.start_cast = true;
            }
            (AbilityCastMode::Channeling { .. }, false) => {
                intent.start_cast = true;
            }
        }
    }

    if just_released {
        match (cast_mode, is_charge) {
            (AbilityCastMode::Instant, false) => {
                intent.start_cast = true;
            }
            (AbilityCastMode::CastTime, false) => {}
            (_, true) => {
                intent.release_cast = true;
            }
            (AbilityCastMode::Channeling { .. }, false) => {
                intent.release_cast = true;
            }
        }
    }

    intent
}

/// Queue a second `release_cast` when the first one raced ahead of the
/// replicated charge snapshot.
pub fn queue_release_until_observed(observed_matches: bool) -> bool {
    !observed_matches
}

/// Fire the queued retry once the snapshot is ours and the key is up.
pub fn flush_queued_release(observed_matches: bool, slot_held: bool) -> bool {
    observed_matches && !slot_held
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::ChannelMovementPolicy;

    fn channel() -> AbilityCastMode {
        AbilityCastMode::Channeling {
            tick_interval_seconds: 0.25,
            movement_policy: ChannelMovementPolicy::InterruptOnMove,
        }
    }

    #[test]
    fn charge_starts_on_press_and_releases_on_release() {
        let press = weapon_cast_intent(
            true,
            false,
            BlueprintExecution::Charge,
            AbilityCastMode::Instant,
        );
        assert_eq!(
            press,
            WeaponCastIntent {
                open_aim: true,
                start_cast: true,
                release_cast: false,
            }
        );

        let release = weapon_cast_intent(
            false,
            true,
            BlueprintExecution::Charge,
            AbilityCastMode::Instant,
        );
        assert_eq!(
            release,
            WeaponCastIntent {
                open_aim: false,
                start_cast: false,
                release_cast: true,
            }
        );
    }

    #[test]
    fn charge_press_is_not_treated_as_release() {
        let press = weapon_cast_intent(
            true,
            false,
            BlueprintExecution::Charge,
            AbilityCastMode::CastTime,
        );
        assert!(press.start_cast);
        assert!(!press.release_cast);
    }

    #[test]
    fn instant_aims_on_press_and_casts_on_release() {
        let press = weapon_cast_intent(
            true,
            false,
            BlueprintExecution::Base,
            AbilityCastMode::Instant,
        );
        assert_eq!(
            press,
            WeaponCastIntent {
                open_aim: true,
                start_cast: false,
                release_cast: false,
            }
        );

        let release = weapon_cast_intent(
            false,
            true,
            BlueprintExecution::Base,
            AbilityCastMode::Instant,
        );
        assert_eq!(
            release,
            WeaponCastIntent {
                open_aim: false,
                start_cast: true,
                release_cast: false,
            }
        );
    }

    #[test]
    fn cast_time_starts_on_press_and_ignores_release() {
        let press = weapon_cast_intent(
            true,
            false,
            BlueprintExecution::Base,
            AbilityCastMode::CastTime,
        );
        let release = weapon_cast_intent(
            false,
            true,
            BlueprintExecution::Base,
            AbilityCastMode::CastTime,
        );
        assert!(press.start_cast && !press.open_aim && !press.release_cast);
        assert!(!release.start_cast && !release.release_cast && !release.open_aim);
    }

    #[test]
    fn cast_time_tap_starts_once_and_does_not_release() {
        let tap = weapon_cast_intent(
            true,
            true,
            BlueprintExecution::Base,
            AbilityCastMode::CastTime,
        );
        assert!(tap.start_cast);
        assert!(!tap.release_cast);
        assert!(!tap.open_aim);
    }

    #[test]
    fn channel_starts_on_press_and_releases_on_release() {
        let press = weapon_cast_intent(true, false, BlueprintExecution::Base, channel());
        let release = weapon_cast_intent(false, true, BlueprintExecution::Base, channel());
        assert_eq!(
            press,
            WeaponCastIntent {
                open_aim: false,
                start_cast: true,
                release_cast: false,
            }
        );
        assert_eq!(
            release,
            WeaponCastIntent {
                open_aim: false,
                start_cast: false,
                release_cast: true,
            }
        );
    }

    #[test]
    fn echo_follows_base_not_charge() {
        let press = weapon_cast_intent(
            true,
            false,
            BlueprintExecution::Echo,
            AbilityCastMode::Instant,
        );
        assert!(press.open_aim);
        assert!(!press.start_cast);
        assert!(!press.release_cast);
    }

    #[test]
    fn charge_tap_starts_and_releases_on_the_same_frame() {
        let tap = weapon_cast_intent(
            true,
            true,
            BlueprintExecution::Charge,
            AbilityCastMode::Instant,
        );
        assert!(tap.start_cast);
        assert!(tap.release_cast);
        assert!(tap.open_aim);
    }

    #[test]
    fn idle_frame_does_nothing() {
        assert_eq!(
            weapon_cast_intent(
                false,
                false,
                BlueprintExecution::Charge,
                AbilityCastMode::Instant,
            ),
            WeaponCastIntent::default()
        );
    }

    #[test]
    fn charge_release_prefers_the_cursor_at_release() {
        let (position, entity) = charge_release_aim(Some(1), Some(7), Some(2), Some(8));
        assert_eq!(position, Some(2));
        assert_eq!(entity, Some(8));
    }

    #[test]
    fn charge_release_keeps_the_press_aim_when_release_sends_nothing() {
        let (position, entity) = charge_release_aim(Some(1), Some(7), None, None);
        assert_eq!(position, Some(1));
        assert_eq!(entity, Some(7));
    }

    #[test]
    fn queued_release_waits_for_the_snapshot_then_fires_when_the_key_is_up() {
        assert!(queue_release_until_observed(false));
        assert!(!queue_release_until_observed(true));
        assert!(flush_queued_release(true, false));
        assert!(!flush_queued_release(true, true));
        assert!(!flush_queued_release(false, false));
    }

    #[test]
    fn charge_roots_even_when_the_base_mode_is_instant() {
        assert_eq!(
            movement_lock_for_ability(AbilityCastMode::Instant, true),
            MovementLock::Charge
        );
    }

    #[test]
    fn instant_and_channel_do_not_root() {
        assert_eq!(
            movement_lock_for_ability(AbilityCastMode::Instant, false),
            MovementLock::None
        );
        assert_eq!(
            movement_lock_for_ability(channel(), false),
            MovementLock::Channel
        );
        assert_eq!(
            movement_lock_for_ability(AbilityCastMode::CastTime, false),
            MovementLock::CastTime
        );
    }
}
