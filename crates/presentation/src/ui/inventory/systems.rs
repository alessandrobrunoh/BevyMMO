//! Systems for Inventory UI rendering, input handling, and server communication.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::{ButtonInput, ButtonState};
use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::stdb::{commands as stdb_commands, StdbConnection};
use bevymmo_gameplay::items::{
    components::{Equipment, Inventory},
    registry::ItemRegistry,
};

use super::{
    components::*,
    detail::despawn_detail_cards,
    equipment_section::equip_slot_label,
    spawn_inventory_window,
    stack::{parse_split_amount, step_split_amount},
    InventoryUiState, ItemDetailUiState,
};
use crate::ui::{
    card::components::{CardKind, CardWindow},
    chat::ChatInput,
    settings::state::{GameSettingsResource, KeyAction},
    theme::UiTheme,
};

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

pub fn update_inventory_ui(
    mut slot_texts: Query<(&ItemSlotText, &mut Text)>,
    mut equip_texts: Query<(&EquipSlotText, &mut Text), Without<ItemSlotText>>,
    mut slot_images: Query<
        (&ItemSlotButton, &mut ImageNode, &InventorySlotImages),
        (
            Without<EquipSlotButton>,
            Without<ItemSlotText>,
            Without<ItemSlotIcon>,
        ),
    >,
    mut equip_images: Query<
        (&EquipSlotButton, &mut ImageNode, &InventorySlotImages),
        (
            Without<ItemSlotButton>,
            Without<EquipSlotText>,
            Without<EquipSlotIcon>,
        ),
    >,
    mut slot_icons: Query<
        (&ItemSlotIcon, &mut ImageNode, &mut Visibility),
        Without<InventorySlotImages>,
    >,
    mut equip_icons: Query<
        (&EquipSlotIcon, &mut ImageNode, &mut Visibility),
        (Without<InventorySlotImages>, Without<ItemSlotIcon>),
    >,
    registry: Res<ItemRegistry>,
    asset_server: Res<AssetServer>,
    player_query: Query<(&Inventory, &Equipment), With<LocalPlayer>>,
) {
    let Some((inventory, equipment)) = player_query.iter().next() else {
        return;
    };

    for (slot_text, mut text) in slot_texts.iter_mut() {
        let instance = inventory
            .slots
            .get(slot_text.index as usize)
            .and_then(|opt| opt.as_ref());
        let has_icon = instance
            .and_then(|item| registry.get(&item.item_id))
            .and_then(|item| item.icon())
            .is_some();
        text.0 = match instance {
            Some(instance) if has_icon => {
                if instance.quantity > 1 {
                    instance.quantity.to_string()
                } else {
                    String::new()
                }
            }
            Some(instance) => {
                let display = registry
                    .get(&instance.item_id)
                    .map(|item| item.display_name().to_string())
                    .unwrap_or_else(|| instance.item_id.as_str().to_string());
                if instance.quantity > 1 {
                    format!("{display} x{}", instance.quantity)
                } else {
                    display
                }
            }
            None => String::new(),
        };
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

    for (icon, mut image, mut visibility) in slot_icons.iter_mut() {
        let path = inventory
            .slots
            .get(icon.index as usize)
            .and_then(|opt| opt.as_ref())
            .and_then(|instance| registry.get(&instance.item_id))
            .and_then(|item| item.icon());
        apply_item_icon(&mut image, &mut visibility, &asset_server, path);
    }

    for (equip_text, mut text) in equip_texts.iter_mut() {
        let has_icon = equipment
            .get(equip_text.slot)
            .as_ref()
            .and_then(|instance| registry.get(&instance.item_id))
            .and_then(|item| item.icon())
            .is_some();
        text.0 = if has_icon {
            String::new()
        } else {
            equip_slot_label(equipment, &registry, equip_text.slot)
        };
    }

    for (icon, mut image, mut visibility) in equip_icons.iter_mut() {
        let path = equipment
            .get(icon.slot)
            .as_ref()
            .and_then(|instance| registry.get(&instance.item_id))
            .and_then(|item| item.icon());
        apply_item_icon(&mut image, &mut visibility, &asset_server, path);
    }

    for (btn, mut image, images) in equip_images.iter_mut() {
        image.image = if equipment.get(btn.slot).is_some() {
            images.active.clone()
        } else {
            images.empty.clone()
        };
    }
}

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

/// Equip / Unequip on the detail card. Slot inspect happens on click-release
/// in [`super::drag::end_item_drag`], so a drag never opens the info panel.
pub fn handle_inventory_interactions(
    mut state: ResMut<InventoryUiState>,
    equip_clicks: EquipClicksQuery,
    unequip_clicks: UnequipClicksQuery,
    conn: Option<Res<StdbConnection>>,
    all_cards: Query<(Entity, &CardWindow)>,
    mut commands: Commands,
) {
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

fn apply_item_icon(
    image: &mut ImageNode,
    visibility: &mut Visibility,
    asset_server: &AssetServer,
    path: Option<&str>,
) {
    match path {
        Some(path) => {
            image.image = asset_server.load(path.to_string());
            *visibility = Visibility::Inherited;
        }
        None => {
            *visibility = Visibility::Hidden;
        }
    }
}

fn despawn_inventory_cards(commands: &mut Commands, cards: &Query<(Entity, &CardWindow)>) {
    for (entity, window) in cards.iter() {
        if window.kind == CardKind::Inventory || window.kind == CardKind::ItemDetail {
            commands.entity(entity).despawn();
        }
    }
}

type SplitClicksQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static SplitButton),
    (Changed<Interaction>, With<Button>),
>;
type CombineClicksQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static CombineButton),
    (Changed<Interaction>, With<Button>),
