//! Mouse picking: ground raycast (placement) and OBB raycast (selection).
//!
//! The editor intentionally uses its own lightweight raycasts instead of the
//! built-in mesh picking: it needs to treat the terrain as a normal pickable
//! body, ignore gizmo handles, and distinguish "clicked empty space".

use bevy::gizmos::transform_gizmo::TransformGizmoState;
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevymmo_shared::world::{Prop, TransformData};

use crate::camera::EditorCamera;
use crate::ground::spawn_terrain_entity;
use crate::history::EditorHistory;
use crate::state::{
    quat_from_rotation_deg, EditorProp, EditorState, EditorTerrain, EditorTool, SelectedMarker,
};
use crate::ui::cursor_over_editor_chrome;

use std::collections::HashMap;

/// Pre-allocated mesh and material handles shared across props of the same
/// kind/color, enabling Bevy's automatic GPU instancing.
#[derive(Resource, Default)]
pub struct PropMeshRegistry {
    cuboid_1x1: Option<Handle<Mesh>>,
    materials: HashMap<[u32; 3], Handle<StandardMaterial>>,
}

impl PropMeshRegistry {
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

/// Cached list of pickable bodies, rebuilt only when transforms change.
#[derive(Resource, Default)]
pub struct PickBodyCache {
    pub bodies: Vec<PickBody>,
}

/// Rebuilds the pick cache only when a prop/terrain transform actually changes or count differs.
pub fn refresh_pick_cache(
    mut cache: ResMut<PickBodyCache>,
    changed_props: Query<
        (),
        Or<(
            Changed<Transform>,
            Added<EditorProp>,
            Added<EditorTerrain>,
        )>,
    >,
    prop_q: Query<(Entity, &EditorProp, &Transform), Without<EditorTerrain>>,
    terrain_q: Query<(Entity, &Transform), (With<EditorTerrain>, Without<EditorProp>)>,
) {
    let prop_count = prop_q.iter().len();
    let terrain_count = terrain_q.iter().len();
    let total_count = prop_count + terrain_count;

    if cache.bodies.len() == total_count && changed_props.is_empty() {
        return;
    }

    cache.bodies = collect_bodies(&prop_q, &terrain_q);
}

/// A body that can be clicked in the editor: a prop or the terrain cube.
/// `half` is the half-extent of the *rendered* mesh (props render a 2x2x2
/// cuboid, terrain a unit cuboid).
#[derive(Clone, Copy)]
pub(crate) struct PickBody {
    entity: Entity,
    center: Vec3,
    rotation: Quat,
    half: Vec3,
}

/// Raycasts the ground plane (y = 0) from the editor camera through the mouse
/// position. Returns the world-space hit point or None if the ray misses.
fn ground_raycast(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor_pos: Vec2,
) -> Option<Vec3> {
    let ray = camera
        .viewport_to_world(camera_transform, cursor_pos)
        .ok()?;
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

/// Oriented box ray test: transforms the ray into the box's local space and
/// runs the slab test there, so rotated/scaled props pick correctly.
fn ray_obb(origin: Vec3, dir: Vec3, center: Vec3, rotation: Quat, half: Vec3) -> Option<f32> {
    let inv_rot = rotation.inverse();
    let local_origin = inv_rot * (origin - center);
    let local_dir = (inv_rot * dir).normalize_or_zero();
    if local_dir == Vec3::ZERO {
        return None;
    }
    ray_aabb(local_origin, local_dir, -half, half)
}

/// Casts a ray and finds the closest pickable body.
fn pick_closest(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor_pos: Vec2,
    bodies: &[PickBody],
) -> Option<Entity> {
    let ray = camera
        .viewport_to_world(camera_transform, cursor_pos)
        .ok()?;
    let origin = ray.origin;
    let dir = ray.direction.normalize_or_zero();
    if dir == Vec3::ZERO {
        return None;
    }
    let mut best: Option<(Entity, f32)> = None;
    for body in bodies {
        if let Some(t) = ray_obb(origin, dir, body.center, body.rotation, body.half) {
            if best.map_or(true, |(_, best_t)| t < best_t) {
                best = Some((body.entity, t));
            }
        }
    }
    best.map(|(entity, _)| entity)
}

/// Collects every clickable body (props + terrain) with its rendered half
/// extents: props render a 2x2x2 cuboid, the terrain a unit cuboid.
fn collect_bodies(
    prop_q: &Query<(Entity, &EditorProp, &Transform), Without<EditorTerrain>>,
    terrain_q: &Query<(Entity, &Transform), (With<EditorTerrain>, Without<EditorProp>)>,
) -> Vec<PickBody> {
    let mut bodies = Vec::with_capacity(prop_q.iter().len() + 1);
    for (entity, _prop, transform) in prop_q {
        bodies.push(PickBody {
            entity,
            center: transform.translation,
            rotation: transform.rotation,
            half: transform.scale.abs(),
        });
    }
    for (entity, transform) in terrain_q {
        bodies.push(PickBody {
            entity,
            center: transform.translation,
            rotation: transform.rotation,
            half: transform.scale.abs() * 0.5,
        });
    }
    bodies
}

/// Handles left clicks: select/move tools pick bodies, Place drops a new prop
/// on the ground, Erase deletes a prop under the cursor.
pub fn place_or_select(
    mut mouse_events: MessageReader<MouseButtonInput>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut registry: ResMut<PropMeshRegistry>,
    cache: Res<PickBodyCache>,
    prop_q: Query<(Entity, &EditorProp, &Transform), Without<EditorTerrain>>,
    selected_q: Query<Entity, With<SelectedMarker>>,
    gizmo_state: Res<TransformGizmoState>,
    mut state: ResMut<EditorState>,
    mut history: ResMut<EditorHistory>,
) {
    // Ignore clicks that land on the gizmo handles or happen mid-drag.
    if gizmo_state.active || gizmo_state.hovered_axis.is_some() {
        return;
    }

    let left_clicked = mouse_events
        .read()
        .any(|event| event.button == MouseButton::Left && event.state.is_pressed());
    if !left_clicked {
        return;
    }
    let Ok(window) = windows.single() else {
        warn!("Editor click ignored: primary window not found");
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        warn!("Editor click ignored: cursor is outside the window");
        return;
    };
    if cursor_over_editor_chrome(window, cursor_pos) {
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        warn!("Editor click ignored: editor camera not found");
        return;
    };

    match state.tool {
        EditorTool::Select | EditorTool::Move | EditorTool::Rotate | EditorTool::Scale => {
            if let Some(entity) = pick_closest(camera, camera_transform, cursor_pos, &cache.bodies) {
                set_selected(&mut commands, &selected_q, entity);
                state.selected = Some(entity);
            } else {
                clear_selected(&mut commands, &selected_q);
                state.selected = None;
            }
        }
        EditorTool::Place => {
            let Some(point) = ground_raycast(camera, camera_transform, cursor_pos) else {
                warn!("Editor placement ignored: mouse ray does not intersect ground plane");
                return;
            };
            let snapped = Vec3::new(
                snap(point.x, state.snap_translation),
                0.0,
                snap(point.z, state.snap_translation),
            );
            if !state.manifest.bounds.contains(snapped.x, snapped.z) {
                warn!(
                    "Editor placement ignored: ({:.1}, {:.1}) is outside map bounds",
                    snapped.x, snapped.z
                );
                return;
            }
            info!(
                "Placing {} at ({:.1}, {:.1}, {:.1})",
                state.current_kind, snapped.x, snapped.y, snapped.z
            );
            history.push(&state.manifest);
            state.validation_dirty = true;
            place_prop(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut registry,
                &mut state,
                snapped,
            );
        }
        EditorTool::Erase => {
            if let Some(entity) = pick_closest(camera, camera_transform, cursor_pos, &cache.bodies) {
                if let Ok((entity, prop, _)) = prop_q.get(entity) {
                    history.push(&state.manifest);
                    state.validation_dirty = true;
                    erase_prop(&mut commands, &mut state, entity, &prop);
                }
            }
        }
    }
}

/// Tracks the hovered body for selection feedback. Cheap enough to run every
/// frame; only meaningful for tools that operate on existing bodies.
pub fn update_hover(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    cache: Res<PickBodyCache>,
    mut state: ResMut<EditorState>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    if cursor_over_editor_chrome(window, cursor_pos) {
        state.hovered = None;
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };

    if !matches!(
        state.tool,
        EditorTool::Select | EditorTool::Move | EditorTool::Rotate | EditorTool::Scale
    ) {
        state.hovered = None;
        return;
    }
    state.hovered = pick_closest(camera, camera_transform, cursor_pos, &cache.bodies);
}

pub fn delete_selected(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    mut history: ResMut<EditorHistory>,
    prop_q: Query<(Entity, &EditorProp), Without<EditorTerrain>>,
) {
    if !keys.just_pressed(KeyCode::Delete) && !keys.just_pressed(KeyCode::Backspace) {
        return;
    }
    let Some(entity) = state.selected else {
        return;
    };
    let Ok((entity, prop)) = prop_q.get(entity) else {
        warn!("Cannot erase the terrain; select a prop instead");
        return;
    };
    history.push(&state.manifest);
    state.validation_dirty = true;
    erase_prop(&mut commands, &mut state, entity, &prop);
}

/// Clears the selection when Escape is pressed.
pub fn deselect_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    selected_q: Query<Entity, With<SelectedMarker>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    clear_selected(&mut commands, &selected_q);
    state.selected = None;
    state.hovered = None;
}

/// Rebuilds every visual (terrain + props) from the manifest after a load.
/// Despawns the old scene first so stale entities never linger.
pub fn rebuild_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut registry: ResMut<PropMeshRegistry>,
    mut state: ResMut<EditorState>,
    prop_q: Query<Entity, With<EditorProp>>,
    terrain_q: Query<Entity, With<EditorTerrain>>,
) {
    if !state.needs_rebuild {
        return;
    }
    for entity in prop_q.iter() {
        commands.entity(entity).despawn();
    }
    for entity in terrain_q.iter() {
        commands.entity(entity).despawn();
    }
    state.selected = None;
    state.hovered = None;
    state.needs_rebuild = false;

    spawn_terrain_entity(&mut commands, &mut meshes, &mut materials, &mut state);
    for prop in &state.manifest.props {
        spawn_prop_entity(&mut commands, &mut meshes, &mut materials, prop, &mut registry);
    }
}

