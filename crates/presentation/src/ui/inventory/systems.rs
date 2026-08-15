//! Systems for Inventory UI rendering, input handling, and server communication.

use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::{
    items::{
        components::{EquipSlot, Equipment, Inventory, INVENTORY_CAPACITY},
        events::{EquipItemCommand, UnequipItemCommand},
        registry::ItemRegistry,
    },
    network::protocol::Channel2,
};
use lightyear::prelude::MessageSender;

use bevymmo_shared::abilities::KnownGlyphs;

use super::weapon_detail::GlyphRegistries;
use super::{components::*, detail::*, InventoryUiState};
use crate::ui::{
    card::{
        builder::CardBuilder,
        components::{CardKind, CardPositioning, CardWindow},
    },
    settings::state::{GameSettingsResource, KeyAction},
    theme::UiTheme,
};

// Sized to still fit the default 800x600 dev window (see
// `ui::card::builder`'s note on why cards use viewport-relative centring):
// header + padding + 3x3 equip grid + mount row + divider + 5x2 item grid
// comfortably clears 600px tall at these dimensions.
const INVENTORY_CARD_WIDTH: f32 = 340.0;
const INVENTORY_CARD_HEIGHT: f32 = 560.0;
const EQUIP_SLOT_SIZE: f32 = 46.0;
const EMPTY_SLOT_PLACEHOLDER: &str = "—";

/// Slot border color for an empty box vs. one holding an item, approximating
/// the rune-lined boxes of the reference design.
const EMPTY_SLOT_BORDER: Color = Color::srgba(0.35, 0.38, 0.45, 0.5);
const FILLED_SLOT_BORDER: Color = Color::srgba(0.35, 0.65, 0.95, 0.85);

