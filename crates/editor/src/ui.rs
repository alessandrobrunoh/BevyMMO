//! egui panels for the editor: toolbar, palette, inspector.

use bevy::prelude::*;
use bevy_egui::egui;

use crate::state::{EditorProp, EditorState, EditorTool, SelectedMarker};

#[derive(Component)]
pub struct NativeEditorHud;

#[derive(Component)]
pub struct NativeEditorStatus;

const PALETTE_KINDS: &[&str] = &["cube", "tree_oak", "rock_01", "house_simple"];

/// Native Bevy fallback HUD. It makes the editor usable even when egui is
/// unavailable or its context is not created for the primary window.
pub fn spawn_native_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                width: Val::Px(330.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.88)),
            NativeEditorHud,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("MAP EDITOR"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.3)),
            ));
            parent.spawn((
                Text::new("B: Place   V: Select   Delete: Erase"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                Text::new("RMB orbit   MMB pan   Wheel zoom"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.75, 0.8)),
            ));
            parent.spawn((
                Text::new("Tool: Select | Kind: cube"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.5, 1.0, 0.6)),
                NativeEditorStatus,
            ));
        });
}

/// Keyboard fallback for the toolbar.
pub fn keyboard_tools(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<EditorState>) {
    if keys.just_pressed(KeyCode::KeyB) {
        state.tool = EditorTool::Place;
    }
    if keys.just_pressed(KeyCode::KeyV) {
        state.tool = EditorTool::Select;
    }
}

pub fn update_native_hud(
    state: Res<EditorState>,
    mut status: Query<&mut Text, With<NativeEditorStatus>>,
) {
    if !state.is_changed() {
        return;
    }
    let Ok(mut text) = status.single_mut() else {
        return;
    };
    let tool = match state.tool {
        EditorTool::Select => "Select",
        EditorTool::Place => "Place",
    };
    text.0 = format!(
        "Tool: {tool} | Kind: {} | Props: {} | Selected: {}{}",
        state.current_kind,
        state.manifest.props.len(),
        state.selected.map(|_| "yes").unwrap_or("no"),
        if state.dirty { " *" } else { "" }
    );
}

pub fn inspector_panel(
    mut ctxs: bevy_egui::EguiContexts,
    mut state: ResMut<EditorState>,
    mut commands: Commands,
    selected_q: Query<(Entity, &EditorProp, &Transform), With<SelectedMarker>>,
) {
    let Ok(ctx) = ctxs.ctx_mut() else {
        return;
    };

    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut state.tool, EditorTool::Select, "Select (V)");
            ui.selectable_value(&mut state.tool, EditorTool::Place, "Place (B)");
            ui.separator();
            ui.label(format!(
                "{} props{}  {}",
                state.manifest.props.len(),
                if state.dirty { " *" } else { "" },
                state
                    .file_path
                    .as_deref()
                    .map(|p| format!("({p})"))
                    .unwrap_or_default()
            ));
        });
    });

    egui::SidePanel::left("palette").show(ctx, |ui| {
        ui.heading("Palette");
        ui.label("Click a kind, then use Place (B).");
        ui.separator();
        for kind in PALETTE_KINDS {
            let selected = state.current_kind == *kind;
            if ui.selectable_label(selected, *kind).clicked() {
                state.current_kind = (*kind).to_string();
                state.tool = EditorTool::Place;
            }
        }
        ui.separator();
        ui.label(format!("Snap: {:.1} m", state.snap_translation));
        ui.label("Ctrl+S to save, Ctrl+O to load");
        ui.label("Delete/Backspace to remove selection");
        ui.label("RMB drag: orbit, MMB drag: pan, scroll: zoom");
    });

    egui::SidePanel::right("inspector").show(ctx, |ui| {
        ui.heading("Inspector");
        ui.separator();
        if let Some((entity, prop, transform)) = selected_q.iter().next() {
            let kind = state
                .manifest
                .props
                .iter()
                .find(|p| p.id == prop.prop_id)
                .map(|p| p.kind.clone())
                .unwrap_or_else(|| "?".to_string());
            ui.label(format!("id: {}", prop.prop_id));
            ui.label(format!("kind: {}", kind));
            ui.separator();
            ui.collapsing("Transform", |ui| {
                let mut t = transform.translation;
                ui.horizontal(|ui| {
                    ui.label("x");
                    ui.add(egui::DragValue::new(&mut t.x).speed(0.5));
                    ui.label("y");
                    ui.add(egui::DragValue::new(&mut t.y).speed(0.5));
                    ui.label("z");
                    ui.add(egui::DragValue::new(&mut t.z).speed(0.5));
                });
                if let Some(prop_entry) =
                    state.manifest.props.iter_mut().find(|p| p.id == prop.prop_id)
                {
                    prop_entry.transform.translation = [t.x, t.y, t.z];
                }
                let mut new_xform = *transform;
                new_xform.translation = t;
                commands.entity(entity).insert(new_xform);
            });
        } else {
            ui.label("Nothing selected.");
            ui.label("Use Place (B) then click the ground.");
        }
        ui.separator();
        ui.collapsing("Map metadata", |ui| {
            ui.horizontal(|ui| {
                ui.label("map_id");
                ui.text_edit_singleline(&mut state.manifest.map_id);
            });
            ui.horizontal(|ui| {
                ui.label("display_name");
                ui.text_edit_singleline(&mut state.manifest.display_name);
            });
        });
        ui.separator();
        ui.collapsing("Manifest", |ui| {
            ui.label(format!("props: {}", state.manifest.props.len()));
            ui.label(format!(
                "bounds: {:.0}..{:.0} x, {:.0}..{:.0} z",
                state.manifest.bounds.min_x,
                state.manifest.bounds.max_x,
                state.manifest.bounds.min_z,
                state.manifest.bounds.max_z
            ));
            for p in &state.manifest.props {
                ui.monospace(format!(
                    "{}  {}  ({:.1}, {:.1}, {:.1})",
                    p.id,
                    p.kind,
                    p.transform.translation[0],
                    p.transform.translation[1],
                    p.transform.translation[2]
                ));
            }
        });
    });
}
