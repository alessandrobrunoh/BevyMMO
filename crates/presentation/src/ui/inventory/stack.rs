//! Split / combine controls on the item-detail footer.

use bevy::prelude::*;
use bevymmo_gameplay::items::{components::Inventory, registry::ItemRegistry};

use super::components::{
    CombineButton, InventorySelection, SplitAmountField, SplitAmountStep, SplitAmountText,
    SplitButton,
};
use crate::ui::{
    button::{spawn_bar_child, BarButtonKind},
    theme::UiTheme,
};

const STEPPER_SIZE: f32 = 32.0;
const AMOUNT_WIDTH: f32 = 72.0;
const CONTROL_HEIGHT: f32 = 36.0;

/// What the item-info footer should offer for the current selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StackFooter {
    pub slot_index: u8,
    pub quantity: u32,
    pub can_split: bool,
    pub can_combine: bool,
}

/// Split / Combine only for mergeable Material piles in the bag.
pub(super) fn stack_footer(
    inventory: &Inventory,
    registry: &ItemRegistry,
    selection: InventorySelection,
) -> Option<StackFooter> {
    let InventorySelection::Slot(index) = selection else {
        return None;
    };
    let instance = inventory.slots.get(index as usize)?.as_ref()?;
    if !instance.is_stack_mergeable() {
        return None;
    }
    let item = registry.get(&instance.item_id)?;
    if !Inventory::stacks_category(item.config().category) {
        return None;
    }
    let can_split = instance.quantity >= 2;
    let can_combine = inventory.has_other_mergeable_stack(index as usize, true);
    if !can_split && !can_combine {
        return None;
    }
    Some(StackFooter {
        slot_index: index,
        quantity: instance.quantity,
        can_split,
        can_combine,
    })
}

/// Parses a typed amount and clamps it into the legal split window.
pub(super) fn parse_split_amount(raw: &str, quantity: u32) -> u32 {
    let parsed = raw.parse::<u32>().unwrap_or(0);
    Inventory::clamp_split_amount(parsed, quantity)
}

/// Steps the peel-off amount by `delta`, staying inside `1 ..= quantity - 1`.
pub(super) fn step_split_amount(current: u32, delta: i32, quantity: u32) -> u32 {
    let next = i64::from(current).saturating_add(i64::from(delta));
    let as_u32 = u32::try_from(next.max(0)).unwrap_or(0);
    Inventory::clamp_split_amount(as_u32, quantity)
}

pub(super) fn resolved_split_amount(stored: u32, quantity: u32) -> u32 {
    if stored == 0 {
        Inventory::default_split_amount(quantity)
    } else {
        Inventory::clamp_split_amount(stored, quantity)
    }
}

pub(super) fn spawn_stack_controls(
    footer: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    stack: StackFooter,
    amount: u32,
) {
    footer
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|column| {
            if stack.can_split {
                spawn_split_row(column, theme, stack.slot_index, stack.quantity, amount);
            }
            if stack.can_combine {
                spawn_bar_child(
                    column,
                    "Combine",
                    theme.button_font_size,
                    theme.button_text_color,
                    Val::Percent(100.0),
                    Val::Px(CONTROL_HEIGHT),
                    BarButtonKind::Neutral,
                    CombineButton {
                        slot_index: stack.slot_index,
                    },
                );
            }
        });
}

fn spawn_split_row(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    slot_index: u8,
    quantity: u32,
    amount: u32,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            spawn_stepper(row, theme, -1);
            spawn_amount_field(row, theme, quantity, amount);
            spawn_stepper(row, theme, 1);
            spawn_bar_child(
                row,
                "Split",
                theme.button_font_size,
                theme.button_text_color,
                Val::Px(160.0),
                Val::Px(CONTROL_HEIGHT),
                BarButtonKind::Primary,
                SplitButton { slot_index },
            );
        });
}

fn spawn_stepper(parent: &mut ChildSpawnerCommands, theme: &UiTheme, delta: i32) {
    let label = if delta < 0 { "−" } else { "+" };
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(STEPPER_SIZE),
                height: Val::Px(STEPPER_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(theme.button_bg),
            BorderColor::all(theme.input_border),
            SplitAmountStep { delta },
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size),
                    ..default()
                },
                TextColor(theme.button_text_color),
            ));
        });
}

