//! User-facing game settings: graphics, keybinds, and general preferences.
//!
//! Pure data + serialization. Lives in `bevymmo_shared` so that both the
//! client runtime (e.g. `targeting`) and the presentation layer can read the
//! same resource. UI-specific types (panels, widgets, etc.) stay in
//! `bevymmo_presentation::ui::settings`.
//!
//! Persistence: JSON at `<user_config_dir>/bevymmo/settings.json`.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::input::keyboard::KeyCode;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Graphics
// ---------------------------------------------------------------------------

/// Window mode selectable from the graphics panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowMode {
    /// OS-decorated, resizable window.
    Windowed,
    /// Fullscreen borderless window covering the whole desktop.
    Borderless,
    /// Exclusive fullscreen (changes video mode).
    Exclusive,
}

impl Default for WindowMode {
    fn default() -> Self {
        Self::Windowed
    }
}

impl WindowMode {
    pub fn to_bevy(self) -> bevy::window::WindowMode {
        // In Bevy 0.19 fullscreen variants take a MonitorSelection (and
        // exclusive fullscreen also a VideoModeSelection). We default to the
        // primary monitor + the current video mode — the safest choice for a
        // settings dropdown that doesn't yet expose per-monitor selection.
        use bevy::window::{MonitorSelection, VideoModeSelection};
        match self {
            Self::Windowed => bevy::window::WindowMode::Windowed,
            Self::Borderless => {
                bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
            }
            Self::Exclusive => bevy::window::WindowMode::Fullscreen(
                MonitorSelection::Primary,
                VideoModeSelection::Current,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn label(self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub mode: WindowMode,
    /// Active resolution. For borderless/exclusive this matches the chosen
    /// monitor's resolution; for windowed it is the inner window size.
    pub resolution: Resolution,
    pub vsync: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            mode: WindowMode::Windowed,
            resolution: Resolution::new(1280, 720),
            vsync: true,
        }
    }
}

// ---------------------------------------------------------------------------
// General
// ---------------------------------------------------------------------------

/// General preferences. `language` is stored as ISO 639-1 but only "en" is
/// honored today (i18n not yet implemented).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub show_fps: bool,
}

fn default_language() -> String {
    "en".to_string()
}

// ---------------------------------------------------------------------------
// Keybinds
// ---------------------------------------------------------------------------

/// Modifier flags for a key binding. Booleans rather than `KeyCode`s because
/// left/right (e.g. `ShiftLeft`/`ShiftRight`) are merged into a single flag:
/// players think in terms of "Shift", not which side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    /// Super = Windows key on PC, Command on macOS.
    pub super_key: bool,
}

impl KeyModifiers {
    /// Returns the modifier flags currently held, normalizing left/right pairs.
    pub fn from_pressed(keys: &bevy::input::ButtonInput<KeyCode>) -> Self {
        Self {
            shift: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
            ctrl: keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight),
            alt: keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight),
            super_key: keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight),
        }
    }

    /// Human-readable prefix, e.g. "Ctrl+Shift+". Empty if no modifiers.
    pub fn label(self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.super_key {
            parts.push("Super");
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("{}+", parts.join("+"))
        }
    }
}

/// A user-facing, rebindable keyboard action.
///
/// Order of variants = order shown in the keybinds panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    TogglePause,
    ShowScoreboard,
    ToggleInventory,
    ToggleSpellbook,
    ClearTarget,
    CastSpellQ,
    CastSpellW,
    CastSpellE,
    CameraZoomIn,
    CameraZoomOut,
}

impl KeyAction {
    /// All rebindable actions in display order.
    pub const ALL: [Self; 10] = [
        Self::TogglePause,
        Self::ShowScoreboard,
        Self::ToggleInventory,
        Self::ToggleSpellbook,
        Self::ClearTarget,
        Self::CastSpellQ,
        Self::CastSpellW,
        Self::CastSpellE,
        Self::CameraZoomIn,
        Self::CameraZoomOut,
    ];

