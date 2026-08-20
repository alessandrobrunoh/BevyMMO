//! Application role used by `bins/game` to compose plugins.

use bevy::prelude::*;

/// Determines which local capabilities are present in the application.
///
/// The Bevy process is a pure client of the SpacetimeDB module now — there is
/// no in-process server or host-client mode any more (see `bins/game`'s CLI
/// doc). `Client` is the only variant actually constructed; the enum stays so
/// `bins/game` can still compose plugins with [`AppMode::has_client`] /
/// [`AppMode::is_windowed`] (windowed client vs a future headless process).
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

#[cfg(test)]
mod tests {
    use super::AppMode;

    #[test]
    fn capabilities_match_the_only_application_mode() {
        assert!(AppMode::Client.has_client());
        assert!(AppMode::Client.is_windowed());
    }
}
