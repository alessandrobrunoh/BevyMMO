//! Local-player buff/debuff bar above the hotbar, plus the compact row under
//! the selected-target frame.

mod systems;

pub(crate) use systems::spawn_target_status_row;
pub use systems::StatusBarPlugin;
