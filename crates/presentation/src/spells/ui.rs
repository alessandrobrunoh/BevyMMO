//! Client-only modular spell HUD.
//!
//! UI isolata e data-driven per mostrare spell/keybind/cooldown.
//! Per rimuoverla basta togliere `spell_hud_systems(app)` dal `SpellsPlugin`
//! e cancellare questo file.

use bevy::prelude::*;
use std::collections::HashMap;

use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::network::mode::has_client;
use bevymmo_shared::network::protocol::{Channel2, NetworkEntityId, SpellCastCommand};
use bevymmo_shared::spells::{HotbarSlot, SpellHotbar, SpellId};
use bevymmo_shared::targeting::CurrentTarget;

use crate::game_state::{GameScreen, Screen};
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
    signature: Vec<(HotbarSlot, Option<SpellId>)>,
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
    spell_id: Option<SpellId>,
    display_name: String,
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
    registry: Res<bevymmo_shared::spells::SpellRegistry>,
    mut layout_state: ResMut<SpellHudLayoutState>,
    player_query: Query<&SpellHotbar, With<lightyear::prelude::Controlled>>,
    hud_query: Query<Entity, With<SpellHudRoot>>,
) {
    let Ok(hotbar) = player_query.single() else {
        return;
    };
    let Ok(root_entity) = hud_query.single() else {
        return;
    };

    let mut signature = Vec::new();
    let mut entries = Vec::new();

    for (slot, key_label) in [
        (HotbarSlot::Q, "Q"),
        (HotbarSlot::W, "W"),
        (HotbarSlot::E, "E"),
    ] {
        let spell_id = hotbar.spell_for_slot(slot).cloned();
        let display_name = spell_id
            .as_ref()
            .and_then(|id| registry.get(id))
            .map(|spell_def| spell_def.display_name().to_string())
            .unwrap_or_else(|| "Empty".to_string());

        signature.push((slot, spell_id.clone()));
        entries.push(SpellHudEntry {
            spell_id,
            display_name,
            key_label,
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
    registry: Res<bevymmo_shared::spells::SpellRegistry>,
) {
    for (interaction, entry) in interactions.iter() {
        let Some(spell_id) = &entry.spell_id else {
            continue;
        };
        if *interaction != Interaction::Pressed || hud_state.is_on_cooldown(spell_id) {
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
                spell_id: spell_id.as_str().to_owned(),
                target_position,
                target_id,
            });
        }

        if let Some(spell_def) = registry.get(spell_id) {
            if spell_def.cast_kind() == bevymmo_shared::spells::CastKind::Instant {
                hud_cooldowns.write(SpellHudCooldownStarted {
                    spell_id: spell_id.clone(),
                    cooldown_seconds: spell_def.config().cooldown_seconds,
                });
            }
        }
    }
}

fn update_spell_hud(
    time: Res<Time>,
    mut elapsed_since_label_update: Local<f32>,
    mut state: ResMut<SpellHudState>,
    mut cooldown_started: MessageReader<SpellHudCooldownStarted>,
    mut roots: Query<&mut Node, With<SpellHudRoot>>,
    mut texts: Query<(&SpellHudEntry, &mut Text)>,
) {
    let mut has_new_cooldown = false;
    for message in cooldown_started.read() {
        has_new_cooldown = true;
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

    *elapsed_since_label_update += delta;
    let should_update_labels = has_new_cooldown || *elapsed_since_label_update >= 0.1;
    if !should_update_labels {
        return;
    }
    *elapsed_since_label_update = 0.0;

    for (entry, mut text) in texts.iter_mut() {
        let remaining = entry
            .spell_id
            .as_ref()
            .and_then(|spell_id| state.remaining_seconds.get(spell_id).copied())
            .unwrap_or_default();
        let next_label = format_spell_label(entry, remaining);
        if text.0 == next_label {
            continue;
        }
        text.0 = next_label;
    }
}

fn hide_spell_hud(mut roots: Query<&mut Node, With<SpellHudRoot>>) {
    if let Ok(mut root) = roots.single_mut() {
        root.display = Display::None;
    }
}

fn format_spell_label(entry: &SpellHudEntry, remaining_seconds: f32) -> String {
    if entry.spell_id.is_none() {
        return format!("[{}] Empty", entry.key_label);
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_label_formats_ready_cooldown_and_empty_states() {
        let entry = SpellHudEntry {
            spell_id: Some(SpellId::new("test")),
            display_name: "Test Spell".to_string(),
            key_label: "Q",
        };
        let empty_entry = SpellHudEntry {
            spell_id: None,
            display_name: "Empty".to_string(),
            key_label: "W",
        };

        assert_eq!(format_spell_label(&entry, 0.0), "[Q] Test Spell - Ready");
        assert_eq!(format_spell_label(&entry, 1.25), "[Q] Test Spell - 1.2s");
        assert_eq!(format_spell_label(&empty_entry, 0.0), "[W] Empty");
    }
}
