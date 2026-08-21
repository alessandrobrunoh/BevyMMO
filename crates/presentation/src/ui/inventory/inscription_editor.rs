//! Interactive inscription editor embedded in the selected item's detail card.

use bevy::prelude::*;
use bevymmo_client::stdb::{StdbConnection, commands as stdb_commands};
use bevymmo_gameplay::{
    abilities::{
        AbilitySlot, AncientWordId, AncientWordRegistry, BaseAbilityRegistry, KnownAncientLanguage,
        RootWordId, RootWordRegistry,
        inscription::{ArmorInscription, SecondaryWord, WeaponInscription},
        resolve_active_ability,
    },
    items::{
        components::{EquipSlot, Equipment},
        definition::Item,
        instance::ItemInstance,
    },
};

use super::{ItemDetailUiState, weapon_detail::WeaponSummary};
use crate::ui::{
    button::{BarButtonKind, UiButtonImages, spawn_bar_child},
    scrollbar::spawn_scroll_view_scrolled,
    theme::UiTheme,
};

const CHOICE_WIDTH: f32 = 104.0;
const CHOICE_HEIGHT: f32 = 28.0;
const CHOICE_FONT_SIZE: f32 = 11.0;
const TAB_WIDTH: f32 = 112.0;
const TAB_HEIGHT: f32 = 30.0;
const SECTION_GAP: f32 = 10.0;
const MUTED_PANEL: Color = Color::srgba(0.07, 0.08, 0.11, 0.72);
const ACTIVE_BORDER: Color = Color::srgba(0.3, 0.72, 0.95, 0.95);
const MUTED_BORDER: Color = Color::srgba(0.48, 0.5, 0.56, 0.45);

#[derive(Component, Debug)]
pub struct ItemConfigurationPanel;

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemAbilityTabButton {
    pub slot: AbilitySlot,
}

#[derive(Component, Debug, Clone, Copy)]
pub(super) struct ItemAbilityPane {
    slot: AbilitySlot,
}

#[derive(Debug, Clone, Copy)]
enum InscriptionTarget {
    Weapon,
    Armor(EquipSlot),
}

#[derive(Debug, Clone)]
enum ItemConfigAction {
    RootWord {
        target: InscriptionTarget,
        root_word_id: String,
    },
    AncientWord {
        target: InscriptionTarget,
        slot: AbilitySlot,
        word_id: String,
    },
    Ability {
        slot: AbilitySlot,
        ability_id: String,
    },
}

#[derive(Component, Debug, Clone)]
pub(super) struct ItemConfigChoice(ItemConfigAction);

#[derive(Clone, Copy)]
pub(super) struct ItemEditorRegistries<'a> {
    pub abilities: &'a BaseAbilityRegistry,
    pub root_words: &'a RootWordRegistry,
    pub ancient_words: &'a AncientWordRegistry,
}

pub(super) struct ItemEditorContext<'a> {
    pub item: &'a dyn Item,
    pub instance: &'a ItemInstance,
    pub equipped_slot: Option<EquipSlot>,
    pub known: &'a KnownAncientLanguage,
    pub registries: ItemEditorRegistries<'a>,
    pub weapon_summary: Option<&'a WeaponSummary>,
    pub active_slot: AbilitySlot,
    pub initial_scroll: f32,
}

