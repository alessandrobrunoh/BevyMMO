use super::components::*;
use super::InscriptionUiState;
use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::stdb::{commands as stdb_commands, StdbConnection};
use bevymmo_gameplay::abilities::{
    inscription::{SecondaryWord, WeaponInscription},
    resolve_active_ability, AbilitySelection, AbilitySlot, AncientWordId, AncientWordRegistry,
    BaseAbilityRegistry, KnownAncientLanguage, RootWordId, RootWordRegistry, WeaponAbilities,
};
use bevymmo_gameplay::items::components::{EquipSlot, Equipment};
use bevymmo_gameplay::items::registry::ItemRegistry;

use crate::ui::scrollbar::spawn_scroll_view;
use crate::ui::settings::state::{GameSettingsResource, KeyAction};
use crate::ui::theme::UiTheme;

const WINDOW_WIDTH: f32 = 900.0;
const WINDOW_HEIGHT: f32 = 560.0;
const SLOTS: [AbilitySlot; 3] = [
    AbilitySlot::Primary,
    AbilitySlot::Secondary,
    AbilitySlot::Ultimate,
];

/// Presentation-only mnemonic — the actual key is rebindable
/// (`KeyAction::CastSpellQ/W/E`); this is just a label.
fn slot_key_label(slot: AbilitySlot) -> &'static str {
    match slot {
        AbilitySlot::Primary => "Q",
        AbilitySlot::Secondary => "W",
        AbilitySlot::Ultimate => "E",
    }
}

/// `true` when the currently equipped weapon has Eidolon gestures — the
/// condition both this window and `spell_selector` use to decide which of
/// the two owns the shared toggle key.
fn equipped_weapon_is_eidolon(equipment: &Equipment, registry: &ItemRegistry) -> bool {
    equipment
        .weapon
        .as_ref()
        .and_then(|weapon| registry.get(&weapon.item_id))
        .is_some_and(|item| item.ability_loadout().is_some())
}

#[allow(clippy::too_many_arguments)]
pub fn toggle_inscription_window(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    mut state: ResMut<InscriptionUiState>,
    mut commands: Commands,
    window_query: Query<Entity, With<InscriptionWindow>>,
    theme: Res<UiTheme>,
    item_registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    root_word_registry: Res<RootWordRegistry>,
    ancient_word_registry: Res<AncientWordRegistry>,
    player_query: Query<(&Equipment, &KnownAncientLanguage), With<LocalPlayer>>,
) {
    if !settings.just_pressed(KeyAction::ToggleSpellbook, &keys) {
        return;
    }

    let Ok((equipment, known)) = player_query.single() else {
        return;
    };
    if !equipped_weapon_is_eidolon(equipment, &item_registry) {
        // Not our weapon type this press — `spell_selector` handles it.
        return;
    }

    state.is_open = !state.is_open;

    if !state.is_open {
        despawn_windows(&mut commands, &window_query);
        return;
    }

    spawn_window(
        &mut commands,
        &theme,
        equipment,
        known,
        &item_registry,
        &ability_registry,
        &root_word_registry,
        &ancient_word_registry,
    );
}

