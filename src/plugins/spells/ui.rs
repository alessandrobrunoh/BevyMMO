//! Client-only modular spell HUD.
//!
//! UI isolata e data-driven per mostrare spell/keybind/cooldown.
//! Per rimuoverla basta togliere `spell_hud_systems(app)` dal `SpellsPlugin`
//! e cancellare questo file.

use bevy::prelude::*;
use std::collections::HashMap;

use crate::game_state::{GameScreen, Screen};
use crate::network::mode::has_client;
use crate::plugins::key_mapping::KeyBindings;
use crate::plugins::spells::SpellId;
use crate::spells::fireball::FireballSpell;
use crate::ui::theme::UiTheme;

#[derive(Message, Debug, Clone, PartialEq)]
pub struct SpellHudCooldownStarted {
    pub spell_id: SpellId,
    pub cooldown_seconds: f32,
}

#[derive(Resource, Default)]
pub struct SpellHudState {
    remaining_seconds: HashMap<SpellId, f32>,
}
// TODO: Da vedere se c'é un modo per integrarlo con lo spellbook

impl SpellHudState {
    /// Returns true if the spell is still on cooldown on the client.
    ///
    /// This is used to gate local cast feedback (visuals, HUD) so the player
    /// cannot spam the cast key while waiting for the server-validated
    /// cooldown to expire.
    pub fn is_on_cooldown(&self, id: &SpellId) -> bool {
        self.remaining_seconds
            .get(id)
            .is_some_and(|remaining| *remaining > 0.0)
    }
}

#[derive(Component)]
struct SpellHudRoot;

#[derive(Component, Clone)]
struct SpellHudEntry {
    spell_id: SpellId,
    display_name: &'static str,
    key_label: &'static str,
}

pub fn spell_hud_systems(app: &mut App) {
    app.init_resource::<SpellHudState>();
    app.add_message::<SpellHudCooldownStarted>();
    app.add_systems(Startup, setup_spell_hud.run_if(has_client));
    app.add_systems(
        Update,
        update_spell_hud
            .run_if(has_client)
            .run_if(in_gameplay_or_paused),
    );
    app.add_systems(
        Update,
        hide_spell_hud
            .run_if(has_client)
            .run_if(not_in_gameplay_or_paused),
    );
}

fn in_gameplay_or_paused(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

fn not_in_gameplay_or_paused(screen: Res<GameScreen>) -> bool {
    !in_gameplay_or_paused(screen)
}

fn setup_spell_hud(mut commands: Commands, theme: Res<UiTheme>, bindings: Res<KeyBindings>) {
    let entries = [SpellHudEntry {
        spell_id: SpellId::new(FireballSpell::ID),
        display_name: FireballSpell::DISPLAY_NAME,
        key_label: key_label(bindings.cast_fireball),
    }];

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Percent(50.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            SpellHudRoot,
        ))
        .with_children(|parent| {
            for entry in entries {
                parent.spawn((
                    Text(format_spell_label(&entry, 0.0)),
                    TextFont {
                        font_size: FontSize::Px(theme.button_font_size),
                        ..default()
                    },
                    TextColor(theme.text_color),
                    entry,
                ));
            }
        });
}

fn update_spell_hud(
    time: Res<Time>,
    mut state: ResMut<SpellHudState>,
    mut cooldown_started: MessageReader<SpellHudCooldownStarted>,
    mut roots: Query<&mut Node, With<SpellHudRoot>>,
    mut texts: Query<(&SpellHudEntry, &mut Text)>,
) {
    for message in cooldown_started.read() {
        state
            .remaining_seconds
            .insert(message.spell_id.clone(), message.cooldown_seconds.max(0.0));
    }

    let delta = time.delta_secs();
    state.remaining_seconds.retain(|_, remaining| {
        *remaining = (*remaining - delta).max(0.0);
        *remaining > 0.0
    });

    if let Ok(mut root) = roots.single_mut() {
        root.display = Display::Flex;
    }

    for (entry, mut text) in texts.iter_mut() {
        let remaining = state
            .remaining_seconds
            .get(&entry.spell_id)
            .copied()
            .unwrap_or_default();
        text.0 = format_spell_label(entry, remaining);
    }
}

fn hide_spell_hud(mut roots: Query<&mut Node, With<SpellHudRoot>>) {
    if let Ok(mut root) = roots.single_mut() {
        root.display = Display::None;
    }
}

fn format_spell_label(entry: &SpellHudEntry, remaining_seconds: f32) -> String {
    let cooldown = if remaining_seconds > 0.0 {
        format!("{remaining_seconds:.1}s")
    } else {
        "Ready".to_string()
    };

    format!(
        "[{}] {} - {}",
        entry.key_label, entry.display_name, cooldown
    )
}

fn key_label(key: KeyCode) -> &'static str {
    match key {
        KeyCode::KeyQ => "Q",
        KeyCode::KeyW => "W",
        KeyCode::KeyE => "E",
        KeyCode::KeyR => "R",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_key_has_expected_label() {
        assert_eq!(key_label(KeyCode::KeyQ), "Q");
    }

    #[test]
    fn spell_label_formats_ready_and_cooldown_states() {
        let entry = SpellHudEntry {
            spell_id: SpellId::new("test"),
            display_name: "Test Spell",
            key_label: "T",
        };

        assert_eq!(format_spell_label(&entry, 0.0), "[T] Test Spell - Ready");
        assert_eq!(format_spell_label(&entry, 1.25), "[T] Test Spell - 1.2s");
    }
}
