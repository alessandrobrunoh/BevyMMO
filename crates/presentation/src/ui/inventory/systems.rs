//! Systems for Inventory UI rendering, input handling, and server communication.

use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::stdb::{commands as stdb_commands, StdbConnection};
use bevymmo_gameplay::items::{
    components::{EquipSlot, Equipment, Inventory, INVENTORY_CAPACITY},
    registry::ItemRegistry,
};

use bevymmo_gameplay::abilities::KnownAncientLanguage;

use super::weapon_detail::GlyphRegistries;
use super::{components::*, detail::*, InventoryUiState};
use crate::ui::{
    card::{
        builder::{CardBuilder, CardFrameAssets},
        components::{CardKind, CardPositioning, CardWindow},
    },
    settings::state::{GameSettingsResource, KeyAction},
    theme::UiTheme,
};

// Width is chosen so 5 columns + gaps + the visible scrollbar still sit
// inside the ornate frame's inner dark area. Height is `Auto`: the card
// docks between the top edge and the hotbar (see `CardBuilder`).
const INVENTORY_CARD_WIDTH: f32 = 448.0;
const INVENTORY_GRID_COLUMNS: u16 = 5;
const INVENTORY_GRID_ROWS: u16 =
    INVENTORY_CAPACITY.div_ceil(INVENTORY_GRID_COLUMNS as usize) as u16;
const INVENTORY_SLOT_WIDTH: f32 = 48.0;
const INVENTORY_SLOT_HEIGHT: f32 = 50.0;
const INVENTORY_SLOT_GAP: f32 = 4.0;
const EQUIP_SLOT_SIZE: f32 = 44.0;
/// Column track is wider than the box so captions like `OFFHAND` still fit.
const EQUIP_CELL_WIDTH: f32 = 52.0;
const EQUIP_GRID_GAP: f32 = 6.0;
const MAIN_COLUMN_GAP: f32 = 4.0;
/// Extra inset on top of `CardBuilder`'s frame padding so slots stay off the
/// gold ornaments when that padding is tight against the 9-slice corners.
const INNER_CONTENT_PADDING: f32 = 16.0;
const INVENTORY_GRID_WIDTH: f32 = INVENTORY_GRID_COLUMNS as f32 * INVENTORY_SLOT_WIDTH
    + (INVENTORY_GRID_COLUMNS - 1) as f32 * INVENTORY_SLOT_GAP;
// 448 − ~2×64 frame inset − ~20 px scrollbar. Inner padding is extra to that.
const _: () = assert!(INVENTORY_GRID_WIDTH <= INVENTORY_CARD_WIDTH - 128.0 - 20.0);
/// Shown in an empty inventory / equipment cell.
///
/// ASCII on purpose: Bevy's built-in font is an ASCII subset, so an em dash
/// renders as a blank box rather than a dash.
const EMPTY_SLOT_PLACEHOLDER: &str = "-";
const SLOT_EMPTY_PATH: &str = "ui/extracted_065811/slot_empty_01.png";
const SLOT_ACTIVE_PATH: &str = "ui/extracted_065811/slot_active.png";

pub fn toggle_inventory(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    mut state: ResMut<InventoryUiState>,
    mut commands: Commands,
    window_query: Query<(Entity, &CardWindow)>,
    theme: Res<UiTheme>,
    registry: Res<ItemRegistry>,
    player_query: Query<(&Inventory, &Equipment), With<LocalPlayer>>,
    asset_server: Res<AssetServer>,
) {
    if !settings.just_pressed(KeyAction::ToggleInventory, &keys) {
        return;
    }

    state.is_open = !state.is_open;
    state.selected = None;

    if !state.is_open {
        despawn_inventory_cards(&mut commands, &window_query);
        return;
    }

    let (inventory, equipment) = player_query
        .iter()
        .next()
        .map(|(i, e)| (i.clone(), e.clone()))
        .unwrap_or_default();

    spawn_inventory_window(
        &mut commands,
        &theme,
        &registry,
        &inventory,
        &equipment,
        &asset_server,
    );
}