/// Rebuilds the window whenever the controlled player's `Equipment` changes
/// (weapon swap, or the server replicating back an inscription this UI just
/// requested) — covers both cases without any local prediction of
/// `ItemInstance.inscriptions`.
#[allow(clippy::too_many_arguments)]
pub fn refresh_inscription_window_on_equipment_change(
    mut commands: Commands,
    state: Res<InscriptionUiState>,
    window_query: Query<Entity, With<InscriptionWindow>>,
    theme: Res<UiTheme>,
    item_registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    root_word_registry: Res<RootWordRegistry>,
    ancient_word_registry: Res<AncientWordRegistry>,
    player_query: Query<
        (&Equipment, &KnownAncientLanguage),
        (With<LocalPlayer>, Changed<Equipment>),
    >,
) {
    if !state.is_open {
        return;
    }
    let Ok((equipment, known)) = player_query.single() else {
        return;
    };

    despawn_windows(&mut commands, &window_query);

    if !equipped_weapon_is_eidolon(equipment, &item_registry) {
        // Swapped to a non-Eidolon weapon while the window was open.
        return;
    }

    spawn_window(
        &mut commands,
        &theme,
        equipment,
        known,
        &item_registry,
        &ability_registry,
        &root_word_registry,
        &ancient_word_registry,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_window(
    commands: &mut Commands,
    theme: &UiTheme,
    equipment: &Equipment,
    known: &KnownAncientLanguage,
    item_registry: &ItemRegistry,
    ability_registry: &BaseAbilityRegistry,
    root_word_registry: &RootWordRegistry,
    ancient_word_registry: &AncientWordRegistry,
) {
    let Some(weapon) = &equipment.weapon else {
        return;
    };
    let Some(item) = item_registry.get(&weapon.item_id) else {
        return;
    };
    let Some(weapon_abilities) = item.ability_loadout() else {
        return;
    };
    let inscription = weapon.root_inscription.clone().unwrap_or_default();

    let window = commands
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
            Button,
            InscriptionWindow,
        ))
        .id();

    commands.entity(window).with_children(|parent| {
        // The title and armor summary stay visible while the three inscription
        // columns scroll below them.
        spawn_header(parent, theme, item.display_name());
        spawn_armor_summary(parent, theme, equipment, item_registry);
        spawn_root_word_section(parent, theme, known, &inscription, root_word_registry);
    });

    let scroll_body = commands
        .spawn((Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            ..default()
        },))
        .id();
    commands.entity(window).add_child(scroll_body);

    spawn_scroll_view(commands, scroll_body, theme, |commands| {
        commands
            .spawn((Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(18.0),
                ..default()
            },))
            .with_children(|body| {
                for slot in SLOTS {
                    spawn_slot_column(
                        body,
                        theme,
                        slot,
                        weapon_abilities,
                        &weapon.ability_selection,
                        &inscription,
                        known,
                        ability_registry,
                        ancient_word_registry,
                    );
                }
            })
            .id()
    });
}

fn armor_key_label(slot: EquipSlot) -> &'static str {
    match slot {
        EquipSlot::Helmet => "D",
        EquipSlot::Armor => "R",
        EquipSlot::Shoes => "F",
        _ => "?",
    }
}

fn spawn_armor_summary(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    equipment: &Equipment,
    item_registry: &ItemRegistry,
) {
    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::bottom(Val::Px(6.0)),
            ..default()
        },))
        .with_children(|section| {
            section.spawn((
                Text::new("Armor inscriptions"),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.9),
                    ..default()
                },
                TextColor(theme.muted_text_color),
            ));
            section
                .spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    ..default()
                },))
                .with_children(|row| {
                    for slot in [EquipSlot::Helmet, EquipSlot::Armor, EquipSlot::Shoes] {
                        let label = equipment
                            .get(slot)
                            .as_ref()
                            .and_then(|instance| {
                                let item_name = item_registry
                                    .get(&instance.item_id)
                                    .map(|item| item.display_name().to_string())?;
                                let inscription = instance.armor_inscription.as_ref();
                                let root = inscription
                                    .and_then(|value| value.root_word.as_ref())
                                    .map(|value| value.as_str().to_string())
                                    .unwrap_or_else(|| "no root".to_string());
                                let words = inscription
                                    .map(|value| {
                                        value
                                            .secondary_words
                                            .iter()
                                            .map(|word| word.word_id.as_str())
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    })
                                    .filter(|value| !value.is_empty())
                                    .unwrap_or_else(|| "no ancient words".to_string());
                                Some(format!("{item_name}\n{root} · {words}"))
                            })
                            .unwrap_or_else(|| "empty".to_string());
                        row.spawn((
                            Node {
                                width: Val::Percent(33.0),
                                min_height: Val::Px(54.0),
                                padding: UiRect::all(Val::Px(5.0)),
                                ..default()
                            },
                            BackgroundColor(theme.button_bg),
                        ))
                        .with_children(|card| {
                            card.spawn((
                                Text::new(format!(
                                    "[{}] {}\n{}",
                                    armor_key_label(slot),
                                    slot.label(),
                                    label
                                )),
                                TextFont {
                                    font_size: FontSize::Px(theme.button_font_size * 0.62),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));
                        });
                    }
                });
        });
}