fn spawn_amount_field(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    quantity: u32,
    amount: u32,
) {
    let value = amount.to_string();
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(AMOUNT_WIDTH),
                height: Val::Px(STEPPER_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(theme.input_bg),
            BorderColor::all(theme.input_border),
            SplitAmountField {
                value: value.clone(),
                focused: false,
                quantity,
            },
        ))
        .with_children(|field| {
            field.spawn((
                Text::new(value),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.85),
                    ..default()
                },
                TextColor(theme.text_color),
                SplitAmountText,
                Pickable::IGNORE,
            ));
        });
}

/// Title shown on the item-info card: `Wood x50` when the pile is stacked.
pub(super) fn stack_title(name: &str, quantity: u32) -> String {
    if quantity > 1 {
        format!("{name} x{quantity}")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::items::{
        components::EquipSlot,
        instance::{ItemInstance, ItemInstanceId},
        registry::ItemId,
        ItemCategory,
    };

    fn wood(id: u64, quantity: u32) -> ItemInstance {
        let mut instance = ItemInstance::new(ItemId::new("wood"));
        instance.instance_id = ItemInstanceId(id);
        instance.quantity = quantity;
        instance
    }

    fn sword(id: u64) -> ItemInstance {
        let mut instance = ItemInstance::new(ItemId::new("sword"));
        instance.instance_id = ItemInstanceId(id);
        instance
    }

    fn registry() -> ItemRegistry {
        bevymmo_content::item_definitions::default_items()
    }

    #[test]
    fn stack_footer_offers_split_for_a_material_pile() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(wood(1, 50));
        let footer =
            stack_footer(&inventory, &registry(), InventorySelection::Slot(0)).expect("wood stack");
        assert!(footer.can_split);
        assert!(!footer.can_combine);
        assert_eq!(footer.quantity, 50);
    }

    #[test]
    fn stack_footer_offers_combine_when_another_pile_exists() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(wood(1, 10));
        inventory.slots[2] = Some(wood(2, 8));
        let footer =
            stack_footer(&inventory, &registry(), InventorySelection::Slot(0)).expect("combine");
        assert!(footer.can_split);
        assert!(footer.can_combine);
    }

    #[test]
    fn stack_footer_hidden_for_weapons_and_equipment() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(sword(1));
        assert_eq!(
            stack_footer(&inventory, &registry(), InventorySelection::Slot(0)),
            None
        );
        assert_eq!(
            stack_footer(
                &inventory,
                &registry(),
                InventorySelection::Equipment(EquipSlot::Weapon)
            ),
            None
        );
        assert_eq!(
            stack_footer(&inventory, &registry(), InventorySelection::Slot(3)),
            None
        );
    }

    #[test]
    fn stack_footer_hidden_for_a_single_piece_with_no_siblings() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(wood(1, 1));
        assert_eq!(
            stack_footer(&inventory, &registry(), InventorySelection::Slot(0)),
            None
        );
    }

    #[test]
    fn parse_and_step_stay_inside_the_split_window() {
        assert_eq!(parse_split_amount("7", 50), 7);
        assert_eq!(parse_split_amount("", 50), 1);
        assert_eq!(parse_split_amount("50", 50), 49);
        assert_eq!(parse_split_amount("abc", 50), 1);
        assert_eq!(step_split_amount(7, -1, 50), 6);
        assert_eq!(step_split_amount(1, -1, 50), 1);
        assert_eq!(step_split_amount(49, 1, 50), 49);
        assert_eq!(resolved_split_amount(0, 50), 25);
        assert_eq!(resolved_split_amount(7, 50), 7);
        assert_eq!(resolved_split_amount(7, 5), 4);
    }

    #[test]
    fn stack_title_includes_quantity_only_when_stacked() {
        assert_eq!(stack_title("Wood", 50), "Wood x50");
        assert_eq!(stack_title("Wood", 1), "Wood");
    }

    #[test]
    fn materials_are_the_only_stacking_category() {
        assert!(Inventory::stacks_category(ItemCategory::Material));
        assert!(!Inventory::stacks_category(ItemCategory::Weapon));
    }
}
