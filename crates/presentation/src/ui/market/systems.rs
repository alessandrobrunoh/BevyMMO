//! Click-to-open market NPC, offer list, buy, list-from-inventory, item ticket.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::pointer::{hud_wants_pointer, PointerOnHud};
use bevymmo_client::stdb::module_bindings::{MarketBuyOrder, MarketSellOrder};
use bevymmo_client::stdb::{
    commands, LocalGold, MarketBuyBook, MarketOrderBook, NpcMarket, StdbConnection,
};
use bevymmo_gameplay::economy::{
    quote_fee, FeeQuote, GoldError, BPS_DENOMINATOR, DEFAULT_ACCOUNT_FEE_BPS,
};
use bevymmo_gameplay::entity::components::{EntityKind, GameEntity};
use bevymmo_gameplay::items::components::Inventory;
use bevymmo_gameplay::items::registry::{ItemId, ItemRegistry};
use bevymmo_gameplay::markets::{
    assert_item_marketable, MarketError, MarketRegistry, MARKET_1_FEE_BPS, MARKET_1_ID,
    MARKET_2_FEE_BPS, MARKET_2_ID,
};
use bevymmo_network::network::protocol::Position;
use bevymmo_network::world_components::NetworkEntityId;

use super::{
    MarketBagList, MarketCancelBuyButton, MarketCancelSellButton, MarketCard, MarketGoldText,
    MarketInventoryPanel, MarketOfferButton, MarketOfferName, MarketOfferPrice, MarketOffersPanel,
    MarketOpenTicket, MarketPriceBump, MarketSellFromBag, MarketTab, MarketTabButton,
    MarketTicketAction, MarketTicketActionButton, MarketTicketBuyList, MarketTicketCard,
    MarketTicketCreateButton, MarketTicketFeeText, MarketTicketPriceText, MarketTicketSellList,
    MarketUiState,
};
use crate::renderer;
use crate::ui::button::{spawn_bar_child, BarButtonKind, UiButtonImages};
use crate::ui::card::{
    builder::{CardBuilder, CardFrameAssets},
    components::{CardKind, CardPositioning},
};
use crate::ui::inventory::components::InventorySelection;
use crate::ui::inventory::InventoryUiState;
use crate::ui::npc_sidebar::systems::{closest_friendly_hit, EntityHit};
use crate::ui::theme::UiTheme;

const NPC_SELECT_RADIUS: f32 = 1.2;

fn market_catalog() -> &'static MarketRegistry {
    static CATALOG: std::sync::OnceLock<MarketRegistry> = std::sync::OnceLock::new();
    CATALOG.get_or_init(bevymmo_content::market_definitions::default_markets)
}

fn item_catalog() -> &'static ItemRegistry {
    static CATALOG: std::sync::OnceLock<ItemRegistry> = std::sync::OnceLock::new();
    CATALOG.get_or_init(bevymmo_content::item_definitions::default_items)
}

/// Occupied bag slots as `(index, item_id)`. Empty slots are omitted.
pub fn occupied_inventory_rows(inventory: &Inventory) -> Vec<(u8, String)> {
    inventory
        .slots
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| {
            slot.as_ref()
                .map(|item| (index as u8, item.item_id.as_str().to_string()))
        })
        .collect()
}

/// Occupied slots this hall will actually list: on the allowlist and `tradable`.
/// Bound items never appear in the sell list.
pub fn listable_inventory_rows(inventory: &Inventory, market_id: &str) -> Vec<(u8, String)> {
    occupied_inventory_rows(inventory)
        .into_iter()
        .filter(|(_, item_id)| item_allowed_in_open_market(market_id, item_id))
        .collect()
}

pub fn item_allowed_in_open_market(market_id: &str, item_id: &str) -> bool {
    item_market_status(market_id, item_id).is_ok()
}

fn item_market_status(market_id: &str, item_id: &str) -> Result<(), MarketError> {
    let market = market_catalog()
        .get(market_id)
        .ok_or(MarketError::UnknownMarket)?;
    let item_id = ItemId::new(item_id.to_string());
    let item = item_catalog()
        .get(&item_id)
        .ok_or(MarketError::ItemNotAllowed)?;
    assert_item_marketable(market, &item_id, item.tradable())
}

pub fn fee_bps_for_market(market_id: &str) -> u16 {
    match market_id {
        MARKET_1_ID => MARKET_1_FEE_BPS,
        MARKET_2_ID => MARKET_2_FEE_BPS,
        _ => MARKET_1_FEE_BPS,
    }
}

pub fn quote_ticket_fee(price: u64, market_id: &str) -> Result<FeeQuote, GoldError> {
    quote_fee(
        price,
        fee_bps_for_market(market_id),
        DEFAULT_ACCOUNT_FEE_BPS,
    )
}

