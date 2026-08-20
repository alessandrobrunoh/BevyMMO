use super::components::*;
use super::InscriptionUiState;
use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::stdb::{commands as stdb_commands, StdbConnection};
use bevymmo_gameplay::abilities::{
    inscription::{ArmorInscription, SecondaryWord, WeaponInscription},
    resolve_active_ability, AbilitySelection, AbilitySlot, AncientWordId, AncientWordRegistry,
    BaseAbilityRegistry, KnownAncientLanguage, RootWordId, RootWordRegistry, WeaponAbilities,
};
use bevymmo_gameplay::items::components::{EquipSlot, Equipment};
use bevymmo_gameplay::items::definition::Item;
use bevymmo_gameplay::items::registry::ItemRegistry;

use crate::ui::button::{spawn_bar_child, BarButtonKind};
use crate::ui::scrollbar::{spawn_scroll_view_scrolled, ScrollView};
use crate::ui::settings::state::{GameSettingsResource, KeyAction};
use crate::ui::theme::{ornate_panel_image, UiTheme};

const PANEL_PATH: &str = "ui/extracted_065811/panel_large_left.png";

const WINDOW_WIDTH: f32 = 760.0;
const WINDOW_HEIGHT: f32 = 520.0;
/// Matches the 9-slice gem inset on `panel_large_left`.
const WINDOW_PAD: f32 = 88.0;
const HEADER_HEIGHT: f32 = 34.0;
const HEADER_TITLE_SIZE: f32 = 20.0;
const HEADER_SUBTITLE_SIZE: f32 = 14.0;
const HEADER_GAP: f32 = 12.0;
const CLOSE_WIDTH: f32 = 92.0;
const CLOSE_HEIGHT: f32 = 30.0;
const CLOSE_FONT_SIZE: f32 = 12.0;
const TOGGLE_WIDTH: f32 = 88.0;
const TOGGLE_HEIGHT: f32 = 28.0;
const TOGGLE_FONT_SIZE: f32 = 11.0;
const SECTION_LABEL_SIZE: f32 = 14.0;
const SLOT_HEADING_SIZE: f32 = 13.0;
const ARMOR_SLOT_MIN_HEIGHT: f32 = 48.0;
const SLOT_PANEL_FILL: Color = Color::srgba(0.08, 0.09, 0.12, 0.85);

const _: () = assert!(WINDOW_PAD >= 88.0);
const _: () = assert!(HEADER_TITLE_SIZE < HEADER_HEIGHT);
const SLOTS: [AbilitySlot; 3] = [
    AbilitySlot::Primary,
    AbilitySlot::Secondary,
    AbilitySlot::Ultimate,
];

/// Presentation-only mnemonic matching the default weapon HUD keys.
fn slot_key_label(slot: AbilitySlot) -> &'static str {
    match slot {
        AbilitySlot::Primary => "1",
        AbilitySlot::Secondary => "2",
        AbilitySlot::Ultimate => "3",
    }
}

/// `true` when the currently equipped weapon has Eidolon gestures.
const ARMOR_INSCRIPTION_SLOTS: [EquipSlot; 3] =
    [EquipSlot::Helmet, EquipSlot::Armor, EquipSlot::Shoes];

fn equipped_weapon_is_eidolon(equipment: &Equipment, registry: &ItemRegistry) -> bool {
    equipment
        .weapon
        .as_ref()
        .and_then(|weapon| registry.get(&weapon.item_id))
        .is_some_and(|item| item.ability_loadout().is_some())
}

fn item_is_inscribable(item: &dyn Item) -> bool {
    item.ability_loadout().is_some() && item.rune_profile().is_some()
}

/// Whether the spellbook key should open this window.
pub(crate) fn owns_inscription_hotkey(equipment: &Equipment, registry: &ItemRegistry) -> bool {
    if equipped_weapon_is_eidolon(equipment, registry) {
        return true;
    }
    ARMOR_INSCRIPTION_SLOTS.iter().any(|slot| {
        equipment
            .get(*slot)
            .as_ref()
            .and_then(|instance| registry.get(&instance.item_id))
            .is_some_and(|item| item_is_inscribable(item.as_ref()))
    })
}

