//! Client application state consumed by presentation systems.

pub use bevymmo_client::app_state::*;

#[cfg(test)]
use bevy::prelude::*;
#[cfg(test)]
use bevy::state::app::StatesPlugin;

/// Registers [`Screen`] / [`PauseOverlay`] on a headless test app.
///
/// `MinimalPlugins` does not include [`StatesPlugin`]; `init_state` panics
/// without it.
#[cfg(test)]
pub fn init_screen_states(app: &mut App) {
    if !app.is_plugin_added::<StatesPlugin>() {
        app.add_plugins(StatesPlugin);
    }
    app.init_state::<Screen>();
    app.add_sub_state::<PauseOverlay>();
}
