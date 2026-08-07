//! Editor chrome rendered with `bevy_egui`.
//!
//! Layout follows a small game-engine convention:
//!
//! ```mermaid
//! flowchart TB
//!     MENU[Menu bar]
//!     TOOLBAR[Tool toolbar]
//!     LEFT[Left panel: Outliner / Palette / Snap / Map]
//!     RIGHT[Right panel: Inspector]
//!     STATUS[Status bar]
//!     MENU --> TOOLBAR
//!     TOOLBAR --> LEFT
//!     LEFT --> STATUS
//! ```
//!
//! The center of the screen is intentionally *not* covered by any egui panel
//! so the Bevy 3D viewport shows through. egui's pointer-input capture keeps
//! clicks over panels from leaking into the viewport.

use bevy::prelude::*;
use bevy_egui::egui;
use bevymmo_shared::world::{CollisionShape, TransformData};

use crate::history::EditorHistory;
use crate::io;
use crate::picking::PALETTE_KINDS;
use crate::state::{
    quat_from_rotation_deg, EditorProp, EditorState, EditorTerrain, EditorTool, LeftPanelTab,
    SelectedMarker,
};
use crate::theme;

const LEFT_PANEL_WIDTH: f32 = 280.0;
const RIGHT_PANEL_WIDTH: f32 = 340.0;
const MENU_BAR_HEIGHT: f32 = 22.0;
const TOOLBAR_HEIGHT: f32 = 64.0;
const STATUS_BAR_HEIGHT: f32 = 24.0;

/// No-op: the editor chrome is fully egui-based now. Kept as a startup hook
/// for symmetry with other editor modules.
pub fn spawn_native_hud() {}

/// Keyboard tool switching (`V`/`W`/`E`/`R`/`B`/`X`) and `G` grid toggle.
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
    if keys.just_pressed(KeyCode::KeyG) {
        state.show_grid = !state.show_grid;
    }
}

/// Main editor UI pass: applies the theme and lays out every panel.
///
/// egui 0.34 deprecated the top-level `Panel::show(ctx, ...)` constructor in
/// favour of `show_inside(ui, ...)`, but `show_inside` needs a root `Ui` that
/// bevy_egui does not expose for top-level panels. We allow the deprecated
/// call until bevy_egui ships a migration helper.
#[allow(deprecated, clippy::too_many_arguments)]
pub fn inspector_panel(
    mut ctxs: bevy_egui::EguiContexts,
    mut state: ResMut<EditorState>,
    mut history: ResMut<EditorHistory>,
    mut commands: Commands,
    selected_q: Query<Entity, With<SelectedMarker>>,
    prop_q: Query<(Entity, &EditorProp, &Transform), Without<EditorTerrain>>,
    terrain_q: Query<(Entity, &Transform), With<EditorTerrain>>,
) {
    let Ok(ctx) = ctxs.ctx_mut() else {
        return;
    };
    theme::apply(ctx);
    let palette = theme::EditorPalette::dark();

    egui::Panel::top("menu_bar")
        .exact_size(MENU_BAR_HEIGHT)
        .frame(theme::bar_frame(&palette))
        .show(ctx, |ui| {
            menu_bar_ui(ui, &mut state, &mut history);
        });

    egui::Panel::top("toolbar")
        .exact_size(TOOLBAR_HEIGHT)
        .frame(theme::bar_frame(&palette))
        .show(ctx, |ui| {
            toolbar_ui(ui, &mut state, &palette);
        });

    egui::Panel::bottom("status_bar")
        .exact_size(STATUS_BAR_HEIGHT)
        .frame(theme::bar_frame(&palette))
        .show(ctx, |ui| {
            status_bar_ui(ui, &state, &history, &palette);
        });

    egui::Panel::left("left_panel")
        .resizable(false)
        .exact_size(LEFT_PANEL_WIDTH)
        .frame(theme::panel_frame(&palette))
        .show(ctx, |ui| {
            left_panel_ui(
                ui,
                &mut commands,
                &mut state,
                &selected_q,
                &prop_q,
                &palette,
            );
        });

    egui::Panel::right("right_panel")
        .resizable(false)
        .exact_size(RIGHT_PANEL_WIDTH)
        .frame(theme::panel_frame(&palette))
        .show(ctx, |ui| {
            right_panel_ui(
                ui,
                &mut commands,
                &mut state,
                &selected_q,
                &prop_q,
                &terrain_q,
                &palette,
            );
        });

    // Intentionally no CentralPanel: that area is the Bevy 3D viewport.
}