    /// Display name shown in the keybinds panel.
    pub fn label(self) -> &'static str {
        match self {
            Self::TogglePause => "Toggle Pause",
            Self::ShowScoreboard => "Show Scoreboard",
            Self::ToggleInventory => "Toggle Inventory",
            Self::ToggleSpellbook => "Toggle Spellbook",
            Self::ClearTarget => "Clear Target",
            Self::CastSpellQ => "Cast Spell (Q slot)",
            Self::CastSpellW => "Cast Spell (W slot)",
            Self::CastSpellE => "Cast Spell (E slot)",
            Self::CameraZoomIn => "Camera Zoom In",
            Self::CameraZoomOut => "Camera Zoom Out",
        }
    }

    /// Default binding (no modifiers) when no user config exists.
    pub fn default_binding(self) -> KeyCode {
        match self {
            Self::TogglePause => KeyCode::Escape,
            Self::ShowScoreboard => KeyCode::Tab,
            Self::ToggleInventory => KeyCode::KeyI,
            Self::ToggleSpellbook => KeyCode::KeyK,
            Self::ClearTarget => KeyCode::Escape,
            Self::CastSpellQ => KeyCode::KeyQ,
            Self::CastSpellW => KeyCode::KeyW,
            Self::CastSpellE => KeyCode::KeyE,
            Self::CameraZoomIn => KeyCode::PageUp,
            Self::CameraZoomOut => KeyCode::PageDown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub const fn bare(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers {
                shift: false,
                ctrl: false,
                alt: false,
                super_key: false,
            },
        }
    }

    /// Pretty label, e.g. "Ctrl+Shift+Q" or "Esc".
    pub fn label(self) -> String {
        let prefix = self.modifiers.label();
        format!("{}{:?}", prefix, self.key)
    }

    /// True when the given key + currently-held modifiers match this binding.
    pub fn matches(self, key: KeyCode, pressed_modifiers: KeyModifiers) -> bool {
        self.key == key && self.modifiers == pressed_modifiers
    }
}

#[derive(Clone, Debug, Default, Resource, Serialize, Deserialize)]
pub struct KeybindSettings {
    /// Missing entries fall back to [`KeyAction::default_binding`].
    #[serde(default)]
    pub bindings: HashMap<KeyAction, KeyBinding>,
}

impl KeybindSettings {
    /// Returns the configured binding, or the default one if unset.
    pub fn get(&self, action: KeyAction) -> KeyBinding {
        self.bindings
            .get(&action)
            .copied()
            .unwrap_or_else(|| KeyBinding::bare(action.default_binding()))
    }

    /// True if the key + currently-held modifiers match the binding for `action`.
    pub fn matches(
        &self,
        action: KeyAction,
        key: KeyCode,
        pressed_modifiers: KeyModifiers,
    ) -> bool {
        self.get(action).matches(key, pressed_modifiers)
    }
}

// ---------------------------------------------------------------------------
// Aggregated settings
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GameSettings {
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub graphics: GraphicsSettings,
    #[serde(default)]
    pub keybinds: KeybindSettings,
}

/// Bevy resource holding the live, mutable user settings.
#[derive(Clone, Debug, Default, Resource)]
pub struct GameSettingsResource(pub GameSettings);

impl GameSettingsResource {
    /// Returns a copy of the inner settings.
    pub fn snapshot(&self) -> GameSettings {
        self.0.clone()
    }

    /// True if the configured binding for `action` was just pressed (this
    /// frame) with the right modifiers held.
    ///
    /// Single entry point for game systems that need to react to a rebindable
    /// action; replaces scattered `keys.just_pressed(KeyCode::X)` calls.
    pub fn just_pressed(
        &self,
        action: KeyAction,
        keys: &bevy::input::ButtonInput<KeyCode>,
    ) -> bool {
        let binding = self.0.keybinds.get(action);
        keys.just_pressed(binding.key) && KeyModifiers::from_pressed(keys) == binding.modifiers
    }

