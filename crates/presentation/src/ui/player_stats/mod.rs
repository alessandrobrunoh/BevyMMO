//! Pannello con le statistiche del Player locale.

mod plugin;
mod systems;

pub use plugin::{PlayerStatsPlugin, PlayerStatsText, PlayerStatsUi};
pub use systems::{setup_player_stats, update_player_stats};