fn spawn_root_word_section(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    known: &KnownAncientLanguage,
    inscription: &WeaponInscription,
    registry: &RootWordRegistry,
) {
    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            padding: UiRect::bottom(Val::Px(8.0)),
            ..default()
        },))
        .with_children(|section| {
            section.spawn((
                Text::new("Root Word condivisa"),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.9),
                    ..default()
                },
                TextColor(theme.muted_text_color),
            ));
            section.spawn((
                Text::new("Una sola parola definisce cosa manifesta l'arma."),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.7),
                    ..default()
                },
                TextColor(theme.muted_text_color),
            ));

            if known.root_words.is_empty() {
                spawn_muted_line(section, theme, "Nessuna Root Word conosciuta");
                return;
            }

            section
                .spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                },))
                .with_children(|row| {
                    for root_id in sorted_root_words(known, registry) {
                        let Some(root) = registry.get(root_id) else {
                            continue;
                        };
                        let is_active = inscription.root_word.as_ref() == Some(root_id);
                        spawn_compact_toggle_button(
                            row,
                            theme,
                            root.metadata().display_name,
                            is_active,
                            RootWordToggleButton {
                                root_word_id: root_id.as_str().to_string(),
                            },
                        );
                    }
                });
        });
}

fn sorted_root_words<'a>(
    known: &'a KnownAncientLanguage,
    registry: &RootWordRegistry,
) -> Vec<&'a RootWordId> {
    let mut ids: Vec<_> = known.root_words.iter().collect();
    ids.sort_by(|left, right| {
        let left_name = registry
            .get(left)
            .map(|word| word.metadata().display_name)
            .unwrap_or("");
        let right_name = registry
            .get(right)
            .map(|word| word.metadata().display_name)
            .unwrap_or("");
        left_name
            .cmp(right_name)
            .then_with(|| left.as_str().cmp(right.as_str()))
    });
    ids
}

fn sorted_ancient_words<'a>(
    known: &'a KnownAncientLanguage,
    registry: &AncientWordRegistry,
) -> Vec<&'a AncientWordId> {
    let mut ids: Vec<_> = known.ancient_words.iter().collect();
    ids.sort_by(|left, right| {
        let left_word = registry.get(left);
        let right_word = registry.get(right);
        let left_phase = left_word
            .as_ref()
            .map(|word| word.metadata().phase)
            .unwrap_or(u8::MAX);
        let right_phase = right_word
            .as_ref()
            .map(|word| word.metadata().phase)
            .unwrap_or(u8::MAX);
        left_phase
            .cmp(&right_phase)
            .then_with(|| left.as_str().cmp(right.as_str()))
    });
    ids
}

fn spawn_compact_toggle_button(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    label: &str,
    is_active: bool,
    marker: impl Component,
) {
    let text = if is_active {
        format!("✓ {label}")
    } else {
        label.to_string()
    };
    parent
        .spawn((
            Button,
            Node {
                min_width: Val::Px(112.0),
                min_height: Val::Px(30.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(if is_active {
                theme.button_pressed_bg
            } else {
                theme.button_bg
            }),
            marker,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.82),
                    ..default()
                },
                TextColor(theme.text_color),
            ));
        });
}

