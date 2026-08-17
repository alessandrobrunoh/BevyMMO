//! UI plugin — collects individual game UI plugins.

mod plugin;

pub mod bar;
pub mod boss_bar;
pub mod button;
pub mod card;
pub mod character_roster;
pub mod chat;
pub mod connecting;
pub mod crowd_control_bar;
pub mod death_screen;
pub mod debug_position;
pub mod entity_bar;
pub mod inscription;
pub mod inventory;
pub mod login;
pub mod main_menu;
pub mod notices;
pub mod npc_sidebar;
pub mod pause_menu;
pub mod player_stats;
pub mod scale;
pub mod scoreboard;
pub mod status_bar;
pub mod scrollbar;
pub mod settings;
pub mod spell_selector;
pub mod systems;
pub mod target_frame;
pub mod target_indicator;
pub mod text;
pub mod text_input;
pub mod theme;

pub use plugin::UiPlugin;