fn next_root_word(current: Option<&RootWordId>, clicked: &str) -> Option<RootWordId> {
    if current.map(|id| id.as_str()) == Some(clicked) {
        None
    } else {
        Some(RootWordId::new(clicked.to_string()))
    }
}

/// Title stays the short word "Inscriptions"; the weapon name is a subtitle
/// so it cannot sit on the top gems next to Close.
fn header_copy(weapon_name: Option<&str>) -> (String, Option<String>) {
    let subtitle = weapon_name
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    ("Inscriptions".to_string(), subtitle)
}

fn armor_slot_title(slot: EquipSlot) -> &'static str {
    match slot {
        EquipSlot::Helmet => "Helmet",
        EquipSlot::Armor => "Armor",
        EquipSlot::Shoes => "Shoes",
        _ => slot.label(),
    }
}

fn spawn_section_label(parent: &mut ChildSpawnerCommands, theme: &UiTheme, label: &str) {
    parent.spawn((
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(SECTION_LABEL_SIZE),
            ..default()
        },
        TextColor(theme.muted_text_color),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
    ));
}

fn spawn_ornate_toggle(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    label: &str,
    is_active: bool,
    marker: impl Component,
) {
    let kind = if is_active {
        BarButtonKind::Primary
    } else {
        BarButtonKind::Neutral
    };
    spawn_bar_child(
        parent,
        label,
        TOGGLE_FONT_SIZE,
        theme.button_text_color,
        Val::Px(TOGGLE_WIDTH),
        Val::Px(TOGGLE_HEIGHT),
        kind,
        marker,
    );
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
    asset_server: Res<AssetServer>,
) {
    if !settings.just_pressed(KeyAction::ToggleSpellbook, &keys) {
        return;
    }

    let Ok((equipment, known)) = player_query.single() else {
        return;
    };
    if !owns_inscription_hotkey(equipment, &item_registry) {
        return;
    }

    state.is_open = !state.is_open;

    if !state.is_open {
        state.scroll = 0.0;
        state.shown_equipment = None;
        despawn_windows(&mut commands, &window_query);
        return;
    }

    state.scroll = 0.0;
    state.shown_equipment = Some(equipment.clone());
    spawn_window(
        &mut commands,
        &theme,
        equipment,
        known,
        &item_registry,
        &ability_registry,
        &root_word_registry,
        &ancient_word_registry,
        &asset_server,
        0.0,
    );
}

/// Rebuilds the window whenever the controlled player's `Equipment` changes
/// (weapon swap, or the server replicating back an inscription this UI just
/// requested) — covers both cases without any local prediction of
/// `ItemInstance.inscriptions`.
#[allow(clippy::too_many_arguments)]
pub fn refresh_inscription_window_on_equipment_change(
    mut commands: Commands,
    mut state: ResMut<InscriptionUiState>,
    window_query: Query<Entity, With<InscriptionWindow>>,
    children: Query<&Children>,
    scroll_views: Query<&ScrollView>,
    theme: Res<UiTheme>,
    item_registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    root_word_registry: Res<RootWordRegistry>,
    ancient_word_registry: Res<AncientWordRegistry>,
    player_query: Query<
        (&Equipment, &KnownAncientLanguage),
        (With<LocalPlayer>, Changed<Equipment>),
    >,
    asset_server: Res<AssetServer>,
) {
    if !state.is_open {
        return;
    }
    let Ok((equipment, known)) = player_query.single() else {
        return;
    };
    if state.shown_equipment.as_ref() == Some(equipment) {
        return;
    }

    state.scroll = window_query
        .iter()
        .map(|root| descendant_scroll(root, &children, &scroll_views))
        .fold(state.scroll, f32::max);

    despawn_windows(&mut commands, &window_query);

    if !owns_inscription_hotkey(equipment, &item_registry) {
        state.shown_equipment = None;
        return;
    }

    state.shown_equipment = Some(equipment.clone());
    spawn_window(
        &mut commands,
        &theme,
        equipment,
        known,
        &item_registry,
        &ability_registry,
        &root_word_registry,
        &ancient_word_registry,
        &asset_server,
        state.scroll,
    );
}

