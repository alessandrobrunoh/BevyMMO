//! Server binding for creature placeables.
//!
//! Reads the catalog config DTOs and drives the existing
//! [`spawn_entity::<T>()`] machinery, layering per-archetype stats / hotbar /
//! rotation overrides on top of the default entity bundles.
//!
//! Map-load dispatch lives in [`super::PlaceablesPlugin`] (the
//! `spawn_placeables_on_map_load` system), which iterates the manifest props
//! and routes each creature placement through [`spawn_creature`].

use bevy::prelude::*;

use bevymmo_shared::entity::boss::components::{Boss, BossSpellbook};
use bevymmo_shared::entity::enemy::components::{AggroRange, Enemy};
use bevymmo_shared::entity::player::components::Player;
use bevymmo_shared::entity::spawn::spawn_entity;
use bevymmo_shared::network::protocol::Position;
use bevymmo_shared::placeables::config::{BossConfig, EnemyConfig};
use bevymmo_shared::placeables::{KindId, PlaceableCategory, PlaceableRegistry};

use super::interactables::spawn_interactable;
use super::npcs::spawn_npc;
use super::resources::spawn_resource_node;
use super::triggers::spawn_trigger;

// -------------------------------------------------------------------------
// Archetype tag + spawn-point resource
// -------------------------------------------------------------------------

/// Tag attached to every gameplay entity spawned from a placeable kind.
///
/// Lets gameplay systems recover the originating `KindId` of an entity without
/// keeping a parallel map of `Entity -> KindId`.
#[derive(Component, Clone, Debug)]
pub struct CreatureArchetype {
    pub kind_id: KindId,
}

/// Recorded player spawn positions read from the manifest at map load.
///
/// Players are NOT spawned at map load (the join handler in
/// `network/server.rs` does that with prediction/interpolation). This resource
/// collects the candidate spawn points so the join handler can pick one.
// TODO: join handler should read PlayerSpawnPoints.
#[derive(Resource, Default, Debug)]
pub struct PlayerSpawnPoints {
    pub positions: Vec<Vec3>,
}

/// Run-once guard: the map-load spawn pass runs the first frame the map is
/// loaded, then inserts this resource so it never runs again.
#[derive(Resource, Default, Debug)]
pub struct PlaceablesSpawned;

// -------------------------------------------------------------------------
// Dispatch + spawn helpers
// -------------------------------------------------------------------------

/// Dispatches a creature placement by looking up `kind_id` in the registry's
/// typed submaps.
///
/// Returns the spawned entity for enemies and bosses. Player spawn placements
/// are intentionally NOT spawned here (see [`PlayerSpawnPoints`]); the caller
/// records the position. Logs a warning if the kind is a creature category but
/// matches no specific binding.
pub fn spawn_creature(
    commands: &mut Commands,
    registry: &PlaceableRegistry,
    kind_id: &KindId,
    position: Vec3,
) -> Option<Entity> {
    if let Some(def) = registry.enemies.get(kind_id) {
        let config = def.enemy_config();
        Some(spawn_enemy(commands, kind_id.clone(), position, config))
    } else if let Some(def) = registry.bosses.get(kind_id) {
        let config = def.boss_config();
        Some(spawn_boss(commands, kind_id.clone(), position, config))
    } else if registry.player_spawns.contains_key(kind_id) {
        // Player spawns are recorded by the caller via `PlayerSpawnPoints`.
        None
    } else {
        warn!("No creature binding for kind '{}'", kind_id);
        None
    }
}

/// Spawns an `Enemy` from a catalog config, overriding stats, hotbar, aggro
/// range and position.
fn spawn_enemy(
    commands: &mut Commands,
    kind_id: KindId,
    position: Vec3,
    config: EnemyConfig,
) -> Entity {
    let entity = spawn_entity::<Enemy>(commands);
    let (movement, combat, vital) = config.stats.into_components();
    commands.entity(entity).insert((
        Position(position),
        CreatureArchetype { kind_id },
        movement,
        combat,
        vital,
        config.spell_hotbar,
        AggroRange(config.aggro_range),
    ));
    entity
}

/// Spawns a `Boss` from a catalog config, overriding stats, the spellbook
/// rotation (Slice 5) and position.
fn spawn_boss(
    commands: &mut Commands,
    kind_id: KindId,
    position: Vec3,
    config: BossConfig,
) -> Entity {
    let entity = spawn_entity::<Boss>(commands);
    let (movement, combat, vital) = config.stats.into_components();
    commands.entity(entity).insert((
        Position(position),
        CreatureArchetype { kind_id },
        movement,
        combat,
        vital,
        // Slice 5: override the default BossSpellbook with the per-kind rotation.
        BossSpellbook {
            spells: config.rotation,
        },
    ));
    entity
}

