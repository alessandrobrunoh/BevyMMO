//! Client-side entity visuals.

use bevy::prelude::*;

/// Umbrella for client entity visuals.
///
/// Entity visuals are currently supplied by the generic renderer; this plugin
/// remains the extension point for specialized visuals.
pub struct EntityVisualsPlugin;

impl Plugin for EntityVisualsPlugin {
    fn build(&self, _app: &mut App) {}
}
