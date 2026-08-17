//! Application role and run conditions for Bevy systems.

use bevy::prelude::*;

/// Determines which local capabilities are present in the application.
///
/// The Bevy process is a pure client of the SpacetimeDB module now — there is
/// no in-process server or host-client mode any more (see `bins/game`'s CLI
/// doc). `Client` is the only variant actually constructed; it stays an enum
/// (rather than a unit struct) so `has_client`/`is_windowed` read the same as
/// before at every one of their many `run_if` call sites across the client
/// and presentation crates.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppMode {
    #[default]
    Client,
}

impl AppMode {
    pub const fn has_client(self) -> bool {
        matches!(self, Self::Client)
    }

    pub const fn is_windowed(self) -> bool {
        matches!(self, Self::Client)
    }
}

pub fn has_client(mode: Res<AppMode>) -> bool {
    mode.has_client()
}

#[cfg(test)]
mod tests {
    use super::AppMode;

    #[test]
    fn capabilities_match_the_only_application_mode() {
        assert!(AppMode::Client.has_client());
        assert!(AppMode::Client.is_windowed());
    }
}
