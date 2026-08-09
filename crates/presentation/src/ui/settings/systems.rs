//! Systems wiring the settings UI to [`GameSettingsResource`] and to the rest
//! of the app (window, input consumers, persistence).
//!
//! The contract is one-way per widget:
//!
//! - **Click on widget** → mutate the widget component, then either emit an
//!   event (dropdown, key capture) or expose state for a follow-up system
//!   (toggle).
//! - **`apply_widget_events`** → translates widget changes into
//!   `GameSettingsResource` mutations. Single place where UI → settings.
//! - **`apply_graphics_to_window`** → pushes graphics settings to the primary
//!   `Window` when they change.
//! - **`persist_settings_when_changed`** → JSON save when the resource mutates.

use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input::ButtonInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};

use super::layout::{ActiveSettingsTab, SettingsTabButton};
use super::panels::SettingsPanel;
use super::widgets::dropdown::{Dropdown, DropdownChanged, DropdownValueText};
use super::widgets::key_capture::{KeyBindingChanged, KeyCapture};
use super::widgets::toggle::{Toggle, ToggleDisplay};
use crate::ui::button::UiButtonAction;
use crate::ui::settings::state::{
    save_settings, GameSettings, GameSettingsResource, KeyBinding, KeyModifiers, Resolution,
    WindowMode,
};
use crate::ui::theme::UiTheme;

// ===========================================================================
// Sidebar / tab switching
// ===========================================================================

/// Highlights the active sidebar tab button and dims the others.
pub fn update_tab_button_visuals(
    theme: Res<UiTheme>,
    active: Res<ActiveSettingsTab>,
    mut buttons: Query<(&SettingsTabButton, &mut BackgroundColor)>,
) {
    for (button, mut bg) in buttons.iter_mut() {
        let is_active = button.tab == active.0;
        *bg = BackgroundColor(if is_active {
            theme.button_hovered_bg
        } else {
            theme.button_bg
        });
    }
}

/// Click on a sidebar tab button → set [`ActiveSettingsTab`].
pub fn switch_tab_on_click(
    mut active: ResMut<ActiveSettingsTab>,
    interactions: Query<(&Interaction, &SettingsTabButton), Changed<Interaction>>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            active.0 = button.tab;
        }
    }
}

// ===========================================================================
// Panel visibility
// ===========================================================================

/// Shows only the panel selected in the sidebar.
pub fn update_panel_visibility(
    active: Res<ActiveSettingsTab>,
    mut panels: Query<(&SettingsPanel, &mut Node)>,
) {
    for (panel, mut node) in panels.iter_mut() {
        node.display = if panel.matches(active.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
}

// ===========================================================================
// Dropdown
// ===========================================================================

/// Click on a dropdown cycles to the next item, updates the value text, and
/// emits [`DropdownChanged`].
pub fn cycle_dropdown(
    mut dropdowns: Query<(&Interaction, &mut Dropdown, &Children), Changed<Interaction>>,
    mut value_texts: Query<&mut Text, With<DropdownValueText>>,
    mut changed: MessageWriter<DropdownChanged>,
) {
    for (interaction, mut dropdown, children) in dropdowns.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if dropdown.items.is_empty() {
            continue;
        }
        dropdown.selected = (dropdown.selected + 1) % dropdown.items.len();
        let new_label = dropdown.items[dropdown.selected].label.clone();
        let new_value = dropdown.items[dropdown.selected].value.clone();
        for child in children.iter() {
            if let Ok(mut text) = value_texts.get_mut(child) {
                text.0 = new_label.clone();
            }
        }
        changed.write(DropdownChanged {
            id: dropdown.id.clone(),
            value: new_value,
        });
    }
}

// ===========================================================================
// Toggle
// ===========================================================================

/// Click on a toggle flips its state and updates its visual.
pub fn toggle_on_click(
    theme: Res<UiTheme>,
    mut query: Query<(&Interaction, &mut Toggle, &Children), Changed<Interaction>>,
    mut displays: Query<&mut BackgroundColor, With<ToggleDisplay>>,
) {
    for (interaction, mut toggle, children) in query.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        toggle.on = !toggle.on;
        for child in children.iter() {
            if let Ok(mut bg) = displays.get_mut(child) {
                bg.0 = if toggle.on {
                    theme.button_hovered_bg
                } else {
                    Color::NONE
                };
            }
        }
    }
}

// ===========================================================================
// Key capture
// ===========================================================================

/// Click on a key-capture button toggles capture mode and updates the label
/// to "Press a key…" / current binding.
pub fn toggle_key_capture_on_click(
    mut query: Query<(&Interaction, &mut KeyCapture, &Children), Changed<Interaction>>,
    mut value_texts: Query<&mut Text>,
) {
    for (interaction, mut capture, children) in query.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        capture.capturing = !capture.capturing;
        let new_text = if capture.capturing {
            "Press a key…".to_string()
        } else {
            capture.binding.label()
        };
        update_descendant_text(&mut value_texts, children, &new_text);
    }
}

