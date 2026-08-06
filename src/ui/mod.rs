//! UI plugin — raccoglie i plugin delle singole UI di gioco.

mod plugin;

pub mod bar;
pub mod button;
pub mod connecting;
pub mod death_screen;
pub mod entity_bar;
pub mod main_menu;
pub mod pause_menu;
pub mod player_stats;
pub mod scoreboard;
pub mod settings;
pub mod systems;
pub mod target_frame;
pub mod target_indicator;
pub mod text;
pub mod text_input;
pub mod theme;

pub use plugin::UiPlugin;
