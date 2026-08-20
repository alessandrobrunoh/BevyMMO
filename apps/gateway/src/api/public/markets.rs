//! `/public/markets/*` — isolated order books, readable without a session.
//!
//! Handlers only scan rows the module already replicates publicly (`market`,
//! `market_sell_order`, `market_buy_order`). Isolation is a filter on
//! `market_id`: Market 1's offers never appear on Market 2.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::Serialize;
use uuid::Uuid;

use crate::api::error::AppError;
use crate::stdb::module_bindings::{Market, MarketBuyOrder, MarketSellOrder};
use crate::AppState;

/// One market as the public API exposes it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct MarketSummary {
    pub id: String,
    pub display_name: String,
    pub fee_bps: u16,
    pub allowed_item_ids: Vec<String>,
}

impl From<Market> for MarketSummary {
    fn from(row: Market) -> Self {
        Self {
            id: row.id,
            display_name: row.display_name,
            fee_bps: row.fee_bps,
            allowed_item_ids: row.allowed_item_ids,
        }
    }
}

/// One sell listing, already scoped to a single market by the handler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct SellOffer {
    pub id: u64,
    pub item_id: String,
    pub price_gold: u64,
    pub seller_character_id: Uuid,
}

/// A buy listing (Gold already escrowed on the module).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct BuyOffer {
    pub id: u64,
    pub item_id: String,
    pub price_gold: u64,
    pub buyer_character_id: Uuid,
}

/// Order book for one item inside one market.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ItemTicket {
    pub market_id: String,
    pub item_id: String,
    pub sell_orders: Vec<SellOffer>,
    pub buy_orders: Vec<BuyOffer>,
}

/// Fields the isolation helpers need. Kept separate from the generated row
/// type so unit tests do not have to construct `Timestamp` / `ItemInstanceRow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SellOrderRecord {
    pub id: u64,
    pub market_id: String,
    pub item_id: String,
    pub price_gold: u64,
    pub seller_character_id: Uuid,
}

impl From<MarketSellOrder> for SellOrderRecord {
    fn from(row: MarketSellOrder) -> Self {
        Self {
            id: row.id,
            market_id: row.market_id,
            item_id: row.item_id,
            price_gold: row.price_gold,
            seller_character_id: Uuid::from_u128(row.seller_character_id.as_u128()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuyOrderRecord {
    pub id: u64,
    pub market_id: String,
    pub item_id: String,
    pub price_gold: u64,
    pub buyer_character_id: Uuid,
}

impl From<MarketBuyOrder> for BuyOrderRecord {
    fn from(row: MarketBuyOrder) -> Self {
        Self {
            id: row.id,
            market_id: row.market_id,
            item_id: row.item_id,
            price_gold: row.price_gold,
            buyer_character_id: Uuid::from_u128(row.buyer_character_id.as_u128()),
        }
    }
}

impl From<&BuyOrderRecord> for BuyOffer {
    fn from(row: &BuyOrderRecord) -> Self {
        Self {
            id: row.id,
            item_id: row.item_id.clone(),
            price_gold: row.price_gold,
            buyer_character_id: row.buyer_character_id,
        }
    }
}

/// Buy orders for one item inside one market.
pub fn bids_for_item<'a>(
    orders: &'a [BuyOrderRecord],
    market_id: &str,
    item_id: &str,
) -> Vec<&'a BuyOrderRecord> {
    orders
        .iter()
        .filter(|order| order.market_id == market_id && order.item_id == item_id)
        .collect()
}

impl From<&SellOrderRecord> for SellOffer {
    fn from(row: &SellOrderRecord) -> Self {
        Self {
            id: row.id,
            item_id: row.item_id.clone(),
            price_gold: row.price_gold,
            seller_character_id: row.seller_character_id,
        }
    }
}

/// Markets whose `id` equals `market_id`.
pub fn find_market<'a>(markets: &'a [MarketSummary], market_id: &str) -> Option<&'a MarketSummary> {
    markets.iter().find(|market| market.id == market_id)
}

/// Sell orders belonging to `market_id` and no other market.
pub fn offers_in_market<'a>(
    orders: &'a [SellOrderRecord],
    market_id: &str,
) -> Vec<&'a SellOrderRecord> {
    orders
        .iter()
        .filter(|order| order.market_id == market_id)
        .collect()
}