/// Display numbers for Screen B. Market/account cuts are quoted separately;
/// `total_fee` is [`quote_fee`]'s combined cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TicketFeeLines {
    pub market_fee: u64,
    pub account_fee: u64,
    pub total_fee: u64,
    pub you_pay: u64,
    pub you_receive: u64,
}

pub fn ticket_fee_lines(price: u64, market_id: &str) -> Option<TicketFeeLines> {
    let quote = quote_ticket_fee(price, market_id).ok()?;
    let market_bps = u128::from(fee_bps_for_market(market_id));
    let account_bps = u128::from(DEFAULT_ACCOUNT_FEE_BPS);
    let price128 = u128::from(price);
    let denom = u128::from(BPS_DENOMINATOR);
    Some(TicketFeeLines {
        market_fee: (price128 * market_bps / denom) as u64,
        account_fee: (price128 * account_bps / denom) as u64,
        total_fee: quote.fee_gold,
        you_pay: quote.buyer_pays.amount(),
        you_receive: quote.seller_receives.amount(),
    })
}

pub fn format_ticket_fees(lines: &TicketFeeLines) -> String {
    format!(
        "Market fee: {}\nAccount fee: {}\nTotal fee: {}\nYou pay: {}\nYou receive: {}",
        lines.market_fee, lines.account_fee, lines.total_fee, lines.you_pay, lines.you_receive
    )
}

pub fn filter_book_ids<'a>(
    rows: impl Iterator<Item = (&'a str, &'a str, u64)>,
    market_id: &str,
    item_id: &str,
) -> Vec<u64> {
    rows.filter(|(row_market, row_item, _)| *row_market == market_id && *row_item == item_id)
        .map(|(_, _, id)| id)
        .collect()
}

pub fn offers_for_market<'a>(
    orders: impl Iterator<Item = &'a MarketSellOrder>,
    market_id: &str,
) -> Vec<&'a MarketSellOrder> {
    let mut rows: Vec<_> = orders.filter(|row| row.market_id == market_id).collect();
    rows.sort_by_key(|row| (row.item_id.clone(), row.price_gold, row.id));
    rows
}

pub fn offers_for_item<'a>(
    orders: impl Iterator<Item = &'a MarketSellOrder>,
    market_id: &str,
    item_id: &str,
) -> Vec<&'a MarketSellOrder> {
    let mut rows: Vec<_> = orders
        .filter(|row| row.market_id == market_id && row.item_id == item_id)
        .collect();
    rows.sort_by_key(|row| (row.price_gold, row.id));
    rows
}

pub fn bids_for_item<'a>(
    orders: impl Iterator<Item = &'a MarketBuyOrder>,
    market_id: &str,
    item_id: &str,
) -> Vec<&'a MarketBuyOrder> {
    let mut rows: Vec<_> = orders
        .filter(|row| row.market_id == market_id && row.item_id == item_id)
        .collect();
    rows.sort_by_key(|row| (std::cmp::Reverse(row.price_gold), row.id));
    rows
}

pub fn close_ticket_if_browse_closed(
    mut commands: Commands,
    browse: Query<(), With<MarketCard>>,
    tickets: Query<Entity, With<MarketTicketCard>>,
) {
    if !browse.is_empty() {
        return;
    }
    for entity in tickets.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn npc_market_on_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    pointer_on_hud: Res<PointerOnHud>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &Transform), With<Camera3d>>,
    theme: Res<UiTheme>,
    asset_server: Res<AssetServer>,
    mut ui: ResMut<MarketUiState>,
    entity_query: Query<(Entity, &Position, &EntityKind, Option<&NpcMarket>), With<GameEntity>>,
    existing: Query<Entity, Or<(With<MarketCard>, With<MarketTicketCard>)>>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if hud_wants_pointer(&pointer_on_hud) {
        return;
    }
    let Some(ray) = cursor_ray(&windows, &cameras) else {
        return;
    };
    let mut hits: Vec<EntityHit> = Vec::new();
    let mut market_of: std::collections::HashMap<Entity, String> = std::collections::HashMap::new();
    for (entity, position, kind, npc_market) in entity_query.iter() {
        if *kind != EntityKind::Friendly {
            continue;
        }
        let Some(npc_market) = npc_market else {
            continue;
        };
        let distance = point_to_ray_distance(position.0, ray.origin, *ray.direction);
        if distance > NPC_SELECT_RADIUS {
            continue;
        }
        hits.push(EntityHit { entity, distance });
        market_of.insert(entity, npc_market.market_id.clone());
    }
    let Some(target) = closest_friendly_hit(&hits) else {
        return;
    };
    let Some(market_id) = market_of.remove(&target) else {
        return;
    };
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
    ui.open_market_id = Some(market_id.clone());
    ui.npc = Some(target);
    ui.tab = MarketTab::Offers;
    ui.bag_slot = None;
    ui.ticket_action = MarketTicketAction::Buy;
    if ui.list_price == 0 {
        ui.list_price = 1;
    }
    spawn_market_card(&mut commands, &theme, &asset_server, &market_id);
}