fn spawn_header(parent: &mut ChildSpawnerCommands, theme: &UiTheme, weapon_name: &str) {
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
                Text(format!("Inscriptions \u{2014} {weapon_name}")),
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
                    CloseInscriptionButton,
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

#[allow(clippy::too_many_arguments)]
fn spawn_slot_column(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    slot: AbilitySlot,
    weapon_abilities: &WeaponAbilities,
    selection: &AbilitySelection,
    inscription: &WeaponInscription,
    known: &KnownAncientLanguage,
    ability_registry: &BaseAbilityRegistry,
    ancient_word_registry: &AncientWordRegistry,
) {
    let Some(ability_id) = resolve_active_ability(slot, weapon_abilities, selection) else {
        return;
    };
    let Some(ability) = ability_registry.get(ability_id) else {
        return;
    };
    let slot_ins = inscription.get(slot);

    parent
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(33.0),
            row_gap: Val::Px(6.0),
            ..default()
        },))
        .with_children(|column| {
            column.spawn((
                Text(format!(
                    "{} \u{2014} {}",
                    slot_key_label(slot),
                    ability.display_name()
                )),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size),
                    ..default()
                },
                TextColor(theme.text_color),
            ));

            // Only worth a picker when the weapon actually offers a choice.
            let options = weapon_abilities.options_for(slot);
            if options.len() > 1 {
                column.spawn((
                    Text("Gesto".to_string()),
                    TextFont {
                        font_size: FontSize::Px(theme.button_font_size * 0.85),
                        ..default()
                    },
                    TextColor(theme.muted_text_color),
                ));
                for option_id in options {
                    let Some(option) = ability_registry.get(option_id) else {
                        continue;
                    };
                    spawn_toggle_button(
                        column,
                        theme,
                        option.display_name(),
                        option_id == ability_id,
                        AbilitySelectButton {
                            slot,
                            ability_id: option_id.as_str().to_string(),
                        },
                    );
                }
            }

            column.spawn((
                Text("Ancient Words".to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.85),
                    ..default()
                },
                TextColor(theme.muted_text_color),
            ));
            if known.ancient_words.is_empty() {
                spawn_muted_line(column, theme, "No Ancient Word known yet");
            }
            for word_id in sorted_ancient_words(known, ancient_word_registry) {
                let Some(word) = ancient_word_registry.get(word_id) else {
                    continue;
                };
                let is_active = slot_ins
                    .secondary_words
                    .iter()
                    .any(|w| w.word_id == *word_id);
                spawn_toggle_button(
                    column,
                    theme,
                    word.display_name(),
                    is_active,
                    AncientWordToggleButton {
                        slot,
                        word_id: word_id.as_str().to_string(),
                    },
                );
            }
        });
}

fn spawn_muted_line(parent: &mut ChildSpawnerCommands, theme: &UiTheme, text: &str) {
    parent.spawn((
        Text(text.to_string()),
        TextFont {
            font_size: FontSize::Px(theme.button_font_size * 0.8),
            ..default()
        },
        TextColor(theme.muted_text_color),
    ));
}

fn spawn_toggle_button(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    label: &str,
    is_active: bool,
    marker: impl Component,
) {
    let text = if is_active {
        format!("\u{2713} {label}")
    } else {
        label.to_string()
    };
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(30.0),
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
            marker,
        ))
        .with_children(|button| {
            button.spawn((
                Text(text),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size),
                    ..default()
                },
                TextColor(theme.text_color),
            ));
        });
}

