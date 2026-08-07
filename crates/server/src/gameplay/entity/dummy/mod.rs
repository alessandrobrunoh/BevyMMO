//! Server-side dummy entity plugin.

use bevy::prelude::*;

/// Static training dummy has no server systems of its own.
pub struct DummyPlugin;

impl Plugin for DummyPlugin {
    fn build(&self, _app: &mut App) {}
}