const MARKET_CARD_WIDTH: f32 = 980.0;
const MARKET_CARD_HEIGHT: f32 = 640.0;
const TICKET_CARD_WIDTH: f32 = 1080.0;
const TICKET_CARD_HEIGHT: f32 = 640.0;

fn spawn_market_card(
    commands: &mut Commands,
    theme: &UiTheme,
    asset_server: &AssetServer,
    market_id: &str,
) {
    let title = match market_id {
        "market_1" => "Market 1",
        "market_2" => "Market 2",
        other => other,
    };
    let text_color = theme.text_color;
    let card = CardBuilder::new(CardKind::Market, title)
        .frame(CardFrameAssets::load(asset_server))
        .width(Val::Px(MARKET_CARD_WIDTH))
        .height(Val::Px(MARKET_CARD_HEIGHT))
        .positioning(CardPositioning::Center)
        .draggable()
        .closeable()
        .coexist()
        .with_body(move |body| {
            body.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|root| {
                root.spawn(Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|bar| {
                    bar.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|tabs| {
                        spawn_bar_child(
                            tabs,
                            "Offers",
                            15.0,
                            text_color,
                            Val::Px(140.0),
                            Val::Px(32.0),
                            BarButtonKind::Primary,
                            MarketTabButton {
                                tab: MarketTab::Offers,
                            },
                        );
                        spawn_bar_child(
                            tabs,
                            "Inventory",
                            15.0,
                            text_color,
                            Val::Px(140.0),
                            Val::Px(32.0),
                            BarButtonKind::Neutral,
                            MarketTabButton {
                                tab: MarketTab::Inventory,
                            },
                        );
                    });
                    bar.spawn((
                        Text::new("Gold: 0"),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.82, 0.35)),
                        MarketGoldText,
                    ));
                });
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        min_height: Val::Px(0.0),
                        ..default()
                    },
                    MarketOffersPanel,
                ))
                .with_children(|offers| {
                    spawn_offer_header(offers, text_color);
                    offers.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(6.0),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        MarketOfferList,
                    ));
                });
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        min_height: Val::Px(0.0),
                        display: Display::None,
                        ..default()
                    },
                    MarketInventoryPanel,
                ))
                .with_children(|bag| {
                    spawn_bag_header(bag, text_color);
                    bag.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(6.0),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        MarketBagList,
                    ));
                });
            });
        })
        .spawn(commands, theme);
    commands.entity(card).insert(MarketCard);
}

pub fn select_market_tab(
    interactions: Query<(&Interaction, &MarketTabButton), Changed<Interaction>>,
    mut ui: ResMut<MarketUiState>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        ui.tab = button.tab;
    }
}

pub fn sync_market_tab_visibility(
    ui: Res<MarketUiState>,
    asset_server: Res<AssetServer>,
    mut offers: Query<&mut Node, (With<MarketOffersPanel>, Without<MarketInventoryPanel>)>,
    mut bag: Query<&mut Node, (With<MarketInventoryPanel>, Without<MarketOffersPanel>)>,
    mut tabs: Query<(&MarketTabButton, &mut UiButtonImages, &mut ImageNode)>,
    mut last: Local<Option<MarketTab>>,
) {
    if offers.is_empty() && bag.is_empty() {
        *last = None;
        return;
    }
    if *last == Some(ui.tab) {
        return;
    }
    *last = Some(ui.tab);
    let offers_on = ui.tab == MarketTab::Offers;
    for mut node in offers.iter_mut() {
        node.display = if offers_on {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut node in bag.iter_mut() {
        node.display = if offers_on {
            Display::None
        } else {
            Display::Flex
        };
    }
    for (button, mut images, mut image) in tabs.iter_mut() {
        let kind = if button.tab == ui.tab {
            BarButtonKind::Primary
        } else {
            BarButtonKind::Neutral
        };
        *images = UiButtonImages::load_kind(&asset_server, kind);
        image.image = images.default.clone();
    }
}

fn spawn_offer_header(parent: &mut ChildSpawnerCommands, text_color: Color) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            border: UiRect::bottom(Val::Px(1.0)),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Text::new("Item"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(text_color.with_alpha(0.7)),
            ));
            row.spawn((
                Node {
                    width: Val::Px(120.0),
                    ..default()
                },
                Text::new("Price"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(text_color.with_alpha(0.7)),
            ));
            row.spawn((
                Node {
                    width: Val::Px(100.0),
                    ..default()
                },
                Text::new(""),
            ));
        });
}

