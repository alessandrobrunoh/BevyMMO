use bevy::prelude::*;

#[derive(Resource)]
pub struct KeyBindings {
    pub show_scoreboard: KeyCode,
    pub toggle_pause: KeyCode,
    pub spells: std::collections::HashMap<crate::plugins::spells::SpellId, KeyCode>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut spells = std::collections::HashMap::new();
        spells.insert(
            crate::plugins::spells::SpellId::new("attack"),
            KeyCode::Space,
        );
        spells.insert(
            crate::plugins::spells::SpellId::new("ray_of_light"),
            KeyCode::KeyQ,
        );
        spells.insert(
            crate::plugins::spells::SpellId::new("fireball"),
            KeyCode::KeyE,
        );
        spells.insert(
            crate::plugins::spells::SpellId::new("healing_circle"),
            KeyCode::KeyR,
        );
        spells.insert(
            crate::plugins::spells::SpellId::new("meteorite"),
            KeyCode::KeyT,
        );
        spells.insert(
            crate::plugins::spells::SpellId::new("stun_field"),
            KeyCode::KeyG,
        );
        spells.insert(crate::plugins::spells::SpellId::new("swift"), KeyCode::KeyF);
        Self {
            show_scoreboard: KeyCode::Tab,
            toggle_pause: KeyCode::Escape,
            spells,
        }
    }
}

pub struct KeyMappingPlugin;

impl Plugin for KeyMappingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyBindings>();
    }
}
