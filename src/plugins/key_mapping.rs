use bevy::prelude::*;

#[derive(Resource)]
pub struct KeyBindings {
    pub show_scoreboard: KeyCode,
    pub toggle_pause: KeyCode,
    pub cast_fireball: KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            show_scoreboard: KeyCode::Tab,
            toggle_pause: KeyCode::Escape,
            cast_fireball: KeyCode::KeyQ,
        }
    }
}

pub struct KeyMappingPlugin;

impl Plugin for KeyMappingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyBindings>();
    }
}