fn spawn_bag_header(parent: &mut ChildSpawnerCommands, text_color: Color) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            border: UiRect::bottom(Val::Px(1.0)),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Text::new("Item"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(text_color.with_alpha(0.7)),
            ));
            row.spawn((
                Node {
                    width: Val::Px(120.0),
                    ..default()
                },
                Text::new(""),
            ));
        });
}

fn spawn_price_bumps(parent: &mut ChildSpawnerCommands, theme: &UiTheme) {
    let bumps = [(-100_i64, "−100"), (-10, "−10"), (10, "+10"), (100, "+100")];
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            for (delta, label) in bumps {
                spawn_bar_child(
                    row,
                    label,
                    13.0,
                    theme.text_color,
                    Val::Px(72.0),
                    Val::Px(28.0),
                    BarButtonKind::Neutral,
                    MarketPriceBump { delta },
                );
            }
        });
}

#[derive(Component)]
pub struct MarketOfferList;

#[derive(Default)]
pub struct OfferListFingerprint {
    rows: Vec<(u64, u64, String)>,
}

#[allow(clippy::too_many_arguments)]
pub fn refresh_market_rows(
    mut commands: Commands,
    theme: Res<UiTheme>,
    ui: Res<MarketUiState>,
    gold: Res<LocalGold>,
    book: Res<MarketOrderBook>,
    registry: Res<ItemRegistry>,
    mut gold_text: Query<&mut Text, With<MarketGoldText>>,
    lists: Query<Entity, With<MarketOfferList>>,
    cards: Query<(), With<MarketCard>>,
    mut fingerprint: Local<OfferListFingerprint>,
) {
    if cards.is_empty() {
        fingerprint.rows.clear();
        return;
    }
    for mut text in gold_text.iter_mut() {
        text.0 = format!("Gold: {}", gold.amount);
    }
    let Some(market_id) = ui.open_market_id.as_deref() else {
        return;
    };
    let offers = offers_for_market(book.orders.values(), market_id);
    let next: Vec<(u64, u64, String)> = offers
        .iter()
        .map(|order| (order.id, order.price_gold, order.item_id.clone()))
        .collect();
    if fingerprint.rows == next {
        return;
    }
    fingerprint.rows = next;
    for list in lists.iter() {
        commands.entity(list).despawn_related::<Children>();
        commands.entity(list).with_children(|parent| {
            if offers.is_empty() {
                parent.spawn((
                    Text::new("No offers in this market yet. Open Inventory and press Sell."),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(theme.text_color.with_alpha(0.75)),
                ));
                return;
            }
            for order in &offers {
                let name = registry
                    .get(&bevymmo_gameplay::items::registry::ItemId::new(
                        order.item_id.clone(),
                    ))
                    .map(|item| item.display_name().to_string())
                    .unwrap_or_else(|| order.item_id.clone());
                let item_id = order.item_id.clone();
                parent
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(40.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Button,
                            Node {
                                flex_grow: 1.0,
                                justify_content: JustifyContent::FlexStart,
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            MarketOpenTicket {
                                item_id,
                                price_gold: order.price_gold,
                            },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(name),
                                TextFont {
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                                MarketOfferName,
                            ));
                        });
                        row.spawn((
                            Node {
                                width: Val::Px(120.0),
                                ..default()
                            },
                            Text::new(format!("{}", order.price_gold)),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.95, 0.82, 0.35)),
                            MarketOfferPrice,
                        ));
                        spawn_bar_child(
                            row,
                            "Buy",
                            14.0,
                            theme.text_color,
                            Val::Px(96.0),
                            Val::Px(30.0),
                            BarButtonKind::Primary,
                            MarketOfferButton { order_id: order.id },
                        );
                    });
            }
        });
    }
}

#[derive(Default)]
pub struct BagListFingerprint {
    rows: Vec<(u8, String)>,
}

#[allow(clippy::too_many_arguments)]
pub fn refresh_bag_rows(
    mut commands: Commands,
    theme: Res<UiTheme>,
    ui: Res<MarketUiState>,
    registry: Res<ItemRegistry>,
    inventory: Query<&Inventory, With<LocalPlayer>>,
    lists: Query<Entity, With<MarketBagList>>,
    cards: Query<(), With<MarketCard>>,
    mut fingerprint: Local<BagListFingerprint>,
) {
    if cards.is_empty() {
        fingerprint.rows.clear();
        return;
    }
    let Some(market_id) = ui.open_market_id.as_deref() else {
        return;
    };
    let Ok(inventory) = inventory.single() else {
        return;
    };
    let occupied = occupied_inventory_rows(inventory);
    if fingerprint.rows == occupied {
        return;
    }
    fingerprint.rows = occupied.clone();
    let rows = listable_inventory_rows(inventory, market_id);
    for list in lists.iter() {
        commands.entity(list).despawn_related::<Children>();
        commands.entity(list).with_children(|parent| {
            if occupied.is_empty() {
                parent.spawn((
                    Text::new("Your bag is empty."),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(theme.text_color.with_alpha(0.75)),
                ));
                return;
            }
            if rows.is_empty() {
                parent.spawn((
                    Text::new("Nothing you can sell here."),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(theme.text_color.with_alpha(0.75)),
                ));
                return;
            }
            for (slot, item_id) in &rows {
                let name = registry
                    .get(&ItemId::new(item_id.clone()))
                    .map(|item| item.display_name().to_string())
                    .unwrap_or_else(|| item_id.clone());
                parent
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(40.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                            Text::new(name),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(theme.text_color),
                        ));
                        spawn_bar_child(
                            row,
                            "Sell",
                            14.0,
                            theme.text_color,
                            Val::Px(96.0),
                            Val::Px(30.0),
                            BarButtonKind::Primary,
                            MarketSellFromBag {
                                slot: *slot,
                                item_id: item_id.clone(),
                            },
                        );
                    });
            }
        });
    }
}