/// While any key-capture widget is in capture mode, the next non-modifier key
/// press becomes the new binding (modifiers are read from the held state).
/// `Escape` cancels capture without rebinding.
pub fn update_key_capture_input(
    mut events: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut captures: Query<(Entity, &mut KeyCapture, &Children)>,
    mut value_texts: Query<&mut Text>,
    mut changed: MessageWriter<KeyBindingChanged>,
) {
    let mut cancel_requested = false;
    let mut main_key: Option<KeyCode> = None;

    for ev in events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        if is_modifier_key(ev.key_code) {
            continue;
        }
        if ev.key_code == KeyCode::Escape {
            cancel_requested = true;
            continue;
        }
        // First non-modifier, non-Escape key wins.
        if main_key.is_none() {
            main_key = Some(ev.key_code);
        }
    }

    if !cancel_requested && main_key.is_none() {
        return;
    }

    let modifiers = KeyModifiers::from_pressed(&keys);

    for (_entity, mut capture, children) in captures.iter_mut() {
        if !capture.capturing {
            continue;
        }
        if let Some(key) = main_key {
            let binding = KeyBinding { key, modifiers };
            capture.binding = binding;
            capture.capturing = false;
            update_descendant_text(&mut value_texts, children, &binding.label());
            changed.write(KeyBindingChanged {
                action: capture.action,
                binding,
            });
        } else if cancel_requested {
            capture.capturing = false;
            update_descendant_text(&mut value_texts, children, &capture.binding.label());
        }
    }
}

fn is_modifier_key(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
    )
}

/// Writes `new_text` into the first descendant (within `children`) text node
/// found via the value-texts query. The key-capture button has a single child
/// text node, so depth-1 search is enough.
fn update_descendant_text(value_texts: &mut Query<&mut Text>, children: &Children, new_text: &str) {
    for child in children.iter() {
        if let Ok(mut text) = value_texts.get_mut(child) {
            text.0 = new_text.to_string();
            return;
        }
    }
}

// ===========================================================================
// Apply widget changes → GameSettingsResource
// ===========================================================================

/// Single place where UI events turn into settings mutations.
pub fn apply_widget_events(
    mut dropdowns: MessageReader<DropdownChanged>,
    mut keybinds: MessageReader<KeyBindingChanged>,
    mut settings: ResMut<GameSettingsResource>,
    toggle_changes: Query<&Toggle, Changed<Toggle>>,
) {
    for ev in dropdowns.read() {
        match ev.id.as_str() {
            "window_mode" => {
                settings.0.graphics.mode = match ev.value.as_str() {
                    "borderless" => WindowMode::Borderless,
                    "exclusive" => WindowMode::Exclusive,
                    _ => WindowMode::Windowed,
                };
            }
            "resolution" => {
                if let Some(res) = parse_resolution(&ev.value) {
                    settings.0.graphics.resolution = res;
                }
            }
            "language" => {
                settings.0.general.language = ev.value.clone();
            }
            "interface_scale" => {
                if let Ok(scale) = ev.value.parse::<f32>() {
                    settings.0.general.interface_scale = scale.clamp(0.5, 3.0);
                }
            }
            _ => {}
        }
    }

    for toggle in toggle_changes.iter() {
        match toggle.id.as_str() {
            "vsync" => settings.0.graphics.vsync = toggle.on,
            "show_fps" => settings.0.general.show_fps = toggle.on,
            _ => {}
        }
    }

    for ev in keybinds.read() {
        settings.0.keybinds.bindings.insert(ev.action, ev.binding);
    }
}

/// "Reset to defaults" button → wipe all custom bindings and reset key-capture
/// widgets. Other actions are ignored here.
pub fn reset_keybinds_on_button(
    query: Query<(&Interaction, &crate::ui::button::UiButton), Changed<Interaction>>,
    mut settings: ResMut<GameSettingsResource>,
    mut captures: Query<&mut KeyCapture>,
) {
    let mut triggered = false;
    for (interaction, button) in query.iter() {
        if *interaction == Interaction::Pressed && button.action == UiButtonAction::ResetKeybinds {
            triggered = true;
        }
    }
    if !triggered {
        return;
    }
    settings.0.keybinds.bindings.clear();
    for mut capture in captures.iter_mut() {
        capture.binding = KeyBinding::bare(capture.action.default_binding());
        capture.capturing = false;
    }
}

fn parse_resolution(label: &str) -> Option<Resolution> {
    let (w_str, h_str) = label.split_once('x')?;
    Some(Resolution::new(w_str.parse().ok()?, h_str.parse().ok()?))
}

// ===========================================================================
// Apply GameSettingsResource → Window
// ===========================================================================

