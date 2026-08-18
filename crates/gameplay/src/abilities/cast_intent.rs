//! Pure press/release policy for weapon (Eidolon) casts.
//!
//! The HUD and the keyboard system must agree on this: a Charge starts on
//! press (`eidolon_cast`) and fires on release (`release_cast`). Instant and
//! CastTime open an aim window on press and send `eidolon_cast` on release.
//! Channeling starts on press and ends on release.

use super::base_ability::AbilityCastMode;
use super::blueprint::BlueprintExecution;

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
            (AbilityCastMode::Instant | AbilityCastMode::CastTime, false) => {
                intent.open_aim = true;
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
            (AbilityCastMode::Instant | AbilityCastMode::CastTime, false) => {
                intent.start_cast = true;
            }
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
    fn cast_time_matches_instant_edges() {
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
        assert!(press.open_aim && !press.start_cast && !press.release_cast);
        assert!(release.start_cast && !release.release_cast && !release.open_aim);
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
}