pub fn sell_from_bag(
    mut commands: Commands,
    interactions: Query<(&Interaction, &MarketSellFromBag), Changed<Interaction>>,
    theme: Res<UiTheme>,
    asset_server: Res<AssetServer>,
    registry: Res<ItemRegistry>,
    mut ui: ResMut<MarketUiState>,
    existing: Query<(Entity, &MarketTicketCard)>,
) {
    let Some((_, sell)) = interactions
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Pressed)
    else {
        return;
    };
    let Some(market_id) = ui.open_market_id.clone() else {
        return;
    };
    if !item_allowed_in_open_market(&market_id, &sell.item_id) {
        return;
    }
    ui.bag_slot = Some(sell.slot);
    ui.ticket_action = MarketTicketAction::SellOrder;
    let item_id = sell.item_id.clone();
    let slot = sell.slot;
    for (entity, open) in existing.iter() {
        commands.entity(entity).despawn();
        let _ = open;
    }
    let display = registry
        .get(&ItemId::new(item_id.clone()))
        .map(|item| item.display_name().to_string())
        .unwrap_or_else(|| item_id.clone());
    spawn_market_ticket(
        &mut commands,
        &theme,
        &asset_server,
        &item_id,
        &display,
        Some(slot),
    );
}

pub fn open_market_ticket(
    mut commands: Commands,
    interactions: Query<(&Interaction, &MarketOpenTicket), Changed<Interaction>>,
    theme: Res<UiTheme>,
    asset_server: Res<AssetServer>,
    registry: Res<ItemRegistry>,
    mut ui: ResMut<MarketUiState>,
    existing: Query<(Entity, &MarketTicketCard)>,
) {
    let Some((_, ticket)) = interactions
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Pressed)
    else {
        return;
    };
    let item_id = ticket.item_id.clone();
    if ticket.price_gold > 0 {
        ui.list_price = ticket.price_gold;
    }
    ui.ticket_action = MarketTicketAction::Buy;
    ui.bag_slot = None;
    for (entity, open) in existing.iter() {
        if open.item_id == item_id {
            return;
        }
        commands.entity(entity).despawn();
    }
    let display = registry
        .get(&ItemId::new(item_id.clone()))
        .map(|item| item.display_name().to_string())
        .unwrap_or_else(|| item_id.clone());
    spawn_market_ticket(
        &mut commands,
        &theme,
        &asset_server,
        &item_id,
        &display,
        None,
    );
}

fn spawn_market_ticket(
    commands: &mut Commands,
    theme: &UiTheme,
    asset_server: &AssetServer,
    item_id: &str,
    display_name: &str,
    bag_slot: Option<u8>,
) {
    let item_id_owned = item_id.to_string();
    let display_owned = display_name.to_string();
    let text_color = theme.text_color;
    let card = CardBuilder::new(CardKind::Market, display_name)
        .frame(CardFrameAssets::load(asset_server))
        .width(Val::Px(TICKET_CARD_WIDTH))
        .height(Val::Px(TICKET_CARD_HEIGHT))
        .positioning(CardPositioning::Center)
        .draggable()
        .closeable()
        .coexist()
        .with_body(move |body| {
            body.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|columns| {
                columns
                    .spawn(Node {
                        width: Val::Px(260.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|left| {
                        left.spawn((
                            Text::new(display_owned),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(text_color),
                        ));
                        spawn_ticket_actions(left, text_color);
                        left.spawn((
                            Text::new("Price: 1"),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(text_color),
                            MarketTicketPriceText,
                        ));
                        spawn_price_bumps(left, theme);
                        left.spawn((
                            Text::new(""),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(text_color),
                            MarketTicketFeeText,
                        ));
                        spawn_bar_child(
                            left,
                            "Create",
                            14.0,
                            text_color,
                            Val::Percent(100.0),
                            Val::Px(32.0),
                            BarButtonKind::Primary,
                            MarketTicketCreateButton,
                        );
                    });
                columns
                    .spawn(Node {
                        flex_grow: 1.0,
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        ..default()
                    })
                    .with_children(|books| {
                        spawn_ticket_book_column(books, "Sell Orders", MarketTicketSellList, theme);
                        spawn_ticket_book_column(books, "Buy Orders", MarketTicketBuyList, theme);
                    });
            });
        })
        .spawn(commands, theme);
    commands.entity(card).insert(MarketTicketCard {
        item_id: item_id_owned,
        bag_slot,
    });
}

fn spawn_ticket_actions(parent: &mut ChildSpawnerCommands, text_color: Color) {
    let actions = [
        [MarketTicketAction::Sell, MarketTicketAction::Buy],
        [MarketTicketAction::SellOrder, MarketTicketAction::BuyOrder],
    ];
    for row_actions in actions {
        parent
            .spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|row| {
                for action in row_actions {
                    row.spawn((
                        Button,
                        Node {
                            padding: UiRect::all(Val::Px(6.0)),
                            flex_grow: 1.0,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.18, 0.18, 0.22, 0.9)),
                        MarketTicketActionButton { action },
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(action.label()),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(text_color),
                        ));
                    });
                }
            });
    }
}

