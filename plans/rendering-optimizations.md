# Rendering Optimization Plan — BevyMMO

> **Status**: Planned  
> **Priority**: High  
> **Components**: `crates/editor`, `crates/presentation`

Full analysis of the rendering code, health bars, and editor to identify performance
bottlenecks and propose concrete fixes.

---

## Background & Diagnosis

After reading all of `crates/presentation` and `crates/editor`, **6 problems** were
found in 3 distinct families that explain the editor lag (already at 50 props) and
the health bar stuttering.

---

## Problems Found

### 🔴 Problem 1 — Editor: O(N²) in the Outliner (main cause of lag)

**File**: `crates/editor/src/ui.rs`, function `outliner_ui` (~line 401)

```rust
// CURRENT CODE — O(N²)!
let scene_entries: Vec<_> = state.manifest.props
    .iter()
    .map(|prop| {
        // For each prop in the manifest, does a linear scan
        // through the ENTIRE ECS query!
        let prop_entity = prop_q
            .iter()
            .find(|(_, editor_prop, _)| editor_prop.prop_id == prop.id)  // O(N) per prop!
            .map(|(entity, _, _)| entity);
        ...
    })
    .collect();
```

With 500 props: 500 × 500 = **250,000 string comparisons per frame**,
just to draw the egui outliner.

---

### 🔴 Problem 2 — Editor: `collect_bodies` allocates a `Vec` every frame

**File**: `crates/editor/src/picking.rs`, functions `update_hover` and `place_or_select`

`update_hover` runs **every frame** and calls `collect_bodies`, which allocates a fresh
`Vec<PickBody>` and iterates over all props. With 500 props: **500 iterations +
one heap allocation per frame**, even when the mouse is not moving.

---

### 🟡 Problem 3 — Editor + Client: unique Mesh and Material per prop (no GPU instancing)

**File**: `crates/editor/src/picking.rs` (`spawn_prop_entity`)  
**File**: `crates/presentation/src/world.rs` (`spawn_prop_visual`)

```rust
// Both in the editor and in the client, for EVERY prop:
Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),    // unique Handle<Mesh>!
MeshMaterial3d(materials.add(StandardMaterial {     // unique Handle<Material>!
    base_color: color,
    ..default()
})),
```

With 500 "rock_01" props (same color): 500 identical meshes uploaded to the GPU,
500 separate draw calls instead of 1 instanced one. This is the main **GPU
scalability** problem.

Bevy performs GPU instancing automatically when multiple entities share the **same
`Handle<Mesh>` + same `Handle<StandardMaterial>`**. The current code prevents this.

---

### 🟡 Problem 4 — HealthBar: relayout every frame even for stationary entities

**File**: `crates/presentation/src/ui/entity_bar/systems.rs`, `update_floating_ui_position`

The position system runs **every frame for every bar**, even for entities that have
not moved. Although the code already compares values (`if node.left != new_left`),
`world_to_viewport` is still computed for every bar every frame, and any write to a
Bevy UI `Node` triggers a relayout of its subtree.

---

### 🟡 Problem 5 — HealthBar: content update iterates everything every frame

**File**: `crates/presentation/src/ui/entity_bar/systems.rs`, `update_floating_ui_content`

The system already uses a cache (`last_fill_pct`, `last_name`, etc.) to avoid
unnecessary writes, but it **iterates all bars every frame** to check the cache.
With 100+ entities this becomes measurable. The cache is a good safety net but does
not replace upstream filtering.

---

### 🟢 Problem 6 — Entity renderer: new meshes for each projectile and mob

**File**: `crates/presentation/src/renderer.rs`, `spawn_entity_meshes`

Every non-player entity (projectiles, mob placeholders) creates its own `Cuboid` and
`StandardMaterial`. Same pattern as Problem 3, but with lower impact since gameplay
entities are typically few.

---

## Impact Summary

| # | Where | Impact | Fix difficulty |
|---|-------|--------|----------------|
| 1 | Editor Outliner | 🔴 Critical — O(N²) per frame | Easy |
| 2 | Editor Hover | 🔴 Critical — Vec alloc every frame | Medium |
| 3 | Editor + Client Mesh/Mat | 🟡 High — no GPU batching | Medium |
| 4 | HealthBar position | 🟡 Medium — unnecessary relayout | Easy |
| 5 | HealthBar content | 🟡 Medium — iterates everything | Easy |
| 6 | Entity renderer | 🟢 Low (few entities) | Medium |

---

## Proposed Changes

---

### Fix 1 — Outliner O(N²) → O(N)

#### [MODIFY] `crates/editor/src/ui.rs`