// --- Menu bar -------------------------------------------------------------

fn menu_bar_ui(ui: &mut egui::Ui, state: &mut EditorState, history: &mut EditorHistory) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("New map  Ctrl+N").clicked() {
                io::new_map(state, history);
                ui.close();
            }
            if ui.button("Open map…  Ctrl+O").clicked() {
                let _ = io::load(state, history);
                ui.close();
            }
            if ui.button("Save map  Ctrl+S").clicked() {
                let _ = io::save(state);
                ui.close();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                std::process::exit(0);
            }
        });

        ui.menu_button("Edit", |ui| {
            if ui
                .add_enabled(history.can_undo(), egui::Button::new("Undo  Ctrl+Z"))
                .clicked()
            {
                io::undo(state, history);
                ui.close();
            }
            if ui
                .add_enabled(history.can_redo(), egui::Button::new("Redo  Ctrl+Y"))
                .clicked()
            {
                io::redo(state, history);
                ui.close();
            }
            ui.separator();
            if ui.button("Duplicate  Ctrl+D").clicked() {
                state.pending_duplicate = true;
                ui.close();
            }
            if ui.button("Focus selection  F").clicked() {
                state.pending_focus_selection = true;
                ui.close();
            }
            if ui.button("Deselect  Esc").clicked() {
                state.pending_focus_selection = false;
                ui.close();
            }
        });

        ui.menu_button("View", |ui| {
            ui.checkbox(&mut state.show_grid, "Show grid  (G)");
            ui.checkbox(&mut state.confirm_delete, "Confirm delete");
            ui.separator();
            ui.label("Camera");
            ui.horizontal(|ui| {
                ui.label("Distance");
                ui.add(egui::DragValue::new(&mut state.camera_distance).range(5.0..=150.0));
            });
            ui.horizontal(|ui| {
                ui.label("Yaw");
                ui.add(egui::DragValue::new(&mut state.camera_yaw).speed(0.05));
            });
            ui.horizontal(|ui| {
                ui.label("Pitch");
                ui.add(
                    egui::DragValue::new(&mut state.camera_pitch)
                        .range(0.2..=1.3)
                        .speed(0.05),
                );
            });
        });

        ui.menu_button("Help", |ui| {
            ui.label("Tools");
            ui.label(theme::caption("V Select   W Move   E Rotate   R Scale   B Place   X Erase", &theme::EditorPalette::dark()));
            ui.separator();
            ui.label("Camera");
            ui.label(theme::caption("Space+WASD pan   Shift fast   RMB orbit   MMB pan   Wheel zoom   F frame", &theme::EditorPalette::dark()));
            ui.separator();
            ui.label("Files");
            ui.label(theme::caption("Ctrl+N new   Ctrl+O open   Ctrl+S save   Ctrl+Z undo   Ctrl+Y redo   Ctrl+D duplicate", &theme::EditorPalette::dark()));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(theme::caption(
                "BevyMMO Editor",
                &theme::EditorPalette::dark(),
            ));
        });
    });
}

// --- Toolbar --------------------------------------------------------------