fn spawn_ticket_book_column(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    marker: impl Bundle,
    theme: &UiTheme,
) {
    parent
        .spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(theme.text_color),
            ));
            col.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                marker,
            ));
        });
}

#[allow(clippy::too_many_arguments)]
pub fn refresh_market_ticket(
    mut commands: Commands,
    theme: Res<UiTheme>,
    ui: Res<MarketUiState>,
    book: Res<MarketOrderBook>,
    bids: Res<MarketBuyBook>,
    tickets: Query<&MarketTicketCard>,
    mut price_text: Query<&mut Text, (With<MarketTicketPriceText>, Without<MarketTicketFeeText>)>,
    mut fee_text: Query<&mut Text, (With<MarketTicketFeeText>, Without<MarketTicketPriceText>)>,
    mut actions: Query<(&MarketTicketActionButton, &mut BackgroundColor)>,
    sell_lists: Query<Entity, With<MarketTicketSellList>>,
    buy_lists: Query<Entity, With<MarketTicketBuyList>>,
) {
    let Some(ticket) = tickets.iter().next() else {
        return;
    };
    let Some(market_id) = ui.open_market_id.as_deref() else {
        return;
    };
    let quote_price = match ui.ticket_action {
        MarketTicketAction::Buy => {
            offers_for_item(book.orders.values(), market_id, &ticket.item_id)
                .first()
                .map(|order| order.price_gold)
                .unwrap_or_else(|| ui.listing_price())
        }
        _ => ui.listing_price(),
    };
    for mut text in price_text.iter_mut() {
        text.0 = format!("Price: {}", ui.listing_price());
    }
    let fee_body = ticket_fee_lines(quote_price, market_id)
        .map(|lines| format_ticket_fees(&lines))
        .unwrap_or_else(|| "Fee: —".to_string());
    for mut text in fee_text.iter_mut() {
        text.0 = fee_body.clone();
    }
    let selected = Color::srgba(0.28, 0.32, 0.45, 0.95);
    let idle = Color::srgba(0.18, 0.18, 0.22, 0.9);
    for (button, mut bg) in actions.iter_mut() {
        bg.0 = if button.action == ui.ticket_action {
            selected
        } else {
            idle
        };
    }

    let sells = offers_for_item(book.orders.values(), market_id, &ticket.item_id);
    for list in sell_lists.iter() {
        commands.entity(list).despawn_related::<Children>();
        commands.entity(list).with_children(|parent| {
            if sells.is_empty() {
                parent.spawn((
                    Text::new("No sell orders."),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(theme.muted_text_color),
                ));
                return;
            }
            for order in &sells {
                parent
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new(format!("{}", order.price_gold)),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(theme.text_color),
                        ));
                        // Local character id is private; server rejects non-owners.
                        row.spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            MarketCancelSellButton { order_id: order.id },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Cancel"),
                                TextFont {
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));
                        });
                    });
            }
        });
    }
    let buys = bids_for_item(bids.orders.values(), market_id, &ticket.item_id);
    for list in buy_lists.iter() {
        commands.entity(list).despawn_related::<Children>();
        commands.entity(list).with_children(|parent| {
            if buys.is_empty() {
                parent.spawn((
                    Text::new("No buy orders."),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(theme.muted_text_color),
                ));
                return;
            }
            for order in &buys {
                parent
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new(format!("{}", order.price_gold)),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(theme.text_color),
                        ));
                        row.spawn((
                            Button,
                            Node {
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            MarketCancelBuyButton { order_id: order.id },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Cancel"),
                                TextFont {
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(theme.text_color),
                            ));
                        });
                    });
            }
        });
    }
}

