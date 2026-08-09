//! Client-side authored map loading and placeholder rendering.
//!
//! The first world slice intentionally renders logical `kind`s as simple
//! primitives. A future asset registry will replace these meshes with GLB
//! scenes without changing the map manifest contract.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevymmo_shared::game_state::{GameScreen, Screen};
use bevymmo_shared::movement::ClientSurfaceQuery;
use bevymmo_shared::paths;
use bevymmo_shared::placeables::{AssetHint, PlaceableRegistry};
use bevymmo_shared::world::{
    load_map_auto, CollisionGrid, MapManifest, Prop, SurfaceQuery, Terrain,
};

#[derive(Resource, Default)]
pub struct ClientWorldMap {
    pub manifest: Option<MapManifest>,
    pub collision: Option<CollisionGrid>,
    pub surface_query: Option<SurfaceQuery>,
    /// Whether a load attempt has already happened (success or failure).
    /// Prevents the loader from re-running every frame when the map file is
    /// missing or invalid, which would otherwise spam the log.
    load_attempted: bool,
}

#[derive(Component)]
pub struct MapPropVisual {
    pub prop_id: String,
}

#[derive(Component)]
pub struct MapTerrainVisual;

#[derive(Component)]
pub struct MapSceneVisual;

use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct ClientPropMeshRegistry {
    cuboid_1x1: Option<Handle<Mesh>>,
    materials: HashMap<[u32; 3], Handle<StandardMaterial>>,
}

impl ClientPropMeshRegistry {
    pub fn get_or_create_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.cuboid_1x1
            .get_or_insert_with(|| meshes.add(Cuboid::new(1.0, 1.0, 1.0)))
            .clone()
    }

    pub fn get_or_create_material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        color: Color,
    ) -> Handle<StandardMaterial> {
        let key = color_key(color);
        self.materials
            .entry(key)
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: color,
                    ..default()
                })
            })
            .clone()
    }
}

fn color_key(color: Color) -> [u32; 3] {
    let [r, g, b, _] = color.to_srgba().to_f32_array();
    [r.to_bits(), g.to_bits(), b.to_bits()]
}

pub struct WorldMapPlugin;

impl Plugin for WorldMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientWorldMap>()
            .init_resource::<ClientSurfaceQuery>()
            .init_resource::<ClientPropMeshRegistry>()
            .add_systems(
                Update,
                (
                    load_map_when_in_game,
                    cleanup_map_when_not_in_game,
                    sync_surface_query,
                )
                    .chain(),
            );
    }
}

fn load_map_when_in_game(
    mut commands: Commands,
    screen: Res<GameScreen>,
    mut world_map: ResMut<ClientWorldMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut registry: ResMut<ClientPropMeshRegistry>,
    placeables: Res<PlaceableRegistry>,
    asset_server: Res<AssetServer>,
) {
    if world_map.load_attempted || !matches!(screen.0, Screen::InGame | Screen::Paused) {
        return;
    }
    world_map.load_attempted = true;

    let map_path = paths::default_map_file();
    let manifest = match load_map_auto(&map_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            error!("Unable to load client map {}: {error}", map_path.display());
            return;
        }
    };

    info!(
        "Loaded client map {:?} with {} props",
        manifest.map_id,
        manifest.props.len()
    );

    if should_load_map_scene(&manifest) {
        spawn_heightfield_visual(&mut commands, &mut meshes, &mut materials, &manifest);
        spawn_map_scene_visual(&mut commands, &asset_server, &manifest);
    } else {
        spawn_terrain_visual(
            &mut commands,
            &mut meshes,
            &mut materials,
            &manifest.terrain,
        );
        for prop in &manifest.props {
            spawn_prop_visual(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut registry,
                &placeables,
                &asset_server,
                prop,
            );
        }
    }

    world_map.collision = Some(CollisionGrid::build(&manifest));
    world_map.surface_query = Some(SurfaceQuery::from_manifest(&manifest));
    world_map.manifest = Some(manifest);
}

