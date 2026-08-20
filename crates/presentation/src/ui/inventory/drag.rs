//! Drag-and-drop for inventory/equipment slots.
//!
//! Press on a slot records the origin. A ghost appears only after the pointer
//! travels [`DRAG_THRESHOLD_PX`]. Release then resolves through
//! [`drag_outcome`]: a click inspects, a drop on another slot moves, a drop
//! outside the inventory window asks to destroy, and a drop on inventory
//! chrome puts the item back.

use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::stdb::{commands as stdb_commands, StdbConnection};
use bevymmo_gameplay::abilities::KnownAncientLanguage;
use bevymmo_gameplay::items::{
    components::{Equipment, Inventory},
    instance::ItemInstanceId,
    registry::{ItemId, ItemRegistry},
};

use super::components::{
    CancelDestroyButton, ConfirmDestroyButton, DestroyItemDialog, EquipSlotButton,
    InventorySelection, ItemDragGhost, ItemSlotButton, ItemSlotOrigin,
};
use super::detail::{despawn_detail_cards, spawn_item_detail_card};
use super::weapon_detail::GlyphRegistries;
use super::InventoryUiState;
use crate::ui::{
    card::components::{CardKind, CardWindow},
    scale::{physical_to_ui_px, window_to_ui_px},
    theme::UiTheme,
};

/// Size of the floating item icon that follows the cursor while dragging.
const DRAG_GHOST_SIZE: f32 = 48.0;

/// Pixels the pointer must travel before a press becomes a drag.
pub const DRAG_THRESHOLD_PX: f32 = 6.0;

/// What happens when the pointer is released after picking a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragOutcome {
    /// Movement stayed under the threshold and no ghost spawned: inspect.
    ClickInspect,
    /// Drag ended over another slot: move or equip.
    MoveItem,
    /// Drag ended over inventory chrome (not a slot): put the item back.
    Cancel,
    /// Drag ended outside the inventory window: confirm destroy.
    RequestDestroy,
}

/// Resolves a slot press/release into inspect, move, cancel, or destroy.
///
/// A click (never crossed the drag threshold) is always inspect, even if the
/// pointer is no longer over a slot. Once the pointer has actually travelled,
/// releasing outside the inventory window destroys; releasing on inventory
/// chrome that is not a slot puts the item back.
pub fn drag_outcome(
    start: Vec2,
    end: Vec2,
    over_slot: bool,
    over_inventory: bool,
    did_drag: bool,
    threshold: f32,
) -> DragOutcome {
    if !did_drag && start.distance(end) < threshold {
        return DragOutcome::ClickInspect;
    }
    if over_slot {
        return DragOutcome::MoveItem;
    }
    if over_inventory {
        return DragOutcome::Cancel;
    }
    DragOutcome::RequestDestroy
}

/// Background alpha applied to the origin slot while its item is being
/// carried, so it visibly reads as "picked up" rather than duplicated.
const DRAGGED_FROM_ALPHA: f32 = 0.25;

/// A drag in progress: the slot it was picked up from, the item being
/// carried, and the floating ghost entity following the cursor (spawned
/// only after the pointer travels [`DRAG_THRESHOLD_PX`]).
struct PendingDrag {
    origin: ItemSlotOrigin,
    origin_entity: Entity,
    item_id: ItemId,
    instance_id: ItemInstanceId,
    start: Vec2,
    label: String,
    ghost: Option<Entity>,
}

/// Tracks the currently in-progress item drag, if any.
#[derive(Resource, Default)]
pub struct ItemDragState {
    pending: Option<PendingDrag>,
    /// Filled on a click-release (no drag). Consumed by [`inspect_clicked_item`].
    inspect: Option<ItemSlotOrigin>,
}

