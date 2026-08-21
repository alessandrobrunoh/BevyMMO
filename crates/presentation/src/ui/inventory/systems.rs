//! Systems for Inventory UI rendering, input handling, and server communication.

use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::stdb::{StdbConnection, commands as stdb_commands};
use bevymmo_gameplay::items::{
    components::{Equipment, Inventory},
    registry::ItemRegistry,
};

use super::{
    InventoryUiState, components::*, detail::despawn_detail_cards,
    equipment_section::equip_slot_label, spawn_inventory_window,
};
use crate::ui::{
    card::components::{CardKind, CardWindow},
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
            .map(|instance| {
                let display = registry
                    .get(&instance.item_id)
                    .map(|item| item.display_name().to_string())
                    .unwrap_or_else(|| instance.item_id.as_str().to_string());
                if instance.quantity > 1 {
                    format!("{display} x{}", instance.quantity)
                } else {
                    display
                }
            })
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

fn despawn_inventory_cards(commands: &mut Commands, cards: &Query<(Entity, &CardWindow)>) {
    for (entity, window) in cards.iter() {
        if window.kind == CardKind::Inventory || window.kind == CardKind::ItemDetail {
            commands.entity(entity).despawn();
        }
    }
}
