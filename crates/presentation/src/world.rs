//! Client-side authored map loading and placeholder rendering.
//!
//! The first world slice intentionally renders logical `kind`s as simple
//! primitives. A future asset registry will replace these meshes with GLB
//! scenes without changing the map manifest contract.

use bevy::prelude::*;
use bevymmo_shared::game_state::{GameScreen, Screen};
use bevymmo_shared::paths;
use bevymmo_shared::placeables::{AssetHint, PlaceableRegistry};
use bevymmo_shared::world::{load_map, CollisionGrid, MapManifest, Prop, Terrain};

#[derive(Resource, Default)]
pub struct ClientWorldMap {
    pub manifest: Option<MapManifest>,
    pub collision: Option<CollisionGrid>,
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
            .or_insert_with(|| materials.add(StandardMaterial {
                base_color: color,
                ..default()
            }))
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
            .init_resource::<ClientPropMeshRegistry>()
            .add_systems(
                Update,
                (load_map_when_in_game, cleanup_map_when_not_in_game).chain(),
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
    let manifest = match load_map(&map_path) {
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

    world_map.collision = Some(CollisionGrid::build(&manifest));
    world_map.manifest = Some(manifest);
}

fn cleanup_map_when_not_in_game(
    mut commands: Commands,
    screen: Res<GameScreen>,
    mut world_map: ResMut<ClientWorldMap>,
    props: Query<Entity, With<MapPropVisual>>,
    terrain: Query<Entity, With<MapTerrainVisual>>,
) {
    if world_map.load_attempted && !matches!(screen.0, Screen::InGame | Screen::Paused) {
        for entity in props.iter().chain(terrain.iter()) {
            commands.entity(entity).despawn();
        }
        world_map.manifest = None;
        world_map.collision = None;
        world_map.load_attempted = false;
    }
}

/// Renders the authored ground cube (unit mesh, so `scale` is the full size).
/// Mirrors the editor's terrain visual so maps look the same in-game.
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
        scale: Vec3::from_array(prop.transform.scale)
            * Vec3::from_array(defaults.transform.scale),
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
                .or_else(|| {
                    defaults
                        .tint
                        .map(|rgb| Color::srgb(rgb[0], rgb[1], rgb[2]))
                })
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