fn cleanup_map_when_not_in_game(
    mut commands: Commands,
    screen: Res<GameScreen>,
    mut world_map: ResMut<ClientWorldMap>,
    props: Query<Entity, With<MapPropVisual>>,
    terrain: Query<Entity, With<MapTerrainVisual>>,
    scenes: Query<Entity, With<MapSceneVisual>>,
) {
    if world_map.load_attempted && !matches!(screen.0, Screen::InGame | Screen::Paused) {
        for entity in props.iter().chain(terrain.iter()).chain(scenes.iter()) {
            commands.entity(entity).despawn();
        }
        world_map.manifest = None;
        world_map.collision = None;
        world_map.surface_query = None;
        world_map.load_attempted = false;
    }
}

/// Synchronizes the shared `ClientSurfaceQuery` resource with the local `ClientWorldMap`.
///
/// This bridges the presentation layer (which owns `ClientWorldMap`) with the client
/// layer (which needs surface data for click-to-move) by copying the surface query
/// data to a shared resource that both crates can access without cross-dependencies.
fn sync_surface_query(
    world_map: Res<ClientWorldMap>,
    mut shared_surface_query: ResMut<ClientSurfaceQuery>,
) {
    if !world_map.is_changed() {
        return;
    }

    shared_surface_query.0.clone_from(&world_map.surface_query);
}

fn should_load_map_scene(manifest: &MapManifest) -> bool {
    manifest.version >= 2 && !manifest.surfaces.is_empty()
}

/// Loads the visual GLB for maps whose gameplay data comes from a `.world.json`
/// sidecar.
///
/// Version 2 maps keep authoritative gameplay data in JSON and visual meshes in
/// the sibling GLB. Loading the whole scene here prevents the client from
/// falling back to the old placeholder terrain path while the server can still
/// ignore visual content.
///
/// # Example
/// ```ignore
/// // rolling_hills_test.world.json -> maps/rolling_hills_test.glb#Scene0
/// spawn_map_scene_visual(&mut commands, &asset_server, &manifest);
/// ```
fn spawn_map_scene_visual(
    commands: &mut Commands,
    asset_server: &AssetServer,
    manifest: &MapManifest,
) {
    let scene_path = format!("maps/{}.glb#Scene0", manifest.map_id);
    let handle = asset_server.load::<WorldAsset>(scene_path);
    commands.spawn((
        Name::new(format!("Map Scene {}", manifest.map_id)),
        Transform::default(),
        WorldAssetRoot(handle),
        MapSceneVisual,
    ));
}

/// Renders the authored ground cube (unit mesh, so `scale` is the full size).
/// Mirrors the editor's terrain visual so maps look the same in-game.
fn spawn_heightfield_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    manifest: &MapManifest,
) {
    let Some(heightfield) = manifest
        .surfaces
        .iter()
        .find_map(|surface| surface.heightfield.as_ref())
    else {
        return;
    };

    let resolution = heightfield.resolution as usize;
    let side = resolution + 1;
    if resolution == 0 || heightfield.heights.len() != side * side {
        warn!("Skipping invalid heightfield for map {:?}", manifest.map_id);
        return;
    }

    let bounds = heightfield.bounds;
    let mut positions = Vec::with_capacity(side * side);
    let mut normals = Vec::with_capacity(side * side);
    for row in 0..side {
        let z = bounds.min_z + (bounds.max_z - bounds.min_z) * row as f32 / resolution as f32;
        for column in 0..side {
            let x =
                bounds.min_x + (bounds.max_x - bounds.min_x) * column as f32 / resolution as f32;
            positions.push([x, heightfield.heights[row * side + column] - 0.02, z]);
            normals.push([0.0, 1.0, 0.0]);
        }
    }

    let mut indices = Vec::with_capacity(resolution * resolution * 6);
    for row in 0..resolution {
        for column in 0..resolution {
            let top_left = (row * side + column) as u32;
            let top_right = top_left + 1;
            let bottom_left = top_left + side as u32;
            let bottom_right = bottom_left + 1;
            indices.extend_from_slice(&[
                top_left,
                bottom_left,
                top_right,
                top_right,
                bottom_left,
                bottom_right,
            ]);
        }
    }

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.30, 0.10),
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.spawn((
        Name::new(format!("Map Heightfield {}", manifest.map_id)),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        MapTerrainVisual,
    ));
}

