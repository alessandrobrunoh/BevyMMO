//! Mouse picking: ground raycast (for placement) and entity raycast (for selection).

use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevymmo_shared::world::TransformData;

use crate::state::{EditorProp, EditorState, EditorTool, SelectedMarker};

/// Raycasts the ground plane (y = 0) from the editor camera through the mouse
/// position. Returns the world-space hit point or None if the ray misses.
fn ground_raycast(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor_pos: Vec2,
) -> Option<Vec3> {
    let ray = camera.viewport_to_world(camera_transform, cursor_pos).ok()?;
    let dir = ray.direction;
    if dir.y.abs() < 1e-6 {
        return None;
    }
    let t = -ray.origin.y / dir.y;
    if t < 0.0 {
        return None;
    }
    Some(ray.origin + dir * t)
}

/// Snaps a world position to the editor grid.
fn snap(value: f32, step: f32) -> f32 {
    (value / step).round() * step
}

/// Standard slab AABB ray test. Returns the entry distance along `dir` if the
/// ray hits the AABB, otherwise None.
fn ray_aabb(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut tmin = f32::NEG_INFINITY;
    let mut tmax = f32::INFINITY;
    for axis in 0..3 {
        let o = origin[axis];
        let d = dir[axis];
        let mn = min[axis];
        let mx = max[axis];
        if d.abs() < 1e-6 {
            if o < mn || o > mx {
                return None;
            }
        } else {
            let inv = 1.0 / d;
            let mut t1 = (mn - o) * inv;
            let mut t2 = (mx - o) * inv;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
    }
    if tmax < 0.0 {
        None
    } else {
        Some(tmin.max(0.0))
    }
}

/// Casts a ray and finds the closest editor prop hit (AABB test).
fn pick_prop(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor_pos: Vec2,
    props: &[(Entity, Vec3, Vec3)],
) -> Option<Entity> {
    let ray = camera.viewport_to_world(camera_transform, cursor_pos).ok()?;
    let origin = ray.origin;
    let dir = ray.direction.normalize_or_zero();
    if dir == Vec3::ZERO {
        return None;
    }
    let mut best: Option<(Entity, f32)> = None;
    for (entity, center, half_extents) in props {
        let min = *center - *half_extents;
        let max = *center + *half_extents;
        if let Some(t) = ray_aabb(origin, dir, min, max) {
            if best.map_or(true, |(_, best_t)| t < best_t) {
                best = Some((*entity, t));
            }
        }
    }
    best.map(|(e, _)| e)
}

pub fn place_or_select(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    prop_q: Query<(Entity, &EditorProp, &Transform)>,
    selected_q: Query<Entity, With<SelectedMarker>>,
    mut state: ResMut<EditorState>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };

    match state.tool {
        EditorTool::Select => {
            let candidates: Vec<(Entity, Vec3, Vec3)> = prop_q
                .iter()
                .map(|(e, _p, t)| (e, t.translation, t.scale.abs() * 0.5))
                .collect();
            if let Some(entity) = pick_prop(camera, camera_transform, cursor_pos, &candidates) {
                set_selected(&mut commands, &selected_q, entity);
                state.selected = Some(entity);
            } else {
                clear_selected(&mut commands, &selected_q);
                state.selected = None;
            }
        }
        EditorTool::Place => {
            let Some(point) = ground_raycast(camera, camera_transform, cursor_pos) else {
                return;
            };
            let snapped = Vec3::new(
                snap(point.x, state.snap_translation),
                0.0,
                snap(point.z, state.snap_translation),
            );
            if !state.manifest.bounds.contains(snapped.x, snapped.z) {
                return;
            }
            place_prop(&mut commands, &mut meshes, &mut materials, &mut state, snapped);
        }
    }
}

pub fn delete_selected(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    prop_q: Query<(Entity, &EditorProp)>,
) {
    if !keys.just_pressed(KeyCode::Delete) && !keys.just_pressed(KeyCode::Backspace) {
        return;
    }
    let Some(entity) = state.selected else {
        return;
    };
    let Ok((entity, prop)) = prop_q.get(entity) else {
        return;
    };
    state.manifest.props.retain(|p| p.id != prop.prop_id);
    state.dirty = true;
    commands.entity(entity).despawn();
    state.selected = None;
}

fn place_prop(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    state: &mut ResMut<EditorState>,
    position: Vec3,
) {
    let id = state.next_prop_id();
    let tint = tint_for_kind(&state.current_kind);
    let transform_data = TransformData {
        translation: [position.x, position.y, position.z],
        rotation_deg: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
    };
    let prop = bevymmo_shared::world::Prop {
        id: id.clone(),
        kind: state.current_kind.clone(),
        transform: transform_data,
        tint,
        collision: None,
        blocks_movement: false,
    };
    state.manifest.props.push(prop);
    state.dirty = true;

    let entity = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 0.5, 0.7),
                ..default()
            })),
            Transform::from_translation(position),
            EditorProp { prop_id: id },
        ))
        .id();
    state.selected = Some(entity);
}

fn tint_for_kind(kind: &str) -> Option<[f32; 3]> {
    match kind {
        "tree_oak" => Some([0.2, 0.5, 0.2]),
        "rock_01" => Some([0.5, 0.5, 0.5]),
        "house_simple" => Some([0.7, 0.6, 0.4]),
        _ => None,
    }
}

fn set_selected(
    commands: &mut Commands,
    selected_q: &Query<Entity, With<SelectedMarker>>,
    new: Entity,
) {
    for entity in selected_q.iter() {
        commands.entity(entity).remove::<SelectedMarker>();
    }
    commands.entity(new).insert(SelectedMarker);
}

fn clear_selected(commands: &mut Commands, selected_q: &Query<Entity, With<SelectedMarker>>) {
    for entity in selected_q.iter() {
        commands.entity(entity).remove::<SelectedMarker>();
    }
}