/// Sell orders for one item inside one market.
pub fn offers_for_item<'a>(
    orders: &'a [SellOrderRecord],
    market_id: &str,
    item_id: &str,
) -> Vec<&'a SellOrderRecord> {
    orders
        .iter()
        .filter(|order| order.market_id == market_id && order.item_id == item_id)
        .collect()
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/public/markets", get(list_markets))
        .route("/v1/public/markets/:market_id/offers", get(list_offers))
        .route(
            "/v1/public/markets/:market_id/items/:item_id",
            get(item_ticket),
        )
}

/// Every seeded market. Empty when the module has not been published with
/// market rows yet — not an error.
#[utoipa::path(
    get,
    tag = "market",
    path = "/v1/public/markets",
    responses(
        (status = 200, description = "Seeded markets", body = Vec<MarketSummary>),
        (status = 503, description = "SpacetimeDB unreachable", body = crate::api::error::ErrorResponse),
    ),
)]
pub async fn list_markets(
    State(state): State<AppState>,
) -> Result<Json<Vec<MarketSummary>>, AppError> {
    let (markets, _, _) = market_snapshot(&state).await?;
    let mut summaries: Vec<MarketSummary> = markets.into_iter().map(MarketSummary::from).collect();
    summaries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Json(summaries))
}

/// Sell orders listed on `market_id`. Isolated: other markets' rows are not
/// returned. 404 if that market id is not in the `market` table.
#[utoipa::path(
    get,
    tag = "market",
    path = "/v1/public/markets/{market_id}/offers",
    params(("market_id" = String, Path, description = "Stable market id (e.g. market_1)")),
    responses(
        (status = 200, description = "Sell offers on this market only", body = Vec<SellOffer>),
        (status = 404, description = "No market with that id", body = crate::api::error::ErrorResponse),
        (status = 503, description = "SpacetimeDB unreachable", body = crate::api::error::ErrorResponse),
    ),
)]
pub async fn list_offers(
    State(state): State<AppState>,
    Path(market_id): Path<String>,
) -> Result<Json<Vec<SellOffer>>, AppError> {
    let (markets, orders, _) = market_snapshot(&state).await?;
    let summaries: Vec<MarketSummary> = markets.into_iter().map(MarketSummary::from).collect();
    if find_market(&summaries, &market_id).is_none() {
        return Err(unknown_market(&market_id));
    }
    let records: Vec<SellOrderRecord> = orders.into_iter().map(SellOrderRecord::from).collect();
    let mut offers: Vec<SellOffer> = offers_in_market(&records, &market_id)
        .into_iter()
        .map(SellOffer::from)
        .collect();
    offers.sort_by(|a, b| a.price_gold.cmp(&b.price_gold).then(a.id.cmp(&b.id)));
    Ok(Json(offers))
}

/// Sell + buy book for one item on one market. 404 if the market id is unknown.
#[utoipa::path(
    get,
    tag = "market",
    path = "/v1/public/markets/{market_id}/items/{item_id}",
    params(
        ("market_id" = String, Path, description = "Stable market id (e.g. market_1)"),
        ("item_id" = String, Path, description = "Catalog item id (e.g. sword)"),
    ),
    responses(
        (status = 200, description = "Order book for this item on this market", body = ItemTicket),
        (status = 404, description = "No market with that id", body = crate::api::error::ErrorResponse),
        (status = 503, description = "SpacetimeDB unreachable", body = crate::api::error::ErrorResponse),
    ),
)]
pub async fn item_ticket(
    State(state): State<AppState>,
    Path((market_id, item_id)): Path<(String, String)>,
) -> Result<Json<ItemTicket>, AppError> {
    let (markets, orders, bids) = market_snapshot(&state).await?;
    let summaries: Vec<MarketSummary> = markets.into_iter().map(MarketSummary::from).collect();
    if find_market(&summaries, &market_id).is_none() {
        return Err(unknown_market(&market_id));
    }
    let records: Vec<SellOrderRecord> = orders.into_iter().map(SellOrderRecord::from).collect();
    let bid_records: Vec<BuyOrderRecord> = bids.into_iter().map(BuyOrderRecord::from).collect();
    let mut sell_orders: Vec<SellOffer> = offers_for_item(&records, &market_id, &item_id)
        .into_iter()
        .map(SellOffer::from)
        .collect();
    sell_orders.sort_by(|a, b| a.price_gold.cmp(&b.price_gold).then(a.id.cmp(&b.id)));
    let mut buy_orders: Vec<BuyOffer> = bids_for_item(&bid_records, &market_id, &item_id)
        .into_iter()
        .map(BuyOffer::from)
        .collect();
    buy_orders.sort_by(|a, b| b.price_gold.cmp(&a.price_gold).then(a.id.cmp(&b.id)));
    Ok(Json(ItemTicket {
        market_id,
        item_id,
        sell_orders,
        buy_orders,
    }))
}

