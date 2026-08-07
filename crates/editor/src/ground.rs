//! Terrain cube, snap grid overlay and scene lighting for the editor.
//!
//! The terrain is a single large cube (unit mesh) that authors can select and
//! manipulate with the transform gizmo, exactly like a prop. Its transform is
//! persisted in `MapManifest::terrain`.

use bevy::prelude::*;

use crate::state::{quat_from_rotation_deg, EditorState, EditorTerrain};

const TERRAIN_COLOR: Color = Color::srgb(0.16, 0.20, 0.16);
const GRID_COLOR: Color = Color::srgba(0.85, 0.92, 1.0, 0.12);
const BOUNDS_COLOR: Color = Color::srgba(1.0, 0.55, 0.25, 0.6);

/// Spawns the terrain cube and the scene lights. Called once at startup.
pub fn spawn_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<EditorState>,
) {
    spawn_terrain_entity(&mut commands, &mut meshes, &mut materials, &mut state);

    // Sun-like directional light so PBR materials have shading.
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.6, 0.0)),
    ));

    commands.spawn(AmbientLight {
        color: Color::WHITE,
        brightness: 250.0,
        affects_lightmapped_meshes: true,
    });
}

/// Builds (or rebuilds) the terrain cube entity from the manifest and records
/// its id in `state.terrain_entity`. Shared by startup and scene rebuilds.
pub fn spawn_terrain_entity(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    state: &mut ResMut<EditorState>,
) -> Entity {
    let terrain = state.manifest.terrain;
    let color = tint_color(terrain.tint).unwrap_or(TERRAIN_COLOR);
    let entity = commands
        .spawn((
            Name::new("Terrain"),
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })),
            Transform {
                translation: Vec3::from_array(terrain.transform.translation),
                rotation: quat_from_rotation_deg(terrain.transform.rotation_deg),
                scale: Vec3::from_array(terrain.transform.scale),
            },
            EditorTerrain,
        ))
        .id();
    state.terrain_entity = Some(entity);
    entity
}

/// Editor-only visual feedback: snap grid on the ground plane and the map
/// bounds rectangle. Drawn every frame so it always matches the current snap
/// step and map bounds.
pub fn draw_grid(mut gizmos: Gizmos, state: Res<EditorState>) {
    if state.show_grid {
        let bounds = state.manifest.bounds;
        let step = state.snap_translation.max(0.25);
        let min = Vec3::new(bounds.min_x, 0.02, bounds.min_z);
        let max = Vec3::new(bounds.max_x, 0.02, bounds.max_z);

        let mut x = bounds.min_x;
        while x <= bounds.max_x + 0.001 {
            gizmos.line(
                Vec3::new(x, 0.02, bounds.min_z),
                Vec3::new(x, 0.02, bounds.max_z),
                GRID_COLOR,
            );
            x += step;
        }
        let mut z = bounds.min_z;
        while z <= bounds.max_z + 0.001 {
            gizmos.line(
                Vec3::new(bounds.min_x, 0.02, z),
                Vec3::new(bounds.max_x, 0.02, z),
                GRID_COLOR,
            );
            z += step;
        }

        // Map bounds outline.
        gizmos.line(
            Vec3::new(min.x, 0.02, min.z),
            Vec3::new(max.x, 0.02, min.z),
            BOUNDS_COLOR,
        );
        gizmos.line(
            Vec3::new(max.x, 0.02, min.z),
            Vec3::new(max.x, 0.02, max.z),
            BOUNDS_COLOR,
        );
        gizmos.line(
            Vec3::new(max.x, 0.02, max.z),
            Vec3::new(min.x, 0.02, max.z),
            BOUNDS_COLOR,
        );
        gizmos.line(
            Vec3::new(min.x, 0.02, max.z),
            Vec3::new(min.x, 0.02, min.z),
            BOUNDS_COLOR,
        );
    }
}

/// Converts the manifest tint (linear RGB 0..1) to a Bevy color.
fn tint_color(tint: Option<[f32; 3]>) -> Option<Color> {
    tint.map(|rgb| Color::srgb(rgb[0], rgb[1], rgb[2]))
}