/// Resolves the display label for an equip slot's box: the equipped item's
/// name, or a placeholder dash when empty.
fn equip_slot_label(equipment: &Equipment, registry: &ItemRegistry, slot: EquipSlot) -> String {
    equipment
        .get(slot)
        .as_ref()
        .and_then(|instance| registry.get(&instance.item_id))
        .map(|item| item.display_name().to_string())
        .unwrap_or_else(|| EMPTY_SLOT_PLACEHOLDER.to_string())
}

/// Spawns one equipment slot cell: a small caption above a bordered box.
/// Shared by the 3x3 body-slot grid and the standalone Mount row.
fn spawn_equip_slot_cell(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    slot: EquipSlot,
    label: String,
    images: &InventorySlotImages,
) {
    let has_item = label != EMPTY_SLOT_PLACEHOLDER;

    parent
        .spawn((Node {
            width: Val::Px(EQUIP_CELL_WIDTH),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            ..default()
        },))
        .with_children(|cell| {
            cell.spawn((
                Text::new(slot.label().to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.55),
                    ..default()
                },
                TextColor(Color::srgba(0.75, 0.78, 0.85, 0.85)),
            ));

            cell.spawn((
                Button,
                Node {
                    width: Val::Px(EQUIP_SLOT_SIZE),
                    height: Val::Px(EQUIP_SLOT_SIZE),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(2.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                ImageNode::new(if has_item {
                    images.active.clone()
                } else {
                    images.empty.clone()
                })
                .with_mode(NodeImageMode::Stretch),
                InventorySlotImages {
                    empty: images.empty.clone(),
                    active: images.active.clone(),
                },
                EquipSlotButton { slot },
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(theme.button_font_size * 0.48),
                        ..default()
                    },
                    TextColor(theme.text_color),
                    TextLayout::justify(Justify::Center),
                    EquipSlotText { slot },
                ));
            });
        });
}

