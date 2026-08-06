//! Tema UI condiviso: colori e metriche tipografiche comuni.
//!
//! Risorsa volontariamente minimale — niente token annidati o tabelle di stili.

use bevy::prelude::*;

#[derive(Resource)]
pub struct UiTheme {
    pub text_color: Color,
    pub muted_text_color: Color,
    pub panel_bg: Color,
    pub screen_bg: Color,
    pub bar_bg: Color,
    pub hp_fill: Color,

    pub name_font_size: f32,
    pub hp_font_size: f32,
    pub scoreboard_title_size: f32,
    pub scoreboard_entry_size: f32,

    /// Pulsanti e input (menu/settings/pause).
    pub button_bg: Color,
    pub button_hovered_bg: Color,
    pub button_pressed_bg: Color,
    pub button_text_color: Color,
    pub input_bg: Color,
    pub input_border: Color,
    pub input_border_focused: Color,
    pub error_color: Color,

    pub title_font_size: f32,
    pub button_font_size: f32,
    pub input_font_size: f32,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            text_color: Color::WHITE,
            muted_text_color: Color::srgb(0.8, 0.8, 0.8),
            panel_bg: Color::srgba(0.1, 0.1, 0.1, 0.9),
            screen_bg: Color::srgb(0.055, 0.06, 0.075),
            bar_bg: Color::srgb(0.2, 0.2, 0.2),
            hp_fill: Color::srgb(0.8, 0.1, 0.1),

            name_font_size: 16.0,
            hp_font_size: 12.0,
            scoreboard_title_size: 24.0,
            scoreboard_entry_size: 18.0,

            button_bg: Color::srgb(0.18, 0.18, 0.22),
            button_hovered_bg: Color::srgb(0.28, 0.28, 0.34),
            button_pressed_bg: Color::srgb(0.10, 0.10, 0.14),
            button_text_color: Color::WHITE,
            input_bg: Color::srgb(0.12, 0.12, 0.16),
            input_border: Color::srgb(0.4, 0.4, 0.45),
            input_border_focused: Color::srgb(0.7, 0.7, 0.9),
            error_color: Color::srgb(0.95, 0.3, 0.3),

            title_font_size: 40.0,
            button_font_size: 20.0,
            input_font_size: 18.0,
        }
    }
}
