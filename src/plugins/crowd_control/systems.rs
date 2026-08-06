//! Server-authoritative CC lifecycle systems.
//!
//! [`apply_cc_events`] is the single entry point that mutates
//! [`CrowdControlState`]: every gameplay source (AoE, future direct-hit spells,
//! traps) goes through [`ApplyCrowdControlEvent`]. This keeps the rules for
//! stacking/refreshing in one place.

use bevy::prelude::*;

use crate::plugins::crowd_control::{ApplyCrowdControlEvent, CrowdControlState};
use crate::plugins::spells::CastProgress;

/// Consumes [`ApplyCrowdControlEvent`]s and mutates each target's
/// [`CrowdControlState`].
///
/// Inserting the component lazily here (rather than at entity spawn) keeps the
/// CC system opt-in: only entities that have actually been CC'd carry the
/// component, and the empty-state retention policy avoids churn on later
/// re-application.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(FixedUpdate, apply_cc_events.run_if(has_server));
/// ```
pub fn apply_cc_events(
    mut commands: Commands,
    mut events: MessageReader<ApplyCrowdControlEvent>,
    mut states: Query<&mut CrowdControlState>,
) {
    for event in events.read() {
        let Ok(mut state) = states.get_mut(event.target) else {
            commands
                .entity(event.target)
                .insert(CrowdControlState::default());
            // Re-fetch on the next tick; component is now attached. Skip this
            // event to avoid a second mutable borrow dance — the CC source
            // (AoE) is one-shot per impact, so dropping one frame is fine.
            continue;
        };

        state.apply(event.kind, event.duration_seconds);
    }
}

/// Advances every [`CrowdControlState`] timer and drops expired effects.
///
/// Runs after [`apply_cc_events`] so a freshly-applied effect is not ticked on
/// the same frame it was added (which would shave one tick of duration).
///
/// # Example
/// ```rust,ignore
/// app.add_systems(FixedUpdate, tick_crowd_control.run_if(has_server));
/// ```
pub fn tick_crowd_control(time: Res<Time>, mut states: Query<&mut CrowdControlState>) {
    let delta = time.delta_secs();
    for mut state in states.iter_mut() {
        state.tick(delta);
    }
}

/// Removes any in-progress [`CastProgress`] from entities that became blocked
/// by CC this frame.
///
/// Stun must interrupt both cast-time wind-ups and channeling (Swift). We do
/// not emit a `SpellCastEnded` here: the cast bar UI already auto-expires
/// stale cast snapshots within ~1s, which is acceptable for an interrupt.
/// Channeling stat modifiers (e.g. Swift's speed buff) decay on their own
/// short timer, so no explicit cleanup is required.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(FixedUpdate, cancel_active_casts_on_block.run_if(has_server));
/// ```
pub fn cancel_active_casts_on_block(
    mut commands: Commands,
    states: Query<(Entity, &CrowdControlState), With<CastProgress>>,
) {
    for (entity, state) in states.iter() {
        if state.has_blocking_cc() {
            commands.entity(entity).remove::<CastProgress>();
        }
    }
}