pub fn buy_market_offer(
    interactions: Query<(&Interaction, &MarketOfferButton), Changed<Interaction>>,
    ui: Res<MarketUiState>,
    book: Res<MarketOrderBook>,
    npcs: Query<&NetworkEntityId>,
    connection: Option<Res<StdbConnection>>,
) {
    let Some(connection) = connection else {
        return;
    };
    let Some(npc) = ui.npc else {
        return;
    };
    let Ok(network_id) = npcs.get(npc) else {
        return;
    };
    let Some(market_id) = ui.open_market_id.as_deref() else {
        return;
    };
    for (interaction, button) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(order) = book.orders.get(&button.order_id) else {
            continue;
        };
        if !item_allowed_in_open_market(market_id, &order.item_id) {
            continue;
        }
        let _ = commands::market_buy(&connection, network_id.0, button.order_id);
    }
}

pub fn step_list_price(
    interactions: Query<(&Interaction, &MarketPriceBump), Changed<Interaction>>,
    mut ui: ResMut<MarketUiState>,
) {
    for (interaction, bump) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let next = ui.listing_price() as i64 + bump.delta;
        ui.list_price = next.max(1) as u64;
    }
}

pub fn select_ticket_action(
    interactions: Query<(&Interaction, &MarketTicketActionButton), Changed<Interaction>>,
    mut ui: ResMut<MarketUiState>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        ui.ticket_action = button.action;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_ticket_order(
    interactions: Query<&Interaction, (Changed<Interaction>, With<MarketTicketCreateButton>)>,
    ui: Res<MarketUiState>,
    tickets: Query<&MarketTicketCard>,
    book: Res<MarketOrderBook>,
    inventory_ui: Res<InventoryUiState>,
    inventory: Query<&Inventory, With<LocalPlayer>>,
    npcs: Query<&NetworkEntityId>,
    connection: Option<Res<StdbConnection>>,
) {
    if !interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }
    let Some(connection) = connection else {
        return;
    };
    let Some(npc) = ui.npc else {
        return;
    };
    let Ok(network_id) = npcs.get(npc) else {
        return;
    };
    let Some(ticket) = tickets.iter().next() else {
        return;
    };
    let Some(market_id) = ui.open_market_id.as_deref() else {
        return;
    };
    if !item_allowed_in_open_market(market_id, &ticket.item_id) {
        return;
    }
    match ui.ticket_action {
        MarketTicketAction::Buy => {
            let Some(order) = offers_for_item(book.orders.values(), market_id, &ticket.item_id)
                .into_iter()
                .next()
            else {
                return;
            };
            let _ = commands::market_buy(&connection, network_id.0, order.id);
        }
        MarketTicketAction::SellOrder => {
            let Some(instance) = bag_or_selected_instance(ticket, &inventory_ui, &inventory) else {
                return;
            };
            let _ = commands::place_sell_order(
                &connection,
                network_id.0,
                instance.instance_id.0,
                ui.listing_price(),
            );
        }
        MarketTicketAction::Sell => {
            let Some(instance) = bag_or_selected_instance(ticket, &inventory_ui, &inventory) else {
                return;
            };
            let _ = commands::market_sell(
                &connection,
                network_id.0,
                instance.instance_id.0,
                ui.listing_price(),
            );
        }
        MarketTicketAction::BuyOrder => {
            let _ = commands::place_buy_order(
                &connection,
                network_id.0,
                ticket.item_id.clone(),
                ui.listing_price(),
            );
        }
    }
}

pub fn cancel_ticket_sell_order(
    interactions: Query<(&Interaction, &MarketCancelSellButton), Changed<Interaction>>,
    connection: Option<Res<StdbConnection>>,
) {
    let Some(connection) = connection else {
        return;
    };
    for (interaction, button) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let _ = commands::cancel_sell_order(&connection, button.order_id);
    }
}

pub fn cancel_ticket_buy_order(
    interactions: Query<(&Interaction, &MarketCancelBuyButton), Changed<Interaction>>,
    connection: Option<Res<StdbConnection>>,
) {
    let Some(connection) = connection else {
        return;
    };
    for (interaction, button) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let _ = commands::cancel_buy_order(&connection, button.order_id);
    }
}

fn selected_slot_instance<'a>(
    inventory_ui: &InventoryUiState,
    inventory: &'a Query<&Inventory, With<LocalPlayer>>,
) -> Option<&'a bevymmo_gameplay::items::instance::ItemInstance> {
    let InventorySelection::Slot(index) = inventory_ui.selected? else {
        return None;
    };
    let inventory = inventory.single().ok()?;
    inventory.slots.get(index as usize)?.as_ref()
}

