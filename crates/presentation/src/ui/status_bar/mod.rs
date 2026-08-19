//! Top-center status bar for the local player's active Buffs and Debuffs,
//! plus the compact row under the selected-target frame.

mod systems;

pub(crate) use systems::spawn_target_status_row;
pub use systems::StatusBarPlugin;
