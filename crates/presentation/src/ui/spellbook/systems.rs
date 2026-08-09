use super::components::*;
use super::SpellbookUiState;
use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::network::protocol::{Channel2, UpdateHotbarSlotRequest};
use bevymmo_shared::spells::{HotbarSlot, SpellHotbar, SpellId, SpellRegistry};
use lightyear::prelude::MessageSender;

use crate::ui::settings::state::{GameSettingsResource, KeyAction};
use crate::ui::theme::UiTheme;

const SPELLBOOK_WIDTH: f32 = 760.0;
const SPELLBOOK_HEIGHT: f32 = 460.0;

pub fn toggle_spellbook(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    mut state: ResMut<SpellbookUiState>,
    mut commands: Commands,
    window_query: Query<Entity, With<SpellbookWindow>>,
    theme: Res<UiTheme>,
    registry: Res<SpellRegistry>,
    player_query: Query<&SpellHotbar, With<lightyear::prelude::Controlled>>,
) {
    if !settings.just_pressed(KeyAction::ToggleSpellbook, &keys) {
        return;
    }

    state.is_open = !state.is_open;
    state.selected_spell = None;

    if !state.is_open {
        despawn_spellbook_windows(&mut commands, &window_query);
        return;
    }

    let hotbar = player_query.iter().next().cloned().unwrap_or_default();
    spawn_spellbook_window(&mut commands, &theme, &registry, &hotbar);
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
                width: Val::Px(SPELLBOOK_WIDTH),
                height: Val::Px(SPELLBOOK_HEIGHT),
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-SPELLBOOK_WIDTH * 0.5),
                    top: Val::Px(-SPELLBOOK_HEIGHT * 0.5),
                    ..default()
                },
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            SpellbookWindow,
        ))
        .with_children(|parent| {
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
                        Text("Spellbook".to_string()),
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
                });

            parent
                .spawn((Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(18.0),
                    ..default()
                },))
                .with_children(|body| {
                    body.spawn((Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(62.0),
                        row_gap: Val::Px(6.0),
                        ..default()
                    },))
                        .with_children(|list_parent| {
                            list_parent.spawn((
                                Text("Available spells".to_string()),
                                TextFont {
                                    font_size: FontSize::Px(theme.button_font_size),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));

                            for (spell_id, spell) in registry.sorted_spells() {
                                spawn_spell_row(list_parent, theme, spell_id, spell.display_name());
                            }
                        });

                    body.spawn((Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(38.0),
                        row_gap: Val::Px(10.0),
                        ..default()
                    },))
                        .with_children(|slots_parent| {
                            slots_parent.spawn((
                                Text("Current hotbar".to_string()),
                                TextFont {
                                    font_size: FontSize::Px(theme.button_font_size),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));

                            for slot in [HotbarSlot::Q, HotbarSlot::W, HotbarSlot::E] {
                                slots_parent
                                    .spawn((Node {
                                        width: Val::Percent(100.0),
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(8.0),
                                        ..default()
                                    },))
                                    .with_children(|row| {
                                        row.spawn((
                                            Text(format_hotbar_slot_label(slot, hotbar, registry)),
                                            TextFont {
                                                font_size: FontSize::Px(theme.button_font_size),
                                                ..default()
                                            },
                                            TextColor(theme.text_color),
                                            HotbarSlotLabel { slot },
                                            Node {
                                                width: Val::Px(150.0),
                                                ..default()
                                            },
                                        ));

                                        row.spawn((
                                            Button,
                                            Node {
                                                width: Val::Px(64.0),
                                                height: Val::Px(30.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BackgroundColor(theme.button_bg),
                                            ClearHotbarSlotButton { slot },
                                        ))
                                        .with_children(
                                            |button| {
                                                button.spawn((
                                                    Text("Clear".to_string()),
                                                    TextFont {
                                                        font_size: FontSize::Px(
                                                            theme.button_font_size,
                                                        ),
                                                        ..default()
                                                    },
                                                    TextColor(theme.text_color),
                                                ));
                                            },
                                        );
                                    });
                            }
                        });
                });
        });
}

fn spawn_spell_row(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    spell_id: SpellId,
    display_name: &'static str,
) {
    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(38.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text(display_name.to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size),
                    ..default()
                },
                TextColor(theme.text_color),
                Node {
                    width: Val::Px(190.0),
                    ..default()
                },
            ));

            for slot in [HotbarSlot::Q, HotbarSlot::W, HotbarSlot::E] {
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(48.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(theme.button_bg),
                    SpellAssignmentButton {
                        slot,
                        spell_id: spell_id.clone(),
                    },
                ))
                .with_children(|button| {
                    button.spawn((
                        Text(slot_label(slot).to_string()),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size),
                            ..default()
                        },
                        TextColor(theme.text_color),
                    ));
                });
            }
        });
}

pub fn update_spellbook_ui(
    mut slot_labels: Query<(&HotbarSlotLabel, &mut Text)>,
    registry: Res<SpellRegistry>,
    player_query: Query<&SpellHotbar, With<lightyear::prelude::Controlled>>,
) {
    let Some(hotbar) = player_query.iter().next() else {
        return;
    };

    for (slot_label, mut text) in slot_labels.iter_mut() {
        text.0 = format_hotbar_slot_label(slot_label.slot, hotbar, &registry);
    }
}

#[allow(clippy::type_complexity)]
pub fn handle_spellbook_interactions(
    mut state: ResMut<SpellbookUiState>,
    assignment_interactions: Query<
        (&Interaction, &SpellAssignmentButton),
        (Changed<Interaction>, With<Button>),
    >,
    clear_interactions: Query<
        (&Interaction, &ClearHotbarSlotButton),
        (Changed<Interaction>, With<Button>),
    >,
    close_interactions: Query<&Interaction, (Changed<Interaction>, With<CloseSpellbookButton>)>,
    mut player_query: Query<&mut SpellHotbar, With<lightyear::prelude::Controlled>>,
    mut senders: Query<&mut MessageSender<UpdateHotbarSlotRequest>, With<ConnectedClient>>,
    mut commands: Commands,
    window_query: Query<Entity, With<SpellbookWindow>>,
) {
    if close_interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        state.is_open = false;
        state.selected_spell = None;
        despawn_spellbook_windows(&mut commands, &window_query);
        return;
    }

    for (interaction, assignment) in assignment_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_hotbar_assignment(
            assignment.slot,
            Some(assignment.spell_id.clone()),
            &mut player_query,
            &mut senders,
        );
    }

    for (interaction, clear_slot) in clear_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_hotbar_assignment(clear_slot.slot, None, &mut player_query, &mut senders);
    }
}

fn apply_hotbar_assignment(
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

fn despawn_spellbook_windows(
    commands: &mut Commands,
    window_query: &Query<Entity, With<SpellbookWindow>>,
) {
    for entity in window_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn format_hotbar_slot_label(
    slot: HotbarSlot,
    hotbar: &SpellHotbar,
    registry: &SpellRegistry,
) -> String {
    let spell_name = hotbar
        .spell_for_slot(slot)
        .and_then(|spell_id| registry.get(spell_id))
        .map(|spell| spell.display_name().to_string())
        .unwrap_or_else(|| "Empty".to_string());

    format!("{} - {spell_name}", slot_label(slot))
}

fn slot_label(slot: HotbarSlot) -> &'static str {
    match slot {
        HotbarSlot::Q => "Q",
        HotbarSlot::W => "W",
        HotbarSlot::E => "E",
    }
}
