//! Scoreboard mostrato mentre il tasto configurato è premuto.

mod plugin;
pub mod systems;

pub use crate::ui::text::spawn_text;
pub use plugin::{ScoreboardPanel, ScoreboardPlugin, ScoreboardState, ScoreboardUi};
