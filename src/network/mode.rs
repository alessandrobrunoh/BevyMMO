//! Application role and run conditions for Bevy systems.

use bevy::prelude::*;

/// Determines which local capabilities are present in the application.
///
/// `HostClient` runs both authoritative simulation and presentation/client
/// in the same process and is intended for local development and debugging.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppMode {
    Client,
    Server,
    #[default]
    HostClient,
}

impl AppMode {
    pub const fn has_client(self) -> bool {
        matches!(self, Self::Client | Self::HostClient)
    }

    pub const fn has_server(self) -> bool {
        matches!(self, Self::Server | Self::HostClient)
    }
}

pub fn has_client(mode: Res<AppMode>) -> bool {
    mode.has_client()
}

pub fn has_server(mode: Res<AppMode>) -> bool {
    mode.has_server()
}

#[cfg(test)]
mod tests {
    use super::AppMode;

    #[test]
    fn capabilities_match_each_application_mode() {
        assert!(AppMode::Client.has_client());
        assert!(!AppMode::Client.has_server());
        assert!(!AppMode::Server.has_client());
        assert!(AppMode::Server.has_server());
        assert!(AppMode::HostClient.has_client());
        assert!(AppMode::HostClient.has_server());
    }
}