fn descendant_scroll(
    root: Entity,
    children: &Query<&Children>,
    scroll_views: &Query<&ScrollView>,
) -> f32 {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Ok(view) = scroll_views.get(entity) {
            return view.current_scroll;
        }
        if let Ok(child_list) = children.get(entity) {
            stack.extend(child_list.iter());
        }
    }
    0.0
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
    asset_server: &AssetServer,
    initial_scroll: f32,
) {
    let weapon = equipment.weapon.as_ref();
    let weapon_item = weapon.and_then(|instance| item_registry.get(&instance.item_id));
    let weapon_abilities = weapon_item.as_ref().and_then(|item| item.ability_loadout());
    let inscription = weapon
        .and_then(|instance| instance.root_inscription.clone())
        .unwrap_or_default();
    let weapon_name = weapon_item.as_ref().map(|item| item.display_name());

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
                padding: UiRect::all(Val::Px(WINDOW_PAD)),
                row_gap: Val::Px(10.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Button,
            InscriptionWindow,
        ))
        .id();
    commands.entity(window).with_children(|window| {
        window.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            ornate_panel_image(asset_server.load(PANEL_PATH)),
            Pickable::IGNORE,
        ));
    });

    commands.entity(window).with_children(|parent| {
        spawn_header(parent, theme, weapon_name);
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

    spawn_scroll_view_scrolled(commands, scroll_body, theme, initial_scroll, |commands| {
        commands
            .spawn((Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },))
            .with_children(|body| {
                spawn_armor_inscription_section(
                    body,
                    theme,
                    equipment,
                    known,
                    item_registry,
                    root_word_registry,
                );
                if weapon_abilities.is_some() {
                    spawn_root_word_section(body, theme, known, &inscription, root_word_registry);
                }
                let Some(weapon_abilities) = weapon_abilities else {
                    return;
                };
                let Some(weapon) = weapon else {
                    return;
                };
                body.spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(12.0),
                    row_gap: Val::Px(8.0),
                    min_width: Val::Px(0.0),
                    overflow: Overflow::clip(),
                    ..default()
                },))
                    .with_children(|row| {
                        for slot in SLOTS {
                            spawn_slot_column(
                                row,
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
                    });
            })
            .id()
    });
}

fn spawn_armor_inscription_section(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    equipment: &Equipment,
    known: &KnownAncientLanguage,
    item_registry: &ItemRegistry,
    root_word_registry: &RootWordRegistry,
) {
    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::bottom(Val::Px(6.0)),
            ..default()
        },))
        .with_children(|section| {
            spawn_section_label(section, theme, "Armor");
            section
                .spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    min_width: Val::Px(0.0),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },))
                .with_children(|row| {
                    for slot in ARMOR_INSCRIPTION_SLOTS {
                        spawn_armor_slot_card(
                            row,
                            theme,
                            slot,
                            equipment,
                            known,
                            item_registry,
                            root_word_registry,
                        );
                    }
                });
        });
}