#[allow(clippy::type_complexity)]
pub fn handle_inscription_interactions(
    mut state: ResMut<InscriptionUiState>,
    ancient_word_interactions: Query<
        (&Interaction, &AncientWordToggleButton),
        (Changed<Interaction>, With<Button>),
    >,
    root_word_interactions: Query<
        (&Interaction, &RootWordToggleButton),
        (Changed<Interaction>, With<Button>),
    >,
    ability_interactions: Query<
        (&Interaction, &AbilitySelectButton),
        (Changed<Interaction>, With<Button>),
    >,
    close_interactions: Query<&Interaction, (Changed<Interaction>, With<CloseInscriptionButton>)>,
    player_query: Query<&Equipment, With<LocalPlayer>>,
    conn: Option<Res<StdbConnection>>,
    mut commands: Commands,
    window_query: Query<Entity, With<InscriptionWindow>>,
) {
    if close_interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        state.is_open = false;
        despawn_windows(&mut commands, &window_query);
        return;
    }

    let Ok(equipment) = player_query.single() else {
        return;
    };
    let Some(weapon) = &equipment.weapon else {
        return;
    };
    let current = weapon.root_inscription.clone().unwrap_or_default();

    for (interaction, pick) in ability_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if weapon
            .ability_selection
            .get(pick.slot)
            .map(|id| id.as_str())
            == Some(pick.ability_id.as_str())
        {
            // Already the active gesture — nothing to ask the server for.
            continue;
        }
        if let Some(conn) = conn.as_deref() {
            if let Err(err) =
                stdb_commands::set_ability_selection(conn, pick.slot, pick.ability_id.clone())
            {
                error!("could not set ability selection: {err}");
            }
        }
    }

    for (interaction, toggle) in root_word_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let toggled_off =
            current.root_word.as_ref().map(|id| id.as_str()) == Some(toggle.root_word_id.as_str());
        let new_root = if toggled_off {
            None
        } else {
            Some(RootWordId::new(toggle.root_word_id.clone()))
        };
        send_root_update(conn.as_deref(), new_root, &current);
    }

    for (interaction, toggle) in ancient_word_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let mut updated = current.clone();
        let slot_ins = updated.get_mut(toggle.slot);
        let word_id = AncientWordId::new(toggle.word_id.clone());
        if let Some(pos) = slot_ins
            .secondary_words
            .iter()
            .position(|w| w.word_id == word_id)
        {
            slot_ins.secondary_words.remove(pos);
        } else {
            slot_ins.secondary_words.push(SecondaryWord::new(word_id));
        }
        send_full_update(conn.as_deref(), &updated);
    }
}

fn send_root_update(
    conn: Option<&StdbConnection>,
    root_word: Option<RootWordId>,
    current: &WeaponInscription,
) {
    let Some(conn) = conn else {
        return;
    };
    let mut updated = current.clone();
    updated.root_word = root_word;
    send_full_update(Some(conn), &updated);
}

fn send_full_update(conn: Option<&StdbConnection>, inscription: &WeaponInscription) {
    let Some(conn) = conn else {
        return;
    };

    let words_for = |slot: &bevymmo_gameplay::abilities::inscription::SlotInscription| {
        slot.secondary_words
            .iter()
            .map(|word| word.word_id.as_str().to_string())
            .collect()
    };

    if let Err(error) = stdb_commands::set_root_inscription(
        conn,
        inscription
            .root_word
            .as_ref()
            .map(|word| word.as_str().to_string()),
        words_for(&inscription.primary),
        words_for(&inscription.secondary),
        words_for(&inscription.ultimate),
    ) {
        error!("could not update Root Word inscription: {error}");
    }
}

fn despawn_windows(commands: &mut Commands, window_query: &Query<Entity, With<InscriptionWindow>>) {
    for entity in window_query.iter() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_word_order_is_stable_for_hash_set_storage() {
        let known = KnownAncientLanguage {
            root_words: ["stone", "damage", "flame"]
                .into_iter()
                .map(RootWordId::from)
                .collect(),
            ..default()
        };
        let ordered: Vec<_> = sorted_root_words(&known, &RootWordRegistry::default())
            .into_iter()
            .map(|id| id.as_str())
            .collect();
        assert_eq!(ordered, ["damage", "flame", "stone"]);
    }

    #[test]
    fn ancient_word_order_is_stable_for_hash_set_storage() {
        let known = KnownAncientLanguage {
            ancient_words: ["twin", "echo", "anchor"]
                .into_iter()
                .map(AncientWordId::new)
                .collect(),
            ..default()
        };
        let ordered: Vec<_> = sorted_ancient_words(&known, &AncientWordRegistry::default())
            .into_iter()
            .map(|id| id.as_str())
            .collect();
        assert_eq!(ordered, ["anchor", "echo", "twin"]);
    }
}
