//! Isolated player markets: list, bid, fill, cancel.

use std::sync::OnceLock;

use bevymmo_domain::economy::Gold;
use bevymmo_domain::items::components::Inventory;
use bevymmo_domain::items::instance::{ItemInstance, ItemInstanceId};
use bevymmo_domain::items::registry::{ItemId, ItemRegistry};
use bevymmo_domain::markets::{
    assert_item_marketable, assert_order_cap, listing_total, plan_fill, plan_fill_buy_order,
    plan_place_buy_order, select_best_buy_order, BuyBid, MarketRegistry, MARKET_PROXIMITY_SQUARED,
};
use spacetimedb::{reducer, ReducerContext, Table, Uuid};

use crate::reducers::economy::{credit_gold, debit_gold, ensure_account_economy, ensure_wallet};
use crate::reducers::items::{item_stacks, load_inventory, next_instance_id, store_inventory};
use crate::reducers::lifecycle::caller_character;
use crate::rows::ItemInstanceRow;
use crate::tables::{
    game_entity, market, market_buy_order, market_sell_order, npc, player, EntityKindRow,
    MarketBuyOrder, MarketSellOrder, Npc,
};

fn market_registry() -> &'static MarketRegistry {
    static REGISTRY: OnceLock<MarketRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::markets::default_markets)
}

fn item_registry() -> &'static ItemRegistry {
    static REGISTRY: OnceLock<ItemRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::items::default_items)
}

/// The item's own `tradable` flag. Unknown ids fail before the flag.
fn assert_can_trade_item(item_id: &ItemId) -> Result<(), String> {
    let item = item_registry()
        .get(item_id)
        .ok_or_else(|| format!("unknown item {:?}", item_id.as_str()))?;
    assert_item_marketable(item.tradable()).map_err(|err| err.to_string())
}

/// Inserts the two catalog markets if this database does not have them yet.
pub fn seed_markets(ctx: &ReducerContext) {
    for definition in market_registry().iter() {
        let id = definition.id.as_str().to_string();
        if ctx.db.market().id().find(&id).is_some() {
            continue;
        }
        ctx.db.market().insert(crate::tables::Market {
            id: definition.id.as_str().to_string(),
            display_name: definition.display_name.to_string(),
            fee_bps: definition.fee_bps,
        });
    }
}

fn nearby_market_npc(ctx: &ReducerContext, npc_entity_id: u64) -> Result<Npc, String> {
    let character = caller_character(ctx)?;
    let player = ctx
        .db
        .game_entity()
        .entity_id()
        .find(character.entity_id)
        .ok_or_else(|| "character has no entity".to_string())?;
    let npc_entity = ctx
        .db
        .game_entity()
        .entity_id()
        .find(npc_entity_id)
        .ok_or_else(|| "NPC not found".to_string())?;
    if npc_entity.kind != EntityKindRow::Npc {
        return Err("that entity is not an NPC".to_string());
    }
    let dx = player.position.x - npc_entity.position.x;
    let dy = player.position.y - npc_entity.position.y;
    let dz = player.position.z - npc_entity.position.z;
    if dx * dx + dy * dy + dz * dz > MARKET_PROXIMITY_SQUARED {
        return Err("you are too far from that NPC".to_string());
    }
    let npc = ctx
        .db
        .npc()
        .entity_id()
        .find(npc_entity_id)
        .ok_or_else(|| "NPC has no profile".to_string())?;
    if npc.market_id.is_none() {
        return Err("that NPC is not a market".to_string());
    }
    Ok(npc)
}

fn open_order_count(ctx: &ReducerContext, character_id: Uuid, market_id: &str) -> usize {
    let sells = ctx
        .db
        .market_sell_order()
        .iter()
        .filter(|row| row.seller_character_id == character_id && row.market_id == market_id)
        .count();
    let buys = ctx
        .db
        .market_buy_order()
        .iter()
        .filter(|row| row.buyer_character_id == character_id && row.market_id == market_id)
        .count();
    sells + buys
}

fn buy_bids_for(rows: &[MarketBuyOrder], seller_character_id: Uuid) -> Vec<BuyBid> {
    rows.iter()
        .map(|row| BuyBid {
            id: row.id,
            market_id: row.market_id.clone(),
            item_id: row.item_id.clone(),
            price_gold: row.price_gold,
            is_own: row.buyer_character_id == seller_character_id,
        })
        .collect()
}

