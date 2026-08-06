//! Client-only modular spell HUD.
//!
//! UI isolata e data-driven per mostrare spell/keybind/cooldown.
//! Per rimuoverla basta togliere `spell_hud_systems(app)` dal `SpellsPlugin`
//! e cancellare questo file.

use bevy::prelude::*;
use std::collections::HashMap;

use crate::game_state::{GameScreen, Screen};
use crate::network::client::ConnectedClient;
use crate::network::mode::has_client;
use crate::network::protocol::{Channel2, NetworkEntityId, SpellCastCommand};
use crate::plugins::key_mapping::KeyBindings;
use crate::plugins::spells::SpellId;
use crate::plugins::targeting::CurrentTarget;
use crate::ui::theme::UiTheme;
use lightyear::prelude::MessageSender;

#[derive(Message, Debug, Clone, PartialEq)]
pub struct SpellHudCooldownStarted {
    pub spell_id: SpellId,
    pub cooldown_seconds: f32,
}

#[derive(Resource, Default)]
pub struct SpellHudState {
    remaining_seconds: HashMap<SpellId, f32>,
}

#[derive(Resource, Default)]
struct SpellHudLayoutState {
    initialized: bool,
    signature: Vec<(SpellId, KeyCode)>,
}

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
    app.init_resource::<SpellHudLayoutState>();
    app.add_message::<SpellHudCooldownStarted>();
    app.add_systems(Startup, setup_spell_hud.run_if(has_client));
    app.add_systems(
        Update,
        (sync_spell_hud, cast_spell_from_hud_click, update_spell_hud)
            .chain()
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
fn setup_spell_hud(mut commands: Commands, theme: Res<UiTheme>) {
    commands.spawn((
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
    ));
}

fn sync_spell_hud(
    mut commands: Commands,
    theme: Res<UiTheme>,
    bindings: Res<KeyBindings>,
    registry: Res<crate::plugins::spells::SpellRegistry>,
    mut layout_state: ResMut<SpellHudLayoutState>,
    player_query: Query<&crate::plugins::spells::Spellbook, With<lightyear::prelude::Controlled>>,
    hud_query: Query<Entity, With<SpellHudRoot>>,
) {
    let Ok(spellbook) = player_query.single() else {
        return;
    };
    let Ok(root_entity) = hud_query.single() else {
        return;
    };

    let mut signature = Vec::new();
    let mut entries = Vec::new();

    for spell_id in spellbook.spells.iter() {
        let Some(&key) = bindings.spells.get(spell_id) else {
            continue;
        };
        let Some(spell_def) = registry.get(spell_id) else {
            continue;
        };

        signature.push((spell_id.clone(), key));
        entries.push(SpellHudEntry {
            spell_id: spell_id.clone(),
            display_name: spell_def.display_name(),
            key_label: key_label(key),
        });
    }

    if layout_state.initialized && layout_state.signature == signature {
        return;
    }
    layout_state.initialized = true;
    layout_state.signature = signature;

    commands.entity(root_entity).despawn_related::<Children>();

    commands.entity(root_entity).with_children(|parent| {
        for entry in entries {
            parent
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(theme.button_bg),
                    entry.clone(),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text(format_spell_label(&entry, 0.0)),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size),
                            ..default()
                        },
                        TextColor(theme.text_color),
                        entry,
                    ));
                });
        }
    });
}

fn cast_spell_from_hud_click(
    interactions: Query<(&Interaction, &SpellHudEntry), (Changed<Interaction>, With<Button>)>,
    hud_state: Res<SpellHudState>,
    current_target: Res<CurrentTarget>,
    target_ids: Query<&NetworkEntityId>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut senders: Query<&mut MessageSender<SpellCastCommand>, With<ConnectedClient>>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
    registry: Res<crate::plugins::spells::SpellRegistry>,
) {
    for (interaction, entry) in interactions.iter() {
        if *interaction != Interaction::Pressed || hud_state.is_on_cooldown(&entry.spell_id) {
            continue;
        }

        let mut target_position = None;
        if let Ok(window) = windows.single() {
            if let Some(cursor_position) = window.cursor_position() {
                if let Some((camera, camera_transform)) = cameras.iter().next() {
                    if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                        if let Some(target) = ray.plane_intersection_point(
                            Vec3::ZERO,
                            bevy::math::primitives::InfinitePlane3d::new(Vec3::Y),
                        ) {
                            target_position = Some(Vec3::new(target.x, 0.0, target.z));
                        }
                    }
                }
            }
        }

        let mut target_id = None;
        if let Some(target_entity) = current_target.entity {
            if let Ok(net_id) = target_ids.get(target_entity) {
                target_id = Some(net_id.0);
            }
        }

        for mut sender in senders.iter_mut() {
            sender.send::<Channel2>(SpellCastCommand {
                spell_id: entry.spell_id.0.to_string(),
                target_position,
                target_id,
            });
        }

        if let Some(spell_def) = registry.get(&entry.spell_id) {
            hud_cooldowns.write(SpellHudCooldownStarted {
                spell_id: entry.spell_id.clone(),
                cooldown_seconds: spell_def.config().cooldown_seconds,
            });
        }
    }
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
        KeyCode::Space => "Space",
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
