use super::components::*;
use super::SpellbookUiState;
use crate::network::client::ConnectedClient;
use crate::network::protocol::{Channel2, UpdateHotbarSlotRequest};
use crate::plugins::spells::{HotbarSlot, SpellHotbar, SpellRegistry};
use crate::ui::theme::UiTheme;
use bevy::prelude::*;
use lightyear::prelude::MessageSender;

pub fn toggle_spellbook(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SpellbookUiState>,
    mut commands: Commands,
    window_query: Query<Entity, With<SpellbookWindow>>,
    theme: Res<UiTheme>,
    registry: Res<SpellRegistry>,
    player_query: Query<&SpellHotbar, With<lightyear::prelude::Controlled>>,
) {
    if keys.just_pressed(KeyCode::KeyK) {
        state.is_open = !state.is_open;
        state.selected_spell = None;
        if state.is_open {
            let hotbar = player_query.iter().next().cloned().unwrap_or_default();
            spawn_spellbook_window(&mut commands, &theme, &registry, &hotbar);
        } else {
            for entity in window_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn spawn_spellbook_window(
    commands: &mut Commands,
    theme: &UiTheme,
    registry: &SpellRegistry,
    hotbar: &SpellHotbar,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(600.0),
                height: Val::Px(400.0),
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-300.0),
                    top: Val::Px(-200.0),
                    ..default()
                },
                flex_direction: FlexDirection::Row,
                padding: UiRect::all(Val::Px(10.0)),
                column_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            SpellbookWindow,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(8.0),
                        right: Val::Px(8.0),
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(theme.button_bg),
                    CloseSpellbookButton,
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

            // Left panel: List of all spells
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(50.0),
                    row_gap: Val::Px(5.0),
                    ..default()
                },))
                .with_children(|list_parent| {
                    for (spell_id, spell) in registry.sorted_spells() {
                        list_parent
                            .spawn((
                                Button,
                                Node {
                                    padding: UiRect::all(Val::Px(5.0)),
                                    ..default()
                                },
                                BackgroundColor(theme.button_bg),
                                SpellListItem {
                                    spell_id: spell_id.clone(),
                                },
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text(spell.display_name().to_string()),
                                    TextFont {
                                        font_size: FontSize::Px(theme.button_font_size),
                                        ..default()
                                    },
                                    TextColor(theme.text_color),
                                ));
                            });
                    }
                });

            // Right panel: Hotbar Slots
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(50.0),
                    row_gap: Val::Px(10.0),
                    ..default()
                },))
                .with_children(|slots_parent| {
                    for slot in [HotbarSlot::Q, HotbarSlot::W, HotbarSlot::E] {
                        let current_spell = hotbar.spell_for_slot(slot);
                        let spell_name = current_spell
                            .and_then(|id| registry.get(id))
                            .map(|s| s.display_name().to_string())
                            .unwrap_or_else(|| "Empty".to_string());

                        let slot_label = match slot {
                            HotbarSlot::Q => "Q",
                            HotbarSlot::W => "W",
                            HotbarSlot::E => "E",
                        };

                        slots_parent
                            .spawn((
                                Button,
                                Node {
                                    padding: UiRect::all(Val::Px(10.0)),
                                    ..default()
                                },
                                BackgroundColor(theme.button_bg),
                                HotbarSlotUi { slot },
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text(format!("{} - {}", slot_label, spell_name)),
                                    TextFont {
                                        font_size: FontSize::Px(theme.button_font_size),
                                        ..default()
                                    },
                                    TextColor(theme.text_color),
                                    HotbarSlotLabel { slot },
                                ));
                            });
                    }
                });
        });
}

pub fn update_spellbook_ui(
    state: Res<SpellbookUiState>,
    mut list_items: Query<(&SpellListItem, &mut BackgroundColor)>,
    mut slot_labels: Query<(&HotbarSlotLabel, &mut Text)>,
    theme: Res<UiTheme>,
    registry: Res<SpellRegistry>,
    player_query: Query<&SpellHotbar, With<lightyear::prelude::Controlled>>,
) {
    for (item, mut bg) in list_items.iter_mut() {
        if Some(&item.spell_id) == state.selected_spell.as_ref() {
            *bg = BackgroundColor(theme.button_hovered_bg);
        } else {
            *bg = BackgroundColor(theme.button_bg);
        }
    }

    let Some(hotbar) = player_query.iter().next() else {
        return;
    };

    for (slot_label, mut text) in slot_labels.iter_mut() {
        text.0 = format_hotbar_slot_label(slot_label.slot, hotbar, &registry);
    }
}

pub fn handle_spell_selection(
    mut state: ResMut<SpellbookUiState>,
    list_interactions: Query<(&Interaction, &SpellListItem), (Changed<Interaction>, With<Button>)>,
    slot_interactions: Query<(&Interaction, &HotbarSlotUi), (Changed<Interaction>, With<Button>)>,
    close_interactions: Query<&Interaction, (Changed<Interaction>, With<CloseSpellbookButton>)>,
    mut player_query: Query<&mut SpellHotbar, With<lightyear::prelude::Controlled>>,
    mut senders: Query<&mut MessageSender<UpdateHotbarSlotRequest>, With<ConnectedClient>>,
    mut commands: Commands,
    window_query: Query<Entity, With<SpellbookWindow>>,
) {
    for interaction in close_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        state.is_open = false;
        state.selected_spell = None;
        for entity in window_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Select a spell; pressing the selected spell again clears the selection.
    for (interaction, item) in list_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if state.selected_spell.as_ref() == Some(&item.spell_id) {
            state.selected_spell = None;
        } else {
            state.selected_spell = Some(item.spell_id.clone());
        }
    }

    // Assign the selected spell, or clear the slot when no spell is selected.
    for (interaction, slot_ui) in slot_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let selected_spell = state.selected_spell.clone();
        let Some(mut hotbar) = player_query.iter_mut().next() else {
            continue;
        };

        hotbar.assign(slot_ui.slot, selected_spell.clone());

        for mut sender in senders.iter_mut() {
            sender.send::<Channel2>(UpdateHotbarSlotRequest {
                slot: slot_ui.slot,
                spell_id: selected_spell
                    .as_ref()
                    .map(|spell_id| spell_id.as_str().to_string()),
            });
        }
    }
}

fn format_hotbar_slot_label(
    slot: HotbarSlot,
    hotbar: &SpellHotbar,
    registry: &SpellRegistry,
) -> String {
    let slot_label = match slot {
        HotbarSlot::Q => "Q",
        HotbarSlot::W => "W",
        HotbarSlot::E => "E",
    };
    let spell_name = hotbar
        .spell_for_slot(slot)
        .and_then(|spell_id| registry.get(spell_id))
        .map(|spell| spell.display_name().to_string())
        .unwrap_or_else(|| "Empty".to_string());

    format!("{slot_label} - {spell_name}")
}