fn toolbar_ui(ui: &mut egui::Ui, state: &mut EditorState, palette: &theme::EditorPalette) {
    ui.horizontal(|ui| {
        for tool in EditorTool::ALL {
            tool_button(ui, state, tool, palette);
        }
        ui.separator();
        let mut show_grid = state.show_grid;
        let toggled = ui.checkbox(&mut show_grid, "Grid").changed();
        if toggled {
            state.show_grid = show_grid;
        }
        if ui.button("Frame  (F)").clicked() {
            state.pending_focus_selection = true;
        }
    });
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(theme::heading("Tool:", palette));
        ui.label(state.tool.label());
        ui.separator();
        ui.label(theme::heading("Brush:", palette));
        ui.label(&state.current_kind);
        ui.separator();
        let dirty_marker = if state.dirty {
            "● UNSAVED"
        } else {
            "○ saved"
        };
        let dirty_color = if state.dirty {
            palette.warning
        } else {
            palette.success
        };
        ui.label(
            egui::RichText::new(dirty_marker)
                .color(dirty_color)
                .strong(),
        );
    });
}

fn tool_button(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    tool: EditorTool,
    palette: &theme::EditorPalette,
) {
    let is_active = state.tool == tool;
    let icon = tool_icon(tool);
    let label = format!("{icon}  {}", tool.label());
    let response = if is_active {
        ui.add(egui::Button::selectable(
            true,
            egui::RichText::new(label).strong(),
        ))
    } else {
        ui.add(egui::Button::selectable(false, label))
    };
    if response.clicked() {
        state.tool = tool;
    }
    response.on_hover_text(format!("{} [{}]", tool.label(), tool.hotkey()));
    let _ = palette;
}

fn tool_icon(tool: EditorTool) -> &'static str {
    match tool {
        EditorTool::Select => "▣",
        EditorTool::Move => "✛",
        EditorTool::Rotate => "⟳",
        EditorTool::Scale => "⤢",
        EditorTool::Place => "✚",
        EditorTool::Erase => "✕",
    }
}

// --- Left panel -----------------------------------------------------------

fn left_panel_ui(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    state: &mut EditorState,
    selected_q: &Query<Entity, With<SelectedMarker>>,
    prop_q: &Query<(Entity, &EditorProp, &Transform), Without<EditorTerrain>>,
    palette: &theme::EditorPalette,
) {
    ui.vertical(|ui| {
        tab_bar(ui, state, palette);
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("left_panel_scroll")
            .show(ui, |ui| match state.left_tab {
                LeftPanelTab::Outliner => {
                    outliner_ui(ui, commands, state, selected_q, prop_q, palette)
                }
                LeftPanelTab::Palette => palette_ui(ui, state, palette),
                LeftPanelTab::Snap => snap_ui(ui, state, palette),
                LeftPanelTab::MapSettings => map_settings_ui(ui, state, palette),
            });
    });
}

fn tab_bar(ui: &mut egui::Ui, state: &mut EditorState, palette: &theme::EditorPalette) {
    ui.horizontal(|ui| {
        for (tab, label) in [
            (LeftPanelTab::Outliner, "Outliner"),
            (LeftPanelTab::Palette, "Palette"),
            (LeftPanelTab::Snap, "Snap"),
            (LeftPanelTab::MapSettings, "Map"),
        ] {
            let is_active = state.left_tab == tab;
            let text = if is_active {
                egui::RichText::new(label).color(palette.accent).strong()
            } else {
                egui::RichText::new(label).color(palette.text_dim)
            };
            if ui.selectable_label(is_active, text).clicked() {
                state.left_tab = tab;
            }
            ui.separator();
        }
    });
}