/// Mouse-down on an occupied slot: records the origin. The ghost is spawned
/// later, once the pointer has moved past [`DRAG_THRESHOLD_PX`].
pub fn start_item_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui_scale: Res<UiScale>,
    slot_presses: Query<(Entity, &Interaction, &ItemSlotButton), Changed<Interaction>>,
    equip_presses: Query<(Entity, &Interaction, &EquipSlotButton), Changed<Interaction>>,
    player_query: Query<(&Inventory, &Equipment), With<LocalPlayer>>,
    registry: Res<ItemRegistry>,
    mut drag_state: ResMut<ItemDragState>,
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
                .map(|instance| {
                    (
                        entity,
                        ItemSlotOrigin::Inventory(btn.index),
                        instance.item_id,
                        instance.instance_id,
                    )
                })
        })
        .or_else(|| {
            equip_presses
                .iter()
                .find(|(_, interaction, _)| **interaction == Interaction::Pressed)
                .and_then(|(entity, _, btn)| {
                    equipment.get(btn.slot).clone().map(|instance| {
                        (
                            entity,
                            ItemSlotOrigin::Equipment(btn.slot),
                            instance.item_id,
                            instance.instance_id,
                        )
                    })
                })
        });

    let Some((origin_entity, origin, item_id, instance_id)) = picked else {
        return;
    };

    let label = registry
        .get(&item_id)
        .map(|item| item.display_name().to_string())
        .unwrap_or_else(|| item_id.as_str().to_string());

    drag_state.pending = Some(PendingDrag {
        origin,
        origin_entity,
        item_id,
        instance_id,
        start: cursor,
        label,
        ghost: None,
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
    all_cards: Query<(Entity, &CardWindow)>,
    mut state: ResMut<InventoryUiState>,
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
    let Some(pending) = drag_state.pending.as_mut() else {
        return;
    };

    if pending.ghost.is_none() && pending.start.distance(cursor) >= DRAG_THRESHOLD_PX {
        if let Ok(mut bg) = backgrounds.get_mut(pending.origin_entity) {
            bg.0.set_alpha(DRAGGED_FROM_ALPHA);
        }
        despawn_detail_cards(&mut commands, &all_cards);
        state.selected = None;
        pending.ghost = Some(spawn_drag_ghost(
            &mut commands,
            &theme,
            cursor,
            &pending.label,
        ));
    }

    if let Some(ghost) = pending.ghost {
        if let Ok(mut node) = ghost_nodes.get_mut(ghost) {
            node.left = Val::Px(cursor.x - DRAG_GHOST_SIZE * 0.5);
            node.top = Val::Px(cursor.y - DRAG_GHOST_SIZE * 0.5);
        }
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
    windows: Query<&Window>,
    ui_scale: Res<UiScale>,
    mut drag_state: ResMut<ItemDragState>,
    slot_hover: Query<(&Interaction, &ItemSlotButton)>,
    equip_hover: Query<(&Interaction, &EquipSlotButton)>,
    inventory_cards: Query<(&CardWindow, &ComputedNode, &UiGlobalTransform)>,
    registry: Res<ItemRegistry>,
    theme: Res<UiTheme>,
    mut backgrounds: Query<&mut BackgroundColor>,
    conn: Option<Res<StdbConnection>>,
    all_cards: Query<(Entity, &CardWindow)>,
    mut commands: Commands,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    let Some(pending) = drag_state.pending.take() else {
        return;
    };

    let did_drag = pending.ghost.is_some();
    if let Some(ghost) = pending.ghost {
        commands.entity(ghost).despawn();
    }
    if let Ok(mut bg) = backgrounds.get_mut(pending.origin_entity) {
        bg.0 = theme.button_bg;
    }

    let end = windows
        .iter()
        .next()
        .and_then(Window::cursor_position)
        .map(|cursor| window_to_ui_px(cursor, &ui_scale))
        .unwrap_or(pending.start);

    let target = slot_hover
        .iter()
        .find(|(i, _)| matches!(**i, Interaction::Hovered | Interaction::Pressed))
        .map(|(_, b)| ItemSlotOrigin::Inventory(b.index))
        .or_else(|| {
            equip_hover
                .iter()
                .find(|(i, _)| matches!(**i, Interaction::Hovered | Interaction::Pressed))
                .map(|(_, b)| ItemSlotOrigin::Equipment(b.slot))
        });
    let over_inventory = cursor_over_inventory_card(end, &inventory_cards);

    match drag_outcome(
        pending.start,
        end,
        target.is_some(),
        over_inventory,
        did_drag,
        DRAG_THRESHOLD_PX,
    ) {
        DragOutcome::Cancel => {}
        DragOutcome::ClickInspect => {
            drag_state.inspect = Some(pending.origin);
        }
        DragOutcome::RequestDestroy => {
            if matches!(pending.origin, ItemSlotOrigin::Inventory(_)) {
                spawn_destroy_dialog(&mut commands, &theme, pending.instance_id.0, &pending.label);
            }
        }
        DragOutcome::MoveItem => {
            let Some(target) = target else {
                return;
            };
            if target == pending.origin {
                return;
            }
            apply_slot_drop(
                &pending,
                target,
                &registry,
                conn.as_deref(),
                &all_cards,
                &mut commands,
            );
        }
    }
}

/// Opens the item-info card for a click that never became a drag.
#[allow(clippy::too_many_arguments)]
pub fn inspect_clicked_item(
    mut drag_state: ResMut<ItemDragState>,
    mut state: ResMut<InventoryUiState>,
    player_query: Query<(&Inventory, &Equipment, Option<&KnownAncientLanguage>), With<LocalPlayer>>,
    registry: Res<ItemRegistry>,
    glyphs: GlyphRegistries,
    theme: Res<UiTheme>,
    all_cards: Query<(Entity, &CardWindow)>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let Some(origin) = drag_state.inspect.take() else {
        return;
    };
    let Some((inventory, equipment, known)) = player_query.iter().next() else {
        return;
    };
    // `KnownGlyphs` decides whether an inscribed slot is castable at all, so
    // the detail card needs it to mark a locked slot. It is replicated
    // separately from `Inventory`/`Equipment` and may not have arrived yet —
    // an empty Vocabulary is the correct stand-in until it does.
    let known = known.cloned().unwrap_or_default();
    let selection = match origin {
        ItemSlotOrigin::Inventory(idx) => InventorySelection::Slot(idx),
        ItemSlotOrigin::Equipment(slot) => InventorySelection::Equipment(slot),
    };
    state.selected = Some(selection);
    despawn_detail_cards(&mut commands, &all_cards);
    spawn_item_detail_card(
        &mut commands,
        &theme,
        &registry,
        &glyphs,
        &known,
        inventory,
        equipment,
        selection,
        &asset_server,
    );
}

fn cursor_over_inventory_card(
    cursor: Vec2,
    cards: &Query<(&CardWindow, &ComputedNode, &UiGlobalTransform)>,
) -> bool {
    cards.iter().any(|(window, computed, transform)| {
        if window.kind != CardKind::Inventory {
            return false;
        }
        let top_left = physical_to_ui_px(transform.translation - computed.size() * 0.5, computed);
        let size = physical_to_ui_px(computed.size(), computed);
        Rect::from_corners(top_left, top_left + size).contains(cursor)
    })
}

fn apply_slot_drop(
    pending: &PendingDrag,
    target: ItemSlotOrigin,
    registry: &ItemRegistry,
    conn: Option<&StdbConnection>,
    all_cards: &Query<(Entity, &CardWindow)>,
    commands: &mut Commands,
) {
    let sent = match (pending.origin, target) {
        (ItemSlotOrigin::Inventory(from), ItemSlotOrigin::Inventory(to)) => {
            if let Some(conn) = conn {
                if let Err(err) = stdb_commands::move_item(conn, from, to) {
                    error!("could not move item: {err}");
                }
            }
            true
        }
        (ItemSlotOrigin::Inventory(idx), ItemSlotOrigin::Equipment(slot)) => {
            let matches_slot = registry
                .get(&pending.item_id)
                .and_then(|item| item.config().equippable_into)
                == Some(slot);
            if matches_slot {
                if let Some(conn) = conn {
                    if let Err(err) = stdb_commands::equip_item(conn, idx) {
                        error!("could not equip item: {err}");
                    }
                }
            }
            matches_slot
        }
        (ItemSlotOrigin::Equipment(slot), ItemSlotOrigin::Inventory(_)) => {
            if let Some(conn) = conn {
                if let Err(err) = stdb_commands::unequip_item(conn, slot) {
                    error!("could not unequip item: {err}");
                }
            }
            true
        }
        // Equipment-to-equipment swaps aren't a supported command yet.
        (ItemSlotOrigin::Equipment(_), ItemSlotOrigin::Equipment(_)) => false,
    };

    if sent {
        // The open detail card (if any) still shows the pre-drag state;
        // close it rather than leave a stale Equip/Unequip button behind.
        despawn_detail_cards(commands, all_cards);
    }
}

fn spawn_drag_ghost(commands: &mut Commands, theme: &UiTheme, cursor: Vec2, label: &str) -> Entity {
    commands
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
                Text::new(label.to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.55),
                    ..default()
                },
                TextColor(theme.text_color),
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            ));
        })
        .id()
}

