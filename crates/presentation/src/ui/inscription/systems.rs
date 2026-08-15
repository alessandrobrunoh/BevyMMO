use super::components::*;
use super::InscriptionUiState;
use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::abilities::{
    resolve_active_ability, AbilitySelection, AbilitySlot, BaseAbilityRegistry, EssenceId,
    EssenceRegistry, Inscription, KnownGlyphs, ModifierId, ModifierRegistry, WeaponAbilities,
    WeaponInscriptions,
};
use bevymmo_shared::items::components::Equipment;
use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::network::protocol::{
    Channel2, UpdateAbilitySelectionRequest, UpdateInscriptionRequest,
};
use bevymmo_shared::entity::LocalPlayer;
use lightyear::prelude::MessageSender;

use crate::ui::settings::state::{GameSettingsResource, KeyAction};
use crate::ui::theme::UiTheme;

const WINDOW_WIDTH: f32 = 820.0;
const WINDOW_HEIGHT: f32 = 460.0;
const SLOTS: [AbilitySlot; 3] = [AbilitySlot::Primary, AbilitySlot::Secondary, AbilitySlot::Ultimate];

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
        .is_some_and(|item| item.weapon_abilities().is_some())
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
    essence_registry: Res<EssenceRegistry>,
    modifier_registry: Res<ModifierRegistry>,
    player_query: Query<(&Equipment, &KnownGlyphs), With<LocalPlayer>>,
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
        &essence_registry,
        &modifier_registry,
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
    essence_registry: Res<EssenceRegistry>,
    modifier_registry: Res<ModifierRegistry>,
    player_query: Query<(&Equipment, &KnownGlyphs), (With<LocalPlayer>, Changed<Equipment>)>,
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
        &essence_registry,
        &modifier_registry,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_window(
    commands: &mut Commands,
    theme: &UiTheme,
    equipment: &Equipment,
    known: &KnownGlyphs,
    item_registry: &ItemRegistry,
    ability_registry: &BaseAbilityRegistry,
    essence_registry: &EssenceRegistry,
    modifier_registry: &ModifierRegistry,
) {
    let Some(weapon) = &equipment.weapon else {
        return;
    };
    let Some(item) = item_registry.get(&weapon.item_id) else {
        return;
    };
    let Some(weapon_abilities) = item.weapon_abilities() else {
        return;
    };
    let inscriptions = weapon.inscriptions.clone().unwrap_or_default();

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
            InscriptionWindow,
        ))
        .with_children(|parent| {
            spawn_header(parent, theme, item.display_name());

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
                        spawn_slot_column(
                            body,
                            theme,
                            slot,
                            weapon_abilities,
                            &weapon.ability_selection,
                            &inscriptions,
                            known,
                            ability_registry,
                            essence_registry,
                            modifier_registry,
                        );
                    }
                });
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
    inscriptions: &WeaponInscriptions,
    known: &KnownGlyphs,
    ability_registry: &BaseAbilityRegistry,
    essence_registry: &EssenceRegistry,
    modifier_registry: &ModifierRegistry,
) {
    let Some(ability_id) = resolve_active_ability(slot, weapon_abilities, selection) else {
        return;
    };
    let Some(ability) = ability_registry.get(ability_id) else {
        return;
    };
    let inscription = inscriptions.get(slot);

    parent
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(33.0),
            row_gap: Val::Px(6.0),
            ..default()
        },))
        .with_children(|column| {
            column.spawn((
                Text(format!("{} \u{2014} {}", slot_key_label(slot), ability.display_name())),
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
                Text("Essenza".to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.85),
                    ..default()
                },
                TextColor(theme.muted_text_color),
            ));
            if known.essences.is_empty() {
                spawn_muted_line(column, theme, "No Essenza known yet");
            }
            for essence_id in &known.essences {
                let Some(essence) = essence_registry.get(essence_id) else {
                    continue;
                };
                let is_active = inscription.essence.as_ref() == Some(essence_id);
                spawn_toggle_button(
                    column,
                    theme,
                    essence.display_name(),
                    is_active,
                    EssenceToggleButton { slot, essence_id: essence_id.as_str().to_string() },
                );
            }

            column.spawn((
                Text("Modificatori".to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.85),
                    ..default()
                },
                TextColor(theme.muted_text_color),
            ));
            let compatible_modifiers: Vec<_> = known
                .modifiers
                .iter()
                .filter_map(|id| modifier_registry.get(id).map(|m| (id.clone(), m)))
                .filter(|(_, modifier)| ability.has_tag(modifier.required_tag()))
                .collect();
            if compatible_modifiers.is_empty() {
                spawn_muted_line(column, theme, "No compatible Modificatore known");
            }
            for (modifier_id, modifier) in compatible_modifiers {
                let is_active = inscription.modifiers.contains(&modifier_id);
                spawn_toggle_button(
                    column,
                    theme,
                    modifier.display_name(),
                    is_active,
                    ModifierToggleButton { slot, modifier_id: modifier_id.as_str().to_string() },
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
    let text = if is_active { format!("\u{2713} {label}") } else { label.to_string() };
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
            BackgroundColor(if is_active { theme.button_pressed_bg } else { theme.button_bg }),
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
    essence_interactions: Query<
        (&Interaction, &EssenceToggleButton),
        (Changed<Interaction>, With<Button>),
    >,
    modifier_interactions: Query<
        (&Interaction, &ModifierToggleButton),
        (Changed<Interaction>, With<Button>),
    >,
    ability_interactions: Query<
        (&Interaction, &AbilitySelectButton),
        (Changed<Interaction>, With<Button>),
    >,
    close_interactions: Query<&Interaction, (Changed<Interaction>, With<CloseInscriptionButton>)>,
    player_query: Query<&Equipment, With<LocalPlayer>>,
    mut senders: Query<&mut MessageSender<UpdateInscriptionRequest>, With<ConnectedClient>>,
    mut selection_senders: Query<
        &mut MessageSender<UpdateAbilitySelectionRequest>,
        With<ConnectedClient>,
    >,
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
    let current = weapon.inscriptions.clone().unwrap_or_default();

    for (interaction, pick) in ability_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if weapon.ability_selection.get(pick.slot).map(|id| id.as_str())
            == Some(pick.ability_id.as_str())
        {
            // Already the active gesture — nothing to ask the server for.
            continue;
        }
        for mut sender in selection_senders.iter_mut() {
            sender.send::<Channel2>(UpdateAbilitySelectionRequest {
                slot: pick.slot,
                ability_id: pick.ability_id.clone(),
            });
        }
    }

    for (interaction, toggle) in essence_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let mut inscription = current.get(toggle.slot).clone();
        let toggled_off = inscription.essence.as_ref().map(|id| id.as_str()) == Some(toggle.essence_id.as_str());
        inscription.essence = if toggled_off { None } else { Some(EssenceId::new(toggle.essence_id.clone())) };
        send_update(&mut senders, toggle.slot, &inscription);
    }

    for (interaction, toggle) in modifier_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let mut inscription = current.get(toggle.slot).clone();
        if let Some(pos) = inscription
            .modifiers
            .iter()
            .position(|id| id.as_str() == toggle.modifier_id.as_str())
        {
            inscription.modifiers.remove(pos);
        } else {
            inscription.modifiers.push(ModifierId::new(toggle.modifier_id.clone()));
        }
        send_update(&mut senders, toggle.slot, &inscription);
    }
}

fn send_update(
    senders: &mut Query<&mut MessageSender<UpdateInscriptionRequest>, With<ConnectedClient>>,
    slot: AbilitySlot,
    inscription: &Inscription,
) {
    for mut sender in senders.iter_mut() {
        sender.send::<Channel2>(UpdateInscriptionRequest {
            slot,
            essence: inscription.essence.as_ref().map(|id| id.as_str().to_string()),
            modifiers: inscription.modifiers.iter().map(|id| id.as_str().to_string()).collect(),
            ancient_word: inscription.ancient_word.as_ref().map(|id| id.as_str().to_string()),
        });
    }
}

fn despawn_windows(commands: &mut Commands, window_query: &Query<Entity, With<InscriptionWindow>>) {
    for entity in window_query.iter() {
        commands.entity(entity).despawn();
    }
}