fn spawn_armor_slot_card(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    slot: EquipSlot,
    equipment: &Equipment,
    known: &KnownAncientLanguage,
    item_registry: &ItemRegistry,
    root_word_registry: &RootWordRegistry,
) {
    let instance = equipment.get(slot).as_ref();
    let item = instance.and_then(|value| item_registry.get(&value.item_id));
    let filled = item.is_some();

    let mut card = parent.spawn(Node {
        flex_grow: if filled { 1.0 } else { 0.0 },
        flex_shrink: 1.0,
        flex_basis: if filled { Val::Px(0.0) } else { Val::Auto },
        min_width: Val::Px(0.0),
        min_height: Val::Px(if filled { ARMOR_SLOT_MIN_HEIGHT } else { 20.0 }),
        padding: UiRect::all(Val::Px(if filled { 6.0 } else { 2.0 })),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(3.0),
        overflow: Overflow::clip(),
        border_radius: BorderRadius::all(Val::Px(6.0)),
        ..default()
    });
    if filled {
        card.insert(BackgroundColor(SLOT_PANEL_FILL));
    }
    card.with_children(|card| {
        let Some(item) = item else {
            card.spawn((Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                align_items: AlignItems::Baseline,
                min_width: Val::Px(0.0),
                overflow: Overflow::clip_x(),
                ..default()
            },))
                .with_children(|line| {
                    line.spawn((
                        Text::new(armor_slot_title(slot)),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(theme.text_color),
                        TextLayout {
                            linebreak: LineBreak::NoWrap,
                            ..default()
                        },
                    ));
                    spawn_muted_line(line, theme, "empty");
                });
            return;
        };

        card.spawn((
            Text::new(armor_slot_title(slot)),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme.muted_text_color),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
        ));
        card.spawn((
            Text::new(item.display_name().to_string()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme.text_color),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            Node {
                min_width: Val::Px(0.0),
                overflow: Overflow::clip_x(),
                ..default()
            },
        ));

        if !item_is_inscribable(item.as_ref()) {
            spawn_muted_line(card, theme, "not inscribable");
            return;
        }

        let current = instance.and_then(|value| value.armor_inscription.as_ref());
        if known.root_words.is_empty() {
            spawn_muted_line(card, theme, "none");
            return;
        }

        card.spawn((Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            min_width: Val::Px(0.0),
            ..default()
        },))
            .with_children(|row| {
                for root_id in sorted_root_words(known, root_word_registry) {
                    let Some(root) = root_word_registry.get(root_id) else {
                        continue;
                    };
                    let is_active =
                        current.and_then(|value| value.root_word.as_ref()) == Some(root_id);
                    spawn_ornate_toggle(
                        row,
                        theme,
                        root.metadata().display_name,
                        is_active,
                        ArmorRootWordToggleButton {
                            slot,
                            root_word_id: root_id.as_str().to_string(),
                        },
                    );
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
            spawn_section_label(section, theme, "Weapon");

            if known.root_words.is_empty() {
                spawn_muted_line(section, theme, "none");
                return;
            }

            section
                .spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    min_width: Val::Px(0.0),
                    ..default()
                },))
                .with_children(|row| {
                    for root_id in sorted_root_words(known, registry) {
                        let Some(root) = registry.get(root_id) else {
                            continue;
                        };
                        let is_active = inscription.root_word.as_ref() == Some(root_id);
                        spawn_ornate_toggle(
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

fn spawn_header(parent: &mut ChildSpawnerCommands, theme: &UiTheme, weapon_name: Option<&str>) {
    let (title, subtitle) = header_copy(weapon_name);
    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            min_width: Val::Px(0.0),
            ..default()
        },))
        .with_children(|chrome| {
            chrome
                .spawn((Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(HEADER_HEIGHT),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(HEADER_GAP),
                    min_width: Val::Px(0.0),
                    overflow: Overflow::clip(),
                    ..default()
                },))
                .with_children(|header| {
                    header.spawn((
                        Text::new(title),
                        TextFont {
                            font_size: FontSize::Px(HEADER_TITLE_SIZE),
                            ..default()
                        },
                        TextColor(theme.text_color),
                        TextLayout {
                            linebreak: LineBreak::NoWrap,
                            ..default()
                        },
                        Node {
                            flex_shrink: 1.0,
                            min_width: Val::Px(0.0),
                            overflow: Overflow::clip_x(),
                            ..default()
                        },
                    ));

                    spawn_bar_child(
                        header,
                        "Close",
                        CLOSE_FONT_SIZE,
                        theme.button_text_color,
                        Val::Px(CLOSE_WIDTH),
                        Val::Px(CLOSE_HEIGHT),
                        BarButtonKind::Neutral,
                        CloseInscriptionButton,
                    );
                });

            if let Some(subtitle) = subtitle {
                chrome.spawn((
                    Text::new(subtitle),
                    TextFont {
                        font_size: FontSize::Px(HEADER_SUBTITLE_SIZE),
                        ..default()
                    },
                    TextColor(theme.muted_text_color),
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                        ..default()
                    },
                    Node {
                        min_width: Val::Px(0.0),
                        overflow: Overflow::clip_x(),
                        ..default()
                    },
                ));
            }
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
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Val::Px(0.0),
            min_width: Val::Px(0.0),
            row_gap: Val::Px(4.0),
            overflow: Overflow::clip(),
            ..default()
        },))
        .with_children(|column| {
            column.spawn((
                Text::new(format!(
                    "{} - {}",
                    slot_key_label(slot),
                    ability.display_name()
                )),
                TextFont {
                    font_size: FontSize::Px(SLOT_HEADING_SIZE),
                    ..default()
                },
                TextColor(theme.text_color),
                TextLayout {
                    linebreak: LineBreak::WordOrCharacter,
                    ..default()
                },
                Node {
                    min_width: Val::Px(0.0),
                    flex_shrink: 1.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
            ));

            // Only worth a picker when the weapon actually offers a choice.
            let options = weapon_abilities.options_for(slot);
            if options.len() > 1 {
                spawn_section_label(column, theme, "Gesture");
                column
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(4.0),
                        row_gap: Val::Px(4.0),
                        min_width: Val::Px(0.0),
                        ..default()
                    },))
                    .with_children(|row| {
                        for option_id in options {
                            let Some(option) = ability_registry.get(option_id) else {
                                continue;
                            };
                            spawn_ornate_toggle(
                                row,
                                theme,
                                option.display_name(),
                                option_id == ability_id,
                                AbilitySelectButton {
                                    slot,
                                    ability_id: option_id.as_str().to_string(),
                                },
                            );
                        }
                    });
            }

            spawn_section_label(column, theme, "Ancient Words");
            if known.ancient_words.is_empty() {
                spawn_muted_line(column, theme, "none");
            } else {
                column
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(4.0),
                        row_gap: Val::Px(4.0),
                        min_width: Val::Px(0.0),
                        ..default()
                    },))
                    .with_children(|row| {
                        for word_id in sorted_ancient_words(known, ancient_word_registry) {
                            let Some(word) = ancient_word_registry.get(word_id) else {
                                continue;
                            };
                            if !word.metadata().is_compatible_with(ability.tags()) {
                                spawn_muted_line(row, theme, word.display_name());
                                continue;
                            }
                            let is_active = slot_ins
                                .secondary_words
                                .iter()
                                .any(|w| w.word_id == *word_id);
                            spawn_ornate_toggle(
                                row,
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
        });
}

fn spawn_muted_line(parent: &mut ChildSpawnerCommands, theme: &UiTheme, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(theme.muted_text_color),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
    ));
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
    armor_root_interactions: Query<
        (&Interaction, &ArmorRootWordToggleButton),
        (Changed<Interaction>, With<Button>),
    >,
    ability_interactions: Query<
        (&Interaction, &AbilitySelectButton),
        (Changed<Interaction>, With<Button>),
    >,
    close_interactions: Query<&Interaction, (Changed<Interaction>, With<CloseInscriptionButton>)>,
    player_query: Query<&Equipment, With<LocalPlayer>>,
    item_registry: Res<ItemRegistry>,
    ability_registry: Res<BaseAbilityRegistry>,
    ancient_word_registry: Res<AncientWordRegistry>,
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
    let weapon = equipment.weapon.as_ref();
    let current = weapon
        .and_then(|instance| instance.root_inscription.clone())
        .unwrap_or_default();

    for (interaction, toggle) in armor_root_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(instance) = equipment.get(toggle.slot).as_ref() else {
            continue;
        };
        let current_armor = instance.armor_inscription.clone().unwrap_or_default();
        let new_root = next_root_word(current_armor.root_word.as_ref(), &toggle.root_word_id);
        send_armor_root_update(conn.as_deref(), toggle.slot, new_root, &current_armor);
    }

    let Some(weapon) = weapon else {
        return;
    };

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
        let new_root = next_root_word(current.root_word.as_ref(), &toggle.root_word_id);
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
            if !ancient_word_fits_weapon_slot(
                toggle.slot,
                &word_id,
                weapon,
                &item_registry,
                &ability_registry,
                &ancient_word_registry,
            ) {
                continue;
            }
            slot_ins.secondary_words.push(SecondaryWord::new(word_id));
        }
        send_full_update(conn.as_deref(), &updated);
    }
}