async fn market_snapshot(
    state: &AppState,
) -> Result<(Vec<Market>, Vec<MarketSellOrder>, Vec<MarketBuyOrder>), AppError> {
    state.directory.market_snapshot().await.map_err(unavailable)
}

fn unknown_market(market_id: &str) -> AppError {
    AppError::NotFound(format!("no market with id {market_id}"))
}

fn unavailable(reason: String) -> AppError {
    tracing::error!("market directory unavailable: {reason}");
    AppError::ServiceUnavailable
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u64, market_id: &str, item_id: &str) -> SellOrderRecord {
        SellOrderRecord {
            id,
            market_id: market_id.to_string(),
            item_id: item_id.to_string(),
            price_gold: id * 10,
            seller_character_id: Uuid::from_u128(id as u128),
        }
    }

    fn market(id: &str) -> MarketSummary {
        MarketSummary {
            id: id.to_string(),
            display_name: id.to_string(),
            fee_bps: 200,
            allowed_item_ids: vec!["sword".into()],
        }
    }

    #[test]
    fn offers_are_isolated_by_market_id() {
        let orders = vec![
            record(1, "market_1", "sword"),
            record(2, "market_2", "simple_helm"),
            record(3, "market_1", "bow"),
        ];
        let one: Vec<&str> = offers_in_market(&orders, "market_1")
            .into_iter()
            .map(|o| o.item_id.as_str())
            .collect();
        assert_eq!(one, vec!["sword", "bow"]);
        let two: Vec<&str> = offers_in_market(&orders, "market_2")
            .into_iter()
            .map(|o| o.item_id.as_str())
            .collect();
        assert_eq!(two, vec!["simple_helm"]);
        assert!(offers_in_market(&orders, "market_1")
            .iter()
            .all(|o| o.market_id == "market_1"));
    }

    #[test]
    fn unknown_market_is_not_found() {
        let markets = vec![market("market_1"), market("market_2")];
        assert!(find_market(&markets, "market_1").is_some());
        assert!(find_market(&markets, "market_9").is_none());
    }

    #[test]
    fn item_ticket_does_not_leak_other_markets_or_items() {
        let orders = vec![
            record(1, "market_1", "sword"),
            record(2, "market_2", "sword"),
            record(3, "market_1", "bow"),
        ];
        let ticket = offers_for_item(&orders, "market_1", "sword");
        assert_eq!(ticket.len(), 1);
        assert_eq!(ticket[0].id, 1);
        assert!(offers_for_item(&orders, "market_2", "bow").is_empty());
    }

    fn bid(id: u64, market_id: &str, item_id: &str) -> BuyOrderRecord {
        BuyOrderRecord {
            id,
            market_id: market_id.to_string(),
            item_id: item_id.to_string(),
            price_gold: id * 5,
            buyer_character_id: Uuid::from_u128(id as u128),
        }
    }

    #[test]
    fn bids_are_isolated_by_market_and_item() {
        let orders = vec![
            bid(1, "market_1", "sword"),
            bid(2, "market_2", "sword"),
            bid(3, "market_1", "bow"),
        ];
        let ticket = bids_for_item(&orders, "market_1", "sword");
        assert_eq!(ticket.len(), 1);
        assert_eq!(ticket[0].id, 1);
        assert!(bids_for_item(&orders, "market_2", "bow").is_empty());
    }
}
