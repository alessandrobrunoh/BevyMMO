use crate::network::mode::has_server;
use crate::network::protocol::{SpellCastEnded, SpellCastProgress, SpellVisualEffect};
use bevy::prelude::*;

use super::{
    aoe,
    events::{SpellCastRequest, SpellReleaseRequest},
    projectile::update_homing_projectiles,
    registry::SpellRegistry,
    systems::{
        advance_cast_progress, handle_cast_release, process_cast_requests, register_builtin_spells,
        replicate_cast_progress, tick_spell_cooldowns,
    },
};

#[cfg(feature = "client")]
use super::{cast_bar, effects, ui};

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
            .add_message::<SpellReleaseRequest>()
            .add_message::<SpellVisualEffect>()
            .add_message::<SpellCastProgress>()
            .add_message::<SpellCastEnded>()
            .add_systems(Startup, register_builtin_spells)
            .add_systems(
                FixedUpdate,
                (
                    process_cast_requests,
                    handle_cast_release,
                    advance_cast_progress,
                    replicate_cast_progress,
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
            cast_bar::cast_bar_systems(app);
        }
    }
}
