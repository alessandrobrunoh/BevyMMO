//! UI plugin — collects individual game UI plugins.

mod plugin;

pub mod bar;
pub mod boss_bar;
pub mod button;
pub mod card;
pub mod connecting;
pub mod crowd_control_bar;
pub mod death_screen;
pub mod entity_bar;
pub mod inventory;
pub mod main_menu;
pub mod pause_menu;
pub mod player_stats;
pub mod scoreboard;
pub mod settings;
pub mod spellbook;
pub mod systems;
pub mod target_frame;
pub mod target_indicator;
pub mod text;
pub mod text_input;
pub mod theme;

pub use plugin::UiPlugin;
