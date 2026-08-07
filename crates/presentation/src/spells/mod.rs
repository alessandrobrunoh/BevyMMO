//! Client presentation for spells: cast bars, HUD and visual effects.

pub mod cast_bar;
pub mod effects;
pub mod healing_circle;
pub mod input;
pub mod meteorite;
pub mod ray_of_light;
pub mod stun_field;
pub mod ui;
pub mod dragon_enemy;

use bevy::prelude::*;

/// Registers spell HUD, cast-bar and client visual systems.
pub struct SpellsHudPlugin;

impl Plugin for SpellsHudPlugin {
    fn build(&self, app: &mut App) {
        ui::spell_hud_systems(app);
        cast_bar::cast_bar_systems(app);
        app.add_systems(
            Update,
            input::cast_spells_on_key.run_if(bevymmo_shared::network::mode::has_client),
        );
    }
}
