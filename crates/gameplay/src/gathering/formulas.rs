//! Shared gathering formulas. One implementation, used by the module tick.

/// Floor on channel duration so gathering speed cannot outrun the tick.
pub const DEFAULT_MIN_CHANNEL_SECONDS: f32 = 0.25;

/// Channel time for one piece: `base * 100 / (100 + speed)`, clamped to
/// `[min, base]`. Speed 0 is the authored duration; speed 100 is half.
pub fn channel_duration(base: f32, min: f32, speed: f32) -> f32 {
    let base = base.max(0.0);
    let min = min.max(0.0).min(base);
    if base == 0.0 {
        return 0.0;
    }
    let speed = speed.max(0.0);
    (base * 100.0 / (100.0 + speed)).clamp(min, base)
}

/// Extra pieces from gathering bonus. `roll` is in `[0, 1)`.
///
/// `bonus = 0.15` → 15% chance of +1. `bonus = 1.15` → always +1 and 15% of +2.
pub fn bonus_extra_pieces(bonus: f32, roll: f32) -> u32 {
    let bonus = bonus.max(0.0);
    let guaranteed = bonus.floor() as u32;
    let frac = bonus - bonus.floor();
    guaranteed + u32::from(roll >= 0.0 && roll < frac)
}

/// How many pieces a node should hold after `elapsed_seconds` of regen, and
/// the leftover time that has not yet completed an interval.
///
/// A full node is a no-op. Non-positive interval or amount yields `(current, 0.0)`.
pub fn regen_catchup(
    current: u32,
    max: u32,
    elapsed_seconds: f32,
    interval_seconds: f32,
    amount: u32,
) -> (u32, f32) {
    if current >= max
        || interval_seconds <= 0.0
        || amount == 0
        || elapsed_seconds < interval_seconds
    {
        return (current.min(max), elapsed_seconds.max(0.0));
    }
    let intervals = (elapsed_seconds / interval_seconds).floor();
    let gained = (intervals as u32).saturating_mul(amount);
    let next = current.saturating_add(gained).min(max);
    let leftover = elapsed_seconds - intervals * interval_seconds;
    (next, leftover.max(0.0))
}

/// Horizontal (XZ) range check. Y is ignored, matching trigger volumes.
pub fn in_interact_range(ax: f32, az: f32, bx: f32, bz: f32, range: f32) -> bool {
    if range < 0.0 {
        return false;
    }
    let dx = ax - bx;
    let dz = az - bz;
    dx * dx + dz * dz <= range * range
}

/// Inputs for one completed channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatherAttempt {
    pub yield_amount: u32,
    pub bonus_extra: u32,
    pub current_pieces: u32,
    pub inventory_space: u32,
}

/// Result of one completed channel. `granted == 0` means the session should end
/// without mutating the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatherOutcome {
    pub granted: u32,
    pub extra: u32,
    pub remaining_pieces: u32,
    pub node_depleted: bool,
    pub session_ends: bool,
}

/// Applies yield, bonus, remaining pieces and bag space in one step.
pub fn resolve_gather(attempt: GatherAttempt) -> GatherOutcome {
    let wanted = attempt.yield_amount.saturating_add(attempt.bonus_extra);
    let granted = wanted
        .min(attempt.current_pieces)
        .min(attempt.inventory_space);
    if granted == 0 {
        return GatherOutcome {
            granted: 0,
            extra: 0,
            remaining_pieces: attempt.current_pieces,
            node_depleted: attempt.current_pieces == 0,
            session_ends: true,
        };
    }
    let remaining = attempt.current_pieces - granted;
    let extra = granted.saturating_sub(attempt.yield_amount);
    GatherOutcome {
        granted,
        extra,
        remaining_pieces: remaining,
        node_depleted: remaining == 0,
        session_ends: remaining == 0 || attempt.inventory_space - granted == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_duration_identity_at_zero_speed() {
        assert_eq!(channel_duration(2.0, 0.25, 0.0), 2.0);
    }

    #[test]
    fn channel_duration_halves_at_speed_100() {
        assert_eq!(channel_duration(2.0, 0.25, 100.0), 1.0);
    }

    #[test]
    fn channel_duration_floors_at_min() {
        assert_eq!(channel_duration(2.0, 0.25, 1.0e9), 0.25);
    }

    #[test]
    fn bonus_zero_never_adds() {
        assert_eq!(bonus_extra_pieces(0.0, 0.0), 0);
        assert_eq!(bonus_extra_pieces(0.0, 0.99), 0);
    }

    #[test]
    fn bonus_fraction_uses_exclusive_upper_bound() {
        assert_eq!(bonus_extra_pieces(0.15, 0.14), 1);
        assert_eq!(bonus_extra_pieces(0.15, 0.15), 0);
    }

    #[test]
    fn bonus_above_one_guarantees_then_rolls_remainder() {
        assert_eq!(bonus_extra_pieces(1.15, 0.10), 2);
        assert_eq!(bonus_extra_pieces(1.15, 0.20), 1);
    }

    #[test]
    fn regen_adds_one_interval() {
        assert_eq!(regen_catchup(0, 50, 600.0, 600.0, 10), (10, 0.0));
    }

    #[test]
    fn regen_caps_at_max() {
        assert_eq!(regen_catchup(0, 50, 6000.0, 600.0, 10), (50, 0.0));
    }

    #[test]
    fn regen_full_node_is_noop() {
        assert_eq!(regen_catchup(50, 50, 600.0, 600.0, 10), (50, 600.0));
    }

    #[test]
    fn regen_keeps_leftover_partial_interval() {
        let (pieces, leftover) = regen_catchup(0, 50, 900.0, 600.0, 10);
        assert_eq!(pieces, 10);
        assert!((leftover - 300.0).abs() < f32::EPSILON);
    }

    #[test]
    fn range_is_horizontal() {
        assert!(in_interact_range(0.0, 0.0, 2.0, 0.0, 2.5));
        assert!(!in_interact_range(0.0, 0.0, 3.0, 0.0, 2.5));
    }

    #[test]
    fn gather_cannot_exceed_remaining_pieces() {
        let out = resolve_gather(GatherAttempt {
            yield_amount: 1,
            bonus_extra: 1,
            current_pieces: 1,
            inventory_space: 10,
        });
        assert_eq!(out.granted, 1);
        assert_eq!(out.remaining_pieces, 0);
        assert!(out.node_depleted);
        assert!(out.session_ends);
    }

    #[test]
    fn gather_with_no_space_does_not_touch_the_node() {
        let out = resolve_gather(GatherAttempt {
            yield_amount: 1,
            bonus_extra: 0,
            current_pieces: 5,
            inventory_space: 0,
        });
        assert_eq!(out.granted, 0);
        assert_eq!(out.remaining_pieces, 5);
        assert!(out.session_ends);
        assert!(!out.node_depleted);
    }

    #[test]
    fn gather_ends_when_the_bag_fills_on_this_piece() {
        let out = resolve_gather(GatherAttempt {
            yield_amount: 1,
            bonus_extra: 0,
            current_pieces: 5,
            inventory_space: 1,
        });
        assert_eq!(out.granted, 1);
        assert!(out.session_ends);
        assert!(!out.node_depleted);
    }
}