pub(super) fn spawn_item_editor(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    context: ItemEditorContext<'_>,
) {
    let panel = parent
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: Val::Px(0.0),
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            ItemConfigurationPanel,
        ))
        .with_children(|panel| {
            spawn_heading(panel, theme, "ITEM CONFIGURATION");
            if context.equipped_slot.is_none() {
                spawn_notice(panel, theme, "Equip this item to change its configuration");
            }
        })
        .id();

    let mut commands = parent.commands();
    spawn_scroll_view_scrolled(
        &mut commands,
        panel,
        theme,
        context.initial_scroll,
        move |commands| {
            commands
                .spawn(Node::default())
                .with_children(|content| {
                    content
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(SECTION_GAP),
                            padding: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        })
                        .with_children(|editor| {
                            let Some(loadout) = context.item.ability_loadout() else {
                                spawn_muted(
                                    editor,
                                    theme,
                                    "This item has no configurable abilities",
                                );
                                return;
                            };
                            let Some(profile) = context.item.rune_profile() else {
                                spawn_muted(editor, theme, "This item cannot be inscribed");
                                return;
                            };

                            match context.item.config().equippable_into {
                                Some(EquipSlot::Weapon) => spawn_weapon_editor(
                                    editor,
                                    theme,
                                    &context,
                                    loadout,
                                    profile.capacity,
                                ),
                                Some(
                                    slot
                                    @ (EquipSlot::Helmet | EquipSlot::Armor | EquipSlot::Shoes),
                                ) => {
                                    spawn_armor_editor(
                                        editor,
                                        theme,
                                        &context,
                                        loadout,
                                        profile.capacity,
                                        slot,
                                    );
                                }
                                _ => spawn_muted(
                                    editor,
                                    theme,
                                    "This equipment type has no inscription editor",
                                ),
                            }
                        });
                })
                .id()
        },
    );
}

fn spawn_weapon_editor(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    context: &ItemEditorContext<'_>,
    loadout: &bevymmo_gameplay::abilities::WeaponAbilities,
    capacity: u32,
) {
    let inscription = context
        .instance
        .root_inscription
        .clone()
        .unwrap_or_default();
    let editable = context.equipped_slot == Some(EquipSlot::Weapon);
    spawn_root_word_picker(
        parent,
        theme,
        context.known,
        context.registries,
        inscription.root_word.as_ref(),
        rune_usage_weapon(&inscription, context.registries),
        capacity,
        editable,
        InscriptionTarget::Weapon,
    );

    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|tabs| {
            for slot in AbilitySlot::ALL {
                spawn_bar_child(
                    tabs,
                    slot_label(slot),
                    CHOICE_FONT_SIZE,
                    theme.button_text_color,
                    Val::Px(TAB_WIDTH),
                    Val::Px(TAB_HEIGHT),
                    if slot == context.active_slot {
                        BarButtonKind::Primary
                    } else {
                        BarButtonKind::Neutral
                    },
                    ItemAbilityTabButton { slot },
                );
            }
        });

    for slot in AbilitySlot::ALL {
        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    display: if slot == context.active_slot {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                ItemAbilityPane { slot },
            ))
            .with_children(|pane| {
                spawn_weapon_slot_editor(
                    pane,
                    theme,
                    context,
                    loadout,
                    &inscription,
                    slot,
                    editable,
                );
            });
    }
}

fn spawn_weapon_slot_editor(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    context: &ItemEditorContext<'_>,
    loadout: &bevymmo_gameplay::abilities::WeaponAbilities,
    inscription: &WeaponInscription,
    slot: AbilitySlot,
    editable: bool,
) {
    let Some(active_id) =
        resolve_active_ability(slot, loadout, &context.instance.ability_selection)
    else {
        spawn_muted(parent, theme, "No ability offered for this slot");
        return;
    };
    let Some(active_ability) = context.registries.abilities.get(active_id) else {
        spawn_muted(parent, theme, "Unknown ability");
        return;
    };

    spawn_heading(
        parent,
        theme,
        &format!("{} · {}", slot_label(slot), active_ability.display_name()),
    );

    if let Some(summary) = context
        .weapon_summary
        .and_then(|summary| summary.slots.get(slot_index(slot)))
    {
        spawn_muted(parent, theme, &summary.shape);
        spawn_muted(parent, theme, &summary.stats);
        if let Some(blocked) = &summary.blocked {
            spawn_notice(parent, theme, blocked);
        }
    }

    let options = loadout.options_for(slot);
    if options.len() > 1 {
        spawn_label(parent, theme, "ACTIVE ABILITY");
        spawn_choice_row(parent, |row| {
            for option_id in options {
                let Some(option) = context.registries.abilities.get(option_id) else {
                    continue;
                };
                spawn_choice(
                    row,
                    theme,
                    option.display_name(),
                    option_id == active_id,
                    editable,
                    ItemConfigAction::Ability {
                        slot,
                        ability_id: option_id.as_str().to_string(),
                    },
                );
            }
        });
    }

    let selected_words = &inscription.get(slot).secondary_words;
    spawn_label(
        parent,
        theme,
        &format!(
            "ANCIENT WORDS  {}/{}",
            selected_words.len(),
            max_words(slot)
        ),
    );
    if context.known.ancient_words.is_empty() {
        spawn_muted(parent, theme, "No Ancient Words known");
        return;
    }

    spawn_choice_row(parent, |row| {
        for word_id in sorted_ancient_words(context.known, context.registries.ancient_words) {
            let Some(word) = context.registries.ancient_words.get(word_id) else {
                continue;
            };
            let active = selected_words.iter().any(|word| word.word_id == *word_id);
            let compatible = word.metadata().is_compatible_with(active_ability.tags());
            spawn_choice(
                row,
                theme,
                word.display_name(),
                active,
                editable && compatible && (active || selected_words.len() < max_words(slot)),
                ItemConfigAction::AncientWord {
                    target: InscriptionTarget::Weapon,
                    slot,
                    word_id: word_id.as_str().to_string(),
                },
            );
        }
    });
}

