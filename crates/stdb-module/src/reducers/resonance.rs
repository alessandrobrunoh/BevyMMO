//! Resonance XP and level persistence.
//!
//! Resonance tracks a player's progression with individual Ancient Words.
//! Each `(identity, root_word_id)` pair has at most one row; reducers here
//! enforce that invariant and validate inputs.

use spacetimedb::{reducer, ReducerContext, Table};

use bevymmo_domain::abilities::RootWordId;

use crate::reducers::lifecycle::caller_character;
use crate::sim::spells;
use crate::tables::{resonance, Resonance};

/// Awards XP to the caller's resonance for `root_word_id`.
///
/// Creates the row if it does not yet exist. XP additions are clamped via
/// `saturating_add` to prevent unsigned overflow; the caller should use a
/// reasonable upper bound.
#[reducer]
pub fn award_resonance_xp(
    ctx: &ReducerContext,
    root_word_id: String,
    xp_amount: u64,
) -> Result<(), String> {
    validate_root_word(&root_word_id)?;
    if xp_amount == 0 {
        return Err("xp_amount must be positive".to_string());
    }

    let character_id = caller_character(ctx)?.character_id;

    // Scan for existing row; SpacetimeDB 2.8.1 supports single-column indexes
    // on the primary accessor, so we filter by character_id then by root_word_id.
    let existing = ctx
        .db
        .resonance()
        .iter()
        .find(|row| row.character_id == character_id && row.root_word_id == root_word_id);

    match existing {
        Some(row) => {
            let new_xp = row.xp.saturating_add(xp_amount);
            let new_level = compute_level(new_xp);
            ctx.db.resonance().id().update(Resonance {
                id: row.id,
                xp: new_xp,
                level: new_level,
                ..row.clone()
            });
        }
        None => {
            let level = compute_level(xp_amount);
            ctx.db.resonance().insert(Resonance {
                id: 0, // auto_inc fills this
                character_id,
                root_word_id,
                xp: xp_amount,
                level,
            });
        }
    }

    Ok(())
}

/// Deprecated client-facing setter. Resonance is progression state and must
/// only be increased by trusted server gameplay events, never assigned by the
/// player. Keep the reducer during the migration window so old clients receive
/// a deterministic rejection instead of mutating progression.
#[reducer]
pub fn set_resonance_xp(
    _ctx: &ReducerContext,
    _root_word_id: String,
    _xp: u64,
    _level: u32,
) -> Result<(), String> {
    Err("resonance XP can only be awarded by server gameplay events".to_string())
}

fn validate_root_word(root_word_id: &str) -> Result<(), String> {
    if root_word_id.is_empty() {
        return Err("root_word_id must not be empty".to_string());
    }
    let id = RootWordId::new(root_word_id.to_string());
    if spells::root_words().get(&id).is_none() {
        return Err(format!("unknown root word {root_word_id:?}"));
    }
    Ok(())
}

/// Converts total XP into a level using a simple exponential curve.
///
/// This is a **deterministic** pure function so that both the module and the
/// client arrive at the same level from the same XP without round-tripping.
///
/// Current formula: `level = floor(sqrt(xp / 100))`, giving:
/// - Level 0 at 0 XP
/// - Level 1 at 100 XP
/// - Level 4 at 1,600 XP
/// - Level 10 at 10,000 XP
pub fn compute_level(xp: u64) -> u32 {
    let scaled = xp / 100;
    let f_scaled = scaled as f64;
    f_scaled.sqrt() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_level_zero_xp() {
        assert_eq!(compute_level(0), 0);
    }

    #[test]
    fn test_compute_level_thresholds() {
        assert_eq!(compute_level(100), 1);
        assert_eq!(compute_level(400), 2);
        assert_eq!(compute_level(900), 3);
        assert_eq!(compute_level(1600), 4);
    }

    #[test]
    fn test_compute_level_monotonic() {
        let mut prev = 0u32;
        for xp in (0..=10_000).step_by(100) {
            let lvl = compute_level(xp);
            assert!(lvl >= prev, "XP {xp} gave level {lvl}, below previous {prev}");
            prev = lvl;
        }
    }
}