fn spawn_destroy_dialog(
    commands: &mut Commands,
    theme: &UiTheme,
    instance_id: u64,
    item_label: &str,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                width: Val::Px(360.0),
                min_height: Val::Px(150.0),
                margin: UiRect {
                    left: Val::Px(-180.0),
                    top: Val::Px(-75.0),
                    ..default()
                },
                padding: UiRect::all(Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            GlobalZIndex(2000),
            DestroyItemDialog,
        ))
        .with_children(|dialog| {
            dialog.spawn((
                Text::new(format!("Destroy {item_label}?")),
                TextFont {
                    font_size: FontSize::Px(theme.title_font_size),
                    ..default()
                },
                TextColor(theme.text_color),
            ));
            dialog.spawn((
                Text::new("This item will be permanently destroyed."),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size),
                    ..default()
                },
                TextColor(theme.muted_text_color),
            ));
            dialog
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    ..default()
                },))
                .with_children(|buttons| {
                    buttons
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(theme.button_pressed_bg),
                            ConfirmDestroyButton { instance_id },
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("Destroy"),
                                TextFont {
                                    font_size: FontSize::Px(theme.button_font_size),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));
                        });
                    buttons
                        .spawn((
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(theme.button_bg),
                            CancelDestroyButton,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("Cancel"),
                                TextFont {
                                    font_size: FontSize::Px(theme.button_font_size),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));
                        });
                });
        });
}