fn outliner_ui(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    state: &mut EditorState,
    selected_q: &Query<Entity, With<SelectedMarker>>,
    prop_q: &Query<(Entity, &EditorProp, &Transform), Without<EditorTerrain>>,
    palette: &theme::EditorPalette,
) {
    ui.label(theme::heading("Hierarchy", palette));
    ui.add_space(2.0);

    let terrain_selected = state.selected == state.terrain_entity;
    if ui
        .selectable_label(terrain_selected, "🌐  Terrain")
        .clicked()
    {
        if let Some(entity) = state.terrain_entity {
            set_selected_entity(commands, selected_q, state, entity);
        }
    }

    ui.add_space(4.0);
    ui.label(theme::caption(
        &format!("Props ({})", state.manifest.props.len()),
        palette,
    ));
    ui.separator();

    if state.manifest.props.is_empty() {
        ui.label(theme::caption("No props placed yet.", palette));
        return;
    }

    use std::collections::HashMap;

    let prop_entity_map: HashMap<&str, Entity> = prop_q
        .iter()
        .map(|(entity, editor_prop, _)| (editor_prop.prop_id.as_str(), entity))
        .collect();

    let scene_entries: Vec<(String, String, Option<Entity>, bool)> = state
        .manifest
        .props
        .iter()
        .map(|prop| {
            let prop_entity = prop_entity_map.get(prop.id.as_str()).copied();
            (
                prop.id.clone(),
                prop.kind.to_string(),
                prop_entity,
                prop_entity == state.selected,
            )
        })
        .collect();

    for (prop_id, kind, prop_entity, is_selected) in scene_entries {
        let icon = kind_icon(&kind);
        let label = format!("{icon}  {prop_id}  ({kind})");
        let response = ui.selectable_label(is_selected, label);
        if response.clicked() {
            if let Some(entity) = prop_entity {
                set_selected_entity(commands, selected_q, state, entity);
            }
        }
    }
}

fn palette_ui(ui: &mut egui::Ui, state: &mut EditorState, palette: &theme::EditorPalette) {
    ui.label(theme::heading("Palette", palette));
    ui.label(theme::caption("Click to set the active brush.", palette));
    ui.add_space(4.0);

    for category in PALETTE_CATEGORIES {
        ui.collapsing(category.label, |ui| {
            for kind in PALETTE_KINDS {
                if palette_category_of(kind) != category.ident {
                    continue;
                }
                let is_selected = state.current_kind == *kind;
                let icon = kind_icon(kind);
                let response = ui.selectable_label(is_selected, format!("{icon}  {kind}"));
                if response.clicked() {
                    state.current_kind = (*kind).to_string();
                    state.tool = EditorTool::Place;
                }
            }
        });
    }
}

fn snap_ui(ui: &mut egui::Ui, state: &mut EditorState, palette: &theme::EditorPalette) {
    use bevy::gizmos::transform_gizmo::TransformGizmoSpace;

    ui.label(theme::heading("Snap Settings", palette));
    ui.add_space(4.0);

    if ui
        .add(
            egui::DragValue::new(&mut state.snap_translation)
                .range(0.05..=10.0)
                .speed(0.05)
                .prefix("Translation step: "),
        )
        .changed()
    {
        state.validation_dirty = true;
    }
    if ui
        .add(
            egui::DragValue::new(&mut state.snap_rotation_deg)
                .range(1.0..=90.0)
                .speed(0.5)
                .prefix("Rotation step (deg): "),
        )
        .changed()
    {
        state.validation_dirty = true;
    }
    if ui
        .add(
            egui::DragValue::new(&mut state.snap_scale)
                .range(0.01..=2.0)
                .speed(0.01)
                .prefix("Scale step: "),
        )
        .changed()
    {
        state.validation_dirty = true;
    }

    ui.add_space(6.0);
    ui.label(theme::heading("Gizmo", palette));
    ui.horizontal(|ui| {
        ui.radio_value(&mut state.gizmo_space, TransformGizmoSpace::World, "World");
        ui.radio_value(&mut state.gizmo_space, TransformGizmoSpace::Local, "Local");
    });

    ui.add_space(6.0);
    ui.checkbox(&mut state.show_grid, "Show grid overlay");
    ui.checkbox(&mut state.confirm_delete, "Confirm before deleting");
}

