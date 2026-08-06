use bevy::prelude::*;

#[derive(Resource)]
pub struct KeyBindings {
    pub show_scoreboard: KeyCode,
    pub toggle_pause: KeyCode,
    pub cast_fireball: KeyCode,
    pub cast_followball: KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            show_scoreboard: KeyCode::Tab,
            toggle_pause: KeyCode::Escape,
            cast_fireball: KeyCode::KeyQ,
            cast_followball: KeyCode::KeyE,
        }
    }
}

pub struct KeyMappingPlugin;

impl Plugin for KeyMappingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyBindings>();
    }
}