/// Removes a prop from both the manifest and the scene.
fn erase_prop(
    commands: &mut Commands,
    state: &mut ResMut<EditorState>,
    entity: Entity,
    prop: &EditorProp,
) {
    state.manifest.props.retain(|p| p.id != prop.prop_id);
    state.dirty = true;
    commands.entity(entity).despawn();
    if state.selected == Some(entity) {
        state.selected = None;
        state.hovered = None;
    }
}

/// Spawns a visual for a manifest prop (2x2x2 cuboid scaled by the authored
/// scale, matching the client's placeholder rendering).
pub fn spawn_prop_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    prop: &Prop,
    registry: &mut PropMeshRegistry,
) -> Entity {
    let base_color = prop
        .tint
        .map(|rgb| Color::srgb(rgb[0], rgb[1], rgb[2]))
        .unwrap_or_else(|| {
            tint_for_kind(prop.kind.as_str())
                .map(|rgb| Color::srgb(rgb[0], rgb[1], rgb[2]))
                .unwrap_or(Color::srgb(0.4, 0.5, 0.7))
        });
    let mesh = registry.get_or_create_mesh(meshes);
    let mat = registry.get_or_create_material(materials, base_color);
    commands
        .spawn((
            Name::new(format!("{} ({})", prop.id, prop.kind)),
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform {
                translation: Vec3::from_array(prop.transform.translation),
                rotation: quat_from_rotation_deg(prop.transform.rotation_deg),
                scale: Vec3::from_array(prop.transform.scale),
            },
            EditorProp {
                prop_id: prop.id.clone(),
            },
        ))
        .id()
}

