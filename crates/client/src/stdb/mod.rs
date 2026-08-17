//! The SpacetimeDB half of the client.
//!
//! Replaces what lightyear used to do: hold the connection, receive replicated
//! state, and send commands. The shape is different in one way that matters —
//! lightyear pushed components straight into the ECS, whereas here rows arrive
//! on a background thread and have to be handed across to Bevy deliberately.
//!
//! [`module_bindings`] is generated; run `./scripts/stdb.sh generate` after any
//! change to the module's tables or reducers.

#[rustfmt::skip]
#[allow(clippy::all)]
pub mod module_bindings;

pub mod combat_input;
pub mod commands;
pub mod plugin;

pub use plugin::{
    CharacterRoster, RosterCharacter, StdbAuthoritative, StdbConnection, StdbEntityMap, StdbPlugin,
};
