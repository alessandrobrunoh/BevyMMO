//! Detail card spawner for selected inventory or equipment slots.

use bevy::prelude::*;
use bevymmo_shared::{
    items::{
        components::{Equipment, Inventory},
        effects::ItemEffect,
        registry::ItemRegistry,
    },
    stats::events::{ModifierOp, StatField},
};

use super::components::*;
use crate::ui::{
    card::{
        builder::CardBuilder,
        components::{CardKind, CardWindow},
    },
    theme::UiTheme,
};

pub fn spawn_item_detail_card(
    commands: &mut Commands,
    theme: &UiTheme,
    registry: &ItemRegistry,
    inventory: &Inventory,
    equipment: &Equipment,
    selection: InventorySelection,
) {
    let (item_instance, equipped_slot, slot_index) = match selection {
        InventorySelection::Slot(idx) => {
            let item_instance = inventory.slots.get(idx as usize).and_then(|s| s.clone());
            let equipped_slot = item_instance
                .as_ref()
                .and_then(|instance| equipment.slot_holding(instance.instance_id));
            (item_instance, equipped_slot, Some(idx))
        }
        InventorySelection::Equipment(slot) => (equipment.get(slot).clone(), Some(slot), None),
    };

    let Some(item_instance) = item_instance else {
        return;
    };

    let Some(item) = registry.get(&item_instance.item_id) else {
        return;
    };

    let config = item.config();
    let effects = item.effects().to_vec();
    let equippable_into = config.equippable_into;

    CardBuilder::new(CardKind::ItemDetail, config.display_name.to_string())
        .width(Val::Px(380.0))
        .height(Val::Px(320.0))
        .draggable()
        .closeable()
        .coexist()
        .with_body(move |body| {
            body.spawn((
                Text::new(config.description.to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.85),
                    ..default()
                },
                TextColor(Color::srgba(0.85, 0.88, 0.92, 0.9)),
                Node {
                    margin: UiRect::bottom(Val::Px(14.0)),
                    ..default()
                },
            ));

            if !effects.is_empty() {
                body.spawn((
                    Text::new("EFFECTS".to_string()),
                    TextFont {
                        font_size: FontSize::Px(theme.button_font_size * 0.8),
                        ..default()
                    },
                    TextColor(Color::srgba(0.6, 0.75, 0.95, 0.9)),
                    Node {
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    },
                ));

                for effect in effects {
                    let desc = match effect {
                        ItemEffect::StatBonus { field, op, value } => {
                            let field_str = match field {
                                StatField::MaxHealth => "Max Health",
                                StatField::Speed => "Speed",
                                StatField::AttackPower => "Attack Power",
                                StatField::Armor => "Armor",
                                StatField::ManaRegeneration => "Mana Regen",
                            };
                            let op_str = match op {
                                ModifierOp::Add => "+",
                                ModifierOp::Multiply => "x",
                                ModifierOp::Override => "=",
                            };
                            format!("✦ {op_str}{value} {field_str}")
                        }
                        ItemEffect::InstantHeal { amount } => format!("✦ Instant Heal: {amount}"),
                    };

                    body.spawn((
                        Text::new(desc),
                        TextFont {
                            font_size: FontSize::Px(theme.button_font_size * 0.85),
                            ..default()
                        },
                        TextColor(Color::srgba(0.4, 0.9, 0.6, 1.0)),
                    ));
                }
            }
        })
        .with_footer(move |footer| {
            if let Some(slot) = equipped_slot {
                footer
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.7, 0.25, 0.25, 0.8)),
                        UnequipButton { slot },
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("Unequip".to_string()),
                            TextFont {
                                font_size: FontSize::Px(theme.button_font_size),
                                ..default()
                            },
                            TextColor(theme.text_color),
                        ));
                    });
            } else if equippable_into.is_some() {
                if let Some(idx) = slot_index {
                    footer
                        .spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(36.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.25, 0.6, 0.35, 0.85)),
                            EquipButton { slot_index: idx },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Equip".to_string()),
                                TextFont {
                                    font_size: FontSize::Px(theme.button_font_size),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));
                        });
                }
            }
        })
        .spawn(commands, theme);
}

pub fn despawn_detail_cards(commands: &mut Commands, cards: &Query<(Entity, &CardWindow)>) {
    for (entity, window) in cards.iter() {
        if window.kind == CardKind::ItemDetail {
            commands.entity(entity).despawn();
        }
    }
}