/// Peels `quantity` off the bag pile `instance_id`. Taking the whole pile
/// keeps the original id; a partial take mints a new one for the listed copy.
fn take_listed_instance(
    ctx: &ReducerContext,
    inventory: &mut Inventory,
    instance_id: ItemInstanceId,
    quantity: u32,
) -> Result<ItemInstance, String> {
    let Some(slot) = inventory.slots.iter().position(|item| {
        item.as_ref()
            .is_some_and(|item| item.instance_id == instance_id)
    }) else {
        return Err("item instance is not in your inventory".to_string());
    };
    let stacks = match inventory.slots[slot].as_ref() {
        Some(instance) => instance.quantity > 1 || item_stacks(&instance.item_id),
        None => return Err("item instance is not in your inventory".to_string()),
    };
    inventory
        .take_amount(slot, quantity, stacks, || {
            ItemInstanceId(next_instance_id(ctx))
        })
        .map_err(|error| error.to_string())
}

/// Escrows `quantity` of an inventory pile as a sell order in the NPC's market.
///
/// `price` is gold **per unit**. The stored `price_gold` is the total for the
/// listed pile so a fill is still one debit. Named `place_sell_order` because
/// the table accessor is already `market_sell_order` and two items cannot share
/// a type-namespace name.
#[reducer]
pub fn place_sell_order(
    ctx: &ReducerContext,
    npc_entity_id: u64,
    instance_id: u64,
    price: u64,
    quantity: u32,
) -> Result<(), String> {
    if instance_id == 0 {
        return Err("item instance is not assigned".to_string());
    }
    let total = listing_total(price, quantity).map_err(|err| err.to_string())?;
    let npc = nearby_market_npc(ctx, npc_entity_id)?;
    let market_id = npc
        .market_id
        .clone()
        .ok_or_else(|| "that NPC is not a market".to_string())?;
    // Validated for its own sake: an order must belong to a hall that exists.
    market_registry()
        .get(&market_id)
        .ok_or_else(|| "unknown market".to_string())?;

    let character = caller_character(ctx)?;
    assert_order_cap(open_order_count(ctx, character.character_id, &market_id))
        .map_err(|err| err.to_string())?;

    let mut inventory = load_inventory(ctx, character.character_id)?;
    let instance =
        take_listed_instance(ctx, &mut inventory, ItemInstanceId(instance_id), quantity)?;
    assert_can_trade_item(&instance.item_id)?;

    store_inventory(ctx, character.character_id, &inventory);
    ctx.db.market_sell_order().insert(MarketSellOrder {
        id: 0,
        market_id,
        seller_character_id: character.character_id,
        item_id: instance.item_id.as_str().to_string(),
        item: ItemInstanceRow::from(&instance),
        price_gold: total,
        created_at: ctx.timestamp,
    });
    Ok(())
}

/// Buys a sell order in the NPC's market. Buyer pays `price`; seller receives
/// `price - (market fee + seller account fee)`; the rest is burned.
#[reducer]
pub fn market_buy(
    ctx: &ReducerContext,
    npc_entity_id: u64,
    sell_order_id: u64,
) -> Result<(), String> {
    let npc = nearby_market_npc(ctx, npc_entity_id)?;
    let acting_market = npc
        .market_id
        .clone()
        .ok_or_else(|| "that NPC is not a market".to_string())?;
    let order = ctx
        .db
        .market_sell_order()
        .id()
        .find(sell_order_id)
        .ok_or_else(|| "order not found".to_string())?;

    let buyer = caller_character(ctx)?;
    let seller = ctx
        .db
        .player()
        .character_id()
        .find(order.seller_character_id)
        .ok_or_else(|| "seller no longer exists".to_string())?;

    let definition = market_registry()
        .get(&acting_market)
        .ok_or_else(|| "unknown market".to_string())?;
    assert_can_trade_item(&ItemId::new(order.item_id.clone()))?;
    let seller_account = ensure_account_economy(ctx, seller.account_id);
    let buyer_wallet = ensure_wallet(ctx, buyer.character_id);
    let mut inventory = load_inventory(ctx, buyer.character_id)?;
    let free = inventory.slots.iter().position(Option::is_none);

    let plan = plan_fill(
        buyer.character_id == order.seller_character_id,
        &order.market_id,
        &acting_market,
        order.price_gold,
        definition.fee_bps,
        seller_account.fee_bps,
        Gold::from_u64(buyer_wallet.gold),
        free.is_some(),
    )
    .map_err(|err| err.to_string())?;

    let slot = free.expect("plan_fill checked a free slot");
    debit_gold(ctx, buyer.character_id, plan.quote.buyer_pays.amount())?;
    credit_gold(
        ctx,
        seller.character_id,
        plan.quote.seller_receives.amount(),
    )?;
    inventory.slots[slot] = Some((&order.item).into());
    store_inventory(ctx, buyer.character_id, &inventory);
    ctx.db.market_sell_order().id().delete(sell_order_id);
    Ok(())
}

