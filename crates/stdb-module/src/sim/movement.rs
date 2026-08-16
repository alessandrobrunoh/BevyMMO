//! Walking characters towards their destinations.

use bevymmo_domain::movement::{self, TerrainStep};
use glam::Vec3;
use spacetimedb::{ReducerContext, Table};

use crate::rows::Vec3Row;
use crate::tables::{game_entity, grid_cell, EntityStateRow, GameEntity};
use crate::world;

/// Advances every entity that has somewhere to be across the embedded map.
pub fn step(ctx: &ReducerContext, dt: f32) {
    let Some(map) = world::default_map() else {
        log::error!("default map is unavailable; skipping movement step");
        return;
    };
    let max_step_height = map.manifest.get_world_metrics().max_step_height;

    // A reducer must not mutate a table while iterating it. Snapshotting also
    // makes every entity's step observe the same pre-movement world state.
    let moving: Vec<GameEntity> = ctx
        .db
        .game_entity()
        .iter()
        .filter(|entity| entity.state != EntityStateRow::Dead && entity.move_target.is_some())
        .collect();

    for entity in moving {
        let target = entity.move_target.expect("filtered above");
        let mut position = Vec3::from(entity.position);
        movement::snap_to_ground(&mut position, &map.surfaces, max_step_height);

        let look = movement::look_direction(position, Vec3::from(target))
            .map(Vec3Row::from)
            .unwrap_or(entity.look);

        let (position, move_target, state) = match movement::step_on_terrain(
            position,
            target.x,
            target.z,
            entity.speed * dt,
            &map.surfaces,
            &map.collision,
            max_step_height,
        ) {
            TerrainStep::Moved(position) => (position, Some(target), EntityStateRow::Moving),
            // Clearing the target is what tells the client to stop predicting.
            TerrainStep::Arrived(position) => (position, None, EntityStateRow::Idle),
            // Blocked is *this tick*, not the journey: the stepper already tried
            // both slide directions and none fit right now. Keeping the target
            // is what lets a character press along a wall and round it over the
            // following ticks, which is how the Bevy server behaved. Dropping it
            // here made a single click into a slope stop the character dead,
            // with nothing on screen to say why.
            TerrainStep::Blocked => (position, Some(target), EntityStateRow::Idle),
            // No surface under the destination, on the other hand, will not
            // become reachable by trying again — the target is off the map.
            TerrainStep::NoSurface => (position, None, EntityStateRow::Idle),
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