>;
type StepClicksQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static SplitAmountStep),
    (Changed<Interaction>, With<Button>),
>;

/// Split / Combine on the detail card, plus the − / + stepper.
pub(super) fn handle_stack_controls(
    split_clicks: SplitClicksQuery,
    combine_clicks: CombineClicksQuery,
    step_clicks: StepClicksQuery,
    mut fields: Query<&mut SplitAmountField>,
    mut detail_state: ResMut<ItemDetailUiState>,
    conn: Option<Res<StdbConnection>>,
) {
    for (interaction, step) in step_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        for mut field in fields.iter_mut() {
            let current = parse_split_amount(&field.value, field.quantity);
            let next = step_split_amount(current, step.delta, field.quantity);
            field.value = next.to_string();
            field.focused = false;
            detail_state.split_amount = next;
        }
    }

    for (interaction, split) in split_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let amount = fields
            .iter_mut()
            .next()
            .map(|mut field| {
                let amount = parse_split_amount(&field.value, field.quantity);
                field.value = amount.to_string();
                field.focused = false;
                detail_state.split_amount = amount;
                amount
            })
            .unwrap_or(0);
        if amount == 0 {
            continue;
        }
        if let Some(conn) = conn.as_deref() {
            if let Err(err) = stdb_commands::split_item(conn, split.slot_index, amount) {
                error!("could not split stack: {err}");
            }
        }
    }

    for (interaction, combine) in combine_clicks.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(conn) = conn.as_deref() {
            if let Err(err) = stdb_commands::combine_item(conn, combine.slot_index) {
                error!("could not combine stacks: {err}");
            }
        }
    }
}

pub(super) fn unfocus_split_when_chat_focused(
    chat: Query<&ChatInput>,
    mut fields: Query<&mut SplitAmountField>,
) {
    if !chat.iter().any(|input| input.focused) {
        return;
    }
    for mut field in fields.iter_mut() {
        if field.focused {
            field.focused = false;
        }
    }
}

/// Clicking the amount field captures the keyboard so I/WASD don't fire.
pub(super) fn focus_split_amount(
    clicked: Query<&Interaction, (With<SplitAmountField>, Changed<Interaction>)>,
    mut fields: Query<&mut SplitAmountField>,
    mut chat: Query<&mut ChatInput>,
) {
    if !clicked
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }
    for mut input in chat.iter_mut() {
        input.focused = false;
    }
    for mut field in fields.iter_mut() {
        field.focused = true;
    }
}

pub(super) fn edit_split_amount(
    mut events: MessageReader<KeyboardInput>,
    mut fields: Query<&mut SplitAmountField>,
    mut detail_state: ResMut<ItemDetailUiState>,
) {
    let Some(mut field) = fields.iter_mut().find(|field| field.focused) else {
        return;
    };

    for event in events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match &event.logical_key {
            Key::Backspace => {
                field.value.pop();
            }
            Key::Escape | Key::Enter => {
                let amount = parse_split_amount(&field.value, field.quantity);
                field.value = amount.to_string();
                field.focused = false;
                detail_state.split_amount = amount;
            }
            Key::Character(chars) => {
                for character in chars.chars() {
                    if field.value.len() >= 3 {
                        break;
                    }
                    if character.is_ascii_digit() {
                        field.value.push(character);
                    }
                }
            }
            _ => {}
        }
    }
}

pub(super) fn update_split_amount_display(
    theme: Res<UiTheme>,
    fields: Query<(&SplitAmountField, &Children, Entity), Changed<SplitAmountField>>,
    mut texts: Query<&mut Text, With<SplitAmountText>>,
    mut borders: Query<&mut BorderColor>,
) {
    for (field, children, entity) in fields.iter() {
        if let Ok(mut border) = borders.get_mut(entity) {
            *border = BorderColor::all(if field.focused {
                theme.input_border_focused
            } else {
                theme.input_border
            });
        }
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0 = if field.value.is_empty() {
                    String::new()
                } else {
                    field.value.clone()
                };
            }
        }
    }
}

/// Same rule as chat: a world click (move, attack) releases the amount field.
pub(super) fn defocus_split_amount_on_world_click(
    mouse: Res<ButtonInput<MouseButton>>,
    ui_interactions: Query<&Interaction>,
    mut fields: Query<&mut SplitAmountField>,
    mut detail_state: ResMut<ItemDetailUiState>,
) {
    if !(mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right)) {
        return;
    }
    if ui_interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }
    for mut field in fields.iter_mut() {
        if !field.focused {
            continue;
        }
        let amount = parse_split_amount(&field.value, field.quantity);
        field.value = amount.to_string();
        field.focused = false;
        detail_state.split_amount = amount;
    }
}
