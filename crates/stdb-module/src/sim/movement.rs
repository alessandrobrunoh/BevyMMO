//! Walking characters towards their destinations.

use bevymmo_domain::movement::{self, Step};
use glam::Vec3;
use spacetimedb::{ReducerContext, Table};

use crate::rows::Vec3Row;
use crate::tables::{game_entity, grid_cell, EntityStateRow, GameEntity};

/// Advances every entity that has somewhere to be.
///
/// Straight-line only for now: terrain following and collision need the world
/// data, and arrive with it. This mirrors the branch `player_movement.rs` takes
/// when no surface is loaded.
pub fn step(ctx: &ReducerContext, dt: f32) {
    for entity in ctx.db.game_entity().iter() {
        if entity.state == EntityStateRow::Dead {
            continue;
        }
        let Some(target) = entity.move_target else {
            continue;
        };
        let position = Vec3::from(entity.position);
        let target = Vec3::from(target);

        let look = movement::look_direction(position, target)
            .map(Vec3Row::from)
            .unwrap_or(entity.look);

        let (position, move_target, state) =
            match movement::step_towards(position, target, entity.speed, dt) {
                Step::Moving(p) => (p, Some(Vec3Row::from(target)), EntityStateRow::Moving),
                // Clearing the target is what tells the client to stop predicting.
                Step::Arrived(p) => (p, None, EntityStateRow::Idle),
            };

        let position = Vec3Row::from(position);
        let (cell_x, cell_z) = grid_cell(position);
        ctx.db.game_entity().entity_id().update(GameEntity {
            position,
            look,
            move_target,
            state,
            cell_x,
            cell_z,
            ..entity
        });
    }
}