/// Returns a listed instance to the seller's first free inventory slot.
#[reducer]
pub fn cancel_sell_order(ctx: &ReducerContext, order_id: u64) -> Result<(), String> {
    let character = caller_character(ctx)?;
    let order = ctx
        .db
        .market_sell_order()
        .id()
        .find(order_id)
        .ok_or_else(|| "order not found".to_string())?;
    if order.seller_character_id != character.character_id {
        return Err("that order is not yours".to_string());
    }
    let mut inventory = load_inventory(ctx, character.character_id)?;
    let slot = inventory
        .slots
        .iter()
        .position(Option::is_none)
        .ok_or_else(|| "inventory is full".to_string())?;
    inventory.slots[slot] = Some((&order.item).into());
    store_inventory(ctx, character.character_id, &inventory);
    ctx.db.market_sell_order().id().delete(order_id);
    Ok(())
}

/// Escrows Gold as a bid for `item_id` in the NPC's market.
///
/// Named `place_buy_order` because the table accessor is already
/// `market_buy_order`.
#[reducer]
pub fn place_buy_order(
    ctx: &ReducerContext,
    npc_entity_id: u64,
    item_id: String,
    price: u64,
) -> Result<(), String> {
    if price == 0 {
        return Err("price must be greater than 0".to_string());
    }
    let npc = nearby_market_npc(ctx, npc_entity_id)?;
    let market_id = npc
        .market_id
        .clone()
        .ok_or_else(|| "that NPC is not a market".to_string())?;
    market_registry()
        .get(&market_id)
        .ok_or_else(|| "unknown market".to_string())?;
    let item_id = ItemId::new(item_id);
    assert_can_trade_item(&item_id)?;

    let character = caller_character(ctx)?;
    assert_order_cap(open_order_count(ctx, character.character_id, &market_id))
        .map_err(|err| err.to_string())?;
    let wallet = ensure_wallet(ctx, character.character_id);
    plan_place_buy_order(price, Gold::from_u64(wallet.gold)).map_err(|err| err.to_string())?;
    debit_gold(ctx, character.character_id, price)?;

    ctx.db.market_buy_order().insert(MarketBuyOrder {
        id: 0,
        market_id,
        buyer_character_id: character.character_id,
        item_id: item_id.as_str().to_string(),
        price_gold: price,
        created_at: ctx.timestamp,
    });
    Ok(())
}

