//! Reusable screen-space labels anchored to a world point.
//!
//! Gameplay crates send [`SpawnFloatingText`] (or [`bevymmo_client::server_feed::WorldTextCue`]);
//! this plugin projects them through the game camera, rises and fades them, and
//! despawns them. Labels are not parented to 3D entities.

mod plugin;
mod systems;

pub use plugin::{FloatingText, FloatingTextPlugin, SpawnFloatingText};
