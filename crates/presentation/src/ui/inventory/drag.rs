//! Drag-and-drop for inventory/equipment slots.
//!
//! Mirrors the press/hold/release pattern already used by `ui::card::systems::handle_card_drag`,
//! applied to individual item slots instead of whole card windows:
//!
//! 1. [`start_item_drag`]: mouse-down on an occupied slot immediately spawns a
//!    square "ghost" icon centered on the cursor — the item visually leaves
//!    its slot (which dims) and follows the pointer from that first frame.
//! 2. [`update_item_drag`]: while the mouse stays held, the ghost is glued to
//!    the cursor every frame, so it can be carried anywhere on screen.
//! 3. [`end_item_drag`]: on mouse-up, whichever slot is currently `Hovered`
//!    is the drop target. The (origin, target) pair decides which network
//!    command to send — the server is always the final authority; an
//!    inconsistent drop (e.g. wrong equip slot type, empty space) simply
//!    restores the origin slot with no command sent.

use bevy::prelude::*;
use bevymmo_client::network::types::ConnectedClient;
use bevymmo_shared::{
    items::{
        components::{Equipment, Inventory},
        events::{EquipItemCommand, MoveItemCommand, UnequipItemCommand},
        registry::{ItemId, ItemRegistry},
    },
    network::protocol::Channel2,
};
use bevymmo_shared::entity::LocalPlayer;
use lightyear::prelude::MessageSender;

use super::components::{EquipSlotButton, ItemDragGhost, ItemSlotButton, ItemSlotOrigin};
use crate::ui::{
    card::components::CardWindow, inventory::detail::despawn_detail_cards, scale::window_to_ui_px,
    theme::UiTheme,
};

/// Size of the floating item icon that follows the cursor while dragging.
const DRAG_GHOST_SIZE: f32 = 48.0;

/// Background alpha applied to the origin slot while its item is being
/// carried, so it visibly reads as "picked up" rather than duplicated.
const DRAGGED_FROM_ALPHA: f32 = 0.25;

/// A drag in progress: the slot it was picked up from, the item being
/// carried, and the floating ghost entity following the cursor.
struct PendingDrag {
    origin: ItemSlotOrigin,
    origin_entity: Entity,
    item_id: ItemId,
    ghost: Entity,
}

/// Tracks the currently in-progress item drag, if any.
#[derive(Resource, Default)]
pub struct ItemDragState {
    pending: Option<PendingDrag>,
}

/// Mouse-down on an occupied slot: picks the item up immediately — spawns the
/// cursor-following ghost and dims the origin slot in the same frame.
pub fn start_item_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui_scale: Res<UiScale>,
    slot_presses: Query<(Entity, &Interaction, &ItemSlotButton), Changed<Interaction>>,
    equip_presses: Query<(Entity, &Interaction, &EquipSlotButton), Changed<Interaction>>,
    player_query: Query<(&Inventory, &Equipment), With<LocalPlayer>>,
    registry: Res<ItemRegistry>,
    theme: Res<UiTheme>,
    mut backgrounds: Query<&mut BackgroundColor>,
    mut drag_state: ResMut<ItemDragState>,
    mut commands: Commands,
) {
    if !mouse.just_pressed(MouseButton::Left) || drag_state.pending.is_some() {
        return;
    }
    let Some(cursor) = windows
        .iter()
        .next()
        .and_then(Window::cursor_position)
        .map(|cursor| window_to_ui_px(cursor, &ui_scale))
    else {
        return;
    };
    let Some((inventory, equipment)) = player_query.iter().next() else {
        return;
    };

    let picked = slot_presses
        .iter()
        .find(|(_, interaction, _)| **interaction == Interaction::Pressed)
        .and_then(|(entity, _, btn)| {
            inventory
                .slots
                .get(btn.index as usize)
                .and_then(Clone::clone)
                .map(|instance| (entity, ItemSlotOrigin::Inventory(btn.index), instance.item_id))
        })
        .or_else(|| {
            equip_presses
                .iter()
                .find(|(_, interaction, _)| **interaction == Interaction::Pressed)
                .and_then(|(entity, _, btn)| {
                    equipment
                        .get(btn.slot)
                        .clone()
                        .map(|instance| (entity, ItemSlotOrigin::Equipment(btn.slot), instance.item_id))
                })
        });

    let Some((origin_entity, origin, item_id)) = picked else {
        return;
    };

    if let Ok(mut bg) = backgrounds.get_mut(origin_entity) {
        bg.0.set_alpha(DRAGGED_FROM_ALPHA);
    }

    let label = registry
        .get(&item_id)
        .map(|item| item.display_name().to_string())
        .unwrap_or_else(|| item_id.as_str().to_string());

    let ghost = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(cursor.x - DRAG_GHOST_SIZE * 0.5),
                top: Val::Px(cursor.y - DRAG_GHOST_SIZE * 0.5),
                width: Val::Px(DRAG_GHOST_SIZE),
                height: Val::Px(DRAG_GHOST_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(3.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(theme.button_pressed_bg.with_alpha(0.92)),
            BorderColor::all(Color::srgba(0.6, 0.8, 1.0, 0.95)),
            GlobalZIndex(1000),
            // The ghost renders on top of everything and follows the cursor,
            // so it would otherwise win every pointer hit-test and hide the
            // real slot underneath from `end_item_drag`'s hover lookup.
            Pickable::IGNORE,
            ItemDragGhost,
        ))
        .with_children(|ghost| {
            ghost.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.55),
                    ..default()
                },
                TextColor(theme.text_color),
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            ));
        })
        .id();

    drag_state.pending = Some(PendingDrag {
        origin,
        origin_entity,
        item_id,
        ghost,
    });
}