fn selected_slot_matching<'a>(
    inventory_ui: &InventoryUiState,
    inventory: &'a Query<&Inventory, With<LocalPlayer>>,
    item_id: &str,
) -> Option<&'a bevymmo_gameplay::items::instance::ItemInstance> {
    let instance = selected_slot_instance(inventory_ui, inventory)?;
    if instance.item_id.as_str() != item_id || !instance.instance_id.is_assigned() {
        return None;
    }
    Some(instance)
}

fn bag_or_selected_instance<'a>(
    ticket: &MarketTicketCard,
    inventory_ui: &InventoryUiState,
    inventory: &'a Query<&Inventory, With<LocalPlayer>>,
) -> Option<&'a bevymmo_gameplay::items::instance::ItemInstance> {
    if let Some(slot) = ticket.bag_slot {
        let bag = inventory.single().ok()?;
        let instance = bag.slots.get(slot as usize)?.as_ref()?;
        if instance.item_id.as_str() != ticket.item_id || !instance.instance_id.is_assigned() {
            return None;
        }
        return Some(instance);
    }
    selected_slot_matching(inventory_ui, inventory, &ticket.item_id)
}

fn point_to_ray_distance(point: Vec3, ray_origin: Vec3, ray_direction: Vec3) -> f32 {
    let to_point = point - ray_origin;
    let projection = to_point.dot(ray_direction);
    let closest_on_ray = ray_origin + ray_direction * projection.clamp(0.0, f32::MAX);
    point.distance(closest_on_ray)
}

fn cursor_ray(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &Transform), With<Camera3d>>,
) -> Option<Ray3d> {
    let window = windows.single().ok()?;
    let cursor_pos = window.cursor_position()?;
    let (camera, transform) = cameras.iter().next()?;
    let view = renderer::camera_view(transform);
    camera.viewport_to_world(&view, cursor_pos).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_filter_keeps_only_matching_ids() {
        let rows = ["market_1", "market_2", "market_1"];
        let kept: Vec<_> = rows
            .iter()
            .copied()
            .filter(|id| *id == "market_1")
            .collect();
        assert_eq!(kept, ["market_1", "market_1"]);
    }

    #[test]
    fn book_filter_keeps_matching_market_and_item() {
        let rows = [
            ("market_1", "iron_ore", 1),
            ("market_1", "copper_ore", 2),
            ("market_2", "iron_ore", 3),
            ("market_1", "iron_ore", 4),
        ];
        let kept = filter_book_ids(rows.into_iter(), "market_1", "iron_ore");
        assert_eq!(kept, [1, 4]);
        let bids = [
            ("market_1", "iron_ore", 10),
            ("market_2", "iron_ore", 11),
            ("market_1", "iron_ore", 12),
        ];
        assert_eq!(
            filter_book_ids(bids.into_iter(), "market_1", "iron_ore"),
            [10, 12]
        );
    }

    #[test]
    fn occupied_inventory_rows_skip_empty_slots() {
        let mut inventory = Inventory::default();
        inventory.slots[2] = Some(bevymmo_gameplay::items::instance::ItemInstance::new(
            ItemId::new("sword"),
        ));
        inventory.slots[5] = Some(bevymmo_gameplay::items::instance::ItemInstance::new(
            ItemId::new("simple_helm"),
        ));
        let rows = occupied_inventory_rows(&inventory);
        assert_eq!(
            rows,
            vec![(2, "sword".to_string()), (5, "simple_helm".to_string())]
        );
        assert_eq!(
            listable_inventory_rows(&inventory, MARKET_1_ID),
            vec![(2, "sword".to_string())]
        );
        assert_eq!(
            listable_inventory_rows(&inventory, MARKET_2_ID),
            vec![(5, "simple_helm".to_string())]
        );
    }

    #[test]
    fn item_allowed_in_open_market_uses_the_hall_allowlist() {
        assert!(item_allowed_in_open_market(MARKET_1_ID, "sword"));
        assert!(!item_allowed_in_open_market(MARKET_1_ID, "simple_helm"));
        assert!(item_allowed_in_open_market(MARKET_2_ID, "simple_helm"));
    }

    #[test]
    fn ticket_fee_lines_show_two_plus_one_percent_of_10000() {
        let lines = ticket_fee_lines(10_000, MARKET_1_ID).expect("quote");
        assert_eq!(lines.market_fee, 200);
        assert_eq!(lines.account_fee, 100);
        assert_eq!(lines.total_fee, 300);
        assert_eq!(lines.you_pay, 10_000);
        assert_eq!(lines.you_receive, 9_700);
        let shown = format_ticket_fees(&lines);
        assert!(shown.contains("Total fee: 300"), "{shown}");
        let quote = quote_fee(10_000, MARKET_1_FEE_BPS, DEFAULT_ACCOUNT_FEE_BPS).unwrap();
        assert_eq!(quote.fee_gold, 300);
    }
}