```diff
 fn outliner_ui(
     ui: &mut egui::Ui,
     commands: &mut Commands,
     state: &mut EditorState,
     selected_q: &Query<Entity, With<SelectedMarker>>,
     prop_q: &Query<(Entity, &EditorProp, &Transform), Without<EditorTerrain>>,
     palette: &theme::EditorPalette,
 ) {
+    use std::collections::HashMap;
     ...
+    // Build an id→entity map in O(N), then access in O(1) per prop.
+    let entity_by_id: HashMap<&str, Entity> = prop_q
+        .iter()
+        .map(|(entity, editor_prop, _)| (editor_prop.prop_id.as_str(), entity))
+        .collect();
+
     let scene_entries: Vec<(String, String, Option<Entity>, bool)> = state
         .manifest
         .props
         .iter()
         .map(|prop| {
-            let prop_entity = prop_q
-                .iter()
-                .find(|(_, editor_prop, _)| editor_prop.prop_id == prop.id)
-                .map(|(entity, _, _)| entity);
+            let prop_entity = entity_by_id.get(prop.id.as_str()).copied();
             (
                 prop.id.clone(),
                 prop.kind.clone(),
                 prop_entity,
                 prop_entity == state.selected,
             )
         })
         .collect();
```

**Impact**: reduces per-frame CPU work in the outliner from O(N²) to O(N).
With 500 props: ~250k fewer operations every frame.

---

### Fix 2 — Cache `PickBody` with change detection

#### [NEW] `PickBodyCache` resource in `crates/editor/src/picking.rs`

```rust
/// Cached list of pickable bodies, rebuilt only when transforms change.
#[derive(Resource, Default)]
pub struct PickBodyCache {
    pub bodies: Vec<PickBody>,
}
```

#### [MODIFY] `crates/editor/src/picking.rs`

```rust
/// Rebuilds the pick cache only when a prop or terrain transform actually changes.
pub fn refresh_pick_cache(
    mut cache: ResMut<PickBodyCache>,
    // Change-detection filters: system only runs when something changed.
    _changed_props: Query<
        (),
        Or<(Changed<Transform>, Added<EditorProp>, Added<EditorTerrain>)>,
    >,
    all_props: Query<(Entity, &EditorProp, &Transform), Without<EditorTerrain>>,
    all_terrain: Query<(Entity, &Transform), (With<EditorTerrain>, Without<EditorProp>)>,
) {
    cache.bodies = collect_bodies(&all_props, &all_terrain);
}

pub fn update_hover(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    cache: Res<PickBodyCache>,   // <-- reads the cache, no allocation
    mut state: ResMut<EditorState>,
) {
    ...
    state.hovered = pick_closest(camera, camera_transform, cursor_pos, &cache.bodies);
}
```

#### [MODIFY] `crates/editor/src/lib.rs`

```diff
 app.add_systems(
     Update,
     (
+        picking::refresh_pick_cache,  // must run before hover and click
         picking::place_or_select,
         picking::update_hover,
         ...
     )
-        .run_if(not(egui_wants_any_pointer_input)),
+        .chain()
+        .run_if(not(egui_wants_any_pointer_input)),
 );
```

---

### Fix 3 — Shared mesh/material handles (GPU instancing)

#### [NEW] `PropMeshRegistry` resource in `crates/editor/src/picking.rs`

```rust
/// Pre-allocated mesh and material handles shared across props of the same
/// kind/color, enabling Bevy's automatic GPU instancing.
#[derive(Resource, Default)]
pub struct PropMeshRegistry {
    /// Single 1×1×1 cuboid mesh shared by all props.
    cuboid_1x1: Option<Handle<Mesh>>,
    /// Per-color material cache. Key: raw f32 bits of (r, g, b).
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
            .or_insert_with(|| materials.add(StandardMaterial { base_color: color, ..default() }))
            .clone()
    }
}

fn color_key(color: Color) -> [u32; 3] {
    let [r, g, b, _] = color.to_srgba().to_f32_array();
    [r.to_bits(), g.to_bits(), b.to_bits()]
}
```

#### [MODIFY] `crates/editor/src/picking.rs` — `spawn_prop_entity`

```diff
 pub fn spawn_prop_entity(
     commands: &mut Commands,
     meshes: &mut Assets<Mesh>,
     materials: &mut Assets<StandardMaterial>,
     prop: &Prop,
+    registry: &mut PropMeshRegistry,
 ) -> Entity {
     let base_color = ...;
+    let mesh = registry.get_or_create_mesh(meshes);
+    let mat  = registry.get_or_create_material(materials, base_color);
     commands
         .spawn((
             Name::new(...),
-            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
-            MeshMaterial3d(materials.add(StandardMaterial { base_color, ..default() })),
+            Mesh3d(mesh),
+            MeshMaterial3d(mat),
             ...
         ))
         .id()
 }
```