fn spawn_armor_editor(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    context: &ItemEditorContext<'_>,
    loadout: &bevymmo_gameplay::abilities::WeaponAbilities,
    capacity: u32,
    slot: EquipSlot,
) {
    let inscription = context
        .instance
        .armor_inscription
        .clone()
        .unwrap_or_default();
    let editable = context.equipped_slot == Some(slot);
    spawn_root_word_picker(
        parent,
        theme,
        context.known,
        context.registries,
        inscription.root_word.as_ref(),
        rune_usage_armor(&inscription, context.registries),
        capacity,
        editable,
        InscriptionTarget::Armor(slot),
    );

    let Some(ability_id) = resolve_active_ability(
        AbilitySlot::Primary,
        loadout,
        &context.instance.ability_selection,
    ) else {
        spawn_muted(parent, theme, "No ability offered by this item");
        return;
    };
    let Some(ability) = context.registries.abilities.get(ability_id) else {
        spawn_muted(parent, theme, "Unknown item ability");
        return;
    };

    spawn_heading(parent, theme, ability.display_name());
    spawn_label(
        parent,
        theme,
        &format!("ANCIENT WORDS  {}/2", inscription.secondary_words.len()),
    );
    spawn_choice_row(parent, |row| {
        for word_id in sorted_ancient_words(context.known, context.registries.ancient_words) {
            let Some(word) = context.registries.ancient_words.get(word_id) else {
                continue;
            };
            let active = inscription
                .secondary_words
                .iter()
                .any(|selected| selected.word_id == *word_id);
            let compatible = word.metadata().is_compatible_with(ability.tags());
            spawn_choice(
                row,
                theme,
                word.display_name(),
                active,
                editable && compatible && (active || inscription.secondary_words.len() < 2),
                ItemConfigAction::AncientWord {
                    target: InscriptionTarget::Armor(slot),
                    slot: AbilitySlot::Primary,
                    word_id: word_id.as_str().to_string(),
                },
            );
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn spawn_root_word_picker(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    known: &KnownAncientLanguage,
    registries: ItemEditorRegistries<'_>,
    current: Option<&RootWordId>,
    used: u32,
    capacity: u32,
    editable: bool,
    target: InscriptionTarget,
) {
    spawn_heading(parent, theme, "ROOT WORD");
    spawn_muted(
        parent,
        theme,
        &format!("Rune capacity  {used} / {capacity}"),
    );
    if known.root_words.is_empty() {
        spawn_muted(parent, theme, "No Root Words known");
        return;
    }

    spawn_choice_row(parent, |row| {
        for root_id in sorted_root_words(known, registries.root_words) {
            let Some(root) = registries.root_words.get(root_id) else {
                continue;
            };
            spawn_choice(
                row,
                theme,
                root.metadata().display_name,
                current == Some(root_id),
                editable,
                ItemConfigAction::RootWord {
                    target,
                    root_word_id: root_id.as_str().to_string(),
                },
            );
        }
    });
}

fn spawn_choice_row(
    parent: &mut ChildSpawnerCommands,
    spawn: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(5.0),
            row_gap: Val::Px(5.0),
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(spawn);
}

fn spawn_choice(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    label: &str,
    active: bool,
    enabled: bool,
    action: ItemConfigAction,
) {
    if enabled {
        spawn_bar_child(
            parent,
            label,
            CHOICE_FONT_SIZE,
            theme.button_text_color,
            Val::Px(CHOICE_WIDTH),
            Val::Px(CHOICE_HEIGHT),
            if active {
                BarButtonKind::Primary
            } else {
                BarButtonKind::Neutral
            },
            ItemConfigChoice(action),
        );
        return;
    }

    parent
        .spawn((
            Node {
                width: Val::Px(CHOICE_WIDTH),
                height: Val::Px(CHOICE_HEIGHT),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(5.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(MUTED_PANEL),
            BorderColor::all(if active { ACTIVE_BORDER } else { MUTED_BORDER }),
        ))
        .with_children(|choice| {
            choice.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(CHOICE_FONT_SIZE),
                    ..default()
                },
                TextColor(if active {
                    theme.text_color
                } else {
                    theme.muted_text_color
                }),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
            ));
        });
}

fn spawn_heading(parent: &mut ChildSpawnerCommands, theme: &UiTheme, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(theme.button_font_size * 0.72),
            ..default()
        },
        TextColor(theme.text_color),
    ));
}

