//! Pure press/release policy for weapon (weapon) casts.
//!
//! Instant, CastTime and Channeling share the same input: press opens aim,
//! release sends `cast_weapon`. The server then Instant-fires, winds up, or
//! channels from that single command.

use super::base_ability::AbilityCastMode;
use crate::movement::MovementLock;

/// What one frame of a slot key should do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WeaponCastIntent {
    /// Open (or keep) the client aim preview for this slot.
    pub open_aim: bool,
    /// Call `cast_weapon` this frame.
    pub start_cast: bool,
}

/// Movement lock this ability will apply once the reducer accepts it.
pub fn movement_lock_for_ability(cast_mode: AbilityCastMode) -> MovementLock {
    match cast_mode {
        AbilityCastMode::Instant => MovementLock::None,
        AbilityCastMode::CastTime => MovementLock::CastTime,
        AbilityCastMode::Channeling { .. } => MovementLock::Channel,
    }
}

/// Maps a key edge + ability shape onto the reducer calls the client must make.
///
/// `just_pressed` and `just_released` can both be true on a same-frame tap;
/// Instant/CastTime/Channeling then both open aim and start the cast.
pub fn weapon_cast_intent(
    just_pressed: bool,
    just_released: bool,
    cast_mode: AbilityCastMode,
) -> WeaponCastIntent {
    let _ = cast_mode;
    let mut intent = WeaponCastIntent::default();
    if just_pressed {
        intent.open_aim = true;
    }
    if just_released {
        intent.start_cast = true;
    }
    intent
}

/// Queue a second `release_cast` when the first one raced ahead of the
/// replicated snapshot. Kept for Channeling early-end if the HUD uses it.
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
    fn press_opens_aim_for_every_mode() {
        for mode in [
            AbilityCastMode::Instant,
            AbilityCastMode::CastTime,
            channel(),
        ] {
            let press = weapon_cast_intent(true, false, mode);
            assert_eq!(
                press,
                WeaponCastIntent {
                    open_aim: true,
                    start_cast: false,
                }
            );
        }
    }

    #[test]
    fn release_starts_cast_for_every_mode() {
        for mode in [
            AbilityCastMode::Instant,
            AbilityCastMode::CastTime,
            channel(),
        ] {
            let release = weapon_cast_intent(false, true, mode);
            assert_eq!(
                release,
                WeaponCastIntent {
                    open_aim: false,
                    start_cast: true,
                }
            );
        }
    }

    #[test]
    fn tap_opens_aim_and_starts_on_the_same_frame() {
        let tap = weapon_cast_intent(true, true, AbilityCastMode::CastTime);
        assert!(tap.open_aim);
        assert!(tap.start_cast);
    }

    #[test]
    fn idle_frame_does_nothing() {
        assert_eq!(
            weapon_cast_intent(false, false, AbilityCastMode::Instant),
            WeaponCastIntent::default()
        );
    }

    #[test]
    fn movement_lock_follows_cast_mode() {
        assert_eq!(
            movement_lock_for_ability(AbilityCastMode::Instant),
            MovementLock::None
        );
        assert_eq!(
            movement_lock_for_ability(AbilityCastMode::CastTime),
            MovementLock::CastTime
        );
        assert_eq!(movement_lock_for_ability(channel()), MovementLock::Channel);
    }

    #[test]
    fn queued_release_waits_for_the_snapshot_then_fires_when_the_key_is_up() {
        assert!(queue_release_until_observed(false));
        assert!(!queue_release_until_observed(true));
        assert!(flush_queued_release(true, false));
        assert!(!flush_queued_release(true, true));
        assert!(!flush_queued_release(false, false));
    }
}
