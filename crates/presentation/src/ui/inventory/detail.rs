//! Detail card spawner for selected inventory or equipment slots.

use bevy::prelude::*;
use bevymmo_gameplay::{
    items::{
        components::{Equipment, Inventory},
        effects::ItemEffect,
        registry::ItemRegistry,
    },
    stats::events::{ModifierOp, StatField},
};

use bevymmo_gameplay::abilities::KnownAncientLanguage;

use super::components::*;
use super::weapon_detail::{
    meta_line, summarize_weapon, GlyphRegistries, RuneSummary, SlotSummary,
};
use crate::ui::{
    button::{spawn_bar_child, BarButtonKind},
    card::{CardBuilder, CardFrameAssets, CardKind, CardWindow},
    theme::UiTheme,
};

/// Card height for a plain item: description plus a couple of stat lines.
const PLAIN_CARD_HEIGHT: f32 = 320.0;

/// Card height for an Eidolon weapon.
///
/// Three ability blocks of five lines each do not fit the plain height, and
/// clipping them would defeat the point of the card. Kept under the 600 px
/// default window (`bins/game/src/main.rs`) so the whole thing stays on screen
/// at the smallest supported resolution.
const WEAPON_CARD_HEIGHT: f32 = 520.0;
const CARD_WIDTH: f32 = 420.0;
const DESCRIPTION_FONT_SIZE: f32 = 16.0;
const DESCRIPTION_COLOR: Color = Color::srgba(0.78, 0.80, 0.86, 0.95);

#[allow(clippy::too_many_arguments)]
pub fn spawn_item_detail_card(
    commands: &mut Commands,
    theme: &UiTheme,
    registry: &ItemRegistry,
    glyphs: &GlyphRegistries,
    known: &KnownAncientLanguage,
    inventory: &Inventory,
    equipment: &Equipment,
    selection: InventorySelection,
    asset_server: &AssetServer,
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

    let weapon = summarize_weapon(&item_instance, registry, glyphs.catalog(), known);
    let height = if weapon.is_some() {
        WEAPON_CARD_HEIGHT
    } else {
        PLAIN_CARD_HEIGHT
    };
    let meta = meta_line(
        config.category,
        config.rarity,
        config.equippable_into,
        config.weight,
    );

    CardBuilder::new(CardKind::ItemDetail, config.display_name.to_string())
        .frame(CardFrameAssets::load(asset_server))
        .width(Val::Px(CARD_WIDTH))
        .height(Val::Px(height))
        .scrollable()
        .draggable()
        .closeable()
        .coexist()
        .with_body(move |body| {
            body.spawn((
                Text::new(meta),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.75),
                    ..default()
                },
                TextColor(Color::srgba(0.62, 0.68, 0.78, 0.95)),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                Node {
                    width: Val::Percent(100.0),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
            ));

            spawn_description_block(body, &config.description);

            if !effects.is_empty() {
                spawn_section_heading(body, theme, "EFFECTS");

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
                            format!("{op_str}{value} {field_str}")
                        }
                        ItemEffect::InstantHeal { amount } => format!("Instant Heal: {amount}"),
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

            let Some(weapon) = weapon else {
                return;
            };

            if let Some(runes) = &weapon.runes {
                spawn_section_heading(body, theme, "RUNES");
                spawn_rune_lines(body, theme, runes);
            }

            if !weapon.slots.is_empty() {
                spawn_section_heading(body, theme, "ABILITIES");
                for slot in &weapon.slots {
                    spawn_slot_block(body, theme, slot);
                }
            }
        })
        .with_footer(move |footer| {
            if let Some(slot) = equipped_slot {
                spawn_bar_child(
                    footer,
                    "Unequip",
                    theme.button_font_size,
                    theme.button_text_color,
                    Val::Percent(100.0),
                    Val::Px(36.0),
                    BarButtonKind::Neutral,
                    UnequipButton { slot },
                );
            } else if equippable_into.is_some() {
                if let Some(idx) = slot_index {
                    spawn_bar_child(
                        footer,
                        "Equip",
                        theme.button_font_size,
                        theme.button_text_color,
                        Val::Percent(100.0),
                        Val::Px(36.0),
                        BarButtonKind::Primary,
                        EquipButton { slot_index: idx },
                    );
                }
            }
        })
        .spawn(commands, theme);
}

fn spawn_description_block(body: &mut ChildSpawnerCommands, description: &str) {
    body.spawn(Node {
        width: Val::Percent(100.0),
        padding: UiRect {
            top: Val::Px(12.0),
            bottom: Val::Px(10.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
        },
        margin: UiRect::bottom(Val::Px(8.0)),
        ..default()
    })
    .with_children(|block| {
        block.spawn((
            Text::new(description.to_string()),
            TextFont {
                font_size: FontSize::Px(DESCRIPTION_FONT_SIZE),
                ..default()
            },
            TextColor(DESCRIPTION_COLOR),
            TextLayout {
                linebreak: LineBreak::WordOrCharacter,
                ..default()
            },
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
        ));
    });
}

fn spawn_rune_lines(body: &mut ChildSpawnerCommands, theme: &UiTheme, runes: &RuneSummary) {
    body.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        margin: UiRect::bottom(Val::Px(8.0)),
        ..default()
    })
    .with_children(|col| {
        for line in runes.lines() {
            col.spawn((
                Text::new(line),
                TextFont {
                    font_size: FontSize::Px(theme.button_font_size * 0.8),
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.7, 0.95, 1.0)),
            ));
        }
    });
}