fn map_settings_ui(ui: &mut egui::Ui, state: &mut EditorState, palette: &theme::EditorPalette) {
    ui.label(theme::heading("Map Metadata", palette));
    ui.add_space(4.0);

    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Map ID");
        changed |= ui
            .add(egui::TextEdit::singleline(&mut state.manifest.map_id).desired_width(160.0))
            .changed();
    });
    ui.horizontal(|ui| {
        ui.label("Name");
        changed |= ui
            .add(egui::TextEdit::singleline(&mut state.manifest.display_name).desired_width(160.0))
            .changed();
    });

    ui.add_space(4.0);
    ui.label(theme::heading("Bounds", palette));
    ui.horizontal(|ui| {
        ui.label("min X");
        ui.add(egui::DragValue::new(&mut state.manifest.bounds.min_x).speed(0.5));
        ui.label("max X");
        ui.add(egui::DragValue::new(&mut state.manifest.bounds.max_x).speed(0.5));
    });
    ui.horizontal(|ui| {
        ui.label("min Z");
        ui.add(egui::DragValue::new(&mut state.manifest.bounds.min_z).speed(0.5));
        ui.label("max Z");
        ui.add(egui::DragValue::new(&mut state.manifest.bounds.max_z).speed(0.5));
    });

    ui.add_space(6.0);
    ui.label(theme::heading("File", palette));
    ui.label(theme::caption(
        state.file_path.as_deref().unwrap_or("(unsaved — Ctrl+S)"),
        palette,
    ));
    if changed {
        state.dirty = true;
        state.validation_dirty = true;
    }
}

// --- Right panel (inspector) ---------------------------------------------

#[allow(clippy::too_many_arguments)]
fn right_panel_ui(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    state: &mut EditorState,
    selected_q: &Query<Entity, With<SelectedMarker>>,
    prop_q: &Query<(Entity, &EditorProp, &Transform), Without<EditorTerrain>>,
    terrain_q: &Query<(Entity, &Transform), With<EditorTerrain>>,
    palette: &theme::EditorPalette,
) {
    ui.vertical(|ui| {
        ui.label(theme::heading("Inspector", palette));
        ui.separator();

        let Some(selected) = state.selected else {
            ui.label(theme::caption("Nothing selected.", palette));
            ui.label(theme::caption(
                "Click a prop in the viewport or outliner.",
                palette,
            ));
            return;
        };

        if state.terrain_entity == Some(selected) {
            terrain_inspector(ui, commands, state, terrain_q, palette);
            return;
        }

        let Ok((entity, editor_prop, _transform)) = prop_q.get(selected) else {
            ui.label(theme::caption("Selected entity no longer exists.", palette));
            return;
        };
        let Some(prop_index) = state.find_prop_index(&editor_prop.prop_id) else {
            ui.label(theme::caption(
                "Selected prop missing from manifest.",
                palette,
            ));
            return;
        };

        prop_inspector(ui, commands, state, selected_q, entity, prop_index, palette);
    });
}

