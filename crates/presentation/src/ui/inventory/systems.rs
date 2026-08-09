//! Systems for Inventory UI rendering, input handling, and server communication.

use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::{
    items::{
        components::{Equipment, Inventory, INVENTORY_CAPACITY},
        events::{EquipItemCommand, UnequipItemCommand},
        registry::ItemRegistry,
    },
    network::protocol::Channel2,
};
use lightyear::prelude::MessageSender;

use super::{components::*, detail::*, InventoryUiState};
use crate::ui::{
    card::{
        builder::CardBuilder,
        components::{CardKind, CardPositioning, CardWindow},
    },
    theme::UiTheme,
};

const INVENTORY_CARD_WIDTH: f32 = 640.0;
const INVENTORY_CARD_HEIGHT: f32 = 460.0;

pub fn toggle_inventory(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<InventoryUiState>,
    mut commands: Commands,
    window_query: Query<(Entity, &CardWindow)>,
    theme: Res<UiTheme>,
    registry: Res<ItemRegistry>,
    player_query: Query<(&Inventory, &Equipment), With<lightyear::prelude::Controlled>>,
) {
    if !keys.just_pressed(KeyCode::KeyI) {
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

fn spawn_inventory_window(
    commands: &mut Commands,
    theme: &UiTheme,
    registry: &ItemRegistry,
    inventory: &Inventory,
    equipment: &Equipment,
) {
    CardBuilder::new(CardKind::Inventory, "Inventory")
        .width(Val::Px(INVENTORY_CARD_WIDTH))
        .height(Val::Px(INVENTORY_CARD_HEIGHT))
        .positioning(CardPositioning::Right)
        .closeable()
        .exclusive()
        .with_body(|body| {
            // Main container
            body.spawn((Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },))
                .with_children(|main| {
                    // Equipment Section (Top)
                    main.spawn((Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },))
                        .with_children(|eq_section| {
                            eq_section.spawn((
                                Text::new("Equipment".to_string()),
                                TextFont {
                                    font_size: FontSize::Px(theme.button_font_size),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));

                            eq_section
                                .spawn((Node {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(12.0),
                                    ..default()
                                },))
                                .with_children(|row| {
                                    let weapon_name = equipment
                                        .weapon
                                        .as_ref()
                                        .and_then(|id| registry.get(id))
                                        .map(|item| item.display_name().to_string())
                                        .unwrap_or_else(|| "Empty Weapon Slot".to_string());

                                    row.spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(240.0),
                                            height: Val::Px(42.0),
                                            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(theme.button_bg),
                                        WeaponSlotButton,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(format!("Weapon: {weapon_name}")),
                                            TextFont {
                                                font_size: FontSize::Px(theme.button_font_size),
                                                ..default()
                                            },
                                            TextColor(theme.text_color),
                                            WeaponSlotText,
                                        ));
                                    });
                                });
                        });

                    // Inventory Slots Section (Grid 2 rows x 5 columns)
                    main.spawn((Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },))
                        .with_children(|inv_section| {
                            inv_section.spawn((
                                Text::new("Inventory Slots".to_string()),
                                TextFont {
                                    font_size: FontSize::Px(theme.button_font_size),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));

                            inv_section
                                .spawn((Node {
                                    width: Val::Percent(100.0),
                                    display: Display::Grid,
                                    grid_template_columns: RepeatedGridTrack::flex(5, 1.0),
                                    grid_template_rows: RepeatedGridTrack::flex(2, 1.0),
                                    row_gap: Val::Px(8.0),
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },))
                                .with_children(|grid| {
                                    for idx in 0..INVENTORY_CAPACITY {
                                        let item_name = inventory
                                            .slots
                                            .get(idx)
                                            .and_then(|opt| opt.as_ref())
                                            .and_then(|id| registry.get(id))
                                            .map(|item| item.display_name().to_string())
                                            .unwrap_or_else(|| format!("Slot {}", idx + 1));

                                        grid.spawn((
                                            Button,
                                            Node {
                                                height: Val::Px(60.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                padding: UiRect::all(Val::Px(4.0)),
                                                ..default()
                                            },
                                            BackgroundColor(theme.button_bg),
                                            ItemSlotButton { index: idx as u8 },
                                        ))
                                        .with_children(
                                            |btn| {
                                                btn.spawn((
                                                    Text::new(item_name),
                                                    TextFont {
                                                        font_size: FontSize::Px(
                                                            theme.button_font_size * 0.85,
                                                        ),
                                                        ..default()
                                                    },
                                                    TextColor(theme.text_color),
                                                    ItemSlotText { index: idx as u8 },
                                                ));
                                            },
                                        );
                                    }
                                });
                        });
                });
        })
        .spawn(commands, theme);
}

pub fn update_inventory_ui(
    mut slot_texts: Query<(&ItemSlotText, &mut Text), Without<WeaponSlotText>>,
    mut weapon_text: Query<&mut Text, With<WeaponSlotText>>,
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
            .and_then(|id| registry.get(id))
            .map(|item| item.display_name().to_string())
            .unwrap_or_else(|| format!("Slot {}", slot_text.index + 1));

        text.0 = name;
    }

    if let Some(mut text) = weapon_text.iter_mut().next() {
        let name = equipment
            .weapon
            .as_ref()
            .and_then(|id| registry.get(id))
            .map(|item| item.display_name().to_string())
            .unwrap_or_else(|| "Empty Weapon Slot".to_string());

        text.0 = format!("Weapon: {name}");
    }
}

type SlotClicksQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static ItemSlotButton),
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
    weapon_clicks: Query<&Interaction, (Changed<Interaction>, With<WeaponSlotButton>)>,
    equip_clicks: EquipClicksQuery,
    unequip_clicks: UnequipClicksQuery,
    mut equip_senders: Query<&mut MessageSender<EquipItemCommand>, With<ConnectedClient>>,
    mut unequip_senders: Query<&mut MessageSender<UnequipItemCommand>, With<ConnectedClient>>,
    player_query: Query<(&Inventory, &Equipment), With<lightyear::prelude::Controlled>>,
    registry: Res<ItemRegistry>,
    theme: Res<UiTheme>,
    all_cards: Query<(Entity, &CardWindow)>,
    mut commands: Commands,
) {
    let (inventory, equipment) = player_query
        .iter()
        .next()
        .map(|(i, e)| (i.clone(), e.clone()))
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
            &inventory,
            &equipment,
            InventorySelection::Slot(slot_btn.index),
        );
    }

    for interaction in weapon_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        state.selected = Some(InventorySelection::Weapon);
        despawn_detail_cards(&mut commands, &all_cards);
        spawn_item_detail_card(
            &mut commands,
            &theme,
            &registry,
            &inventory,
            &equipment,
            InventorySelection::Weapon,
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
