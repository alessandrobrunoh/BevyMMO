//! Ground plane + visual reference for the editor.
//!
//! The ground is a large flat mesh on the y=0 plane. Its size is driven by the
//! editor state's `MapBounds`.

use bevy::prelude::*;

use crate::state::EditorState;

const GROUND_COLOR: Color = Color::srgb(0.18, 0.22, 0.18);

#[derive(Component)]
pub struct GroundPlane;

pub fn spawn_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<EditorState>,
) {
    let size_x = (state.manifest.bounds.max_x - state.manifest.bounds.min_x).abs().max(1.0);
    let size_z = (state.manifest.bounds.max_z - state.manifest.bounds.min_z).abs().max(1.0);
    // Keep the visible placement surface aligned with the authored bounds.
    // Previously this was 20% larger, which made visible clicks silently fail.
    let size = size_x.max(size_z);

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: GROUND_COLOR,
            ..default()
        })),
        Transform::from_translation(Vec3::ZERO),
        GroundPlane,
    ));

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