fn terrain_inspector(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    state: &mut EditorState,
    terrain_q: &Query<(Entity, &Transform), With<EditorTerrain>>,
    palette: &theme::EditorPalette,
) {
    let Some(selected) = state.selected else {
        return;
    };
    ui.label(
        egui::RichText::new("🌐 Terrain")
            .strong()
            .color(palette.accent_soft),
    );
    ui.add_space(4.0);

    let Ok((_entity, _transform)) = terrain_q.get(selected) else {
        ui.label(theme::caption("Terrain entity missing.", palette));
        return;
    };

    let mut data = state.manifest.terrain.transform;
    let translation_step = state.snap_translation;
    let rotation_step = state.snap_rotation_deg;
    let scale_step = state.snap_scale;
    let transform_changed = transform_editor(
        ui,
        &mut data,
        translation_step,
        rotation_step,
        scale_step,
        palette,
    );
    if transform_changed {
        state.manifest.terrain.transform = data;
        sync_entity_transform(commands, selected, data);
        state.dirty = true;
        state.validation_dirty = true;
    }

    ui.add_space(4.0);
    ui.collapsing("Tint", |ui| {
        tint_editor(ui, &mut state.manifest.terrain.tint);
        if state.manifest.terrain.tint.is_some() {
            state.dirty = true;
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn prop_inspector(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    state: &mut EditorState,
    selected_q: &Query<Entity, With<SelectedMarker>>,
    entity: Entity,
    prop_index: usize,
    palette: &theme::EditorPalette,
) {
    let mut delete_clicked = false;
    let mut dirty = false;
    let translation_step = state.snap_translation;
    let rotation_step = state.snap_rotation_deg;
    let scale_step = state.snap_scale;

    {
        let prop = &mut state.manifest.props[prop_index];
        ui.label(egui::RichText::new(&prop.id).strong().color(palette.accent));
        ui.label(theme::caption(&format!("Kind: {}", prop.kind), palette));
        ui.separator();

        let mut transform_data = prop.transform;
        if transform_editor(
            ui,
            &mut transform_data,
            translation_step,
            rotation_step,
            scale_step,
            palette,
        ) {
            prop.transform = transform_data;
            sync_entity_transform(commands, entity, transform_data);
            dirty = true;
        }

        ui.add_space(2.0);
        ui.collapsing("Tint", |ui| {
            let mut tint = prop.tint;
            tint_editor(ui, &mut tint);
            if tint != prop.tint {
                prop.tint = tint;
                dirty = true;
            }
        });

        ui.collapsing("Gameplay", |ui| {
            dirty |= ui
                .checkbox(&mut prop.blocks_movement, "Blocks movement")
                .changed();

            let mut collision = prop.collision;
            egui::ComboBox::from_label("Collision")
                .selected_text(collision_shape_label(collision))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut collision, None, "None");
                    ui.selectable_value(
                        &mut collision,
                        Some(CollisionShape::Box {
                            half_extents: [1.0, 1.0, 1.0],
                        }),
                        "Box",
                    );
                    ui.selectable_value(
                        &mut collision,
                        Some(CollisionShape::Cylinder {
                            radius: 0.5,
                            height: 2.0,
                        }),
                        "Cylinder",
                    );
                    ui.selectable_value(
                        &mut collision,
                        Some(CollisionShape::Sphere { radius: 0.5 }),
                        "Sphere",
                    );
                });
            if collision != prop.collision {
                prop.collision = collision;
                dirty = true;
            }

            match &mut prop.collision {
                Some(CollisionShape::Box { half_extents }) => {
                    dirty |= vector3_editor(
                        ui,
                        "Half extents",
                        half_extents,
                        0.1,
                        0.01..=100.0,
                        palette,
                    );
                }
                Some(CollisionShape::Cylinder { radius, height }) => {
                    ui.horizontal(|ui| {
                        ui.label("Radius");
                        dirty |= ui
                            .add(egui::DragValue::new(radius).speed(0.1).range(0.01..=100.0))
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height");
                        dirty |= ui
                            .add(egui::DragValue::new(height).speed(0.1).range(0.01..=100.0))
                            .changed();
                    });
                }
                Some(CollisionShape::Sphere { radius }) => {
                    ui.horizontal(|ui| {
                        ui.label("Radius");
                        dirty |= ui
                            .add(egui::DragValue::new(radius).speed(0.1).range(0.01..=100.0))
                            .changed();
                    });
                }
                None => {}
            }
        });
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("Duplicate (Ctrl+D)").clicked() {
            state.pending_duplicate = true;
        }
        if ui.button("Frame (F)").clicked() {
            state.pending_focus_selection = true;
        }
        if ui.button("Delete").clicked() {
            if state.confirm_delete {
                state.pending_delete_dialog = true;
            } else {
                delete_clicked = true;
            }
        }
    });

    if state.pending_delete_dialog {
        let mut accepted = false;
        let mut cancelled = false;
        egui::Window::new("Confirm delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.label(format!(
                    "Delete prop '{}'? This can be undone with Ctrl+Z.",
                    state.manifest.props[prop_index].id
                ));
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        accepted = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancelled = true;
                    }
                });
            });
        if accepted {
            state.pending_delete_dialog = false;
            delete_clicked = true;
        } else if cancelled {
            state.pending_delete_dialog = false;
        }
    }

    if delete_clicked {
        state.manifest.props.remove(prop_index);
        state.selected = None;
        state.hovered = None;
        commands.entity(entity).despawn();
        clear_selected(commands, selected_q);
        state.dirty = true;
        state.validation_dirty = true;
    } else if dirty {
        state.dirty = true;
        state.validation_dirty = true;
    }
}