/// Instant-sells `quantity` of an inventory pile into the best matching bid.
///
/// `min_price` is gold **per unit**; it is compared against the bid's total
/// divided by this quantity so a 10-wood dump does not fill a 5g bid for one.
#[reducer]
pub fn market_sell(
    ctx: &ReducerContext,
    npc_entity_id: u64,
    instance_id: u64,
    min_price: u64,
    quantity: u32,
) -> Result<(), String> {
    if instance_id == 0 {
        return Err("item instance is not assigned".to_string());
    }
    let min_total = listing_total(min_price, quantity).map_err(|err| err.to_string())?;
    let npc = nearby_market_npc(ctx, npc_entity_id)?;
    let acting_market = npc
        .market_id
        .clone()
        .ok_or_else(|| "that NPC is not a market".to_string())?;
    let definition = market_registry()
        .get(&acting_market)
        .ok_or_else(|| "unknown market".to_string())?;

    let seller = caller_character(ctx)?;
    let mut seller_inventory = load_inventory(ctx, seller.character_id)?;
    let instance = take_listed_instance(
        ctx,
        &mut seller_inventory,
        ItemInstanceId(instance_id),
        quantity,
    )?;
    assert_can_trade_item(&instance.item_id)?;

    let item_id = instance.item_id.as_str().to_string();
    let rows: Vec<MarketBuyOrder> = ctx
        .db
        .market_buy_order()
        .by_market_item()
        .filter((&acting_market, &item_id))
        .collect();
    let views = buy_bids_for(&rows, seller.character_id);
    let best = select_best_buy_order(&views, &acting_market, &item_id, min_total)
        .map_err(|err| err.to_string())?;
    let order = rows
        .into_iter()
        .find(|row| row.id == best.id)
        .ok_or_else(|| "order not found".to_string())?;

    let buyer = ctx
        .db
        .player()
        .character_id()
        .find(order.buyer_character_id)
        .ok_or_else(|| "buyer no longer exists".to_string())?;
    let seller_account = ensure_account_economy(ctx, seller.account_id);
    let mut buyer_inventory = load_inventory(ctx, buyer.character_id)?;
    let free = buyer_inventory.slots.iter().position(Option::is_none);

    let plan = plan_fill_buy_order(
        seller.character_id == order.buyer_character_id,
        &order.market_id,
        &acting_market,
        order.price_gold,
        definition.fee_bps,
        seller_account.fee_bps,
        free.is_some(),
    )
    .map_err(|err| err.to_string())?;

    let buyer_slot = free.expect("plan_fill_buy_order checked a free slot");
    buyer_inventory.slots[buyer_slot] = Some(instance);
    store_inventory(ctx, seller.character_id, &seller_inventory);
    store_inventory(ctx, buyer.character_id, &buyer_inventory);
    credit_gold(
        ctx,
        seller.character_id,
        plan.quote.seller_receives.amount(),
    )?;
    ctx.db.market_buy_order().id().delete(order.id);
    Ok(())
}

/// Refunds escrowed Gold and deletes the caller's bid.
#[reducer]
pub fn cancel_buy_order(ctx: &ReducerContext, order_id: u64) -> Result<(), String> {
    let character = caller_character(ctx)?;
    let order = ctx
        .db
        .market_buy_order()
        .id()
        .find(order_id)
        .ok_or_else(|| "order not found".to_string())?;
    if order.buyer_character_id != character.character_id {
        return Err("that order is not yours".to_string());
    }
    credit_gold(ctx, character.character_id, order.price_gold)?;
    ctx.db.market_buy_order().id().delete(order_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_domain::markets::{MarketError, MARKET_1_ID, MARKET_2_ID};

    fn bid(id: u64, market: &str, item: &str, price: u64, is_own: bool) -> BuyBid {
        BuyBid {
            id,
            market_id: market.to_string(),
            item_id: item.to_string(),
            price_gold: price,
            is_own,
        }
    }

    #[test]
    fn select_best_buy_order_picks_highest_then_lowest_id() {
        let bids = [
            bid(4, MARKET_1_ID, "sword", 100, false),
            bid(2, MARKET_1_ID, "sword", 300, false),
            bid(1, MARKET_1_ID, "sword", 300, false),
        ];
        let best = select_best_buy_order(&bids, MARKET_1_ID, "sword", 100).unwrap();
        assert_eq!(best.id, 1);
        assert_eq!(best.price_gold, 300);
    }

    #[test]
    fn select_best_buy_order_rejects_self_and_cross_market() {
        let own_only = [bid(1, MARKET_1_ID, "sword", 500, true)];
        assert_eq!(
            select_best_buy_order(&own_only, MARKET_1_ID, "sword", 1).unwrap_err(),
            MarketError::SelfTrade
        );
        let other_market = [bid(1, MARKET_2_ID, "sword", 500, false)];
        assert_eq!(
            select_best_buy_order(&other_market, MARKET_1_ID, "sword", 1).unwrap_err(),
            MarketError::NoMatchingBid
        );
    }

    #[test]
    fn select_best_buy_order_does_not_list_when_nothing_matches() {
        let bids = [bid(1, MARKET_1_ID, "bow", 50, false)];
        assert_eq!(
            select_best_buy_order(&bids, MARKET_1_ID, "sword", 1).unwrap_err(),
            MarketError::NoMatchingBid
        );
    }

    #[test]
    fn own_bid_is_skipped_in_favour_of_another_buyer() {
        let bids = [
            bid(1, MARKET_1_ID, "sword", 400, true),
            bid(2, MARKET_1_ID, "sword", 100, false),
        ];
        let best = select_best_buy_order(&bids, MARKET_1_ID, "sword", 1).unwrap();
        assert_eq!(best.id, 2);
    }
}
