//! Client-side authored map loading and placeholder rendering.
//!
//! The first world slice intentionally renders logical `kind`s as simple
//! primitives. A future asset registry will replace these meshes with GLB
//! scenes without changing the map manifest contract.

use bevy::prelude::*;
use bevymmo_shared::game_state::{GameScreen, Screen};
use bevymmo_shared::world::{load_map, MapManifest, Prop};

const MAP_PATH: &str = "assets/maps/test_1.ron";

#[derive(Resource, Default)]
pub struct ClientWorldMap {
    pub manifest: Option<MapManifest>,
    loaded: bool,
}

#[derive(Component)]
pub struct MapPropVisual {
    pub prop_id: String,
}

pub struct WorldMapPlugin;

impl Plugin for WorldMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientWorldMap>().add_systems(
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
) {
    if world_map.loaded || !matches!(screen.0, Screen::InGame | Screen::Paused) {
        return;
    }

    let manifest = match load_map(MAP_PATH) {
        Ok(manifest) => manifest,
        Err(error) => {
            error!("Unable to load client map {MAP_PATH}: {error}");
            return;
        }
    };

    info!(
        "Loaded client map {:?} with {} props",
        manifest.map_id,
        manifest.props.len()
    );

    for prop in &manifest.props {
        spawn_prop_visual(&mut commands, &mut meshes, &mut materials, prop);
    }

    world_map.manifest = Some(manifest);
    world_map.loaded = true;
}

fn cleanup_map_when_not_in_game(
    mut commands: Commands,
    screen: Res<GameScreen>,
    mut world_map: ResMut<ClientWorldMap>,
    props: Query<Entity, With<MapPropVisual>>,
) {
    if world_map.loaded && !matches!(screen.0, Screen::InGame | Screen::Paused) {
        for entity in &props {
            commands.entity(entity).despawn();
        }
        world_map.manifest = None;
        world_map.loaded = false;
    }
}

fn spawn_prop_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    prop: &Prop,
) {
    let transform = Transform {
        translation: Vec3::from_array(prop.transform.translation),
        rotation: Quat::from_euler(
            EulerRot::YXZ,
            prop.transform.rotation_deg[1].to_radians(),
            prop.transform.rotation_deg[0].to_radians(),
            prop.transform.rotation_deg[2].to_radians(),
        ),
        scale: Vec3::from_array(prop.transform.scale) * placeholder_scale(&prop.kind),
    };

    let color = prop
        .tint
        .map(|rgb| Color::srgb(rgb[0], rgb[1], rgb[2]))
        .unwrap_or_else(|| placeholder_color(&prop.kind));

    commands.spawn((
        Name::new(format!("Map Prop {} ({})", prop.id, prop.kind)),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            ..default()
        })),
        transform,
        MapPropVisual {
            prop_id: prop.id.clone(),
        },
    ));
}

fn placeholder_scale(kind: &str) -> Vec3 {
    match kind {
        "tree_oak" => Vec3::new(0.8, 2.5, 0.8),
        "rock_01" => Vec3::new(1.4, 0.8, 1.2),
        "house_simple" => Vec3::new(3.0, 2.0, 3.0),
        _ => Vec3::ONE,
    }
}

fn placeholder_color(kind: &str) -> Color {
    match kind {
        "tree_oak" => Color::srgb(0.2, 0.55, 0.2),
        "rock_01" => Color::srgb(0.45, 0.45, 0.48),
        "house_simple" => Color::srgb(0.7, 0.5, 0.25),
        _ => Color::srgb(0.35, 0.5, 0.8),
    }
}
