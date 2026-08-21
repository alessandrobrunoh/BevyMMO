//! Isolated player-market browse card (Albion-like offer list) and item ticket.

pub mod systems;

use bevy::prelude::*;

use crate::game_state::Screen;

use systems::{
    buy_market_offer, cancel_ticket_buy_order, cancel_ticket_sell_order,
    close_ticket_if_browse_closed, create_ticket_order, npc_market_on_click, open_market_ticket,
    refresh_bag_rows, refresh_market_rows, refresh_market_ticket, select_market_tab,
    select_ticket_action, sell_from_bag, step_list_price, step_list_quantity,
    sync_market_tab_visibility,
};

pub struct MarketUiPlugin;

impl Plugin for MarketUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MarketUiState>();
        app.add_systems(
            Update,
            (
                close_ticket_if_browse_closed,
                npc_market_on_click,
                select_market_tab,
                sync_market_tab_visibility,
                open_market_ticket,
                sell_from_bag,
                select_ticket_action,
                step_list_price,
                step_list_quantity,
                refresh_market_rows,
                refresh_bag_rows,
                refresh_market_ticket,
                buy_market_offer,
                create_ticket_order,
                cancel_ticket_sell_order,
                cancel_ticket_buy_order,
            )
                .chain()
                .run_if(in_state(Screen::InGame)),
        );
    }
}

/// Left-column action on the item ticket (Screen B).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarketTicketAction {
    Sell,
    #[default]
    Buy,
    SellOrder,
    BuyOrder,
}

impl MarketTicketAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sell => "Sell",
            Self::Buy => "Buy",
            Self::SellOrder => "Sell Order",
            Self::BuyOrder => "Buy Order",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarketTab {
    #[default]
    Offers,
    Inventory,
}

#[derive(Resource, Default)]
pub struct MarketUiState {
    pub open_market_id: Option<String>,
    pub npc: Option<Entity>,
    pub list_price: u64,
    /// Units to list / instant-sell. `0` means "use the whole pile".
    pub list_quantity: u32,
    pub search: String,
    pub ticket_action: MarketTicketAction,
    pub tab: MarketTab,
    pub bag_slot: Option<u8>,
}

impl MarketUiState {
    pub fn listing_price(&self) -> u64 {
        self.list_price.max(1)
    }

    pub fn listing_quantity(&self, available: u32) -> u32 {
        bevymmo_gameplay::items::components::Inventory::clamp_trade_amount(
            if self.list_quantity == 0 {
                available
            } else {
                self.list_quantity
            },
            available,
        )
    }
}

/// Root of the open market browse card (Screen A).
#[derive(Component)]
pub struct MarketCard;

/// Root of the item ticket card (Screen B).
#[derive(Component)]
pub struct MarketTicketCard {
    pub item_id: String,
    pub bag_slot: Option<u8>,
}

/// One row's Buy control on Screen A.
#[derive(Component)]
pub struct MarketOfferButton {
    pub order_id: u64,
}

/// Opens Screen B for this catalogue id.
#[derive(Component)]
pub struct MarketOpenTicket {
    pub item_id: String,
    pub price_gold: u64,
    pub quantity: u32,
}

#[derive(Component)]
pub struct MarketOfferName;

#[derive(Component)]
pub struct MarketOfferPrice;

#[derive(Component)]
pub struct MarketGoldText;

#[derive(Component)]
pub struct MarketPriceBump {
    pub delta: i64,
}

#[derive(Component)]
pub struct MarketQuantityBump {
    pub delta: i32,
}

#[derive(Component, Clone, Copy)]
pub struct MarketTabButton {
    pub tab: MarketTab,
}

#[derive(Component)]
pub struct MarketOffersPanel;

#[derive(Component)]
pub struct MarketInventoryPanel;

#[derive(Component)]
pub struct MarketBagList;

#[derive(Component, Clone)]
pub struct MarketSellFromBag {
    pub slot: u8,
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Component)]
pub struct MarketTicketActionButton {
    pub action: MarketTicketAction,
}

#[derive(Component)]
pub struct MarketTicketCreateButton;

#[derive(Component)]
pub struct MarketTicketPriceText;

#[derive(Component)]
pub struct MarketTicketQuantityText;

#[derive(Component)]
pub struct MarketTicketFeeText;

#[derive(Component)]
pub struct MarketTicketSellList;

#[derive(Component)]
pub struct MarketTicketBuyList;

#[derive(Component)]
pub struct MarketCancelSellButton {
    pub order_id: u64,
}

#[derive(Component)]
pub struct MarketCancelBuyButton {
    pub order_id: u64,
}
