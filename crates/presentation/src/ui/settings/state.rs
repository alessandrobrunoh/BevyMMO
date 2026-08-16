//! UI-specific settings types. The data model lives in
//! [`bevymmo_shared::user_settings`]; this module re-exports those types for
//! convenience and adds the UI-only [`SettingsTab`] enum (sidebar tabs).

pub use bevymmo_shared::user_settings::{
    load_settings, save_settings, settings_path, GameSettings, GameSettingsResource,
    GeneralSettings, GraphicsSettings, KeyAction, KeyBinding, KeyModifiers, KeybindSettings,
    Resolution, WindowMode,
};

/// Identifies one of the panels shown in the settings sidebar.
///
/// Order of variants = order in the sidebar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    #[default]
    General,
    Graphics,
    Keybinds,
}

impl SettingsTab {
    /// All tabs in sidebar order.
    pub const ALL: [Self; 3] = [Self::General, Self::Graphics, Self::Keybinds];

    /// Sidebar label, shown to the player.
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Graphics => "Graphics",
            Self::Keybinds => "Keybinds",
        }
    }
}
