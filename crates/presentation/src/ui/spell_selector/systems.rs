use super::components::*;
use super::SpellSelectorUiState;
use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::items::AvailableSpellChoices;
use bevymmo_shared::network::protocol::{Channel2, UpdateHotbarSlotRequest};
use bevymmo_shared::spells::{HotbarSlot, SpellHotbar, SpellId, SpellRegistry};
use lightyear::prelude::MessageSender;

use crate::ui::settings::state::{GameSettingsResource, KeyAction};
use crate::ui::theme::UiTheme;

const WINDOW_WIDTH: f32 = 760.0;
const WINDOW_HEIGHT: f32 = 460.0;
const SLOTS: [HotbarSlot; 3] = [HotbarSlot::Q, HotbarSlot::W, HotbarSlot::E];

pub fn toggle_spell_selector(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    mut state: ResMut<SpellSelectorUiState>,
    mut commands: Commands,
    window_query: Query<Entity, With<SpellSelectorWindow>>,
    theme: Res<UiTheme>,
    registry: Res<SpellRegistry>,
    player_query: Query<(&SpellHotbar, &AvailableSpellChoices), With<lightyear::prelude::Controlled>>,
) {
    if !settings.just_pressed(KeyAction::ToggleSpellbook, &keys) {
        return;
    }

    state.is_open = !state.is_open;

    if !state.is_open {
        despawn_spell_selector_windows(&mut commands, &window_query);
        return;
    }

    let Some((hotbar, choices)) = player_query.iter().next() else {
        // Controlled entity not spawned/replicated yet (e.g. still joining).
        state.is_open = false;
        return;
    };
    spawn_spell_selector_window(&mut commands, &theme, &registry, hotbar, choices);
}

fn spawn_spell_selector_window(
    commands: &mut Commands,
    theme: &UiTheme,
    registry: &SpellRegistry,
    hotbar: &SpellHotbar,
    choices: &AvailableSpellChoices,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(WINDOW_WIDTH),
                height: Val::Px(WINDOW_HEIGHT),
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-WINDOW_WIDTH * 0.5),
                    top: Val::Px(-WINDOW_HEIGHT * 0.5),
                    ..default()
                },
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            SpellSelectorWindow,
        ))
        .with_children(|parent| {
            spawn_header(parent, theme);

            parent
                .spawn((Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(18.0),
                    ..default()
                },))
                .with_children(|body| {
                    for slot in SLOTS {
                        spawn_slot_column(body, theme, registry, slot, hotbar, choices);
                    }
                });
        });
}

fn spawn_header(parent: &mut ChildSpawnerCommands, theme: &UiTheme) {
    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            height: Val::Px(40.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|header| {
            header.spawn((
                Text("Spells".to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.title_font_size),
                    ..default()
                },
                TextColor(theme.text_color),
            ));

            header
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(theme.button_bg),
                    CloseSpellSelectorButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text("Close".to_string()),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size),
                            ..default()
                        },
                        TextColor(theme.text_color),
                    ));
                });
        });
}

/// One Q/W/E column: slot header (with the currently active spell), the
/// list of candidates offered by equipped items, and a Clear button.
fn spawn_slot_column(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    registry: &SpellRegistry,
    slot: HotbarSlot,
    hotbar: &SpellHotbar,
    choices: &AvailableSpellChoices,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(33.0),
            row_gap: Val::Px(6.0),
            ..default()
        },))
        .with_children(|column| {
            column.spawn((
                Text(format_hotbar_slot_label(slot, hotbar, registry)),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size),
                    ..default()
                },
                TextColor(theme.text_color),
                HotbarSlotLabel { slot },
            ));

            let candidates = choices.for_slot(slot);
            if candidates.is_empty() {
                column.spawn((
                    Text(format!(
                        "No equipped item grants a {} spell",
                        slot_label(slot)
                    )),
                    TextFont {
                        font_size: FontSize::Px(theme.button_font_size),
                        ..default()
                    },
                    TextColor(theme.muted_text_color),
                ));
            } else {
                for spell_id in candidates {
                    let display_name = registry
                        .get(spell_id)
                        .map(|spell| spell.display_name())
                        .unwrap_or("???");
                    spawn_option_button(column, theme, slot, spell_id.clone(), display_name, hotbar);
                }
            }

            column
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Auto),
                        ..default()
                    },
                    BackgroundColor(theme.button_bg),
                    ClearHotbarSlotButton { slot },
                ))
                .with_children(|button| {
                    button.spawn((
                        Text("Clear".to_string()),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size),
                            ..default()
                        },
                        TextColor(theme.text_color),
                    ));
                });
        });
}

fn spawn_option_button(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    slot: HotbarSlot,
    spell_id: SpellId,
    display_name: &str,
    hotbar: &SpellHotbar,
) {
    let is_active = hotbar.spell_for_slot(slot) == Some(&spell_id);
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(34.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(if is_active {
                theme.button_pressed_bg
            } else {
                theme.button_bg
            }),
            SpellOptionButton {
                slot,
                spell_id: spell_id.clone(),
            },
        ))
        .with_children(|button| {
            button.spawn((
                Text(option_label_text(display_name, is_active)),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size),
                    ..default()
                },
                TextColor(theme.text_color),
                SpellOptionLabel { slot, spell_id },
            ));
        });
}

