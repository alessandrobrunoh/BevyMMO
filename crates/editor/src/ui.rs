//! egui panels for the editor: tool strip, toolbar, palette, inspector.
//!
//! Uses the non-deprecated `egui::Panel` API (egui 0.34). The 3D viewport
//! stays fully visible behind the panels' edges; input systems are gated on
//! `egui_wants_any_pointer_input` / `egui_wants_any_keyboard_input` so the
//! viewport never receives clicks meant for the UI.

use bevy::gizmos::transform_gizmo::TransformGizmoSpace;
use bevy::prelude::*;
use bevy_egui::egui;
use bevymmo_shared::world::{CollisionShape, TransformData};

use crate::picking::{tint_for_kind, update_prop_material, update_terrain_material};
use crate::state::{
    quat_from_rotation_deg, EditorProp, EditorState, EditorTerrain, EditorTool, SelectedMarker,
};

#[derive(Component)]
pub struct NativeEditorHud;

#[derive(Component)]
pub struct NativeEditorStatus;

/// Native Bevy fallback HUD. It makes the editor usable even when egui is
/// unavailable or its context is not created for the primary window.
pub fn spawn_native_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(8.0),
                left: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.7)),
            NativeEditorHud,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("MAP EDITOR  |  V Select  W Move  E Rotate  R Scale  B Place  X Erase"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.3)),
            ));
            parent.spawn((
                Text::new("WASD pan   F focus   RMB orbit   MMB pan   Wheel zoom   Del erase   Esc deselect"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.75, 0.8)),
            ));
            parent.spawn((
                Text::new("Tool: Select | Kind: cube"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.5, 1.0, 0.6)),
                NativeEditorStatus,
            ));
        });
}