fn transform_editor(
    ui: &mut egui::Ui,
    data: &mut TransformData,
    translation_step: f32,
    rotation_step: f32,
    scale_step: f32,
    palette: &theme::EditorPalette,
) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new("Transform")
        .default_open(true)
        .show(ui, |ui| {
            changed |= vector3_editor(
                ui,
                "Position",
                &mut data.translation,
                translation_step.max(0.01),
                -10000.0..=10000.0,
                palette,
            );
            changed |= vector3_editor(
                ui,
                "Rotation (deg)",
                &mut data.rotation_deg,
                rotation_step.max(0.1),
                -3600.0..=3600.0,
                palette,
            );
            changed |= vector3_editor(
                ui,
                "Scale",
                &mut data.scale,
                scale_step.max(0.01),
                0.01..=1000.0,
                palette,
            );
        });
    changed
}

fn vector3_editor(
    ui: &mut egui::Ui,
    label: &str,
    values: &mut [f32; 3],
    speed: f32,
    range: std::ops::RangeInclusive<f32>,
    palette: &theme::EditorPalette,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(theme::caption(label, palette));
        for (axis_idx, axis_label) in ["X", "Y", "Z"].iter().enumerate() {
            ui.label(*axis_label);
            let response = ui.add(
                egui::DragValue::new(&mut values[axis_idx])
                    .speed(speed)
                    .range(range.clone()),
            );
            changed |= response.changed();
        }
    });
    changed
}

fn tint_editor(ui: &mut egui::Ui, tint: &mut Option<[f32; 3]>) {
    ui.horizontal(|ui| {
        let mut has_tint = tint.is_some();
        if ui.checkbox(&mut has_tint, "Enable tint").changed() {
            *tint = if has_tint {
                Some([1.0, 1.0, 1.0])
            } else {
                None
            };
        }
    });
    if let Some(rgb) = tint {
        ui.horizontal(|ui| {
            ui.color_edit_button_rgb(rgb);
            if ui.button("Reset").clicked() {
                *rgb = [1.0, 1.0, 1.0];
            }
        });
    }
}

// --- Status bar ----------------------------------------------------------

