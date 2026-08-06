//! Enemy: entità controllata dal server (AI).

pub mod components;
#[cfg(feature = "client")]
pub mod debug;
pub mod spawn;
pub mod systems;

use bevy::prelude::*;

pub use components::{Enemy, Respawning};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Respawning>();
        app.add_systems(
            FixedUpdate,
            (
                systems::enemy_chase,
                systems::enemy_auto_cast_attack,
                systems::schedule_enemy_respawn,
                systems::enemy_respawn,
            )
                .chain()
                .run_if(crate::network::mode::has_server),
        );
        #[cfg(feature = "client")]
        debug::client_debug_systems(app);
    }
}