pub fn toggle_inventory(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    mut state: ResMut<InventoryUiState>,
    mut commands: Commands,
    window_query: Query<(Entity, &CardWindow)>,
    theme: Res<UiTheme>,
    registry: Res<ItemRegistry>,
    player_query: Query<(&Inventory, &Equipment), With<lightyear::prelude::Controlled>>,
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

    spawn_inventory_window(&mut commands, &theme, &registry, &inventory, &equipment);
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
fn spawn_equip_slot_cell(parent: &mut ChildSpawnerCommands, theme: &UiTheme, slot: EquipSlot, label: String) {
    let has_item = label != EMPTY_SLOT_PLACEHOLDER;

    parent
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
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
                    padding: UiRect::all(Val::Px(3.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(theme.button_bg),
                BorderColor::all(if has_item {
                    FILLED_SLOT_BORDER
                } else {
                    EMPTY_SLOT_BORDER
                }),
                EquipSlotButton { slot },
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(theme.button_font_size * 0.55),
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
                .unwrap_or_else(|| format!("Slot {}", idx + 1))
        })
        .collect();

    CardBuilder::new(CardKind::Inventory, "Inventory")
        .width(Val::Px(INVENTORY_CARD_WIDTH))
        .height(Val::Px(INVENTORY_CARD_HEIGHT))
        .positioning(CardPositioning::Right)
        .closeable()
        .exclusive()
        .with_body(move |body| {
            body.spawn((Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(9.0),
                ..default()
            },))
                .with_children(|main| {
                    // 3x3 equipment grid.
                    main.spawn((Node {
                        display: Display::Grid,
                        grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                        grid_template_rows: RepeatedGridTrack::flex(3, 1.0),
                        row_gap: Val::Px(10.0),
                        column_gap: Val::Px(10.0),
                        ..default()
                    },))
                        .with_children(|grid| {
                            for (slot, label) in grid_labels {
                                spawn_equip_slot_cell(grid, theme, slot, label);
                            }
                        });

                    // Mount: standalone, centered below the 3x3 grid.
                    main.spawn((Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },))
                        .with_children(|row| {
                            spawn_equip_slot_cell(row, theme, mount_slot, mount_label);
                        });

                    // Divider.
                    main.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                    ));

                    main.spawn((
                        Text::new("INVENTORY".to_string()),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size * 0.75),
                            ..default()
                        },
                        TextColor(Color::srgba(0.6, 0.75, 0.95, 0.9)),
                    ));

                    // 5x2 generic inventory grid.
                    main.spawn((Node {
                        width: Val::Percent(100.0),
                        display: Display::Grid,
                        grid_template_columns: RepeatedGridTrack::flex(5, 1.0),
                        grid_template_rows: RepeatedGridTrack::flex(2, 1.0),
                        row_gap: Val::Px(8.0),
                        column_gap: Val::Px(8.0),
                        ..default()
                    },))
                        .with_children(|grid| {
                            for (idx, item_name) in item_labels.into_iter().enumerate() {
                                let has_item = !item_name.starts_with("Slot ");

                                grid.spawn((
                                    Button,
                                    Node {
                                        height: Val::Px(44.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        padding: UiRect::all(Val::Px(4.0)),
                                        border: UiRect::all(Val::Px(1.5)),
                                        border_radius: BorderRadius::all(Val::Px(6.0)),
                                        ..default()
                                    },
                                    BackgroundColor(theme.button_bg),
                                    BorderColor::all(if has_item {
                                        FILLED_SLOT_BORDER
                                    } else {
                                        EMPTY_SLOT_BORDER
                                    }),
                                    ItemSlotButton { index: idx as u8 },
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new(item_name),
                                        TextFont {
                                            font_size: FontSize::Px(
                                                theme.button_font_size * 0.55,
                                            ),
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
    mut slot_borders: Query<
        (&ItemSlotButton, &mut BorderColor),
        (Without<EquipSlotButton>, Without<ItemSlotText>),
    >,
    mut equip_borders: Query<
        (&EquipSlotButton, &mut BorderColor),
        (Without<ItemSlotButton>, Without<EquipSlotText>),
    >,
    registry: Res<ItemRegistry>,
    player_query: Query<(&Inventory, &Equipment), With<lightyear::prelude::Controlled>>,
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
            .unwrap_or_else(|| format!("Slot {}", slot_text.index + 1));

        text.0 = name;
    }

    for (btn, mut border) in slot_borders.iter_mut() {
        let has_item = inventory
            .slots
            .get(btn.index as usize)
            .is_some_and(|opt| opt.is_some());
        *border = BorderColor::all(if has_item {
            FILLED_SLOT_BORDER
        } else {
            EMPTY_SLOT_BORDER
        });
    }

    for (equip_text, mut text) in equip_texts.iter_mut() {
        text.0 = equip_slot_label(equipment, &registry, equip_text.slot);
    }

    for (btn, mut border) in equip_borders.iter_mut() {
        let has_item = equipment.get(btn.slot).is_some();
        *border = BorderColor::all(if has_item {
            FILLED_SLOT_BORDER
        } else {
            EMPTY_SLOT_BORDER
        });
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
    mut equip_senders: Query<&mut MessageSender<EquipItemCommand>, With<ConnectedClient>>,
    mut unequip_senders: Query<&mut MessageSender<UnequipItemCommand>, With<ConnectedClient>>,
    player_query: Query<
        (&Inventory, &Equipment, Option<&KnownGlyphs>),
        With<lightyear::prelude::Controlled>,
    >,
    registry: Res<ItemRegistry>,
    glyphs: GlyphRegistries,
    theme: Res<UiTheme>,
    all_cards: Query<(Entity, &CardWindow)>,
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
        );
    }

    for (interaction, equip_btn) in equip_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        for mut sender in equip_senders.iter_mut() {
            sender.send::<Channel2>(EquipItemCommand {
                slot_index: equip_btn.slot_index,
            });
        }
        despawn_detail_cards(&mut commands, &all_cards);
        state.selected = None;
    }

    for (interaction, unequip_btn) in unequip_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        for mut sender in unequip_senders.iter_mut() {
            sender.send::<Channel2>(UnequipItemCommand {
                slot: unequip_btn.slot,
            });
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