/// Spawns a prop, adds it to the manifest, and selects it.
fn place_prop(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    registry: &mut ResMut<PropMeshRegistry>,
    state: &mut ResMut<EditorState>,
    position: Vec3,
) {
    let id = state.next_prop_id();
    let kind = state.current_kind.clone();
    let tint = tint_for_kind(&kind);
    let visual_scale = visual_scale_for_kind(&kind);
    let prop = Prop {
        id: id.clone(),
        kind: bevymmo_shared::placeables::KindId::new(kind),
        transform: TransformData {
            translation: [position.x, position.y, position.z],
            rotation_deg: [0.0, 0.0, 0.0],
            scale: [visual_scale.x, visual_scale.y, visual_scale.z],
        },
        tint,
        collision: None,
        blocks_movement: false,
    };
    let entity = spawn_prop_entity(commands, meshes, materials, &prop, registry);
    state.manifest.props.push(prop);
    state.dirty = true;

    commands.entity(entity).insert(SelectedMarker);
    state.selected = Some(entity);
}

/// Default authored scale written to the manifest for a placed prop.
fn visual_scale_for_kind(kind: &str) -> Vec3 {
    match kind {
        "tree_oak" => Vec3::new(0.8, 2.5, 0.8),
        "rock_01" => Vec3::new(1.4, 0.8, 1.2),
        "rock_02" => Vec3::new(0.9, 0.6, 0.8),
        "bush_01" => Vec3::new(0.8, 0.7, 0.8),
        "fence_01" => Vec3::new(1.6, 0.9, 0.2),
        "lamp_01" => Vec3::new(0.3, 1.6, 0.3),
        "crate_01" => Vec3::new(0.6, 0.6, 0.6),
        "statue_01" => Vec3::new(0.8, 2.0, 0.8),
        "house_simple" => Vec3::new(3.0, 2.0, 3.0),
        _ => Vec3::ONE,
    }
}

/// Default tint for a placed prop. `None` keeps the engine default color.
pub fn tint_for_kind(kind: &str) -> Option<[f32; 3]> {
    match kind {
        "tree_oak" => Some([0.2, 0.5, 0.2]),
        "rock_01" => Some([0.5, 0.5, 0.5]),
        "rock_02" => Some([0.45, 0.42, 0.38]),
        "bush_01" => Some([0.25, 0.45, 0.2]),
        "fence_01" => Some([0.55, 0.4, 0.25]),
        "lamp_01" => Some([0.9, 0.85, 0.5]),
        "crate_01" => Some([0.6, 0.45, 0.3]),
        "statue_01" => Some([0.75, 0.75, 0.78]),
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

/// Public alias used by other editor modules (e.g. `io::duplicate_on_ctrl_d`).
/// Forwards to [`clear_selected`] without exposing the query parameter order.
pub fn clear_selection(commands: &mut Commands, selected_q: &Query<Entity, With<SelectedMarker>>) {
    clear_selected(commands, selected_q);
}