    /// True if the configured binding for `action` is currently held (this
    /// frame) with the right modifiers.
    ///
    /// Used by continuous-input actions like camera zoom.
    pub fn pressed(
        &self,
        action: KeyAction,
        keys: &bevy::input::ButtonInput<KeyCode>,
    ) -> bool {
        let binding = self.0.keybinds.get(action);
        keys.pressed(binding.key) && KeyModifiers::from_pressed(keys) == binding.modifiers
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Returns the path where user settings are stored.
///
/// `<user config dir>/bevymmo/settings.json` — created on demand by the
/// save routine. Falls back to `./settings.json` if the OS does not expose a
/// config directory.
pub fn settings_path() -> PathBuf {
    match dirs::config_dir() {
        Some(dir) => dir.join("bevymmo").join("settings.json"),
        None => PathBuf::from("settings.json"),
    }
}

/// Loads settings from disk. Missing or malformed file → defaults.
pub fn load_settings() -> GameSettings {
    let path = settings_path();
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return GameSettings::default();
    };
    match serde_json::from_str::<GameSettings>(&contents) {
        Ok(s) => s,
        Err(err) => {
            bevy::log::warn!(
                "Failed to parse settings at {}: {} — using defaults",
                path.display(),
                err
            );
            GameSettings::default()
        }
    }
}

/// Persists settings to disk. Creates parent directories as needed.
pub fn save_settings(settings: &GameSettings) -> std::io::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    std::fs::write(&path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binding_falls_back_to_default() {
        let kb = KeybindSettings::default();
        assert_eq!(
            kb.get(KeyAction::TogglePause),
            KeyBinding::bare(KeyCode::Escape)
        );
    }

    #[test]
    fn custom_binding_overrides_default() {
        let mut kb = KeybindSettings::default();
        kb.bindings.insert(
            KeyAction::TogglePause,
            KeyBinding {
                key: KeyCode::KeyP,
                modifiers: KeyModifiers {
                    ctrl: true,
                    ..Default::default()
                },
            },
        );
        let b = kb.get(KeyAction::TogglePause);
        assert_eq!(b.key, KeyCode::KeyP);
        assert!(b.modifiers.ctrl);
    }

    #[test]
    fn matches_checks_key_and_modifiers() {
        let mut kb = KeybindSettings::default();
        kb.bindings.insert(
            KeyAction::CastSpellQ,
            KeyBinding {
                key: KeyCode::KeyQ,
                modifiers: KeyModifiers {
                    shift: true,
                    ..Default::default()
                },
            },
        );
        assert!(!kb.matches(
            KeyAction::CastSpellQ,
            KeyCode::KeyQ,
            KeyModifiers::default()
        ));
        assert!(kb.matches(
            KeyAction::CastSpellQ,
            KeyCode::KeyQ,
            KeyModifiers {
                shift: true,
                ..Default::default()
            }
        ));
    }

    #[test]
    fn modifiers_label_is_empty_for_bare_binding() {
        assert_eq!(KeyModifiers::default().label(), String::new());
    }

    #[test]
    fn modifiers_label_orders_ctrl_alt_shift_super() {
        let m = KeyModifiers {
            ctrl: true,
            alt: true,
            shift: true,
            super_key: true,
        };
        assert_eq!(m.label(), "Ctrl+Alt+Shift+Super+");
    }

    #[test]
    fn keybind_label_combines_modifier_prefix_and_key() {
        let b = KeyBinding {
            key: KeyCode::KeyQ,
            modifiers: KeyModifiers {
                ctrl: true,
                ..Default::default()
            },
        };
        assert_eq!(b.label(), "Ctrl+KeyQ");
    }

    #[test]
    fn settings_json_roundtrip_preserves_values() {
        let mut settings = GameSettings::default();
        settings.graphics.vsync = false;
        settings.graphics.resolution = Resolution::new(1920, 1080);
        settings.graphics.mode = WindowMode::Borderless;
        settings.general.show_fps = true;
        settings.keybinds.bindings.insert(
            KeyAction::TogglePause,
            KeyBinding {
                key: KeyCode::KeyP,
                modifiers: KeyModifiers {
                    ctrl: true,
                    ..Default::default()
                },
            },
        );

        let json = serde_json::to_string(&settings).unwrap();
        let back: GameSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.graphics.vsync, false);
        assert_eq!(back.graphics.resolution.width, 1920);
        assert_eq!(back.graphics.mode, WindowMode::Borderless);
        assert!(back.general.show_fps);
        assert_eq!(
            back.keybinds.get(KeyAction::TogglePause).key,
            KeyCode::KeyP
        );
    }

    #[test]
    fn malformed_json_falls_back_to_defaults() {
        let settings: GameSettings = serde_json::from_str("{ invalid }").unwrap_or_default();
        assert_eq!(settings.graphics.vsync, true); // default
    }

    #[test]
    fn pressed_matches_default_binding_with_no_modifiers() {
        use bevy::input::ButtonInput;
        let res = GameSettingsResource::default();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyI);
        assert!(res.pressed(KeyAction::ToggleInventory, &keys));
    }

    #[test]
    fn pressed_rejects_unwanted_modifiers() {
        use bevy::input::ButtonInput;
        let res = GameSettingsResource::default();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyI);
        keys.press(KeyCode::ShiftLeft);
        assert!(!res.pressed(KeyAction::ToggleInventory, &keys));
    }

    #[test]
    fn pressed_matches_binding_with_required_modifier() {
        use bevy::input::ButtonInput;
        let mut settings = GameSettings::default();
        settings.keybinds.bindings.insert(
            KeyAction::CastSpellQ,
            KeyBinding {
                key: KeyCode::KeyQ,
                modifiers: KeyModifiers {
                    ctrl: true,
                    ..Default::default()
                },
            },
        );
        let res = GameSettingsResource(settings);
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyQ);
        assert!(!res.pressed(KeyAction::CastSpellQ, &keys));
        keys.press(KeyCode::ControlLeft);
        assert!(res.pressed(KeyAction::CastSpellQ, &keys));
    }
}