fn spawn_section_heading(body: &mut ChildSpawnerCommands, theme: &UiTheme, label: &str) {
    body.spawn((
        Text::new(label.to_string()),
        TextFont {
            font_size: FontSize::Px(theme.button_font_size * 0.8),
            ..default()
        },
        TextColor(Color::srgba(0.6, 0.75, 0.95, 0.9)),
        Node {
            margin: UiRect {
                top: Val::Px(10.0),
                bottom: Val::Px(6.0),
                ..default()
            },
            ..default()
        },
    ));
}

/// One ability slot: which gesture is active, what it does, and what is
/// inscribed on it.
fn spawn_slot_block(body: &mut ChildSpawnerCommands, theme: &UiTheme, slot: &SlotSummary) {
    let detail_size = theme.button_font_size * 0.72;

    body.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        margin: UiRect {
            top: Val::Px(4.0),
            bottom: Val::Px(14.0),
            ..default()
        },
        ..default()
    })
    .with_children(|block| {
        block.spawn((
            Text::new(format!("{}   {}", slot.slot, slot.title)),
            TextFont {
                font_size: FontSize::Px(theme.button_font_size * 0.85),
                ..default()
            },
            TextColor(Color::srgba(0.95, 0.9, 0.7, 1.0)),
        ));

        if let Some(blocked) = &slot.blocked {
            block.spawn((
                Text::new(blocked.clone()),
                TextFont {
                    font_size: FontSize::Px(detail_size),
                    ..default()
                },
                TextColor(Color::srgba(0.95, 0.45, 0.4, 1.0)),
            ));
        }

        for (line, color) in [
            (
                format!("{}   |   {}", slot.shape, slot.tags),
                Color::srgba(0.7, 0.76, 0.86, 0.9),
            ),
            (slot.stats.clone(), Color::srgba(0.85, 0.88, 0.92, 0.95)),
        ] {
            block.spawn((
                Text::new(line),
                TextFont {
                    font_size: FontSize::Px(detail_size),
                    ..default()
                },
                TextColor(color),
            ));
        }

        if let Some(glyphs) = &slot.glyphs {
            block.spawn((
                Text::new(format!("Inscribed: {glyphs}")),
                TextFont {
                    font_size: FontSize::Px(detail_size),
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.7, 0.95, 1.0)),
            ));
        }

        if let Some(alternatives) = &slot.alternatives {
            block.spawn((
                Text::new(alternatives.clone()),
                TextFont {
                    font_size: FontSize::Px(detail_size),
                    ..default()
                },
                TextColor(Color::srgba(0.6, 0.65, 0.72, 0.85)),
            ));
        }
    });
}

pub fn despawn_detail_cards(commands: &mut Commands, cards: &Query<(Entity, &CardWindow)>) {
    for (entity, window) in cards.iter() {
        if window.kind == CardKind::ItemDetail {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> UiTheme {
        UiTheme::default()
    }

    #[test]
    fn description_wraps_on_words_with_readable_style() {
        let mut app = App::new();
        let mut commands = app.world_mut().commands();
        let root = commands.spawn(Node::default()).id();
        commands.entity(root).with_children(|body| {
            spawn_description_block(
                body,
                "A long flavor sentence that should wrap on words, not mid-glyph.",
            );
        });
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(&Text, &TextLayout, &TextColor, &TextFont)>();
        let (_, layout, color, font) = query
            .iter(world)
            .find(|(text, _, _, _)| text.0.contains("flavor sentence"))
            .expect("description text");
        assert_eq!(layout.linebreak, LineBreak::WordOrCharacter);
        assert_eq!(color.0, DESCRIPTION_COLOR);
        assert_eq!(font.font_size, FontSize::Px(DESCRIPTION_FONT_SIZE));
    }

    #[test]
    fn rune_block_spawns_one_line_per_stat() {
        let mut app = App::new();
        let theme = test_theme();
        let runes = RuneSummary {
            used: 3,
            capacity: 12,
            stability: 0.8,
            root_word: Some("Danno".to_string()),
        };
        let mut commands = app.world_mut().commands();
        let root = commands.spawn(Node::default()).id();
        commands.entity(root).with_children(|body| {
            spawn_rune_lines(body, &theme, &runes);
        });
        app.update();

        let world = app.world_mut();
        let mut texts = world.query::<&Text>();
        let lines: Vec<&str> = texts.iter(world).map(|text| text.0.as_str()).collect();
        assert!(lines.iter().any(|line| *line == "3/12 capacity"));
        assert!(lines.iter().any(|line| *line == "80% stability"));
        assert!(lines.iter().any(|line| *line == "Root Word: Danno"));
        assert!(
            lines.iter().all(|line| !line.contains(" | ")),
            "rune stats must not share a piped row"
        );
    }
}
