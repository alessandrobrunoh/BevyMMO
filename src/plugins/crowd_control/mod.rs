//! Server-authoritative Crowd Control (CC) framework.
//!
//! A CC effect is a *behavioral gate* (Stun blocks movement and casting), not a
//! numeric stat change. Keeping it separate from `stats::modifiers` avoids leaky
//! hacks like `Speed = 0` (which would not block casting) and gives us a single
//! place to add future CC kinds (Root, Silence, Slow, Fear).
//!
//! Lifecycle:
//! 1. Any gameplay system (AoE detonation, direct-hit spell, trap) emits an
//!    [`ApplyCrowdControlEvent`] via a `MessageWriter`.
//! 2. [`systems::apply_cc_events`] (server-only) consumes the events, ensures
//!    the target has a [`CrowdControlState`] component, and refreshes the
//!    relevant effect timer.
//! 3. [`systems::tick_crowd_control`] (server-only) advances every timer each
//!    fixed tick and drops expired effects.
//! 4. Movement and cast systems read [`CrowdControlState::has_blocking_cc`]
//!    to suppress actions.
//! 5. The CC bar UI reads the replicated component to render a draining bar.
//!
//! `CrowdControlState` is replicated + predicted so the local player stops
//! moving the instant the server applies the stun, without rubber-banding.

pub mod components;
pub mod events;
mod systems;

use bevy::prelude::*;

use crate::network::mode;

pub use components::{ActiveCrowdControl, CrowdControlKind, CrowdControlState};
pub use events::ApplyCrowdControlEvent;

pub struct CrowdControlPlugin;

impl Plugin for CrowdControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ApplyCrowdControlEvent>();
        app.add_systems(
            FixedUpdate,
            (
                systems::apply_cc_events,
                systems::tick_crowd_control,
                systems::cancel_active_casts_on_block,
            )
                .chain()
                .run_if(mode::has_server),
        );
    }
}