/// Pushes graphics settings to the primary window whenever they change.
pub fn apply_graphics_to_window(
    settings: Res<GameSettingsResource>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !settings.is_changed() {
        return;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let g = &settings.0.graphics;
    window.mode = g.mode.to_bevy();
    // Fullscreen modes own their surface size. Reassigning a windowed
    // resolution here on every settings change can shrink or letterbox the
    // fullscreen surface when an unrelated setting (for example Show FPS) is
    // toggled.
    if matches!(g.mode, WindowMode::Windowed) {
        window.resolution =
            bevy::window::WindowResolution::new(g.resolution.width, g.resolution.height);
    }
    window.present_mode = if g.vsync {
        bevy::window::PresentMode::AutoVsync
    } else {
        bevy::window::PresentMode::AutoNoVsync
    };
}

/// Applies the persisted interface scale to Bevy's UI scale resource.
pub fn apply_interface_scale(settings: Res<GameSettingsResource>, mut ui_scale: ResMut<UiScale>) {
    if settings.is_changed() {
        ui_scale.0 = settings.0.general.interface_scale.clamp(0.5, 3.0);
    }
}

// ===========================================================================
// Persistence
// ===========================================================================

/// Persists [`GameSettingsResource`] to disk whenever its fingerprint changes.
pub(crate) fn persist_settings_when_changed(
    settings: Res<GameSettingsResource>,
    mut last_saved: Local<Option<GameSettingsFingerprint>>,
) {
    let fp = GameSettingsFingerprint::from(&settings.0);
    if last_saved.as_ref() == Some(&fp) {
        return;
    }
    if let Err(err) = save_settings(&settings.0) {
        bevy::log::warn!("Failed to save settings: {}", err);
        return;
    }
    *last_saved = Some(fp);
}

#[derive(Debug, PartialEq)]
pub(crate) struct GameSettingsFingerprint {
    mode: WindowMode,
    resolution: Resolution,
    vsync: bool,
    interface_scale: f32,
    show_fps: bool,
    language: String,
    keybinds_signature: String,
}

impl GameSettingsFingerprint {
    pub(crate) fn from(s: &GameSettings) -> Self {
        // Sort entries so the signature is stable regardless of HashMap order.
        let mut entries: Vec<(String, String)> = s
            .keybinds
            .bindings
            .iter()
            .map(|(action, binding)| {
                let action_str = serde_json::to_string(action).unwrap_or_default();
                let binding_str = format!(
                    "{:?}|{}|{}|{}|{}",
                    binding.key,
                    binding.modifiers.shift,
                    binding.modifiers.ctrl,
                    binding.modifiers.alt,
                    binding.modifiers.super_key
                );
                (action_str, binding_str)
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let keybinds_signature = entries
            .iter()
            .map(|(a, b)| format!("{}={}", a, b))
            .collect::<Vec<_>>()
            .join(";");

        Self {
            mode: s.graphics.mode,
            resolution: s.graphics.resolution,
            vsync: s.graphics.vsync,
            interface_scale: s.general.interface_scale,
            show_fps: s.general.show_fps,
            language: s.general.language.clone(),
            keybinds_signature,
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::settings::state::KeyAction;

    #[test]
    fn parse_resolution_handles_standard_labels() {
        assert_eq!(
            parse_resolution("1920x1080"),
            Some(Resolution::new(1920, 1080))
        );
        assert_eq!(
            parse_resolution("1280x720"),
            Some(Resolution::new(1280, 720))
        );
    }

    #[test]
    fn parse_resolution_returns_none_for_garbage() {
        assert_eq!(parse_resolution("hd"), None);
        assert_eq!(parse_resolution("1920"), None);
        assert_eq!(parse_resolution("1920x"), None);
    }

    #[test]
    fn fingerprint_is_stable_across_keybind_map_permutations() {
        let mut s1 = GameSettings::default();
        let mut s2 = GameSettings::default();

        s1.keybinds
            .bindings
            .insert(KeyAction::CastSpellQ, KeyBinding::bare(KeyCode::KeyQ));
        s1.keybinds
            .bindings
            .insert(KeyAction::CastSpellW, KeyBinding::bare(KeyCode::KeyW));

        s2.keybinds
            .bindings
            .insert(KeyAction::CastSpellW, KeyBinding::bare(KeyCode::KeyW));
        s2.keybinds
            .bindings
            .insert(KeyAction::CastSpellQ, KeyBinding::bare(KeyCode::KeyQ));

        assert_eq!(
            GameSettingsFingerprint::from(&s1),
            GameSettingsFingerprint::from(&s2)
        );
    }

    #[test]
    fn fingerprint_detects_changes() {
        let s1 = GameSettings::default();
        let mut s2 = GameSettings::default();
        s2.graphics.vsync = !s1.graphics.vsync;
        assert_ne!(
            GameSettingsFingerprint::from(&s1),
            GameSettingsFingerprint::from(&s2)
        );
    }
}