/// Handles the irreversible confirmation dialog opened by an outside drop.
pub fn handle_destroy_dialog(
    confirm: Query<(&Interaction, &ConfirmDestroyButton), Changed<Interaction>>,
    cancel: Query<&Interaction, (With<CancelDestroyButton>, Changed<Interaction>)>,
    dialog: Query<Entity, With<DestroyItemDialog>>,
    connection: Option<Res<StdbConnection>>,
    mut commands: Commands,
) {
    let confirmed = confirm
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Pressed)
        .map(|(_, button)| button.instance_id);
    let cancelled = cancel
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);

    if let Some(instance_id) = confirmed {
        if let Some(connection) = connection {
            if let Err(error) = stdb_commands::destroy_item(&connection, instance_id) {
                error!("could not destroy item: {error}");
            }
        }
    }
    if confirmed.is_some() || cancelled {
        for entity in dialog.iter() {
            commands.entity(entity).despawn();
        }
    }
}

fn cancel_pending_drag(
    drag_state: &mut ItemDragState,
    theme: &UiTheme,
    backgrounds: &mut Query<&mut BackgroundColor>,
    commands: &mut Commands,
) {
    if let Some(pending) = drag_state.pending.take() {
        if let Some(ghost) = pending.ghost {
            commands.entity(ghost).despawn();
        }
        if let Ok(mut bg) = backgrounds.get_mut(pending.origin_entity) {
            bg.0 = theme.button_bg;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn click_on_slot_inspects_and_never_destroys() {
        assert_eq!(
            drag_outcome(
                pt(10.0, 10.0),
                pt(12.0, 11.0),
                true,
                true,
                false,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::ClickInspect
        );
        assert_eq!(
            drag_outcome(
                pt(10.0, 10.0),
                pt(12.0, 11.0),
                false,
                false,
                false,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::ClickInspect
        );
        assert_eq!(
            drag_outcome(
                pt(10.0, 10.0),
                pt(10.0, 10.0),
                false,
                false,
                false,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::ClickInspect
        );
    }

    #[test]
    fn drag_onto_another_slot_moves() {
        assert_eq!(
            drag_outcome(
                pt(0.0, 0.0),
                pt(20.0, 0.0),
                true,
                true,
                true,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::MoveItem
        );
    }

    #[test]
    fn drag_onto_slot_wins_over_outside_inventory() {
        assert_eq!(
            drag_outcome(
                pt(0.0, 0.0),
                pt(20.0, 0.0),
                true,
                false,
                true,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::MoveItem
        );
    }

    #[test]
    fn drag_outside_inventory_requests_destroy() {
        assert_eq!(
            drag_outcome(
                pt(0.0, 0.0),
                pt(20.0, 0.0),
                false,
                false,
                true,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::RequestDestroy
        );
    }

    #[test]
    fn drag_onto_inventory_chrome_cancels() {
        assert_eq!(
            drag_outcome(
                pt(0.0, 0.0),
                pt(20.0, 0.0),
                false,
                true,
                true,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::Cancel
        );
    }

    #[test]
    fn returning_near_the_start_after_a_drag_does_not_inspect() {
        assert_eq!(
            drag_outcome(
                pt(0.0, 0.0),
                pt(2.0, 0.0),
                true,
                true,
                true,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::MoveItem
        );
        assert_eq!(
            drag_outcome(
                pt(0.0, 0.0),
                pt(2.0, 0.0),
                false,
                false,
                true,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::RequestDestroy
        );
    }

    #[test]
    fn threshold_must_be_positive_so_clicks_are_not_drags() {
        // A zero threshold would classify a stationary click as a drag.
        assert_eq!(
            drag_outcome(
                pt(4.0, 4.0),
                pt(4.0, 4.0),
                false,
                false,
                false,
                DRAG_THRESHOLD_PX
            ),
            DragOutcome::ClickInspect
        );
        assert_ne!(
            drag_outcome(pt(4.0, 4.0), pt(4.0, 4.0), false, false, false, 0.0),
            DragOutcome::ClickInspect
        );
    }
}