fn status_bar_ui(
    ui: &mut egui::Ui,
    state: &EditorState,
    history: &EditorHistory,
    palette: &theme::EditorPalette,
) {
    ui.horizontal(|ui| {
        ui.label(theme::caption(
            &format!("Tool: {}", state.tool.label()),
            palette,
        ));
        ui.separator();
        ui.label(theme::caption(
            &format!("Brush: {}", state.current_kind),
            palette,
        ));
        ui.separator();
        ui.label(theme::caption(
            &format!("Props: {}", state.manifest.props.len()),
            palette,
        ));
        ui.separator();
        ui.label(theme::caption(
            &format!(
                "Snap: {} u / {}°",
                state.snap_translation, state.snap_rotation_deg
            ),
            palette,
        ));
        ui.separator();
        let dirty_marker = if state.dirty {
            "● unsaved"
        } else {
            "○ saved"
        };
        let dirty_color = if state.dirty {
            palette.warning
        } else {
            palette.success
        };
        ui.label(egui::RichText::new(dirty_marker).color(dirty_color));
        ui.separator();
        let undo_label = format!(
            "Undo: {}  Redo: {}",
            count_history(history.can_undo()),
            count_history(history.can_redo())
        );
        ui.label(theme::caption(&undo_label, palette));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if state.validation_issues.is_empty() {
                ui.label(egui::RichText::new("✓ map valid").color(palette.success));
            } else {
                ui.label(
                    egui::RichText::new(format!(
                        "⚠ {} validation issues",
                        state.validation_issues.len()
                    ))
                    .color(palette.error),
                );
            }
        });
    });
}

fn count_history(can: bool) -> &'static str {
    if can {
        "available"
    } else {
        "empty"
    }
}

// --- Helpers -------------------------------------------------------------

fn set_selected_entity(
    commands: &mut Commands,
    selected_q: &Query<Entity, With<SelectedMarker>>,
    state: &mut EditorState,
    entity: Entity,
) {
    clear_selected(commands, selected_q);
    commands.entity(entity).insert(SelectedMarker);
    state.selected = Some(entity);
}

fn clear_selected(commands: &mut Commands, selected_q: &Query<Entity, With<SelectedMarker>>) {
    for previous in selected_q.iter() {
        commands.entity(previous).remove::<SelectedMarker>();
    }
}

fn sync_entity_transform(commands: &mut Commands, entity: Entity, data: TransformData) {
    commands.entity(entity).insert(Transform {
        translation: Vec3::from_array(data.translation),
        rotation: quat_from_rotation_deg(data.rotation_deg),
        scale: Vec3::from_array(data.scale),
    });
}

fn collision_shape_label(shape: Option<CollisionShape>) -> &'static str {
    match shape {
        None => "None",
        Some(CollisionShape::Box { .. }) => "Box",
        Some(CollisionShape::Cylinder { .. }) => "Cylinder",
        Some(CollisionShape::Sphere { .. }) => "Sphere",
    }
}

/// Palette categories shown in the Palette tab. Drives the visual grouping.
struct PaletteCategory {
    label: &'static str,
    ident: &'static str,
}

const PALETTE_CATEGORIES: [PaletteCategory; 4] = [
    PaletteCategory {
        label: "Vegetation",
        ident: "vegetation",
    },
    PaletteCategory {
        label: "Buildings",
        ident: "buildings",
    },
    PaletteCategory {
        label: "Rocks",
        ident: "rocks",
    },
    PaletteCategory {
        label: "Props",
        ident: "props",
    },
];

/// Returns the stable category identifier for a palette kind.
fn palette_category_of(kind: &str) -> &'static str {
    match kind {
        "tree_oak" | "bush_01" => "vegetation",
        "house_simple" | "fence_01" | "lamp_01" => "buildings",
        "rock_01" | "rock_02" => "rocks",
        _ => "props",
    }
}

/// Lightweight per-kind icon glyph. Falls back to a generic box for unknown
/// kinds so the palette stays readable as new assets are added.
fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "tree_oak" => "🌳",
        "bush_01" => "🌿",
        "house_simple" => "🏠",
        "fence_01" => "🚧",
        "lamp_01" => "💡",
        "rock_01" | "rock_02" => "🪨",
        "crate_01" => "📦",
        "statue_01" => "🗿",
        _ => "▢",
    }
}

/// No native chrome remains, so this is always false. egui's pointer-input
/// capture already prevents clicks over panels from reaching the viewport.
pub fn cursor_over_editor_chrome(_window: &Window, _cursor_pos: Vec2) -> bool {
    false
}