fn spawn_terrain_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain: &Terrain,
) {
    let color = terrain
        .tint
        .map(|rgb| Color::srgb(rgb[0], rgb[1], rgb[2]))
        .unwrap_or(Color::srgb(0.16, 0.2, 0.16));
    commands.spawn((
        Name::new("Map Terrain"),
        Transform {
            translation: Vec3::from_array(terrain.transform.translation),
            rotation: Quat::from_euler(
                EulerRot::YXZ,
                terrain.transform.rotation_deg[1].to_radians(),
                terrain.transform.rotation_deg[0].to_radians(),
                terrain.transform.rotation_deg[2].to_radians(),
            ),
            scale: Vec3::from_array(terrain.transform.scale),
        },
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            ..default()
        })),
        MapTerrainVisual,
    ));
}

fn spawn_prop_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    registry: &mut ClientPropMeshRegistry,
    placeables: &PlaceableRegistry,
    asset_server: &AssetServer,
    prop: &Prop,
) {
    // Only static props render a client-side visual. Creature / trigger /
    // resource / interactable kinds are spawned by the server and reach the
    // client through entity replication, so we must not also build a local
    // placeholder cuboid for them.
    let Some(definition) = placeables.props.get(&prop.kind) else {
        return;
    };

    let defaults = definition.defaults();

    // The per-placement scale multiplies the kind's inherent size, so a prop
    // placed with the manifest's default (unit) scale renders at its
    // definition size.
    let transform = Transform {
        translation: Vec3::from_array(prop.transform.translation),
        rotation: Quat::from_euler(
            EulerRot::YXZ,
            prop.transform.rotation_deg[1].to_radians(),
            prop.transform.rotation_deg[0].to_radians(),
            prop.transform.rotation_deg[2].to_radians(),
        ),
        scale: Vec3::from_array(prop.transform.scale) * Vec3::from_array(defaults.transform.scale),
    };

    let mut entity = commands.spawn((
        Name::new(format!("Map Prop {} ({})", prop.id, prop.kind)),
        transform,
        MapPropVisual {
            prop_id: prop.id.clone(),
        },
    ));

    match definition.asset_hint() {
        AssetHint::Scene(path) => {
            // The hint points at the .glb file; load its first scene as a
            // WorldAsset. Bevy's built-in scene spawner instantiates the
            // scene as children of the WorldAssetRoot entity.
            let handle = asset_server.load::<WorldAsset>(format!("{path}#Scene0"));
            entity.insert(WorldAssetRoot(handle));
        }
        AssetHint::Placeholder => {
            let color = prop
                .tint
                .map(|rgb| Color::srgb(rgb[0], rgb[1], rgb[2]))
                .or_else(|| defaults.tint.map(|rgb| Color::srgb(rgb[0], rgb[1], rgb[2])))
                .unwrap_or_else(|| Color::srgb(0.35, 0.5, 0.8));

            let mesh = registry.get_or_create_mesh(meshes);
            let mat = registry.get_or_create_material(materials, color);
            entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
        }
        AssetHint::Invisible => {
            // Marker-only placement: keep the tagged entity, attach no visual.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct SurfaceQueryChangeCount(usize);

    fn count_surface_query_changes(mut count: ResMut<SurfaceQueryChangeCount>) {
        count.0 += 1;
    }

    #[test]
    fn surface_query_is_not_rewritten_when_world_map_is_unchanged() {
        let mut app = App::new();
        app.init_resource::<ClientWorldMap>()
            .init_resource::<ClientSurfaceQuery>()
            .init_resource::<SurfaceQueryChangeCount>()
            .add_systems(
                Update,
                (
                    sync_surface_query,
                    count_surface_query_changes.run_if(resource_changed::<ClientSurfaceQuery>),
                )
                    .chain(),
            );

        app.update();
        assert_eq!(app.world().resource::<SurfaceQueryChangeCount>().0, 1);

        app.update();
        assert_eq!(app.world().resource::<SurfaceQueryChangeCount>().0, 1);
    }
}
