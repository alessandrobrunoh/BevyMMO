//! Fixed equipment section shown at the top of the inventory card.

use bevy::prelude::*;
use bevymmo_gameplay::items::{
    components::{EquipSlot, Equipment},
    registry::ItemRegistry,
};

use super::components::{EquipSlotButton, EquipSlotText, InventorySlotImages};
use crate::ui::theme::UiTheme;

const EQUIP_SLOT_SIZE: f32 = 44.0;
/// Column track is wider than the box so captions like `OFFHAND` still fit.
const EQUIP_CELL_WIDTH: f32 = 56.0;
const EQUIP_GRID_GAP: f32 = 6.0;
const EQUIP_MOUNT_GAP: f32 = 6.0;
/// Shown in an empty equipment cell.
///
/// ASCII on purpose: Bevy's built-in font is an ASCII subset, so an em dash
/// renders as a blank box rather than a dash.
const EMPTY_SLOT_PLACEHOLDER: &str = "-";

/// Fixed section containing all equipped item slots.
#[derive(Component, Debug)]
pub struct EquipmentPanel;

/// Resolves the display label for an equip slot's box: the equipped item's
/// name, or a placeholder dash when empty.
pub(super) fn equip_slot_label(
    equipment: &Equipment,
    registry: &ItemRegistry,
    slot: EquipSlot,
) -> String {
    equipment
        .get(slot)
        .as_ref()
        .and_then(|instance| registry.get(&instance.item_id))
        .map(|item| item.display_name().to_string())
        .unwrap_or_else(|| EMPTY_SLOT_PLACEHOLDER.to_string())
}

pub(super) fn spawn_equipment_section(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    equipment: &Equipment,
    registry: &ItemRegistry,
    images: &InventorySlotImages,
) {
    let grid_slots = &EquipSlot::ALL[..9];
    let mount_slot = EquipSlot::ALL[9];

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(EQUIP_MOUNT_GAP),
                ..default()
            },
            EquipmentPanel,
        ))
        .with_children(|section| {
            section
                .spawn(Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::px(3, EQUIP_CELL_WIDTH),
                    grid_template_rows: RepeatedGridTrack::auto(3),
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(EQUIP_GRID_GAP),
                    column_gap: Val::Px(EQUIP_GRID_GAP),
                    ..default()
                })
                .with_children(|grid| {
                    for &slot in grid_slots {
                        spawn_equip_slot_cell(
                            grid,
                            theme,
                            slot,
                            equip_slot_label(equipment, registry, slot),
                            images,
                        );
                    }
                });

            section
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|row| {
                    spawn_equip_slot_cell(
                        row,
                        theme,
                        mount_slot,
                        equip_slot_label(equipment, registry, mount_slot),
                        images,
                    );
                });
        });
}

/// Spawns one equipment slot cell: a small caption above a bordered box.
fn spawn_equip_slot_cell(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    slot: EquipSlot,
    label: String,
    images: &InventorySlotImages,
) {
    let has_item = label != EMPTY_SLOT_PLACEHOLDER;

    parent
        .spawn(Node {
            width: Val::Px(EQUIP_CELL_WIDTH),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            ..default()
        })
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
                images.clone(),
                EquipSlotButton { slot },
            ))
            .with_children(|button| {
                button.spawn((
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