fn option_label_text(display_name: &str, is_active: bool) -> String {
    if is_active {
        format!("\u{2713} {display_name}")
    } else {
        display_name.to_string()
    }
}

/// Refreshes the slot header text and the checkmark on option labels every
/// frame — cheap (at most a handful of Q/W/E entries) and keeps the window
/// correct without a full despawn/respawn on every click.
pub fn update_spell_selector_ui(
    mut slot_labels: Query<(&HotbarSlotLabel, &mut Text), Without<SpellOptionLabel>>,
    mut option_labels: Query<(&SpellOptionLabel, &mut Text, &ChildOf)>,
    mut option_buttons: Query<(Entity, &mut BackgroundColor), With<SpellOptionButton>>,
    registry: Res<SpellRegistry>,
    theme: Res<UiTheme>,
    player_query: Query<&SpellHotbar, With<lightyear::prelude::Controlled>>,
) {
    let Some(hotbar) = player_query.iter().next() else {
        return;
    };

    for (slot_label, mut text) in slot_labels.iter_mut() {
        text.0 = format_hotbar_slot_label(slot_label.slot, hotbar, &registry);
    }

    for (option, mut text, parent) in option_labels.iter_mut() {
        let display_name = registry
            .get(&option.spell_id)
            .map(|spell| spell.display_name())
            .unwrap_or("???");
        let is_active = hotbar.spell_for_slot(option.slot) == Some(&option.spell_id);
        text.0 = option_label_text(display_name, is_active);

        if let Ok((_, mut bg)) = option_buttons.get_mut(parent.0) {
            *bg = BackgroundColor(if is_active {
                theme.button_pressed_bg
            } else {
                theme.button_bg
            });
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn handle_spell_selector_interactions(
    mut state: ResMut<SpellSelectorUiState>,
    option_interactions: Query<(&Interaction, &SpellOptionButton), (Changed<Interaction>, With<Button>)>,
    clear_interactions: Query<
        (&Interaction, &ClearHotbarSlotButton),
        (Changed<Interaction>, With<Button>),
    >,
    close_interactions: Query<&Interaction, (Changed<Interaction>, With<CloseSpellSelectorButton>)>,
    mut player_query: Query<&mut SpellHotbar, With<lightyear::prelude::Controlled>>,
    mut senders: Query<&mut MessageSender<UpdateHotbarSlotRequest>, With<ConnectedClient>>,
    mut commands: Commands,
    window_query: Query<Entity, With<SpellSelectorWindow>>,
) {
    if close_interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        state.is_open = false;
        despawn_spell_selector_windows(&mut commands, &window_query);
        return;
    }

    for (interaction, option) in option_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_hotbar_selection(
            option.slot,
            Some(option.spell_id.clone()),
            &mut player_query,
            &mut senders,
        );
    }

    for (interaction, clear_slot) in clear_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_hotbar_selection(clear_slot.slot, None, &mut player_query, &mut senders);
    }
}

/// Applies the pick locally (client-predicted, matches the existing pattern
/// for `Equipment`/`Inventory` commands) and sends the request the server
/// will validate against `AvailableSpellChoices`.
fn apply_hotbar_selection(
    slot: HotbarSlot,
    spell_id: Option<SpellId>,
    player_query: &mut Query<&mut SpellHotbar, With<lightyear::prelude::Controlled>>,
    senders: &mut Query<&mut MessageSender<UpdateHotbarSlotRequest>, With<ConnectedClient>>,
) {
    if let Some(mut hotbar) = player_query.iter_mut().next() {
        hotbar.assign(slot, spell_id.clone());
    }

    for mut sender in senders.iter_mut() {
        sender.send::<Channel2>(UpdateHotbarSlotRequest {
            slot,
            spell_id: spell_id
                .as_ref()
                .map(|spell_id| spell_id.as_str().to_string()),
        });
    }
}

fn despawn_spell_selector_windows(
    commands: &mut Commands,
    window_query: &Query<Entity, With<SpellSelectorWindow>>,
) {
    for entity in window_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn format_hotbar_slot_label(slot: HotbarSlot, hotbar: &SpellHotbar, registry: &SpellRegistry) -> String {
    let spell_name = hotbar
        .spell_for_slot(slot)
        .and_then(|spell_id| registry.get(spell_id))
        .map(|spell| spell.display_name().to_string())
        .unwrap_or_else(|| "Empty".to_string());

    format!("{} \u{2014} {spell_name}", slot_label(slot))
}

fn slot_label(slot: HotbarSlot) -> &'static str {
    match slot {
        HotbarSlot::Q => "Q",
        HotbarSlot::W => "W",
        HotbarSlot::E => "E",
    }
}
