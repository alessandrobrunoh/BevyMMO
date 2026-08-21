//! Per-tick gathering: advance channels, grant pieces, interrupt on move/range.

use std::sync::OnceLock;

use bevymmo_domain::content::placeables::register_all;
use bevymmo_domain::gathering::{
    channel_duration, in_interact_range, resolve_gather, GatherAttempt,
};
use bevymmo_domain::items::components::Inventory;
use bevymmo_domain::items::registry::ItemId;
use bevymmo_domain::placeables::{PlaceableRegistry, ResourceNodePlaceable};
use bevymmo_domain::spells::components::MOVEMENT_INTERRUPT_EPSILON;
use spacetimedb::{ReducerContext, Table};

use crate::reducers::items::{grant_items, item_category, load_inventory};
use crate::reducers::parties::notify_character;
use crate::tables::{
    cast_state, game_entity, gather_session, gather_yield, resource_node, EntityStateRow,
    GatherSession, GatherYieldEvent, ResourceNode,
};

const DEPLETED_MESSAGE: &str = "Questa risorsa è già stata completamente raccolta";

fn placeables() -> &'static PlaceableRegistry {
    static PLACEABLES: OnceLock<PlaceableRegistry> = OnceLock::new();
    PLACEABLES.get_or_init(|| {
        let mut registry = PlaceableRegistry::default();
        register_all(&mut registry);
        registry
    })
}

pub fn resource_definition(kind_id: &str) -> Option<&'static dyn ResourceNodePlaceable> {
    let id = bevymmo_domain::placeables::KindId::new(kind_id.to_string());
    placeables().resources.get(&id).map(std::sync::Arc::as_ref)
}

pub fn cancel_session(ctx: &ReducerContext, entity_id: u64) {
    if ctx
        .db
        .gather_session()
        .entity_id()
        .find(&entity_id)
        .is_some()
    {
        ctx.db.gather_session().entity_id().delete(&entity_id);
    }
}

/// Advances every open gather. Called from `game_tick` after movement.
pub fn step(ctx: &ReducerContext, dt: f32) {
    let sessions: Vec<GatherSession> = ctx.db.gather_session().iter().collect();
    for session in sessions {
        advance_session(ctx, session, dt);
    }
}

fn advance_session(ctx: &ReducerContext, mut session: GatherSession, dt: f32) {
    let Some(gatherer) = ctx.db.game_entity().entity_id().find(&session.entity_id) else {
        cancel_session(ctx, session.entity_id);
        return;
    };
    if gatherer.state == EntityStateRow::Dead
        || crate::sim::spells::casting_blocked(ctx, gatherer.entity_id)
        || ctx
            .db
            .cast_state()
            .entity_id()
            .find(&gatherer.entity_id)
            .is_some()
    {
        cancel_session(ctx, session.entity_id);
        return;
    }

    let dx = gatherer.position.x - session.start_position.x;
    let dz = gatherer.position.z - session.start_position.z;
    if gatherer.move_target.is_some()
        || dx * dx + dz * dz > MOVEMENT_INTERRUPT_EPSILON * MOVEMENT_INTERRUPT_EPSILON
    {
        cancel_session(ctx, session.entity_id);
        return;
    }

    let Some(node_entity) = ctx
        .db
        .game_entity()
        .entity_id()
        .find(&session.node_entity_id)
    else {
        cancel_session(ctx, session.entity_id);
        return;
    };
    let Some(node) = ctx
        .db
        .resource_node()
        .entity_id()
        .find(&session.node_entity_id)
    else {
        cancel_session(ctx, session.entity_id);
        return;
    };
    let Some(definition) = resource_definition(&node.kind_id) else {
        cancel_session(ctx, session.entity_id);
        return;
    };
    let config = definition.resource_config();

    if !in_interact_range(
        gatherer.position.x,
        gatherer.position.z,
        node_entity.position.x,
        node_entity.position.z,
        config.interact_range,
    ) {
        cancel_session(ctx, session.entity_id);
        return;
    }

    if node.current_pieces == 0 {
        if let Some(character_id) = gatherer.owner_character_id {
            notify_character(ctx, character_id, DEPLETED_MESSAGE.to_string());
        }
        cancel_session(ctx, session.entity_id);
        return;
    }

    session.elapsed_seconds += dt;
    if session.elapsed_seconds < session.required_seconds {
        ctx.db.gather_session().entity_id().update(session);
        return;
    }

    complete_channel(ctx, session, gatherer.owner_character_id, &node, &config);
}

fn complete_channel(
    ctx: &ReducerContext,
    session: GatherSession,
    character_id: Option<spacetimedb::Uuid>,
    node: &ResourceNode,
    config: &bevymmo_domain::placeables::ResourceConfig,
) {
    let Some(character_id) = character_id else {
        cancel_session(ctx, session.entity_id);
        return;
    };
    let Ok(inventory) = load_inventory(ctx, character_id) else {
        cancel_session(ctx, session.entity_id);
        return;
    };
    let Some(category) = item_category(config.yield_item.as_str()) else {
        cancel_session(ctx, session.entity_id);
        return;
    };
    let stacks = Inventory::stacks_category(category);
    let space = inventory.space_for(&ItemId::new(config.yield_item.as_str().to_string()), stacks);
    let outcome = resolve_gather(GatherAttempt {
        yield_amount: config.yield_amount,
        bonus_extra: 0,
        current_pieces: node.current_pieces,
        inventory_space: space,
    });

    if outcome.granted == 0 {
        if outcome.node_depleted {
            notify_character(ctx, character_id, DEPLETED_MESSAGE.to_string());
        }
        cancel_session(ctx, session.entity_id);
        return;
    }

    if let Err(reason) = grant_items(
        ctx,
        character_id,
        config.yield_item.as_str(),
        outcome.granted,
    ) {
        notify_character(ctx, character_id, reason);
        cancel_session(ctx, session.entity_id);
        return;
    }

    ctx.db.resource_node().placement_id().update(ResourceNode {
        placement_id: node.placement_id.clone(),
        entity_id: node.entity_id,
        kind_id: node.kind_id.clone(),
        current_pieces: outcome.remaining_pieces,
        last_regen_at: node.last_regen_at,
    });
    ctx.db.gather_yield().insert(GatherYieldEvent {
        entity_id: session.entity_id,
        node_entity_id: session.node_entity_id,
        item_id: config.yield_item.as_str().to_string(),
        amount: outcome.granted,
        extra: outcome.extra,
        node_depleted: outcome.node_depleted,
    });

    if outcome.session_ends {
        cancel_session(ctx, session.entity_id);
        return;
    }

    let required_seconds =
        channel_duration(config.channel_seconds, config.min_channel_seconds, 0.0);
    ctx.db.gather_session().entity_id().update(GatherSession {
        elapsed_seconds: 0.0,
        required_seconds,
        ..session
    });
}