fn spawn_label(parent: &mut ChildSpawnerCommands, theme: &UiTheme, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(theme.button_font_size * 0.58),
            ..default()
        },
        TextColor(theme.muted_text_color),
    ));
}

fn spawn_muted(parent: &mut ChildSpawnerCommands, theme: &UiTheme, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(theme.button_font_size * 0.58),
            ..default()
        },
        TextColor(theme.muted_text_color),
        TextLayout {
            linebreak: LineBreak::WordOrCharacter,
            ..default()
        },
    ));
}

fn spawn_notice(parent: &mut ChildSpawnerCommands, theme: &UiTheme, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(theme.button_font_size * 0.58),
            ..default()
        },
        TextColor(Color::srgba(0.95, 0.78, 0.35, 1.0)),
        TextLayout {
            linebreak: LineBreak::WordOrCharacter,
            ..default()
        },
    ));
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

fn rune_usage_weapon(inscription: &WeaponInscription, registries: ItemEditorRegistries<'_>) -> u32 {
    let root = inscription
        .root_word
        .as_ref()
        .and_then(|id| registries.root_words.get(id))
        .map(|word| word.metadata().rune_cost)
        .unwrap_or(0);
    root + AbilitySlot::ALL
        .iter()
        .flat_map(|&slot| inscription.get(slot).secondary_words.iter())
        .filter_map(|word| registries.ancient_words.get(&word.word_id))
        .map(|word| word.metadata().rune_cost)
        .sum::<u32>()
}

fn rune_usage_armor(inscription: &ArmorInscription, registries: ItemEditorRegistries<'_>) -> u32 {
    let root = inscription
        .root_word
        .as_ref()
        .and_then(|id| registries.root_words.get(id))
        .map(|word| word.metadata().rune_cost)
        .unwrap_or(0);
    root + inscription
        .secondary_words
        .iter()
        .filter_map(|word| registries.ancient_words.get(&word.word_id))
        .map(|word| word.metadata().rune_cost)
        .sum::<u32>()
}

const fn slot_label(slot: AbilitySlot) -> &'static str {
    match slot {
        AbilitySlot::Primary => "PRIMARY",
        AbilitySlot::Secondary => "SECONDARY",
        AbilitySlot::Ultimate => "ULTIMATE",
    }
}

const fn slot_index(slot: AbilitySlot) -> usize {
    match slot {
        AbilitySlot::Primary => 0,
        AbilitySlot::Secondary => 1,
        AbilitySlot::Ultimate => 2,
    }
}

