//! Transform gizmo integration: settings, focus management, manifest
//! writeback and selection/hover overlays.

use bevy::gizmos::transform_gizmo::{
    TransformGizmoFocus, TransformGizmoSettings, TransformGizmoState,
};
use bevy::prelude::*;
use bevymmo_shared::world::TransformData;

use crate::history::EditorHistory;
use crate::state::{EditorProp, EditorState, EditorTerrain, SelectedMarker};

const SELECTED_COLOR: Color = Color::srgb(1.0, 0.72, 0.2);
const HOVERED_COLOR: Color = Color::srgb(0.4, 0.85, 1.0);

/// Pushes the tool's gizmo mode + snap settings into the shared gizmo
/// resource every frame (the gizmo plugin does not read our state).
pub fn sync_gizmo_settings(state: Res<EditorState>, mut settings: ResMut<TransformGizmoSettings>) {
    if let Some(mode) = state.gizmo_mode() {
        settings.mode = mode;
    }
    settings.space = state.gizmo_space;
    settings.snap_translate = Some(state.snap_translation);
    settings.snap_rotate = Some(state.snap_rotation_deg.to_radians());
    settings.snap_scale = Some(state.snap_scale);
    settings.confine_cursor = true;
    settings.screen_scale_factor = 0.1;
}

/// Attaches `TransformGizmoFocus` to the selection only while a manipulation
/// tool is active, and strips it everywhere else. The gizmo plugin always
/// renders on the first focus entity, so "no focus" is how we hide it.
pub fn sync_gizmo_focus(
    state: Res<EditorState>,
    mut commands: Commands,
    selected_q: Query<(Entity, Has<TransformGizmoFocus>), With<SelectedMarker>>,
    stale_focus_q: Query<Entity, (With<TransformGizmoFocus>, Without<SelectedMarker>)>,
) {
    let wants_focus = state.gizmo_mode().is_some();
    for (entity, has_focus) in selected_q.iter() {
        if wants_focus && !has_focus {
            commands.entity(entity).insert(TransformGizmoFocus);
        } else if !wants_focus && has_focus {
            commands.entity(entity).remove::<TransformGizmoFocus>();
        }
    }
    for entity in stale_focus_q.iter() {
        commands.entity(entity).remove::<TransformGizmoFocus>();
    }
}

/// Pushes an undo snapshot when a gizmo drag *starts* (false → true
/// transition on `TransformGizmoState::active`). Running this only on the
/// transition edge keeps a single drag from flooding the history stack.
pub fn record_gizmo_drag_start(
    gizmo_state: Res<TransformGizmoState>,
    mut state: ResMut<EditorState>,
    mut history: ResMut<EditorHistory>,
) {
    let active_now = gizmo_state.active;
    if active_now && !state.gizmo_was_active {
        history.push(&state.manifest);
        state.validation_dirty = true;
    }
    state.gizmo_was_active = active_now;
}

/// Copies the selected entity's transform back into the manifest, so gizmo
/// drags and inspector edits persist. Marks the map dirty only when the
/// authored values actually changed.
pub fn writeback_manifest(
    mut state: ResMut<EditorState>,
    selected_prop_q: Query<
        (&EditorProp, &Transform),
        (Without<EditorTerrain>, With<SelectedMarker>, Changed<Transform>),
    >,
    terrain_q: Query<&Transform, (With<EditorTerrain>, With<SelectedMarker>, Changed<Transform>)>,
) {
    let Some(selected) = state.selected else {
        return;
    };

    if let Ok((prop, transform)) = selected_prop_q.get(selected) {
        let Some(entry) = state
            .manifest
            .props
            .iter_mut()
            .find(|p| p.id == prop.prop_id)
        else {
            return;
        };
        let authored = transform_data_from_bevy(transform);
        if entry.transform != authored {
            entry.transform = authored;
            state.dirty = true;
        }
    }

    if let Ok(transform) = terrain_q.get(selected) {
        let authored = transform_data_from_bevy(transform);
        if state.manifest.terrain.transform != authored {
            state.manifest.terrain.transform = authored;
            state.dirty = true;
        }
    }
}

/// Draws wireframe overlays on the hovered and selected bodies so the current
/// target is unmistakable, even when the gizmo is hidden.
pub fn draw_overlays(
    mut gizmos: Gizmos,
    state: Res<EditorState>,
    transforms: Query<&Transform, Or<(With<EditorProp>, With<EditorTerrain>)>>,
) {
    // Draw overlay for selected entity (O(1) lookup).
    if let Some(entity) = state.selected {
        if let Ok(transform) = transforms.get(entity) {
            let half = if state.terrain_entity == Some(entity) {
                transform.scale.abs() * 0.5
            } else {
                transform.scale.abs()
            };
            draw_box_edges(
                &mut gizmos,
                transform.translation,
                transform.rotation,
                half,
                SELECTED_COLOR,
            );
        }
    }
    // Draw overlay for hovered entity if different from selected (O(1) lookup).
    if let Some(entity) = state.hovered {
        if state.selected != Some(entity) {
            if let Ok(transform) = transforms.get(entity) {
                let half = if state.terrain_entity == Some(entity) {
                    transform.scale.abs() * 0.5
                } else {
                    transform.scale.abs()
                };
                draw_box_edges(
                    &mut gizmos,
                    transform.translation,
                    transform.rotation,
                    half,
                    HOVERED_COLOR,
                );
            }
        }
    }
}

/// Draws the 12 edges of an oriented box.
fn draw_box_edges(gizmos: &mut Gizmos, center: Vec3, rotation: Quat, half: Vec3, color: Color) {
    let corners = [
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
    ]
    .map(|corner| center + rotation * (corner * half));
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for (a, b) in edges {
        gizmos.line(corners[a], corners[b], color);
    }
}

/// Converts a Bevy transform into the manifest's YXZ euler degrees format
/// (`[pitch, yaw, roll]`), mirroring `quat_from_rotation_deg`.
fn transform_data_from_bevy(transform: &Transform) -> TransformData {
    let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
    TransformData {
        translation: transform.translation.to_array(),
        rotation_deg: [pitch.to_degrees(), yaw.to_degrees(), roll.to_degrees()],
        scale: transform.scale.to_array(),
    }
}