/// Keyboard fallback for the toolbar (also used with egui as the source of
/// truth for hotkeys).
pub fn keyboard_tools(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<EditorState>) {
    let tool = if keys.just_pressed(KeyCode::KeyV) {
        Some(EditorTool::Select)
    } else if keys.just_pressed(KeyCode::KeyW) {
        Some(EditorTool::Move)
    } else if keys.just_pressed(KeyCode::KeyE) {
        Some(EditorTool::Rotate)
    } else if keys.just_pressed(KeyCode::KeyR) {
        Some(EditorTool::Scale)
    } else if keys.just_pressed(KeyCode::KeyB) {
        Some(EditorTool::Place)
    } else if keys.just_pressed(KeyCode::KeyX) {
        Some(EditorTool::Erase)
    } else {
        None
    };
    if let Some(tool) = tool {
        state.tool = tool;
    }
    if keys.just_pressed(KeyCode::Space) {
        state.show_grid = !state.show_grid;
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
    text.0 = format!(
        "Tool: {} | Kind: {} | Props: {} | Selected: {}{}",
        state.tool.label(),
        state.current_kind,
        state.manifest.props.len(),
        state.selected.map(|_| "yes").unwrap_or("no"),
        if state.dirty { " *" } else { "" }
    );
}

/// Main egui entry: lays out the tool strip, toolbar, palette and inspector.
pub fn inspector_panel(
    mut ctxs: bevy_egui::EguiContexts,
    mut state: ResMut<EditorState>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    prop_q: Query<
        (Entity, &EditorProp, &Transform),
        (With<SelectedMarker>, Without<EditorTerrain>),
    >,
    terrain_q: Query<(Entity, &Transform), (With<SelectedMarker>, With<EditorTerrain>)>,
) {
    let Ok(ctx) = ctxs.ctx_mut() else {
        return;
    };
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "editor_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    egui::Panel::top("toolbar")
        .resizable(false)
        .show_inside(&mut viewport_ui, |ui| toolbar_ui(ui, &mut state));

    egui::Panel::left("tools")
        .resizable(false)
        .exact_size(52.0)
        .show_inside(&mut viewport_ui, |ui| tools_strip(ui, &mut state));

    egui::Panel::left("palette")
        .resizable(true)
        .default_size(170.0)
        .min_size(130.0)
        .show_inside(&mut viewport_ui, |ui| palette_ui(ui, &mut state));

    egui::Panel::right("inspector")
        .resizable(true)
        .default_size(280.0)
        .min_size(230.0)
        .show_inside(&mut viewport_ui, |ui| {
            inspector_ui(
                ui,
                &mut state,
                &mut commands,
                &mut materials,
                &prop_q,
                &terrain_q,
            )
        });
}

/// Top bar: map info, snap settings, space toggle, grid toggle.
fn toolbar_ui(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal_wrapped(|ui| {
        ui.set_min_height(26.0);
        ui.label(
            egui::RichText::new(format!(
                "{}  —  {} props{}",
                state.manifest.display_name,
                state.manifest.props.len(),
                if state.dirty { " *" } else { "" }
            ))
            .strong(),
        );
        if let Some(path) = &state.file_path {
            ui.monospace(path);
        }
        ui.separator();
        ui.label("Snap");
        ui.add(
            egui::DragValue::new(&mut state.snap_translation)
                .speed(0.1)
                .range(0.1..=50.0),
        )
        .on_hover_text("Translation snap (m)");
        ui.add(
            egui::DragValue::new(&mut state.snap_rotation_deg)
                .speed(1.0)
                .range(1.0..=90.0),
        )
        .on_hover_text("Rotation snap (deg)");
        ui.add(
            egui::DragValue::new(&mut state.snap_scale)
                .speed(0.05)
                .range(0.01..=10.0),
        )
        .on_hover_text("Scale snap");
        ui.separator();
        let local = state.gizmo_space == TransformGizmoSpace::Local;
        if ui.selectable_label(local, "Local").clicked() {
            state.gizmo_space = TransformGizmoSpace::Local;
        }
        if ui.selectable_label(!local, "World").clicked() {
            state.gizmo_space = TransformGizmoSpace::World;
        }
        ui.separator();
        ui.checkbox(&mut state.show_grid, "Grid");
        ui.separator();
        ui.label("Ctrl+S save  ·  Ctrl+O load");
    });
}

/// Slim Photoshop-style tool strip with hotkeys in tooltips.
fn tools_strip(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.set_width(48.0);
    for tool in EditorTool::ALL {
        let selected = state.tool == tool;
        let button = egui::Button::new(egui::RichText::new(tool.label()).size(11.0).strong())
            .min_size(egui::vec2(46.0, 42.0))
            .selected(selected)
            .fill(if selected {
                ui.visuals().selection.bg_fill
            } else {
                egui::Color32::TRANSPARENT
            });
        if ui
            .add(button)
            .on_hover_text(format!("{} ({})", tool.label(), tool.hotkey()))
            .clicked()
        {
            state.tool = tool;
        }
    }
}

/// Left palette: pick a kind, then click the ground to place it.
fn palette_ui(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.heading("Palette");
    ui.label("Pick a kind, then Place (B).");
    ui.separator();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for kind in crate::picking::PALETTE_KINDS {
                let selected = state.current_kind == *kind;
                let swatch = swatch_color(kind);
                let inner = ui
                    .horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 3.0, swatch);
                        ui.selectable_label(selected, *kind)
                    })
                    .inner;
                if inner.clicked() {
                    state.current_kind = (*kind).to_string();
                    state.tool = EditorTool::Place;
                }
            }
        });
    ui.separator();
    ui.label(format!("Terrain selectable: click the ground"));
}