fn max_words(slot: AbilitySlot) -> usize {
    match slot {
        AbilitySlot::Primary | AbilitySlot::Secondary => 2,
        AbilitySlot::Ultimate => 1,
    }
}

fn toggled_root(current: Option<&RootWordId>, clicked: &str) -> Option<RootWordId> {
    if current.map(RootWordId::as_str) == Some(clicked) {
        None
    } else {
        Some(RootWordId::new(clicked.to_string()))
    }
}

pub(super) fn handle_item_editor_tabs(
    mut state: ResMut<ItemDetailUiState>,
    interactions: Query<(&Interaction, &ItemAbilityTabButton), Changed<Interaction>>,
    mut panes: Query<(&ItemAbilityPane, &mut Node)>,
) {
    for (interaction, tab) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        state.active_slot = tab.slot;
        for (pane, mut node) in &mut panes {
            node.display = if pane.slot == tab.slot {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

pub(super) fn update_item_editor_tabs(
    state: Res<ItemDetailUiState>,
    asset_server: Res<AssetServer>,
    mut tabs: Query<(&ItemAbilityTabButton, &mut ImageNode, &mut UiButtonImages)>,
) {
    if !state.is_changed() {
        return;
    }
    for (tab, mut image, mut images) in &mut tabs {
        *images = UiButtonImages::load_kind(
            &asset_server,
            if tab.slot == state.active_slot {
                BarButtonKind::Primary
            } else {
                BarButtonKind::Neutral
            },
        );
        image.image = images.default.clone();
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_item_editor_choices(
    interactions: Query<(&Interaction, &ItemConfigChoice), Changed<Interaction>>,
    equipment_query: Query<&Equipment, With<bevymmo_client::local_player::LocalPlayer>>,
    item_registry: Res<bevymmo_gameplay::items::ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    ancient_word_registry: Res<AncientWordRegistry>,
    conn: Option<Res<StdbConnection>>,
) {
    let Ok(equipment) = equipment_query.single() else {
        return;
    };

    for (interaction, choice) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match &choice.0 {
            ItemConfigAction::RootWord {
                target,
                root_word_id,
            } => match target {
                InscriptionTarget::Weapon => {
                    let Some(weapon) = equipment.weapon.as_ref() else {
                        continue;
                    };
                    let mut inscription = weapon.root_inscription.clone().unwrap_or_default();
                    inscription.root_word =
                        toggled_root(inscription.root_word.as_ref(), root_word_id);
                    send_weapon_update(conn.as_deref(), &inscription);
                }
                InscriptionTarget::Armor(slot) => {
                    let Some(item) = equipment.get(*slot).as_ref() else {
                        continue;
                    };
                    let mut inscription = item.armor_inscription.clone().unwrap_or_default();
                    inscription.root_word =
                        toggled_root(inscription.root_word.as_ref(), root_word_id);
                    send_armor_update(conn.as_deref(), *slot, &inscription);
                }
            },
            ItemConfigAction::Ability { slot, ability_id } => {
                if let Some(conn) = conn.as_deref() {
                    if let Err(error) =
                        stdb_commands::set_ability_selection(conn, *slot, ability_id.clone())
                    {
                        error!("could not choose item ability: {error}");
                    }
                }
            }
            ItemConfigAction::AncientWord {
                target,
                slot,
                word_id,
            } => match target {
                InscriptionTarget::Weapon => {
                    let Some(weapon) = equipment.weapon.as_ref() else {
                        continue;
                    };
                    let word_id = AncientWordId::new(word_id.clone());
                    if !ancient_word_fits(
                        *slot,
                        &word_id,
                        weapon,
                        &item_registry,
                        &ability_registry,
                        &ancient_word_registry,
                    ) {
                        continue;
                    }
                    let mut inscription = weapon.root_inscription.clone().unwrap_or_default();
                    if toggle_secondary(inscription.get_mut(*slot), word_id, max_words(*slot)) {
                        send_weapon_update(conn.as_deref(), &inscription);
                    }
                }
                InscriptionTarget::Armor(equip_slot) => {
                    let Some(item) = equipment.get(*equip_slot).as_ref() else {
                        continue;
                    };
                    let word_id = AncientWordId::new(word_id.clone());
                    if !ancient_word_fits(
                        AbilitySlot::Primary,
                        &word_id,
                        item,
                        &item_registry,
                        &ability_registry,
                        &ancient_word_registry,
                    ) {
                        continue;
                    }
                    let mut inscription = item.armor_inscription.clone().unwrap_or_default();
                    if toggle_words(&mut inscription.secondary_words, word_id, 2) {
                        send_armor_update(conn.as_deref(), *equip_slot, &inscription);
                    }
                }
            },
        }
    }
}

fn toggle_secondary(
    slot: &mut bevymmo_gameplay::abilities::SlotInscription,
    word_id: AncientWordId,
    limit: usize,
) -> bool {
    toggle_words(&mut slot.secondary_words, word_id, limit)
}

fn toggle_words(words: &mut Vec<SecondaryWord>, word_id: AncientWordId, limit: usize) -> bool {
    if let Some(index) = words.iter().position(|word| word.word_id == word_id) {
        words.remove(index);
        true
    } else if words.len() < limit {
        words.push(SecondaryWord::new(word_id));
        true
    } else {
        false
    }
}

fn ancient_word_fits(
    slot: AbilitySlot,
    word_id: &AncientWordId,
    instance: &ItemInstance,
    item_registry: &bevymmo_gameplay::items::ItemRegistry,
    ability_registry: &BaseAbilityRegistry,
    ancient_word_registry: &AncientWordRegistry,
) -> bool {
    let Some(item) = item_registry.get(&instance.item_id) else {
        return false;
    };
    let Some(loadout) = item.ability_loadout() else {
        return false;
    };
    let Some(ability_id) = resolve_active_ability(slot, loadout, &instance.ability_selection)
    else {
        return false;
    };
    let Some(ability) = ability_registry.get(ability_id) else {
        return false;
    };
    ancient_word_registry
        .get(word_id)
        .is_some_and(|word| word.metadata().is_compatible_with(ability.tags()))
}

fn send_weapon_update(conn: Option<&StdbConnection>, inscription: &WeaponInscription) {
    let Some(conn) = conn else {
        return;
    };
    let words_for = |slot: &bevymmo_gameplay::abilities::SlotInscription| {
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
        error!("could not update item inscription: {error}");
    }
}

fn send_armor_update(
    conn: Option<&StdbConnection>,
    slot: EquipSlot,
    inscription: &ArmorInscription,
) {
    let Some(conn) = conn else {
        return;
    };
    if let Err(error) = stdb_commands::set_armor_inscription(
        conn,
        slot,
        inscription
            .root_word
            .as_ref()
            .map(|word| word.as_str().to_string()),
        inscription
            .secondary_words
            .iter()
            .map(|word| word.word_id.as_str().to_string())
            .collect(),
    ) {
        error!("could not update armor inscription: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicking_the_active_root_word_clears_it() {
        let current = RootWordId::new("flame");
        assert_eq!(toggled_root(Some(&current), "flame"), None);
    }

    #[test]
    fn weapon_word_limits_match_server_policy() {
        assert_eq!(max_words(AbilitySlot::Primary), 2);
        assert_eq!(max_words(AbilitySlot::Secondary), 2);
        assert_eq!(max_words(AbilitySlot::Ultimate), 1);
    }

    #[test]
    fn toggling_a_secondary_word_is_reversible() {
        let mut words = Vec::new();
        assert!(toggle_words(&mut words, AncientWordId::new("echo"), 2));
        assert_eq!(words.len(), 1);
        assert!(toggle_words(&mut words, AncientWordId::new("echo"), 2));
        assert!(words.is_empty());
    }

    #[test]
    fn secondary_word_limit_rejects_an_extra_choice() {
        let mut words = vec![
            SecondaryWord::new(AncientWordId::new("echo")),
            SecondaryWord::new(AncientWordId::new("twin")),
        ];
        assert!(!toggle_words(&mut words, AncientWordId::new("anchor"), 2));
        assert_eq!(words.len(), 2);
    }
}