fn spawn_inventory_window(
    commands: &mut Commands,
    theme: &UiTheme,
    registry: &ItemRegistry,
    inventory: &Inventory,
    equipment: &Equipment,
    asset_server: &AssetServer,
) {
    let equipment = equipment.clone();
    let inventory = inventory.clone();
    let registry_snapshot = registry;
    // Body-slot grid (everything but Mount, which gets its own centered row).
    let grid_slots: Vec<EquipSlot> = EquipSlot::ALL[..9].to_vec();
    let mount_slot = EquipSlot::ALL[9];

    let grid_labels: Vec<(EquipSlot, String)> = grid_slots
        .iter()
        .map(|s| (*s, equip_slot_label(&equipment, registry_snapshot, *s)))
        .collect();
    let mount_label = equip_slot_label(&equipment, registry_snapshot, mount_slot);

    let item_labels: Vec<String> = (0..INVENTORY_CAPACITY)
        .map(|idx| {
            inventory
                .slots
                .get(idx)
                .and_then(|opt| opt.as_ref())
                .and_then(|instance| registry_snapshot.get(&instance.item_id))
                .map(|item| item.display_name().to_string())
                .unwrap_or_default()
        })
        .collect();
    let slot_images = InventorySlotImages {
        empty: asset_server.load(SLOT_EMPTY_PATH),
        active: asset_server.load(SLOT_ACTIVE_PATH),
    };
    CardBuilder::new(CardKind::Inventory, "Inventory")
        .frame(CardFrameAssets::load(asset_server))
        .headerless()
        .width(Val::Px(INVENTORY_CARD_WIDTH))
        .height(Val::Auto)
        .positioning(CardPositioning::Right)
        .scrollable()
        .exclusive()
        .with_body(move |body| {
            // Inner spacer: extra padding so slots stay off the gold frame
            // even when CardBuilder's inset is tight against the ornaments.
            body.spawn((Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(INNER_CONTENT_PADDING)),
                row_gap: Val::Px(MAIN_COLUMN_GAP),
                ..default()
            },))
                .with_children(|main| {
                    // 3x3 equipment grid.
                    //
                    // Rows are `auto`, not `flex`: `flex()` tracks are
                    // `minmax(0, Nfr)`, and inside the scrollable body the
                    // grid's height is indefinite (it sizes to its own
                    // content), so there is no space to distribute the `fr`
                    // against — every row collapsed to its zero minimum and
                    // all three rows of captions/boxes drew stacked on top of
                    // each other. `auto` sizes each row to its content
                    // instead, which works regardless of the container's
                    // height being definite or not.
                    main.spawn((
                        Node {
                            display: Display::Grid,
                            grid_template_columns: RepeatedGridTrack::px(3, EQUIP_CELL_WIDTH),
                            grid_template_rows: RepeatedGridTrack::auto(3),
                            justify_content: JustifyContent::Center,
                            row_gap: Val::Px(EQUIP_GRID_GAP),
                            column_gap: Val::Px(EQUIP_GRID_GAP),
                            ..default()
                        },
                        EquipmentPanel,
                    ))
                    .with_children(|grid| {
                        for (slot, label) in grid_labels {
                            spawn_equip_slot_cell(grid, theme, slot, label, &slot_images);
                        }
                    });

                    // Mount: standalone, centered below the 3x3 grid.
                    main.spawn((Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },))
                        .with_children(|row| {
                            spawn_equip_slot_cell(
                                row,
                                theme,
                                mount_slot,
                                mount_label,
                                &slot_images,
                            );
                        });

                    // Divider.
                    main.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                    ));

                    // Generic inventory grid. The card body scrolls when all
                    // rows exceed the available height.
                    main.spawn((
                        Node {
                            display: Display::Grid,
                            grid_template_columns: RepeatedGridTrack::px(
                                INVENTORY_GRID_COLUMNS,
                                INVENTORY_SLOT_WIDTH,
                            ),
                            // Item names can wrap to two lines. Fixed tracks keep
                            // each row tall enough and make the grid expand inside
                            // the scroll view instead of compressing its contents.
                            grid_template_rows: RepeatedGridTrack::px(
                                INVENTORY_GRID_ROWS,
                                INVENTORY_SLOT_HEIGHT,
                            ),
                            row_gap: Val::Px(INVENTORY_SLOT_GAP),
                            column_gap: Val::Px(INVENTORY_SLOT_GAP),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        InventoryPanel,
                    ))
                    .with_children(|grid| {
                        for (idx, item_name) in item_labels.into_iter().enumerate() {
                            let has_item = !item_name.is_empty();

                            grid.spawn((
                                Button,
                                Node {
                                    width: Val::Px(INVENTORY_SLOT_WIDTH),
                                    height: Val::Px(INVENTORY_SLOT_HEIGHT),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(2.0)),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                ImageNode::new(if has_item {
                                    slot_images.active.clone()
                                } else {
                                    slot_images.empty.clone()
                                })
                                .with_mode(NodeImageMode::Stretch),
                                InventorySlotImages {
                                    empty: slot_images.empty.clone(),
                                    active: slot_images.active.clone(),
                                },
                                ItemSlotButton { index: idx as u8 },
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(item_name),
                                    TextFont {
                                        font_size: FontSize::Px(theme.button_font_size * 0.55),
                                        ..default()
                                    },
                                    TextColor(theme.text_color),
                                    TextLayout::justify(Justify::Center),
                                    ItemSlotText { index: idx as u8 },
                                ));
                            });
                        }
                    });
                });
        })
        .spawn(commands, theme);
}

pub fn update_inventory_ui(
    mut slot_texts: Query<(&ItemSlotText, &mut Text)>,
    mut equip_texts: Query<(&EquipSlotText, &mut Text), Without<ItemSlotText>>,
    mut slot_images: Query<
        (&ItemSlotButton, &mut ImageNode, &InventorySlotImages),
        (Without<EquipSlotButton>, Without<ItemSlotText>),
    >,
    mut equip_images: Query<
        (&EquipSlotButton, &mut ImageNode, &InventorySlotImages),
        (Without<ItemSlotButton>, Without<EquipSlotText>),
    >,
    registry: Res<ItemRegistry>,
    player_query: Query<(&Inventory, &Equipment), With<LocalPlayer>>,
) {
    let Some((inventory, equipment)) = player_query.iter().next() else {
        return;
    };

    for (slot_text, mut text) in slot_texts.iter_mut() {
        let name = inventory
            .slots
            .get(slot_text.index as usize)
            .and_then(|opt| opt.as_ref())
            .and_then(|instance| registry.get(&instance.item_id))
            .map(|item| item.display_name().to_string())
            .unwrap_or_default();

        text.0 = name;
    }

    for (btn, mut image, images) in slot_images.iter_mut() {
        let has_item = inventory
            .slots
            .get(btn.index as usize)
            .is_some_and(|opt| opt.is_some());
        image.image = if has_item {
            images.active.clone()
        } else {
            images.empty.clone()
        };
    }

    for (equip_text, mut text) in equip_texts.iter_mut() {
        text.0 = equip_slot_label(equipment, &registry, equip_text.slot);
    }

    for (btn, mut image, images) in equip_images.iter_mut() {
        image.image = if equipment.get(btn.slot).is_some() {
            images.active.clone()
        } else {
            images.empty.clone()
        };
    }
}

type SlotClicksQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static ItemSlotButton),
    (Changed<Interaction>, With<Button>),
>;
type EquipSlotClicksQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static EquipSlotButton),
    (Changed<Interaction>, With<Button>),
>;
type EquipClicksQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static EquipButton),
    (Changed<Interaction>, With<Button>),
>;
type UnequipClicksQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static UnequipButton),
    (Changed<Interaction>, With<Button>),
>;

#[allow(clippy::too_many_arguments)]
pub fn handle_inventory_interactions(
    mut state: ResMut<InventoryUiState>,
    slot_clicks: SlotClicksQuery,
    equip_slot_clicks: EquipSlotClicksQuery,
    equip_clicks: EquipClicksQuery,
    unequip_clicks: UnequipClicksQuery,
    conn: Option<Res<StdbConnection>>,
    player_query: Query<(&Inventory, &Equipment, Option<&KnownAncientLanguage>), With<LocalPlayer>>,
    registry: Res<ItemRegistry>,
    glyphs: GlyphRegistries,
    theme: Res<UiTheme>,
    all_cards: Query<(Entity, &CardWindow)>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    // `KnownGlyphs` decides whether an inscribed slot is castable at all, so
    // the detail card needs it to mark a locked slot. It is replicated
    // separately from `Inventory`/`Equipment` and may not have arrived yet —
    // an empty Vocabulary is the correct stand-in until it does.
    let (inventory, equipment, known) = player_query
        .iter()
        .next()
        .map(|(i, e, k)| (i.clone(), e.clone(), k.cloned().unwrap_or_default()))
        .unwrap_or_default();

    for (interaction, slot_btn) in slot_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        state.selected = Some(InventorySelection::Slot(slot_btn.index));
        despawn_detail_cards(&mut commands, &all_cards);
        spawn_item_detail_card(
            &mut commands,
            &theme,
            &registry,
            &glyphs,
            &known,
            &inventory,
            &equipment,
            InventorySelection::Slot(slot_btn.index),
            &asset_server,
        );
    }

    for (interaction, equip_btn) in equip_slot_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if equipment.get(equip_btn.slot).is_none() {
            // Empty equipment slot: nothing to inspect.
            continue;
        }
        state.selected = Some(InventorySelection::Equipment(equip_btn.slot));
        despawn_detail_cards(&mut commands, &all_cards);
        spawn_item_detail_card(
            &mut commands,
            &theme,
            &registry,
            &glyphs,
            &known,
            &inventory,
            &equipment,
            InventorySelection::Equipment(equip_btn.slot),
            &asset_server,
        );
    }

    for (interaction, equip_btn) in equip_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(conn) = conn.as_deref() {
            if let Err(err) = stdb_commands::equip_item(conn, equip_btn.slot_index) {
                error!("could not equip item: {err}");
            }
        }
        despawn_detail_cards(&mut commands, &all_cards);
        state.selected = None;
    }

    for (interaction, unequip_btn) in unequip_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(conn) = conn.as_deref() {
            if let Err(err) = stdb_commands::unequip_item(conn, unequip_btn.slot) {
                error!("could not unequip item: {err}");
            }
        }
        despawn_detail_cards(&mut commands, &all_cards);
        state.selected = None;
    }
}

fn despawn_inventory_cards(commands: &mut Commands, cards: &Query<(Entity, &CardWindow)>) {
    for (entity, window) in cards.iter() {
        if window.kind == CardKind::Inventory || window.kind == CardKind::ItemDetail {
            commands.entity(entity).despawn();
        }
    }
}
