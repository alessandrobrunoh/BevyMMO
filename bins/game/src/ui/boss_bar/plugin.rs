//! Plugin for the boss bar + phase banner.

use bevy::prelude::*;

use super::components::BossBannerState;
use super::systems::{setup_boss_bar, tick_boss_banner, update_boss_banner, update_boss_bar};

pub struct BossBarPlugin;

impl Plugin for BossBarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BossBannerState>();
        app.add_systems(Startup, setup_boss_bar);
        app.add_systems(
            Update,
            (update_boss_bar, update_boss_banner, tick_boss_banner)
                .chain()
                .run_if(crate::network::mode::has_client),
        );
    }
}