/// Right inspector: transform, tint, collision and metadata for the selection.
#[allow(clippy::too_many_arguments)]
fn inspector_ui(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    prop_q: &Query<
        (Entity, &EditorProp, &Transform),
        (With<SelectedMarker>, Without<EditorTerrain>),
    >,
    terrain_q: &Query<(Entity, &Transform), (With<SelectedMarker>, With<EditorTerrain>)>,
) {
    ui.heading("Inspector");
    ui.separator();

    if let Some((entity, prop, _transform)) = prop_q.iter().next() {
        let Some(entry_index) = state.find_prop_index(&prop.prop_id) else {
            return;
        };
        let kind = state.manifest.props[entry_index].kind.clone();
        prop_header(
            ui,
            &prop.prop_id,
            &kind,
            &state.manifest.props[entry_index].tint,
        );
        transform_editor(
            ui,
            commands,
            entity,
            &mut state.manifest.props[entry_index].transform,
            &mut state.dirty,
        );
        tint_editor(
            ui,
            commands,
            materials,
            entity,
            &mut state.manifest.props[entry_index].tint,
            &kind,
            &mut state.dirty,
        );
        collision_editor(ui, entry_index, state);
        ui.separator();
        if ui.button("Delete prop").clicked() {
            let id = state.manifest.props[entry_index].id.clone();
            state.manifest.props.retain(|p| p.id != id);
            state.dirty = true;
            commands.entity(entity).despawn();
            state.selected = None;
            state.hovered = None;
        }
    } else if let Some((entity, _transform)) = terrain_q.iter().next() {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Terrain").strong());
            ui.label("(the ground)");
        });
        ui.label("Selectable and transformable like any prop.");
        ui.separator();
        transform_editor(
            ui,
            commands,
            entity,
            &mut state.manifest.terrain.transform,
            &mut state.dirty,
        );
        tint_editor(
            ui,
            commands,
            materials,
            entity,
            &mut state.manifest.terrain.tint,
            "terrain",
            &mut state.dirty,
        );
    } else {
        ui.label("Nothing selected.");
        ui.label("Click a prop or the terrain to select it.");
    }

    ui.separator();
    ui.collapsing("Map metadata", |ui| {
        ui.horizontal(|ui| {
            ui.label("map_id");
            ui.text_edit_singleline(&mut state.manifest.map_id);
        });
        ui.horizontal(|ui| {
            ui.label("name");
            ui.text_edit_singleline(&mut state.manifest.display_name);
        });
        ui.horizontal(|ui| {
            ui.label("bounds");
            ui.add(egui::DragValue::new(&mut state.manifest.bounds.min_x).speed(1.0));
            ui.label("..");
            ui.add(egui::DragValue::new(&mut state.manifest.bounds.max_x).speed(1.0));
            ui.label("x");
            ui.add(egui::DragValue::new(&mut state.manifest.bounds.min_z).speed(1.0));
            ui.label("..");
            ui.add(egui::DragValue::new(&mut state.manifest.bounds.max_z).speed(1.0));
            ui.label("z");
        });
    });

    ui.collapsing("Manifest", |ui| {
        ui.label(format!("props: {}", state.manifest.props.len()));
        ui.label(format!(
            "terrain: size {:.0}x{:.0} at ({:.0}, {:.0})",
            state.manifest.terrain.transform.scale[0],
            state.manifest.terrain.transform.scale[2],
            state.manifest.terrain.transform.translation[0],
            state.manifest.terrain.transform.translation[2],
        ));
        for prop in &state.manifest.props {
            ui.monospace(format!(
                "{}  {}  ({:.1}, {:.1}, {:.1})",
                prop.id,
                prop.kind,
                prop.transform.translation[0],
                prop.transform.translation[1],
                prop.transform.translation[2]
            ));
        }
    });

    ui.separator();
    ui.label(egui::RichText::new("Shortcuts").strong());
    ui.label("W/E/R gizmo · V select · B place · X erase");
    ui.label("WASD/F camera · RMB orbit · MMB pan · wheel zoom");
    ui.label("Delete erase · Esc deselect · Ctrl+S/O save/load");
}

fn prop_header(ui: &mut egui::Ui, id: &str, kind: &str, tint: &Option<[f32; 3]>) {
    let swatch = tint
        .map(|rgb| {
            egui::Color32::from_rgb(
                (rgb[0] * 255.0) as u8,
                (rgb[1] * 255.0) as u8,
                (rgb[2] * 255.0) as u8,
            )
        })
        .unwrap_or(egui::Color32::GRAY);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 3.0, swatch);
        ui.label(egui::RichText::new(kind).strong());
    });
    ui.label(egui::RichText::new(id).monospace().weak());
}