/// While the mouse stays held, keeps the ghost glued to the cursor anywhere
/// on screen. Loses the item and cancels the drag if the cursor leaves the
/// window entirely.
pub fn update_item_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui_scale: Res<UiScale>,
    mut drag_state: ResMut<ItemDragState>,
    mut ghost_nodes: Query<&mut Node, With<ItemDragGhost>>,
    theme: Res<UiTheme>,
    mut backgrounds: Query<&mut BackgroundColor>,
    mut commands: Commands,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = windows
        .iter()
        .next()
        .and_then(Window::cursor_position)
        .map(|cursor| window_to_ui_px(cursor, &ui_scale))
    else {
        cancel_pending_drag(&mut drag_state, &theme, &mut backgrounds, &mut commands);
        return;
    };
    let Some(pending) = drag_state.pending.as_ref() else {
        return;
    };

    if let Ok(mut node) = ghost_nodes.get_mut(pending.ghost) {
        node.left = Val::Px(cursor.x - DRAG_GHOST_SIZE * 0.5);
        node.top = Val::Px(cursor.y - DRAG_GHOST_SIZE * 0.5);
    }
}

/// On mouse-up, resolves the hovered slot as the drop target and sends the
/// matching network command. The server re-validates everything; an
/// inconsistent drop (wrong equip slot, dropping on empty space, ...) is
/// simply ignored client-side and the origin slot regains its item once
/// replication confirms nothing changed (it never actually left, visually).
#[allow(clippy::too_many_arguments)]
pub fn end_item_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<ItemDragState>,
    slot_hover: Query<(&Interaction, &ItemSlotButton)>,
    equip_hover: Query<(&Interaction, &EquipSlotButton)>,
    registry: Res<ItemRegistry>,
    theme: Res<UiTheme>,
    mut backgrounds: Query<&mut BackgroundColor>,
    mut equip_senders: Query<&mut MessageSender<EquipItemCommand>, With<ConnectedClient>>,
    mut unequip_senders: Query<&mut MessageSender<UnequipItemCommand>, With<ConnectedClient>>,
    mut move_senders: Query<&mut MessageSender<MoveItemCommand>, With<ConnectedClient>>,
    all_cards: Query<(Entity, &CardWindow)>,
    mut commands: Commands,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    let Some(pending) = drag_state.pending.take() else {
        return;
    };

    commands.entity(pending.ghost).despawn();
    if let Ok(mut bg) = backgrounds.get_mut(pending.origin_entity) {
        bg.0 = theme.button_bg;
    }

    let target = slot_hover
        .iter()
        .find(|(i, _)| **i == Interaction::Hovered)
        .map(|(_, b)| ItemSlotOrigin::Inventory(b.index))
        .or_else(|| {
            equip_hover
                .iter()
                .find(|(i, _)| **i == Interaction::Hovered)
                .map(|(_, b)| ItemSlotOrigin::Equipment(b.slot))
        });

    let Some(target) = target else {
        // Dropped on empty space: cancel, item stays put.
        return;
    };
    if target == pending.origin {
        return;
    }

    let sent = match (pending.origin, target) {
        (ItemSlotOrigin::Inventory(from), ItemSlotOrigin::Inventory(to)) => {
            for mut sender in move_senders.iter_mut() {
                sender.send::<Channel2>(MoveItemCommand { from, to });
            }
            true
        }
        (ItemSlotOrigin::Inventory(idx), ItemSlotOrigin::Equipment(slot)) => {
            let matches_slot = registry
                .get(&pending.item_id)
                .and_then(|item| item.config().equippable_into)
                == Some(slot);
            if matches_slot {
                for mut sender in equip_senders.iter_mut() {
                    sender.send::<Channel2>(EquipItemCommand { slot_index: idx });
                }
            }
            matches_slot
        }
        (ItemSlotOrigin::Equipment(slot), ItemSlotOrigin::Inventory(_)) => {
            for mut sender in unequip_senders.iter_mut() {
                sender.send::<Channel2>(UnequipItemCommand { slot });
            }
            true
        }
        // Equipment-to-equipment swaps aren't a supported command yet.
        (ItemSlotOrigin::Equipment(_), ItemSlotOrigin::Equipment(_)) => false,
    };

    if sent {
        // The open detail card (if any) still shows the pre-drag state;
        // close it rather than leave a stale Equip/Unequip button behind.
        despawn_detail_cards(&mut commands, &all_cards);
    }
}

/// Cancels an in-progress drag (e.g. the cursor left the window), despawning
/// its ghost and restoring the origin slot's normal background.
fn cancel_pending_drag(
    drag_state: &mut ItemDragState,
    theme: &UiTheme,
    backgrounds: &mut Query<&mut BackgroundColor>,
    commands: &mut Commands,
) {
    if let Some(pending) = drag_state.pending.take() {
        commands.entity(pending.ghost).despawn();
        if let Ok(mut bg) = backgrounds.get_mut(pending.origin_entity) {
            bg.0 = theme.button_bg;
        }
    }
}
