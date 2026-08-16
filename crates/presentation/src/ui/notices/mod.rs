//! On-screen log for what the server says back.
//!
//! Two things arrive here: the module's `player_message` broadcasts, and the
//! refusals a reducer returns when it will not do what was asked. Both used to
//! have nowhere to go — the first was never subscribed to, the second was
//! discarded by a fire-and-forget send — so a player who tried to equip into a
//! full inventory or cast out of range saw a button that simply did nothing.
//!
//! Deliberately a log and not a modal: none of these interrupt play, and a
//! dialog for "out of range" during a fight would be worse than silence. Lines
//! stack in the lower-left, fade, and remove themselves.

mod systems;

use bevy::prelude::*;

pub use systems::NoticeLog;

pub struct NoticesPlugin;

impl Plugin for NoticesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NoticeLog>();
        app.add_systems(Startup, systems::setup_notice_log);
        app.add_systems(
            Update,
            (systems::collect_notices, systems::expire_notices).chain(),
        );
    }
}