/// Spawns a `Player` at the given position.
///
/// Provided for completeness and tests; the map-load system does NOT call this
/// — the join handler in `network/server.rs` owns player spawning.
pub fn spawn_player(commands: &mut Commands, position: Vec3) -> Entity {
    let entity = spawn_entity::<Player>(commands);
    commands.entity(entity).insert(Position(position));
    entity
}

// -------------------------------------------------------------------------
// Map-load dispatch system
// -------------------------------------------------------------------------

/// Iterates the loaded map's props and spawns every non-prop placement.
///
/// Run-once: gated by [`PlaceablesSpawned`] so it executes exactly once, the
/// first `Update` frame after `ServerWorldMap` is available. Dispatch is by
/// [`PlaceableRegistry::category_of`]:
/// - [`PlaceableCategory::Prop`] — skipped (collision only, no entity).
/// - [`PlaceableCategory::Creature`] — enemies/bosses via [`spawn_creature`],
///   player spawns recorded into [`PlayerSpawnPoints`], NPCs via
///   [`spawn_npc`].
/// - [`PlaceableCategory::Trigger`] — [`spawn_trigger`].
/// - [`PlaceableCategory::ResourceNode`] — [`spawn_resource_node`].
/// - [`PlaceableCategory::Interactable`] — [`spawn_interactable`].
pub fn spawn_placeables_on_map_load(
    mut commands: Commands,
    world_map: Res<crate::world::ServerWorldMap>,
    registry: Res<PlaceableRegistry>,
    mut spawn_points: ResMut<PlayerSpawnPoints>,
) {
    for prop in &world_map.manifest.props {
        let Some(category) = registry.category_of(&prop.kind) else {
            continue;
        };

        let position = Vec3::from_array(prop.transform.translation);

        match category {
            PlaceableCategory::Prop => {
                // Static collision props; no entity spawned here.
            }
            PlaceableCategory::Creature => {
                if registry.enemies.contains_key(&prop.kind)
                    || registry.bosses.contains_key(&prop.kind)
                {
                    spawn_creature(&mut commands, &registry, &prop.kind, position);
                } else if registry.player_spawns.contains_key(&prop.kind) {
                    spawn_points.positions.push(position);
                } else if registry.npcs.contains_key(&prop.kind) {
                    spawn_npc(&mut commands, prop.kind.clone(), position);
                }
            }
            PlaceableCategory::Trigger => {
                if registry.triggers.contains_key(&prop.kind) {
                    spawn_trigger(&mut commands, prop.kind.clone(), position);
                }
            }
            PlaceableCategory::ResourceNode => {
                if let Some(def) = registry.resources.get(&prop.kind) {
                    spawn_resource_node(
                        &mut commands,
                        prop.kind.clone(),
                        position,
                        def.resource_config(),
                    );
                }
            }
            PlaceableCategory::Interactable => {
                if registry.interactables.contains_key(&prop.kind) {
                    spawn_interactable(&mut commands, prop.kind.clone(), position);
                }
            }
        }
    }

    commands.insert_resource(PlaceablesSpawned);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_shared::placeables_impl::creatures::goblin::GoblinDefinition;
    use bevymmo_shared::stats::components::VitalStats;
    use std::sync::Arc;

    #[test]
    fn spawn_creature_produces_goblin_enemy_with_archetype_and_stats() {
        let mut app = App::new();
        let mut registry = PlaceableRegistry::default();
        registry.register_enemy(Arc::new(GoblinDefinition));

        let goblin_id = KindId::new("mob_goblin");
        let entity = {
            let mut commands = app.world_mut().commands();
            spawn_creature(
                &mut commands,
                &registry,
                &goblin_id,
                Vec3::new(1.0, 0.0, 2.0),
            )
            .expect("goblin should spawn")
        };
        app.update();

        let entity_ref = app.world().entity(entity);
        assert!(entity_ref.contains::<Enemy>(), "Enemy marker present");
        let archetype = entity_ref
            .get::<CreatureArchetype>()
            .expect("CreatureArchetype present");
        assert_eq!(archetype.kind_id, goblin_id);
        let vital = entity_ref.get::<VitalStats>().expect("VitalStats present");
        assert_eq!(vital.max_health, 30.0, "goblin config HP override applied");
        assert_eq!(vital.current_health, 30.0);
    }

    #[test]
    fn spawn_player_records_position() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            spawn_player(&mut commands, Vec3::new(7.0, 0.0, -3.0))
        };
        app.update();

        let position = app
            .world()
            .entity(entity)
            .get::<Position>()
            .expect("Position present");
        assert_eq!(position.0, Vec3::new(7.0, 0.0, -3.0));
        assert!(app.world().entity(entity).contains::<Player>());
    }
}
