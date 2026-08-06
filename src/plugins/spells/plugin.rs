use crate::network::mode::has_server;
use crate::network::protocol::SpellVisualEffect;
use bevy::prelude::*;

use super::{
    aoe,
    events::SpellCastRequest,
    projectile::update_homing_projectiles,
    registry::SpellRegistry,
    systems::{process_cast_requests, register_builtin_spells, tick_spell_cooldowns},
};

#[cfg(feature = "client")]
use super::{effects, ui};

/// Plugin that sets up the spells framework.
///
/// This plugin:
/// - Initializes the SpellRegistry resource
/// - Registers spell-related events
/// - Registers built-in spells (like AttackSpell)
/// - Adds systems to process spell cast requests (server-only)
pub struct SpellsPlugin;

impl Plugin for SpellsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpellRegistry>()
            .add_message::<SpellCastRequest>()
            .add_message::<SpellVisualEffect>()
            .add_systems(Startup, register_builtin_spells)
            .add_systems(
                FixedUpdate,
                (
                    process_cast_requests,
                    update_homing_projectiles,
                    aoe::update_aoe_regions,
                    tick_spell_cooldowns,
                )
                    .chain()
                    .run_if(has_server),
            );

        #[cfg(feature = "client")]
        {
            effects::client_effect_systems(app);
            ui::spell_hud_systems(app);
        }
    }
}