fn send_armor_root_update(
    conn: Option<&StdbConnection>,
    slot: EquipSlot,
    root_word: Option<RootWordId>,
    current: &ArmorInscription,
) {
    let Some(conn) = conn else {
        return;
    };
    let secondary_words = current
        .secondary_words
        .iter()
        .map(|word| word.word_id.as_str().to_string())
        .collect();
    if let Err(error) = stdb_commands::set_armor_inscription(
        conn,
        slot,
        root_word.map(|word| word.as_str().to_string()),
        secondary_words,
    ) {
        error!("could not update armor Root Word: {error}");
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

fn ancient_word_fits_weapon_slot(
    slot: AbilitySlot,
    word_id: &AncientWordId,
    weapon: &bevymmo_gameplay::items::instance::ItemInstance,
    item_registry: &ItemRegistry,
    ability_registry: &BaseAbilityRegistry,
    ancient_word_registry: &AncientWordRegistry,
) -> bool {
    let Some(item) = item_registry.get(&weapon.item_id) else {
        return false;
    };
    let Some(loadout) = item.ability_loadout() else {
        return false;
    };
    let Some(ability_id) = resolve_active_ability(slot, loadout, &weapon.ability_selection) else {
        return false;
    };
    let Some(ability) = ability_registry.get(ability_id) else {
        return false;
    };
    let Some(word) = ancient_word_registry.get(word_id) else {
        return false;
    };
    word.metadata().is_compatible_with(ability.tags())
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
    fn next_root_word_toggles_off_the_same_pick() {
        let current = RootWordId::new("flame");
        assert_eq!(next_root_word(Some(&current), "flame"), None);
        assert_eq!(
            next_root_word(Some(&current), "stone").map(|id| id.as_str().to_string()),
            Some("stone".to_string())
        );
        assert_eq!(
            next_root_word(None, "flame").map(|id| id.as_str().to_string()),
            Some("flame".to_string())
        );
    }

    #[test]
    fn inscription_hotkey_opens_for_inscribable_armor_without_a_weapon() {
        let registry = bevymmo_content::item_definitions::default_items();
        let mut equipment = Equipment::default();
        assert!(!owns_inscription_hotkey(&equipment, &registry));

        equipment.helmet = Some(bevymmo_gameplay::items::instance::ItemInstance::new(
            bevymmo_gameplay::items::registry::ItemId::new("simple_helm"),
        ));
        assert!(owns_inscription_hotkey(&equipment, &registry));
    }

    #[test]
    fn staff_secondary_only_toggles_compatible_ancient_words() {
        let mut app = test_app();
        let (mut equipment, mut known) = staff_and_boots();
        known.ancient_words = ["anchor", "echo", "hunger", "return", "reversal", "twin"]
            .into_iter()
            .map(AncientWordId::new)
            .collect();
        equipment
            .weapon
            .as_mut()
            .expect("staff")
            .ability_selection
            .secondary = Some(bevymmo_gameplay::abilities::AbilityId::new("arcane_wave"));
        spawn_test_window(&mut app, &equipment, &known);

        let world = app.world_mut();
        let mut toggles = world.query::<&AncientWordToggleButton>();
        let secondary: Vec<&str> = toggles
            .iter(world)
            .filter(|toggle| toggle.slot == AbilitySlot::Secondary)
            .map(|toggle| toggle.word_id.as_str())
            .collect();
        assert!(
            secondary.contains(&"reversal"),
            "Ranged words must stay clickable on Arcane Wave, got {secondary:?}"
        );
        assert!(
            secondary.contains(&"echo"),
            "Echo should apply to a ranged area wave, got {secondary:?}"
        );
        assert!(
            !secondary.contains(&"return"),
            "Return is projectile-only and must not be clickable on Arcane Wave, got {secondary:?}"
        );
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

    #[test]
    fn long_weapon_names_become_a_subtitle() {
        let long = "X".repeat(80);
        let (title, subtitle) = header_copy(Some(&long));
        assert_eq!(title, "Inscriptions");
        assert_eq!(subtitle.as_deref(), Some(long.as_str()));
    }

    #[test]
    fn short_weapon_names_stay_in_the_title() {
        let (title, subtitle) = header_copy(Some("Bow"));
        assert_eq!(title, "Inscriptions");
        assert_eq!(subtitle.as_deref(), Some("Bow"));
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_resource::<UiTheme>();
        app
    }

    fn spawn_test_window(app: &mut App, equipment: &Equipment, known: &KnownAncientLanguage) {
        let theme = UiTheme::default();
        let item_registry = bevymmo_content::item_definitions::default_items();
        let ability_registry = bevymmo_content::ability_definitions::default_base_abilities();
        let root_word_registry = bevymmo_content::root_word_definitions::default_root_words();
        let ancient_word_registry =
            bevymmo_content::ancient_word_definitions::default_ancient_words();
        let asset_server = app.world().resource::<AssetServer>().clone();
        {
            let mut commands = app.world_mut().commands();
            spawn_window(
                &mut commands,
                &theme,
                equipment,
                known,
                &item_registry,
                &ability_registry,
                &root_word_registry,
                &ancient_word_registry,
                &asset_server,
                0.0,
            );
        }
        app.world_mut().flush();
    }

    fn collected_text(app: &mut App) -> Vec<String> {
        let world = app.world_mut();
        let mut query = world.query::<&Text>();
        query.iter(world).map(|text| text.0.clone()).collect()
    }

    fn staff_and_boots() -> (Equipment, KnownAncientLanguage) {
        let equipment = Equipment {
            weapon: Some(bevymmo_gameplay::items::instance::ItemInstance::new(
                bevymmo_gameplay::items::registry::ItemId::new("mage_staff"),
            )),
            shoes: Some(bevymmo_gameplay::items::instance::ItemInstance::new(
                bevymmo_gameplay::items::registry::ItemId::new("simple_boots"),
            )),
            ..Default::default()
        };
        let known = KnownAncientLanguage {
            root_words: ["flame", "stone"]
                .into_iter()
                .map(RootWordId::from)
                .collect(),
            ..default()
        };
        (equipment, known)
    }

    #[test]
    fn window_pad_clears_the_nine_slice_gems() {
        let mut app = test_app();
        let (equipment, known) = staff_and_boots();
        spawn_test_window(&mut app, &equipment, &known);

        let world = app.world_mut();
        let window = world
            .query_filtered::<(Entity, &Node), With<InscriptionWindow>>()
            .iter(world)
            .next()
            .map(|(entity, node)| (entity, node.clone()))
            .expect("window");
        assert_eq!(window.1.padding.left, Val::Px(WINDOW_PAD));
        assert_eq!(window.1.padding.top, Val::Px(WINDOW_PAD));
        assert_eq!(window.1.overflow, Overflow::clip());

        let children = world.get::<Children>(window.0).expect("frame child");
        let frame = children[0];
        let frame_node = world.get::<Node>(frame).expect("frame node");
        assert_eq!(frame_node.position_type, PositionType::Absolute);
        assert!(world.get::<Pickable>(frame).is_some());
    }

    #[test]
    fn header_uses_compact_title_and_ornate_close() {
        let mut app = test_app();
        let (equipment, known) = staff_and_boots();
        spawn_test_window(&mut app, &equipment, &known);

        let texts = collected_text(&mut app);
        assert!(
            texts.iter().any(|text| text.starts_with("Inscriptions")),
            "expected Inscriptions title, got {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text.contains("Staffa da Mago")),
            "weapon name should appear in the header chrome, got {texts:?}"
        );
        assert!(
            !texts.iter().any(|text| {
                text.contains("Armor Root Words")
                    || text.contains("Helmet, chest and boots")
                    || text.contains("Root Word condivisa")
                    || text.contains("Una sola parola")
            }),
            "instructional copy must be gone, got {texts:?}"
        );

        let world = app.world_mut();
        let mut titles = world.query::<(&Text, &TextFont, &TextLayout, &Node)>();
        let (_, font, layout, node) = titles
            .iter(world)
            .find(|(text, _, _, _)| text.0.starts_with("Inscriptions"))
            .expect("title");
        assert_eq!(font.font_size, FontSize::Px(HEADER_TITLE_SIZE));
        assert_eq!(layout.linebreak, LineBreak::NoWrap);
        assert_eq!(node.flex_shrink, 1.0);

        let mut close = world.query::<(
            &CloseInscriptionButton,
            &Node,
            &ImageNode,
            &crate::ui::button::UiButtonImages,
        )>();
        let (_, node, image, _) = close.iter(world).next().expect("close button");
        assert_eq!(node.width, Val::Px(CLOSE_WIDTH));
        assert_eq!(node.height, Val::Px(CLOSE_HEIGHT));
        assert_eq!(node.flex_shrink, 0.0);
        assert!(matches!(image.image_mode, NodeImageMode::Sliced(_)));
    }

    #[test]
    fn root_word_toggles_use_ornate_bars() {
        let mut app = test_app();
        let (equipment, known) = staff_and_boots();
        spawn_test_window(&mut app, &equipment, &known);

        let world = app.world_mut();
        let mut toggles = world.query::<(
            &RootWordToggleButton,
            &Node,
            &ImageNode,
            &crate::ui::button::UiButtonImages,
        )>();
        let count = toggles.iter(world).count();
        assert!(count >= 2, "expected weapon root-word bars, found {count}");
        for (_, node, image, _) in toggles.iter(world) {
            assert_eq!(node.width, Val::Px(TOGGLE_WIDTH));
            assert_eq!(node.height, Val::Px(TOGGLE_HEIGHT));
            assert!(matches!(image.image_mode, NodeImageMode::Sliced(_)));
        }

        let mut armor = world.query::<(
            &ArmorRootWordToggleButton,
            &ImageNode,
            &crate::ui::button::UiButtonImages,
        )>();
        assert!(
            armor.iter(world).next().is_some(),
            "boots should expose ornate root-word bars"
        );
    }

    #[test]
    fn empty_armor_slots_are_compact_labels() {
        let mut app = test_app();
        let (equipment, known) = staff_and_boots();
        spawn_test_window(&mut app, &equipment, &known);

        let texts = collected_text(&mut app);
        assert!(texts.iter().any(|text| text == "Helmet"));
        assert!(texts.iter().any(|text| text == "empty"));
        assert!(texts.iter().any(|text| text == "Shoes"));
        assert!(texts.iter().any(|text| text == "Simple Boots"));
        assert!(texts.iter().any(|text| text == "Armor"));
        assert!(texts.iter().any(|text| text == "Weapon"));
    }
}