/// Shared position/rotation/scale editor. Writes both the manifest and the
/// entity, so gizmo edits (entity -> manifest) and UI edits stay in sync.
fn transform_editor(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    entity: Entity,
    data: &mut TransformData,
    dirty: &mut bool,
) {
    let before = *data;
    ui.collapsing("Transform", |ui| {
        ui.horizontal(|ui| {
            ui.label("Pos");
            ui.add(egui::DragValue::new(&mut data.translation[0]).speed(0.25));
            ui.add(egui::DragValue::new(&mut data.translation[1]).speed(0.25));
            ui.add(egui::DragValue::new(&mut data.translation[2]).speed(0.25));
        });
        ui.horizontal(|ui| {
            ui.label("Rot");
            ui.add(
                egui::DragValue::new(&mut data.rotation_deg[0])
                    .speed(1.0)
                    .suffix("°"),
            );
            ui.add(
                egui::DragValue::new(&mut data.rotation_deg[1])
                    .speed(1.0)
                    .suffix("°"),
            );
            ui.add(
                egui::DragValue::new(&mut data.rotation_deg[2])
                    .speed(1.0)
                    .suffix("°"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Scl");
            ui.add(egui::DragValue::new(&mut data.scale[0]).speed(0.1));
            ui.add(egui::DragValue::new(&mut data.scale[1]).speed(0.1));
            ui.add(egui::DragValue::new(&mut data.scale[2]).speed(0.1));
        });
        ui.label("Or drag with the gizmo (W/E/R).");
    });
    if *data != before {
        *dirty = true;
        commands.entity(entity).insert(Transform {
            translation: Vec3::from_array(data.translation),
            rotation: quat_from_rotation_deg(data.rotation_deg),
            scale: Vec3::from_array(data.scale),
        });
    }
}

/// Color picker for a prop's tint. Applies to the manifest and the material.
fn tint_editor(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    entity: Entity,
    tint: &mut Option<[f32; 3]>,
    kind: &str,
    dirty: &mut bool,
) {
    let before = *tint;
    ui.collapsing("Color", |ui| {
        let mut color32 = tint
            .map(|rgb| {
                egui::Color32::from_rgb(
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8,
                )
            })
            .unwrap_or(egui::Color32::GRAY);
        let changed = ui.color_edit_button_srgba(&mut color32).changed();
        let reset = ui.button("Reset tint").clicked();
        if changed {
            *tint = Some([
                color32.r() as f32 / 255.0,
                color32.g() as f32 / 255.0,
                color32.b() as f32 / 255.0,
            ]);
        }
        if reset {
            *tint = None;
        }
        if *tint != before {
            *dirty = true;
            if kind == "terrain" {
                update_terrain_material(commands, materials, entity, *tint);
            } else {
                update_prop_material(commands, materials, entity, kind, *tint);
            }
        }
    });
}

/// Collision shape + movement-blocking editor for a prop.
fn collision_editor(ui: &mut egui::Ui, entry_index: usize, state: &mut EditorState) {
    let prop = &mut state.manifest.props[entry_index];
    let before_collision = prop.collision;
    let before_blocks = prop.blocks_movement;

    ui.collapsing("Collision", |ui| {
        ui.checkbox(&mut prop.blocks_movement, "Blocks movement");
        let current = collision_label(prop.collision);
        egui::ComboBox::from_label("Shape")
            .selected_text(current)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut prop.collision, None, "None");
                ui.selectable_value(
                    &mut prop.collision,
                    Some(CollisionShape::Box {
                        half_extents: [1.0, 1.0, 1.0],
                    }),
                    "Box",
                );
                ui.selectable_value(
                    &mut prop.collision,
                    Some(CollisionShape::Cylinder {
                        radius: 0.5,
                        height: 2.0,
                    }),
                    "Cylinder",
                );
                ui.selectable_value(
                    &mut prop.collision,
                    Some(CollisionShape::Sphere { radius: 0.5 }),
                    "Sphere",
                );
            });
        match &mut prop.collision {
            Some(CollisionShape::Box { half_extents }) => {
                ui.horizontal(|ui| {
                    ui.label("half ext");
                    ui.add(egui::DragValue::new(&mut half_extents[0]).speed(0.1));
                    ui.add(egui::DragValue::new(&mut half_extents[1]).speed(0.1));
                    ui.add(egui::DragValue::new(&mut half_extents[2]).speed(0.1));
                });
            }
            Some(CollisionShape::Cylinder { radius, height }) => {
                ui.horizontal(|ui| {
                    ui.label("radius");
                    ui.add(egui::DragValue::new(radius).speed(0.1));
                    ui.label("height");
                    ui.add(egui::DragValue::new(height).speed(0.1));
                });
            }
            Some(CollisionShape::Sphere { radius }) => {
                ui.horizontal(|ui| {
                    ui.label("radius");
                    ui.add(egui::DragValue::new(radius).speed(0.1));
                });
            }
            None => {}
        }
    });

    if prop.collision != before_collision || prop.blocks_movement != before_blocks {
        state.dirty = true;
    }
}

fn collision_label(collision: Option<CollisionShape>) -> &'static str {
    match collision {
        None => "None",
        Some(CollisionShape::Box { .. }) => "Box",
        Some(CollisionShape::Cylinder { .. }) => "Cylinder",
        Some(CollisionShape::Sphere { .. }) => "Sphere",
    }
}

/// Palette swatch color: matches the tint a placed prop of this kind gets.
fn swatch_color(kind: &str) -> egui::Color32 {
    tint_for_kind(kind)
        .map(|rgb| {
            egui::Color32::from_rgb(
                (rgb[0] * 255.0) as u8,
                (rgb[1] * 255.0) as u8,
                (rgb[2] * 255.0) as u8,
            )
        })
        .unwrap_or(egui::Color32::from_rgb(102, 128, 179))
}