Same pattern for `crates/presentation/src/world.rs` (`spawn_prop_visual`), using an
analogous `ClientPropMeshRegistry` resource.

---

### Fix 4 — HealthBar: cache the computed viewport position

#### [MODIFY] `crates/presentation/src/ui/entity_bar/components.rs`

```diff
 pub struct FloatingUi {
     pub target: Entity,
     pub offset: Vec3,
+    /// Last computed viewport position. Used to skip relayout when unchanged.
+    pub last_viewport: Option<Vec2>,
 }
```

#### [MODIFY] `crates/presentation/src/ui/entity_bar/systems.rs`

```diff
 pub fn update_floating_ui_position(
     camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
     target_query: Query<&Position>,
-    mut ui_query: Query<(&FloatingUi, &mut Node)>,
+    mut ui_query: Query<(&mut FloatingUi, &mut Node)>,
 ) {
     let Ok((camera, camera_transform)) = camera_query.single() else { return; };

-    for (floating_ui, mut node) in ui_query.iter_mut() {
+    for (mut floating_ui, mut node) in ui_query.iter_mut() {
         let Ok(pos) = target_query.get(floating_ui.target) else { continue; };
         let world_pos = pos.0 + floating_ui.offset;
         let Ok(viewport_pos) = camera.world_to_viewport(camera_transform, world_pos) else {
             if node.display != Display::None { node.display = Display::None; }
+            floating_ui.last_viewport = None;
             continue;
         };

+        // Skip write if the viewport position has not changed (sub-pixel threshold).
+        if floating_ui.last_viewport
+            .map_or(false, |last| (last - viewport_pos).length_squared() < 0.25)
+        {
+            continue;
+        }
+        floating_ui.last_viewport = Some(viewport_pos);

         node.left    = Val::Px(viewport_pos.x - BAR_WIDTH * 0.5);
         node.top     = Val::Px(viewport_pos.y - STACK_HEIGHT);
         node.display = Display::Flex;
     }
 }
```

---

### Fix 5 — HealthBar content: skip with `Changed<VitalStats>`

#### [MODIFY] `crates/presentation/src/ui/entity_bar/systems.rs`

```diff
 pub fn update_floating_ui_content(
     target_query: Query<(&VitalStats, Option<&PlayerName>, Option<&EntityKind>)>,
+    changed_vitals: Query<Entity, Changed<VitalStats>>,
     theme: Res<UiTheme>,
     mut ui_query: Query<(&FloatingUi, &mut EntityBarParts)>,
     mut text_query: Query<&mut Text>,
     mut node_query: Query<&mut Node>,
     mut bg_query: Query<&mut BackgroundColor>,
 ) {
     for (floating_ui, mut parts) in ui_query.iter_mut() {
+        // Skip if the target's VitalStats did not change this frame.
+        if !changed_vitals.contains(floating_ui.target) {
+            continue;
+        }
         let Ok((vital, name, entity_kind)) = target_query.get(floating_ui.target) else {
             continue;
         };
         ...
     }
 }
```

> **Note**: the existing `last_fill_pct` / `last_hp_text` / `last_name` cache is kept
> as a safety net for the first frame and other edge cases. Both mechanisms complement
> each other.

---

## Verification Plan

### Build

```powershell
cargo build
```

### Automated Tests

```powershell
cargo test -p bevymmo_presentation
cargo test -p bevymmo_editor
cargo clippy -- -D warnings
```

### Manual Verification

| Test | How to verify |
|------|--------------|
| Editor lag with 500 props | Place 500 props and confirm stable FPS (≥60) |
| Smooth outliner scroll | With 200+ props, open the Outliner tab and scroll fast — no hitching |
| Hover correctness with cache | Move cursor over various props → highlight updates correctly |
| HealthBar no jitter | 20+ entities with VitalStats, move camera → bars follow smoothly |
| Reduced GPU draw calls | Use `bevy_diagnostic` or RenderDoc to confirm draw calls drop proportionally to shared prop types |

---

## Open Questions

1. **Scope**: implement all 5 fixes in one go, or start with the most critical
   ones (1 and 2) first?

2. **PickBodyCache and click**: the cache is invalidated when prop transforms change,
   not when the camera moves. For hover this is fine (world positions don't change
   with the camera). Should `place_or_select` also read from the cache, or always
   recompute live at click time?

3. **Per-prop tint and material registry**: `PropMeshRegistry` uses the color as the
   material cache key. The manifest already supports a custom `tint` per prop. Can
   you confirm the intended behaviour — props with different tints get separate
   materials (no sharing), which is correct?
