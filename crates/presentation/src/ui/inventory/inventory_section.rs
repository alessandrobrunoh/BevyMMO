//! Scrollable inventory grid shown below the fixed equipment section.

use bevy::prelude::*;
use bevymmo_gameplay::items::{
    components::{Inventory, INVENTORY_CAPACITY},
    registry::ItemRegistry,
};

use super::components::{InventorySlotImages, ItemSlotButton, ItemSlotIcon, ItemSlotText};
use super::load_item_icon;
use crate::ui::{scrollbar::spawn_scroll_view, theme::UiTheme};

const INVENTORY_GRID_COLUMNS: u16 = 4;
const INVENTORY_GRID_ROWS: u16 =
    INVENTORY_CAPACITY.div_ceil(INVENTORY_GRID_COLUMNS as usize) as u16;
const INVENTORY_SLOT_SIZE: f32 = 58.0;
const INVENTORY_SLOT_GAP: f32 = 6.0;
const INVENTORY_GRID_PADDING_Y: f32 = 4.0;

/// Scrollable section containing the character's carried items.
#[derive(Component, Debug)]
pub struct InventoryPanel;

pub(super) fn spawn_inventory_section(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    inventory: &Inventory,
    registry: &ItemRegistry,
    images: &InventorySlotImages,
    asset_server: &AssetServer,
) {
    let slot_visuals: Vec<(bool, String, Option<Handle<Image>>)> = (0..INVENTORY_CAPACITY)
        .map(|index| {
            let Some(instance) = inventory.slots.get(index).and_then(|item| item.as_ref()) else {
                return (false, String::new(), None);
            };
            let icon = load_item_icon(asset_server, registry, &instance.item_id);
            let label = if icon.is_some() {
                if instance.quantity > 1 {
                    instance.quantity.to_string()
                } else {
                    String::new()
                }
            } else {
                let display = registry
                    .get(&instance.item_id)
                    .map(|item| item.display_name().to_string())
                    .unwrap_or_else(|| instance.item_id.as_str().to_string());
                if instance.quantity > 1 {
                    format!("{display} x{}", instance.quantity)
                } else {
                    display
                }
            };
            (true, label, icon)
        })
        .collect();

    let section = parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            InventoryPanel,
        ))
        .id();

    let images = images.clone();
    let text_color = theme.text_color;
    let font_size = theme.button_font_size * 0.55;
    let mut commands = parent.commands();
    spawn_scroll_view(&mut commands, section, theme, move |commands| {
        commands
            .spawn(Node::default())
            .with_children(|content| {
                content
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        display: Display::Grid,
                        grid_template_columns: RepeatedGridTrack::px(
                            INVENTORY_GRID_COLUMNS,
                            INVENTORY_SLOT_SIZE,
                        ),
                        grid_template_rows: RepeatedGridTrack::px(
                            INVENTORY_GRID_ROWS,
                            INVENTORY_SLOT_SIZE,
                        ),
                        row_gap: Val::Px(INVENTORY_SLOT_GAP),
                        column_gap: Val::Px(INVENTORY_SLOT_GAP),
                        justify_content: JustifyContent::Center,
                        padding: UiRect::vertical(Val::Px(INVENTORY_GRID_PADDING_Y)),
                        ..default()
                    })
                    .with_children(|grid| {
                        for (index, (has_item, item_name, icon)) in
                            slot_visuals.into_iter().enumerate()
                        {
                            let icon_visible = icon.is_some();

                            grid.spawn((
                                Button,
                                Node {
                                    width: Val::Px(INVENTORY_SLOT_SIZE),
                                    height: Val::Px(INVENTORY_SLOT_SIZE),
                                    position_type: PositionType::Relative,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(3.0)),
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
                                ItemSlotButton { index: index as u8 },
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(6.0),
                                        right: Val::Px(6.0),
                                        top: Val::Px(6.0),
                                        bottom: Val::Px(6.0),
                                        ..default()
                                    },
                                    ImageNode {
                                        image: icon.unwrap_or_default(),
                                        image_mode: NodeImageMode::Stretch,
                                        ..default()
                                    },
                                    if icon_visible {
                                        Visibility::Inherited
                                    } else {
                                        Visibility::Hidden
                                    },
                                    ItemSlotIcon { index: index as u8 },
                                ));
                                button.spawn((
                                    Text::new(item_name),
                                    TextFont {
                                        font_size: FontSize::Px(font_size),
                                        ..default()
                                    },
                                    TextColor(text_color),
                                    TextLayout::justify(Justify::Center),
                                    ItemSlotText { index: index as u8 },
                                ));
                            });
                        }
                    });
            })
            .id()
    });
}
