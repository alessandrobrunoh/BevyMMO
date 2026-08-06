//! Runtime components for the Crowd Control framework.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Kinds of crowd control an entity can suffer.
///
/// Only [`CrowdControlKind::Stun`] is implemented today. The enum is kept
/// open so future CC types (Root, Silence, Slow, Fear) can share the same
/// component, replication, UI, and gating plumbing without further plumbing
/// changes.
#[derive(
    Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default,
)]
#[reflect(Component)]
pub enum CrowdControlKind {
    /// Fully blocks movement and casting until expiry.
    #[default]
    Stun,
    // Future: Root, Silence, Slow, Fear, ...
}

impl CrowdControlKind {
    /// Returns `true` when this kind suppresses *all* actions (movement and
    /// casting).
    ///
    /// Used by movement and cast gating. A future `Slow` would return `false`
    /// here and be modeled as a stat modifier instead, while still being
    /// representable in [`CrowdControlState`] for UI/immunity purposes.
    ///
    /// # Example
    /// ```rust,ignore
    /// if kind.is_blocking() { freeze_entity(target); }
    /// ```
    pub fn is_blocking(self) -> bool {
        matches!(self, CrowdControlKind::Stun)
    }
}

/// One active CC effect on an entity.
///
/// `total_seconds` is retained alongside `remaining_seconds` so the UI can
/// render the bar fill as a stable ratio even under network jitter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
pub struct ActiveCrowdControl {
    pub kind: CrowdControlKind,
    /// Remaining time before this effect expires (server-authoritative;
    /// clients read this as a snapshot).
    pub remaining_seconds: f32,
    /// Original duration. Used by the UI to compute the fill percentage.
    pub total_seconds: f32,
}

/// Server-authoritative CC state, replicated (and predicted) to clients.
///
/// Holds every active CC effect on the entity. Applying a new effect of an
/// already-present kind **refreshes** it (prevents stacking); different kinds
/// coexist so a Stun and a future Silence could overlap.
///
/// The component stays attached (empty) after all effects expire, to avoid
/// insert/remove churn on the entity. UI and gating systems treat an empty
/// state as "no CC".
///
/// # Example
/// ```rust,ignore
/// let mut state = CrowdControlState::default();
/// state.apply(CrowdControlKind::Stun, 2.0);
/// assert!(state.has_blocking_cc());
/// ```
#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct CrowdControlState {
    pub effects: Vec<ActiveCrowdControl>,
}

impl CrowdControlState {
    /// Returns `true` when no CC effect is currently active.
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Returns `true` if any active effect blocks actions (movement + casting).
    ///
    /// This is the single predicate movement and cast gating consult.
    ///
    /// # Example
    /// ```rust,ignore
    /// if state.has_blocking_cc() { return; } // frozen
    /// ```
    pub fn has_blocking_cc(&self) -> bool {
        self.effects.iter().any(|effect| effect.kind.is_blocking())
    }

    /// Refreshes (or inserts) a CC effect of the given kind.
    ///
    /// Refreshing — rather than stacking — keeps stun duration bounded even if
    /// multiple sources apply the same kind within a short window.
    ///
    /// # Example
    /// ```rust,ignore
    /// let mut state = CrowdControlState::default();
    /// state.apply(CrowdControlKind::Stun, 2.0);
    /// state.apply(CrowdControlKind::Stun, 1.0); // refreshes to 1.0s
    /// ```
    pub fn apply(&mut self, kind: CrowdControlKind, duration_seconds: f32) {
        if let Some(active) = self.effects.iter_mut().find(|effect| effect.kind == kind) {
            active.remaining_seconds = duration_seconds;
            active.total_seconds = duration_seconds;
            return;
        }
        self.effects.push(ActiveCrowdControl {
            kind,
            remaining_seconds: duration_seconds,
            total_seconds: duration_seconds,
        });
    }

    /// Advances every effect timer by `delta_seconds` and drops expired ones.
    ///
    /// Runs server-side each fixed tick. Clients only read the replicated
    /// snapshot, so they never call this.
    ///
    /// # Example
    /// ```rust,ignore
    /// state.tick(delta);
    /// ```
    pub fn tick(&mut self, delta_seconds: f32) {
        for effect in &mut self.effects {
            effect.remaining_seconds = (effect.remaining_seconds - delta_seconds).max(0.0);
        }
        self.effects.retain(|effect| effect.remaining_seconds > 0.0);
    }

    /// Returns the blocking effect with the longest remaining time, if any.
    ///
    /// Used by the CC bar UI to pick which effect to render when multiple
    /// blocking kinds coexist in the future.
    ///
    /// # Example
    /// ```rust,ignore
    /// if let Some(active) = state.longest_blocking() { render_bar(active); }
    /// ```
    pub fn longest_blocking(&self) -> Option<&ActiveCrowdControl> {
        self.effects
            .iter()
            .filter(|effect| effect.kind.is_blocking())
            .max_by(|left, right| {
                left.remaining_seconds
                    .partial_cmp(&right.remaining_seconds)
                    .expect("finite CC timer")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_inserts_new_effect() {
        let mut state = CrowdControlState::default();
        state.apply(CrowdControlKind::Stun, 2.0);
        assert_eq!(state.effects.len(), 1);
        assert!(state.has_blocking_cc());
    }

    #[test]
    fn apply_refreshes_existing_kind_instead_of_stacking() {
        let mut state = CrowdControlState::default();
        state.apply(CrowdControlKind::Stun, 2.0);
        state.apply(CrowdControlKind::Stun, 0.5);
        assert_eq!(state.effects.len(), 1);
        assert_eq!(state.effects[0].remaining_seconds, 0.5);
        assert_eq!(state.effects[0].total_seconds, 0.5);
    }

    #[test]
    fn tick_advances_and_drops_expired_effects() {
        let mut state = CrowdControlState::default();
        state.apply(CrowdControlKind::Stun, 1.0);
        state.tick(0.4);
        assert_eq!(state.effects.len(), 1);
        assert!((state.effects[0].remaining_seconds - 0.6).abs() < 1e-6);
        state.tick(0.6);
        assert!(state.is_empty());
        assert!(!state.has_blocking_cc());
    }

    #[test]
    fn longest_blocking_picks_max_remaining() {
        let mut state = CrowdControlState::default();
        state.apply(CrowdControlKind::Stun, 1.0);
        state.effects[0].remaining_seconds = 0.3;
        assert_eq!(
            state.longest_blocking().expect("present").remaining_seconds,
            0.3
        );
    }

    #[test]
    fn longest_blocking_returns_none_when_empty() {
        let state = CrowdControlState::default();
        assert!(state.longest_blocking().is_none());
    }
}
