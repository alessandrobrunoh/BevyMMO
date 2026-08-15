//! Where the caller wants to go. The tick does the walking.

use spacetimedb::{reducer, ReducerContext};

use crate::reducers::lifecycle::caller_entity;
use crate::rows::Vec3Row;
use crate::tables::{game_entity, EntityStateRow, GameEntity};

/// Sets the caller's destination.
#[reducer]
pub fn move_to(ctx: &ReducerContext, x: f32, y: f32, z: f32) -> Result<(), String> {
    let entity = caller_entity(ctx)?;
    if entity.state == EntityStateRow::Dead {
        return Err("dead characters do not walk".to_string());
    }
    ctx.db.game_entity().entity_id().update(GameEntity {
        move_target: Some(Vec3Row { x, y, z }),
        ..entity
    });
    Ok(())
}

/// Cancels any pending movement, stopping the character where it stands.
#[reducer]
pub fn stop(ctx: &ReducerContext) -> Result<(), String> {
    let entity = caller_entity(ctx)?;
    ctx.db.game_entity().entity_id().update(GameEntity {
        move_target: None,
        state: EntityStateRow::Idle,
        ..entity
    });
    Ok(())
}
