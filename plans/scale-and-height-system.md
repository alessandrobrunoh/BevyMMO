# Plan: Stairs and World Height System

**Branch**: `feat/scale-and-height`
**Status**: Draft — approval required before implementation

## Goal

Introduce a deterministic, server-authoritative system for variable-height terrain and traversable stairs while keeping client prediction consistent with the server and preventing visual model scale from becoming the gameplay source of truth.

## Current context

The project uses Bevy ECS and Lightyear with an authoritative server and client-side prediction.

- `crates/shared/src/network/protocol.rs` contains `Position(Vec3)`, so the vertical `Y` coordinate can already be replicated.
- `crates/shared/src/movement.rs` contains movement math shared by the server and client.
- `crates/server/src/player_movement.rs` applies authoritative movement.
- `crates/presentation/src/player_movement.rs` applies client prediction.
- `crates/shared/src/world/manifest.rs` describes `Terrain`, `Prop`, and `TransformData`.
- `crates/shared/src/world/collision.rs` builds an AABB collision grid and currently ignores `Y` during movement collision checks.
- `crates/server/src/world.rs` loads the map manifest and builds the server collision resource.
- `crates/presentation/src/player_movement.rs` currently calculates click-to-move targets by intersecting the camera ray with the `Y = 0` plane.
- The camera already follows replicated `Position`, so it should not need a separate height system.

## Design decisions

### Gameplay data must be separate from visual transforms

`TransformData.scale` remains responsible for visual placement and model size. It must not determine whether an entity is walkable, climbable, or physically blocking.

Gameplay-specific properties should be stored separately in the world manifest. This prevents a visual asset change from silently changing movement behavior.

### Walkable surfaces: flat ground, ramps, stairs, and hills

The gameplay system must support more than two-point ramps. Blender-authored maps may contain:

- flat ground;
- ramps and stairs;
- hills with arbitrary height variation;
- cliffs and ledges that are not walkable across;
- upper floors;
- bridges and walkable platforms;
- separate walkable regions at different heights.

The authoritative representation should therefore be a simplified, explicitly authored **walkable surface mesh** or equivalent height representation, not the visual mesh itself. Each walkable surface is made from triangles or patches that can answer:

```text
Given X/Z and the relevant current surface/layer,
what is the valid ground height Y and surface normal?
```

For the first runtime implementation, a low-poly gameplay mesh exported from Blender is preferred over trying to reconstruct a surface from the detailed visual `.glb`. A flat floor, a ramp, and a hill can all use the same surface-query model. A stair can use a smooth ramp mesh even when its visual model contains individual steps.

The mesh must be separate from visual geometry because:

- decorative geometry must not become walkable accidentally;
- a visual mesh can contain ceilings, walls, and overlapping parts;
- detailed meshes are unnecessarily expensive for server queries;
- server and client must consume the same simplified data;
- level designers need explicit control over walkable and non-walkable areas.

Solid blockers remain separate from walkable surfaces. A hill surface tells the game where the player stands; a cliff or wall blocker tells the game where the player cannot move.

### Server authority

The client may send a target containing an estimated height, but the server must recalculate the walkable surface and final `Y` coordinate. Client-supplied height is never trusted for authoritative movement.

The client prediction path must call the same pure shared helpers as the server wherever possible.

### Initial traversal rules

The initial system should use explicit constants, subject to gameplay tuning later:

```rust
pub const MAX_STEP_HEIGHT: f32 = 0.45;
pub const MAX_SLOPE_ANGLE_DEG: f32 = 45.0;
pub const GROUND_SNAP_DISTANCE: f32 = 0.2;
```

These values are placeholders for the first implementation and must be centralized rather than duplicated across crates.

### World-unit reference

The map pipeline should define one explicit scale convention:

```text
1 Blender unit = 1 game world unit = 1 meter
```

The initial world metrics should be documented and centralized, without coupling them to the surface data model:

```rust
pub struct WorldMetrics {
    pub player_radius: f32,
    pub player_height: f32,
    pub eye_height: f32,
    pub max_step_height: f32,
    pub max_walkable_slope_deg: f32,
}
```

The exact values are gameplay decisions. The important requirement is that Blender authoring, entity dimensions, collision radii, stair dimensions, and movement constants use the same unit convention.

## Blender-to-game authoring contract

Blender is the source of authoring information, but the `.glb` file must not be the only source of gameplay data. The export pipeline should produce two coordinated artifacts:

```text
starting_village.glb
starting_village.world.json
```

### Responsibilities of each file

`starting_village.glb` contains client-facing visual content:

- terrain and floor meshes;
- stair models;
- buildings and decorations;
- materials and visual transforms;
- optional debug marker meshes.

`starting_village.world.json` contains gameplay content:

- walkable surfaces;
- upper floors and their heights;
- stair/ramp traversal links;
- blocking collision volumes;
- map bounds;
- stable ids and authoring metadata.

The server loads the gameplay manifest and does not need to load the `.glb`. The client loads both files: the `.glb` for rendering and the manifest for movement, click targeting, prediction, and debug visualization.

### Required Blender Collection layout

The map authoring convention should use separate Collections:

```text
Map
├── VISUAL
│   ├── Ground
│   ├── House_01
│   ├── StairModel_GroundToUpper
│   └── UpperFloorModel
│
├── GAMEPLAY
│   ├── WALKABLE_Ground
│   ├── WALKABLE_Hill_North
│   ├── WALKABLE_UpperFloor
│   └── STAIR_GroundToUpper
│
└── COLLISION
    ├── BLOCKING_Wall_01
    ├── BLOCKING_Pillar_01
    └── BLOCKING_Rock_01
```

Collection membership is authoring information. It should be converted into explicit manifest records by the exporter; runtime code should not need to infer gameplay semantics from arbitrary visual node names.

### Visual objects

Objects in `VISUAL` are rendered objects only. They may use normal Blender transforms and may be exported into the `.glb` using their normal mesh, material, position, rotation, and scale.

A visual stair model may show individual steps, railings, and decorations. Its mesh topology does not define the gameplay surface. The associated `STAIR_*` marker defines the traversable ramp.

### Walkable surface markers and gameplay meshes

A walkable area should have an explicit gameplay object in `GAMEPLAY`. Use a dedicated low-poly mesh for hills, ramps, and irregular terrain, and use an Empty or non-rendered cuboid only for simple flat rectangular floors.

Recommended naming:

```text
WALKABLE_Ground
WALKABLE_Hill_North
WALKABLE_UpperFloor
```

Recommended custom properties for a walkable mesh:

```text
gameplay_type = "walkable_surface"
surface_id = "hill_north"
surface_kind = "terrain"
walkable = true
max_slope_deg = 45.0
```

The walkable mesh must contain only the surface on which characters may stand. It must not include walls, ceilings, decorative rocks, underside faces, or hidden geometry. The exporter should apply the object's world transform, triangulate or validate the mesh, and write a simplified representation to the gameplay manifest.

For a flat upper floor, a dedicated gameplay marker can still be an Empty or a non-rendered cuboid, named using this convention:

```text
SURFACE_UpperFloor
```

Recommended custom properties:

```text
gameplay_type = "surface"
surface_type = "flat"
surface_id = "upper_floor"
height_mode = "object_origin"
```

The exporter should read the marker's world-space transform and dimensions and generate a manifest record such as:

```json
{
  "id": "upper_floor",
  "kind": "flat",
  "bounds": {
    "min_x": 5.0,
    "max_x": 15.0,
    "min_z": -4.0,
    "max_z": 6.0
  },
  "height": 3.0
}
```

For a flat marker, its dimensions define the walkable region and its world-space `Y` position defines the floor height. For a walkable mesh, the mesh vertices define the varying height. The visual floor or hill model should be aligned to the gameplay object, but the gameplay object remains the source of truth.

### Hills and arbitrary-height terrain

A hill should not be represented by one `height` value or one `start/end` ramp. In Blender, create a simplified duplicate of the walkable portion of the hill in `GAMEPLAY`:

```text
VISUAL
└── Hill_North_Detailed

GAMEPLAY
└── WALKABLE_Hill_North_LowPoly
```

The gameplay hill mesh should:

- cover only the area where the player may walk;
- use significantly fewer polygons than the visual hill;
- preserve important ridges, valleys, paths, and cliff boundaries;
- avoid near-vertical or inverted triangles unless explicitly supported;
- have consistent vertex winding and valid normals;
- use world-space coordinates after Blender transforms are applied.

The exporter should validate every triangle and generate a spatially queryable surface representation. At runtime, the surface query projects a candidate `X/Z` point onto the relevant gameplay triangles and interpolates `Y` using barycentric coordinates. The result includes the interpolated height and triangle normal.

This supports arbitrary height variation without requiring the game to understand how the hill was modeled visually.

For very large maps, the exported triangles should later be indexed in a spatial grid or BVH. The initial implementation may use a deterministic linear scan for small fixtures, but the runtime API must not expose the storage strategy.

### Surface layers and upper floors

Two surfaces may share an `X/Z` region at different heights. The query must therefore not be defined only as `ground_at(x, z)` internally. It must be able to consider the current surface or current `Y`, for example:

```rust
fn surfaces_at(&self, x: f32, z: f32) -> Vec<GroundContact>;
```

or an equivalent API that accepts the current position and returns the reachable surface. The first implementation should use stable `surface_id` links and explicit traversal connections. A numeric `floor_id` is not required until multi-floor navigation is implemented.

The exporter must make the coordinate convention explicit:

- marker coordinates are converted to world space before export;
- Blender's `X`, `Y`, and `Z` axes are mapped consistently to the game;
- object rotation and scale are applied consistently when deriving bounds;
- negative scale and unapplied transforms are either normalized by the exporter or rejected with a diagnostic.

### Stair/ramp markers

A stair should have a dedicated marker named using this convention:

```text
STAIR_GroundToUpper
```

The preferred Blender representation is a simple oriented Empty or cuboid whose local axis defines the direction of travel. The exporter should not try to reconstruct the stair from individual visual steps.

Recommended custom properties:

```text
gameplay_type = "traversal"
traversal_type = "ramp"
traversal_id = "stair_ground_to_upper"
start_surface = "ground"
end_surface = "upper_floor"
```

The exporter should derive or validate these values:

- start point in world space;
- end point in world space;
- horizontal length;
- width;
- start height;
- end height;
- orientation;
- linked lower and upper surface ids.

A more explicit marker setup may use two child empties:

```text
STAIR_GroundToUpper
├── START
└── END
```

In that setup:

- `START` defines the lower entry point;
- `END` defines the upper exit point;
- the parent marker's local `X` or `Y` dimension defines the stair width;
- the exporter converts both points to world space;
- the exporter verifies that `START` and `END` are not coincident in horizontal space.

The generated manifest should contain data similar to:

```json
{
  "id": "stair_ground_to_upper",
  "kind": "ramp",
  "start": [2.0, 0.0, 5.0],
  "end": [8.0, 3.0, 5.0],
  "width": 2.5,
  "start_surface": "ground",
  "end_surface": "upper_floor"
}
```

The runtime computes the height along the stair using the progress between `start` and `end`. A character at the beginning, middle, and end of the stair therefore resolves to approximately `Y = 0`, `Y = 1.5`, and `Y = 3` in this example.

### Blocking collision markers

Blocking collision should be represented by simplified marker geometry in `COLLISION`, not by detailed visual meshes.

Recommended naming:

```text
BLOCKING_Wall_01
BLOCKING_Pillar_01
BLOCKING_Rock_01
```

Recommended custom properties:

```text
gameplay_type = "blocking"
collision_type = "box"
collision_id = "wall_01"
```

The exporter should convert the marker's world-space transform and dimensions into the existing shared collision representation. Visual decorations should not accidentally become colliders.

### Blender custom properties versus sidecar manifest

glTF/GLB supports node metadata through `extras`, and the exporter may preserve marker properties there for debugging. However, runtime gameplay must not depend on arbitrary GLB metadata being preserved by every importer.

The sidecar manifest is the authoritative gameplay artifact because:

- the headless server can load it without loading rendering assets;
- it can be validated before gameplay starts;
- it is stable even if visual GLB organization changes;
- it can be versioned and migrated independently;
- it avoids coupling gameplay to Bevy asset-loader behavior.

### Blender export script

Add a dedicated Blender Python export script, with the final location chosen according to the repository's asset/tooling layout. Its responsibilities should be:

1. Open or operate on the active Blender scene.
2. Validate required Collections: `VISUAL`, `GAMEPLAY`, and `COLLISION`.
3. Validate unique ids for surfaces, traversals, and blockers.
4. Convert marker transforms to world space.
5. Extract flat surfaces from simple `WALKABLE_*` markers.
6. Extract and validate low-poly walkable meshes from `WALKABLE_*` objects for hills, ramps, and irregular terrain.
7. Extract traversal links from `STAIR_*` markers and `START`/`END` children.
8. Extract simplified blocking collision from `BLOCKING_*` markers.
9. Compute map bounds from an explicit map-bounds marker or configured map bounds.
10. Validate that walkable meshes have valid triangles, finite coordinates, consistent winding, and acceptable slopes.
11. Validate that stair start/end surfaces exist.
12. Validate that stair endpoints are close enough to the linked surfaces.
13. Validate that visual objects and gameplay markers are not accidentally exported into the wrong category.
14. Export the visual scene to `.glb`.
15. Write the gameplay manifest to `.world.json`.
16. Print a summary containing map id, visual object count, walkable triangle count, surface count, traversal count, and blocker count.
17. Fail with actionable diagnostics instead of writing a partially valid manifest.

The script should be deterministic: the same Blender scene and export settings must produce equivalent manifest data regardless of object iteration order.

### Export-time validation rules

The exporter should reject or report:

- missing required Collections;
- duplicate ids;
- empty or invalid ids;
- non-finite coordinates, rotations, scales, or dimensions;
- zero or negative surface dimensions;
- empty walkable meshes;
- degenerate triangles;
- non-finite mesh vertices;
- inconsistent triangle winding where it affects normal calculation;
- unsupported or excessive walkable slopes;
- zero or negative stair width;
- stairs with no horizontal length;
- stairs referencing unknown surfaces;
- stairs whose endpoints do not lie on or near their linked surfaces;
- blocking markers with unsupported geometry or dimensions;
- unapplied or negative scale when it makes world-space bounds ambiguous;
- gameplay markers with unsupported `gameplay_type` values.

Warnings may be used for visual alignment issues that do not make the manifest mathematically invalid. Errors must prevent the manifest from being emitted as a valid map.

### Runtime debug visualization

The client should eventually provide a debug mode that draws:

- walkable flat surfaces and hills in green;
- stair/ramp traversal links in blue;
- solid blockers in red;
- stair start and end points;
- surface heights and ids;
- optional surface normals.

This is important because the most likely authoring error is not a Rust movement bug but a mismatch between the visible `.glb` model and the gameplay marker.

## Glossary

- **Walkable surface**: an explicitly authored area or mesh that supplies a valid height and surface normal for a candidate `X/Z` point.
- **Walkable gameplay mesh**: a simplified Blender-authored mesh used for terrain, hills, ramps, or irregular walkable geometry; it is separate from the visual mesh.
- **Ramp/stair**: an inclined walkable surface or traversal link connecting two elevations.
- **Surface layer**: a distinct walkable region that may overlap another region in `X/Z` at a different height.
- **Solid obstacle**: geometry that blocks movement independently of walkable surfaces.
- **Ground height**: the authoritative `Y` value at a point on a walkable surface.
- **Step height**: the largest vertical difference accepted during ordinary movement.
- **Ground snap**: correcting an entity's `Y` coordinate to the walkable surface beneath it.
- **Traversal volume**: the gameplay representation of a special movement area such as a stair or ladder.

## Global acceptance criteria

- [ ] A map can describe flat terrain, a hill with arbitrary height variation, and at least one stair/ramp with different start and end heights.
- [ ] Walkable gameplay meshes are separate from visual `.glb` meshes and are exported into the gameplay manifest.
- [ ] Invalid surface definitions and invalid walkable triangles are rejected during map validation.
- [ ] A deterministic shared function can resolve the ground height and normal for a candidate `X/Z` point.
- [ ] Surface queries can distinguish or reject ambiguous upper-floor surfaces instead of silently choosing by iteration order.
- [ ] The authoritative server moves a player up and down a stair while preserving the correct `Y` coordinate.
- [ ] A player cannot climb an ordinary obstacle whose height difference exceeds `MAX_STEP_HEIGHT`.
- [ ] Solid obstacles continue to block movement even when a walkable surface exists nearby.
- [ ] Client prediction and authoritative server movement produce equivalent results for the same map and input.
- [ ] Clicking a stair produces a target that uses the correct surface height when possible.
- [ ] The server recalculates and validates the target height instead of trusting the client.
- [ ] The camera continues to follow replicated `Position`, including `Y`, without duplicating gameplay height logic in presentation code.
- [ ] Existing flat manifests remain loadable through a compatibility path and receive an implicit flat walkable surface where appropriate.
- [ ] The client can visualize walkable meshes, traversal links, and blockers in an early debug mode.
- [ ] The server remains headless and does not register meshes, materials, scenes, or rendering systems.
- [ ] `cargo test` passes after every completed slice.
- [ ] `cargo clippy -- -D warnings` passes before each slice is considered complete.

## Out of scope

- Jumping and free-fall physics.
- A full physics engine integration.
- Ladders with an explicit climbing state.
- Elevators or moving platforms.
- Full 3D navmesh/pathfinding.
- Triangle-level collision against detailed visual meshes. The first implementation may use triangles from a separately authored, simplified gameplay mesh.
- Persisting the player's exact `Y` coordinate in the database.
- Dynamically deformable terrain.
- Curved or procedurally generated stair surfaces.

## Implementation slices

Every slice follows RED-GREEN-MUTATE-KILL MUTANTS-REFACTOR. No production implementation should be written before its behavior test exists and fails for the expected reason.

### Slice 1: Define the Blender export contract and validated walkable-surface manifest

**Value**: Map authors can describe stairs and variable-height surfaces using stable gameplay data that both server and client can load.

**Production path**:

Blender scene -> export script -> `.glb` + `.world.json` -> `MapManifest` -> shared validation -> server/client map loading.

**Likely files**:

- `crates/shared/src/world/manifest.rs`
- `crates/shared/src/world/shapes.rs`
- `crates/shared/src/world/loader.rs`
- `crates/shared/src/world/mod.rs`
- map fixtures under the existing map/assets directory
- the new Blender export script in the repository's map tooling/assets location
- an example Blender scene or documented marker setup for one flat floor, one upper floor, one stair, and one blocker

**Proposed data model**:

The manifest should contain top-level gameplay collections in addition to visual props:

```rust
pub struct MapManifest {
    pub version: u32,
    pub map_id: String,
    pub display_name: String,
    pub bounds: MapBounds,
    pub terrain: Terrain,
    pub surfaces: Vec<WalkableSurface>,
    pub traversals: Vec<TraversalData>,
    pub props: Vec<Prop>,
}
```

`Prop` remains appropriate for visual/static content and solid blockers. `WalkableSurface` and `TraversalData` should be explicit gameplay records generated from Blender markers rather than inferred from visual props.

```rust
pub enum TraversalKind {
    Ramp,
}

pub struct TraversalData {
    pub kind: TraversalKind,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub width: f32,
}
```

The exact names may follow the repository's existing manifest naming conventions, but the model must distinguish visual transform data from traversal data.

**Acceptance criteria**:

- [ ] A manifest can contain flat walkable surfaces, traversal records, and existing visual/static props.
- [ ] The Blender export contract is documented with the required Collection, object-name, and custom-property conventions.
- [ ] The exporter can produce a deterministic `.world.json` fixture for one flat floor, one upper floor, one stair, and one blocker.
- [ ] Existing manifests without traversal data remain loadable through defaults or an explicit compatible format migration.
- [ ] A valid ramp has a positive width and a non-zero horizontal traversal length.
- [ ] Invalid values produce a clear validation issue rather than a panic in ordinary map validation.
- [ ] Serialization and deserialization preserve traversal and surface data exactly.
- [ ] Existing solid collision definitions continue to work unchanged.

**RED**:

- Add tests for valid round-trip serialization.
- Add tests for zero/negative width.
- Add tests for coincident start/end points.
- Add tests proving an existing flat map still loads.
- Add fixture tests for a generated upper floor and stair.
- Add validation tests for unknown surface references and invalid marker-derived geometry.
- Add deterministic-output tests for the exporter/manifest conversion layer where the tooling can be executed in CI.

**GREEN**:

Implement the manifest types, Blender export contract, exporter/fixture path, defaults, and validation needed by these tests. Do not change movement yet.

**MUTATE / KILL MUTANTS**:

Check that tests fail if validation is removed, if the width comparison is reversed, or if traversal data is silently discarded during serialization.

**REFACTOR**:

Keep the model engine-agnostic and avoid introducing Bevy types into `shared::world`. Keep Blender scripting as an offline authoring tool; do not make the server execute Blender or parse `.glb` files.

**Done when**: Manifest and exporter-contract tests pass, an example `.glb`/`.world.json` map pair exists, existing map fixtures load, and the acceptance criteria are reviewed and approved.

---

### Slice 2: Resolve height and surface normal from flat, ramp, and hill gameplay geometry

**Value**: Server and client can calculate the same ground height for flat terrain, stairs/ramps, and hills with arbitrary height variation.

**Production path**:

`MapManifest` -> collision/world geometry resource -> shared surface query -> `GroundContact` result.

**Likely files**:

- `crates/shared/src/world/collision.rs`
- `crates/shared/src/world/shapes.rs`
- `crates/shared/src/world/mod.rs`
- `crates/shared/src/movement.rs`

**Proposed result type**:

```rust
pub struct GroundContact {
    pub height: f32,
    pub normal: [f32; 3],
}
```

The existing collision grid may be extended or replaced internally, but it must continue to support solid-obstacle queries. Walkable geometry should support simple flat patches and triangulated gameplay meshes. The initial implementation may use a deterministic linear scan over small fixtures; the storage strategy must remain hidden behind the query API so a spatial grid can be added after profiling.

**Surface query behavior**:

- flat terrain returns a constant height;
- a ramp returns height interpolated from its gameplay geometry;
- a hill returns height interpolated from the containing gameplay triangle;
- points outside the walkable mesh return no contact;
- triangles with unacceptable slope are rejected or treated as non-walkable;
- the query returns the appropriate surface when multiple surfaces overlap according to current height and traversal rules;
- the returned normal is normalized and consistent with the gameplay geometry.

**Acceptance criteria**:

- [ ] Flat terrain returns the expected height.
- [ ] The beginning, middle, and end of a ramp return expected heights.
- [ ] Points on a hill return interpolated heights at multiple arbitrary locations.
- [ ] Points outside the walkable mesh return no contact.
- [ ] Excessively steep triangles are rejected according to the configured slope limit.
- [ ] The result is deterministic and independent of entity or iteration order.
- [ ] Existing solid-obstacle `is_blocked` behavior remains covered by tests.
- [ ] The server and presentation crates can consume the same shared query API.

**RED**:

Write pure unit tests for flat patches, triangle containment, barycentric height interpolation, width boundaries, normal calculation, slope limits, and overlapping-surface selection.

**GREEN**:

Implement the minimum pure geometry required by the tests. Do not integrate with Bevy systems or click-to-move yet.

**MUTATE / KILL MUTANTS**:

Cover likely mutants involving incorrect barycentric interpolation, wrong triangle containment, inclusive/exclusive boundaries, wrong axis selection, missing normalization, incorrect slope comparison, and inverted surface precedence.

**REFACTOR**:

Keep surface calculations independent of `World`, `Commands`, assets, and rendering.

**Done when**: Shared geometry tests pass for flat surfaces, ramps, and hills, and the existing collision tests still pass.

---

### Slice 2.5: Add early debug visualization for authored gameplay geometry

**Value**: Level designers and developers can inspect the actual walkable surfaces and blockers before movement integration is complete.

**Production path**:

`world.json` -> client world/debug resource -> local debug meshes or Bevy gizmos -> visible surfaces, traversal links, and blockers.

**Likely files**:

- `crates/presentation/src/scenes/`
- `crates/presentation/src/renderer.rs`
- `crates/presentation/src/lib.rs`
- shared manifest/world types as needed

**Acceptance criteria**:

- [ ] A debug toggle displays walkable flat patches and hill gameplay meshes.
- [ ] Stair start/end points and traversal lines are visible.
- [ ] Blocking collision volumes are visible.
- [ ] Surface ids and heights can be inspected through logs or debug labels.
- [ ] Debug visualization is client-only and is not registered in server mode.
- [ ] Debug entities are cleaned up when leaving the game scene.

**RED**:

Add a presentation test that loads a fixture world and verifies the expected number and types of debug entities.

**GREEN**:

Render the minimum debug representation needed to validate exported geometry. Do not add a production UI or editor workflow yet.

**MUTATE / KILL MUTANTS**:

Cover missing surface categories, wrong transforms, missing cleanup, duplicate debug spawning, and accidental server registration.

**REFACTOR**:

Keep debug visualization local to presentation and driven by the shared manifest; do not duplicate geometry calculations in the renderer.

**Done when**: A developer can visually confirm flat terrain, a hill, an upper floor, a stair, and a blocker before movement integration.

---

### Slice 3: Apply height-aware movement on the authoritative server

**Value**: A player can move across flat ground and stairs on the server, with `Position.y` resolved from the authoritative walkable surface.

**Production path**:

`MoveCommand` / buffered input -> candidate `X/Z` -> shared surface query -> step-height validation -> solid collision validation -> authoritative `Position`.

**Likely files**:

- `crates/server/src/player_movement.rs`
- `crates/server/src/world.rs`
- `crates/shared/src/movement.rs`
- `crates/shared/src/world/collision.rs`

**Movement algorithm**:

1. Read the current player position and requested target.
2. Calculate the candidate horizontal step.
3. Resolve candidate walkable surfaces at the candidate `X/Z` position.
4. Select a reachable surface using current `Y`, surface identity, and traversal rules.
5. Reject the move if no valid surface exists.
6. Compare candidate ground height with the current height.
7. Reject or stop if the per-tick height difference exceeds `MAX_STEP_HEIGHT` and no valid traversal permits the transition.
8. Check solid collision at the candidate position.
9. Set the final candidate position, including the resolved `Y`.
10. Preserve the existing dead, crowd-control, cast-blocking, and arrival behavior.

The shared movement helper should expose a narrow, reusable operation such as `resolve_walkable_position` rather than placing all world-specific logic inside the server system.

**Acceptance criteria**:

- [ ] A player remains on flat ground at the expected height.
- [ ] A player ascends a ramp over multiple fixed updates.
- [ ] A player descends a ramp without losing or gaining extra height.
- [ ] A player follows a hill while `Y` changes continuously according to the gameplay mesh.
- [ ] A player does not teleport between overlapping upper/lower surfaces without a valid traversal.
- [ ] A wall higher than `MAX_STEP_HEIGHT` blocks movement.
- [ ] A solid obstacle blocks movement on a ramp as well as on flat terrain.
- [ ] Existing dead/crowd-control/cast movement restrictions remain unchanged.
- [ ] Movement outside the map or outside all walkable surfaces is rejected safely.

**RED**:

Add server movement tests for flat movement, ascent, descent, hill following, excessive step height, upper/lower surface ambiguity, obstacle blocking, traversal transitions, and no-surface rejection.

**GREEN**:

Integrate the shared surface query into the existing authoritative movement path with the smallest possible change. Preserve the existing `PlayerMoveTarget` and input flow.

**MUTATE / KILL MUTANTS**:

Cover mutants that omit the height check, apply the height before collision validation, use the client-provided `Y`, allow movement with no surface, choose an arbitrary overlapping surface, or compare the wrong sign of the height delta. Confirm that `MAX_STEP_HEIGHT` applies per movement step rather than to the total height of a stair or hill.

**REFACTOR**:

Avoid duplicating candidate-position logic. Keep the server system as orchestration and the geometry/movement rules in shared helpers.

**Done when**: Server movement tests pass and the server remains headless.

---

### Slice 4: Mirror height-aware movement in client prediction

**Value**: The local player follows stairs smoothly without visible rubber-banding caused by client/server height disagreement.

**Production path**:

`ActionState<Inputs>` -> predicted candidate `X/Z` -> same shared surface query -> same validation -> predicted `Position`.

**Likely files**:

- `crates/presentation/src/player_movement.rs`
- `crates/shared/src/movement.rs`
- presentation movement tests

**Acceptance criteria**:

- [ ] Predicted movement follows the same flat surfaces as the server.
- [ ] Predicted ascent and descent produce the same positions as authoritative movement for the same tick/input sequence.
- [ ] Predicted movement follows hill geometry with the same interpolated heights as the server.
- [ ] The client does not predict climbing a wall that the server rejects.
- [ ] The client does not apply a client-supplied height as authoritative state.
- [ ] Existing prediction behavior for casts, crowd control, death, and arrival remains unchanged.

**RED**:

Add a deterministic parity test that runs the same movement scenario through the shared/server and prediction paths and compares the resulting position.

**GREEN**:

Replace the client-only flat-ground assumption with the shared height-aware movement operation. Use the existing optional client world-map resource where appropriate.

**MUTATE / KILL MUTANTS**:

Cover mutants that use different step limits, skip ground resolution, update only `X/Z`, or apply height before the server-equivalent validation.

**REFACTOR**:

Remove any newly duplicated server/client formulas. The two paths should differ only where their Lightyear data sources differ.

**Done when**: Parity tests pass and local movement remains stable in host-client mode.

---

### Slice 5: Make click-to-move height-aware

**Value**: Clicking a stair selects a usable point on the stair instead of always targeting the flat `Y = 0` plane.

**Production path**:

Mouse position -> camera ray -> world surface intersection -> target `Vec3` -> existing move command/input buffering.

**Likely files**:

- `crates/presentation/src/player_movement.rs`
- `crates/presentation/src/spells/input.rs` if spell targeting should share the same ground query
- shared world query APIs

**Implementation approach**:

For the first version, the client should intersect the camera ray with authored flat/ramp surfaces. The closest valid hit to the camera should be selected deterministically. If no authored surface is hit, the existing flat-ground fallback may be retained only if it is explicitly valid for the current map.

The server must recalculate the surface height after receiving the target.

**Acceptance criteria**:

- [ ] A click on flat terrain creates a target at flat-ground height.
- [ ] A click on the lower, middle, and upper portions of a ramp creates the corresponding height.
- [ ] A click outside walkable geometry does not create an invalid target.
- [ ] Holding the mouse button continues to use the existing throttled command behavior.
- [ ] The server ignores a malicious or incorrect client-supplied `Y`.
- [ ] Existing click indicators remain visually aligned with the resolved target.

**RED**:

Add pure tests for ray/surface intersection where feasible, plus system-level tests for target generation and invalid-hit handling.

**GREEN**:

Implement only the surface intersection required for flat terrain and ramps. Do not add generalized mesh picking.

**MUTATE / KILL MUTANTS**:

Cover wrong ray direction, wrong surface selection when surfaces overlap, incorrect height interpolation, and fallback behavior outside map bounds.

**REFACTOR**:

If spell targeting needs the same ground selection, extract a shared presentation helper rather than duplicating ray intersection code. Do not introduce gameplay authority into presentation.

**Done when**: Click-to-move selects valid heights and existing targeting behavior remains intact.

---

### Slice 6: Render manifest-defined terrain and stairs

**Value**: Authored stair and height data is visible in the client and visually aligned with gameplay surfaces.

**Production path**:

Loaded `MapManifest` -> client scene/renderer -> visual terrain and prop transforms -> local meshes/materials.

**Likely files**:

- `crates/presentation/src/scenes/`
- `crates/presentation/src/renderer.rs`
- `crates/presentation/src/assets.rs`
- map asset fixtures

**Rules**:

- The client uses `TransformData` for visual placement.
- Traversal data controls gameplay surface placement.
- The server does not load or spawn these visual assets.
- A visual stair model may be scaled independently from its traversal volume, but map authoring tools should make misalignment visible.

**Acceptance criteria**:

- [ ] A manifest-defined terrain is rendered at its authored transform.
- [ ] A stair model is rendered at the authored transform and aligns with its logical traversal surface.
- [ ] The scene is not duplicated when entering or remaining in `InGame`.
- [ ] Leaving the game cleans up local visual entities as before.
- [ ] The server build remains free of presentation-only dependencies and systems.

**RED**:

Add scene/renderer tests for terrain and stair spawning, transform application, lifecycle idempotency, and cleanup.

**GREEN**:

Add the minimum client-side visual spawning needed for the first authored map.

**MUTATE / KILL MUTANTS**:

Cover missing transform scale, wrong vertical offset, duplicate spawn, missing cleanup, and accidental server registration.

**REFACTOR**:

Follow the existing presentation pattern: replicated/shared state drives local visual components, while the renderer owns meshes and materials.

**Done when**: The map displays stairs that match the movement surface and scene lifecycle tests pass.

---

### Slice 7: Add authoring, compatibility, and level-designer safeguards

**Value**: Future map edits do not silently create unwalkable or visually misaligned stairs.

**Production path**:

Editor/map authoring data -> manifest validation -> diagnostics -> saved map consumed by server and client.

**Likely files**:

- `crates/editor/src/`
- `crates/shared/src/world/loader.rs`
- `crates/shared/src/world/manifest.rs`
- map fixtures and documentation

**Safeguards**:

- validate traversal width, length, height range, and finite numeric values;
- warn when traversal data and visual transform are suspiciously misaligned where that can be checked safely;
- preserve compatibility for manifests that predate traversal data;
- document the coordinate convention and whether `start`/`end` are local or world coordinates;
- document the meaning of `TransformData.scale` separately from gameplay dimensions.

**Acceptance criteria**:

- [ ] Invalid traversal data produces actionable diagnostics.
- [ ] Old manifests remain loadable or receive an explicit version migration.
- [ ] Map documentation includes an example flat surface and stair.
- [ ] Authoring code does not require the server or rendering crates.
- [ ] A malformed map fails before gameplay starts rather than producing undefined movement behavior.

**RED / GREEN / MUTATE / REFACTOR**:

Follow the same test-first sequence with validation-focused tests and compatibility fixtures.

**Done when**: A map author can create, validate, load, and visually inspect a stair without manually editing unrelated gameplay code.

## Level Designer workflow

The intended workflow for a level designer should be explicit and repeatable.

### Step 1: Model visual content in Blender

Create the visible map in the `VISUAL` Collection:

- terrain and hills;
- houses and buildings;
- stair models;
- bridges and platforms;
- decoration and props.

Use the project world-unit convention: one Blender unit equals one game world unit. Apply object transforms where required by the exporter and keep visual topology independent from gameplay topology.

### Step 2: Create gameplay geometry

Create a simplified gameplay representation in `GAMEPLAY`:

- `WALKABLE_Ground` for flat ground;
- `WALKABLE_Hill_*` for hills or irregular terrain;
- `WALKABLE_UpperFloor` for elevated floors;
- `STAIR_*` for explicit links between surfaces.

For hills, duplicate only the walkable terrain area and simplify it. Do not use the detailed visual hill mesh as gameplay geometry. Preserve the shape features that affect movement: ridges, valleys, paths, cliff boundaries, and changes in slope.

For a floor, use a simple flat marker or low-poly plane. For stairs, create the visual stair model separately and add a gameplay ramp marker with explicit `START` and `END` points.

### Step 3: Create blocking geometry

In `COLLISION`, add simplified boxes, cylinders, or other supported blockers:

```text
BLOCKING_Wall_01
BLOCKING_Cliff_01
BLOCKING_Pillar_01
```

Do not mark every visual object as blocking. Only objects that must prevent movement should receive gameplay collision markers.

### Step 4: Assign ids and metadata

Every gameplay surface, traversal, and blocker receives a stable unique id. The id should describe gameplay identity rather than Blender object numbering:

```text
surface_ground
surface_hill_north
surface_town_upper_floor
traversal_stair_town_upper
blocker_town_wall_01
```

Configure the required custom properties and verify that each stair references existing lower and upper surface ids.

### Step 5: Run the Blender exporter

Run the project export command or Blender Python script. The exporter should:

1. validate Collections and object types;
2. apply/resolve world transforms;
3. extract walkable meshes and flat surface markers;
4. extract stairs and their endpoints;
5. extract simplified blockers;
6. validate slopes, degenerate triangles, ids, and references;
7. write the `.glb` visual asset;
8. write the `.world.json` gameplay manifest;
9. print counts and warnings.

The export must fail if a gameplay error exists. A designer should never receive a seemingly successful build with an invalid surface manifest.

### Step 6: Inspect debug geometry

Launch the client with gameplay debug visualization enabled. Check:

- the walkable hill covers exactly the intended area;
- the player can reach the top and bottom of every stair;
- the upper floor is at the same height as the visual floor;
- cliffs and walls are blocked where expected;
- no decorative mesh is accidentally walkable;
- no walkable mesh extends through a wall or outside the map;
- surface normals and slope warnings are correct.

### Step 7: Test movement in-game

Use a standard checklist:

- start on flat ground;
- walk up the hill;
- walk down the hill;
- cross the transition between flat ground and hill;
- walk up and down every stair;
- attempt to walk up a cliff;
- attempt to cross a blocker;
- approach the edge of an upper floor;
- click on lower, middle, and upper points of a stair;
- click on several points of a hill;
- test the same map in client mode and host-client mode.

### Step 8: Commit the map artifacts together

The `.glb`, `.world.json`, Blender source file, and any exporter configuration must be versioned as one map change. A visual-only change and a gameplay-geometry change should be reviewable independently when possible, but the final map state must keep the artifacts synchronized.

### Level Designer definition of done

A map is ready for gameplay review when:

- the exporter completes without errors;
- the manifest is versioned with the `.glb`;
- all walkable surfaces have stable ids;
- all hills use simplified gameplay geometry;
- all stairs reference valid surfaces;
- collision markers are intentional and simplified;
- debug geometry visually matches the intended map;
- the player can traverse every intended route;
- unintended shortcuts and inaccessible areas are documented or fixed.

## Suggested shared APIs

The exact names should follow existing conventions, but the design should converge toward APIs with responsibilities similar to these:

```rust
pub struct GroundContact {
    pub height: f32,
    pub normal: Vec3Like,
}

pub trait SurfaceQuery {
    fn ground_at(&self, x: f32, z: f32) -> Option<GroundContact>;
}

pub fn resolve_walkable_position(
    current: Position,
    desired: Position,
    surface: &impl SurfaceQuery,
    collision: &CollisionGrid,
) -> Option<Position>;
```

The final API should remain compatible with the crate boundary rule: shared world and movement code must not depend on server, client, presentation, sockets, or rendering.

## Testing strategy

### Pure shared tests

Prefer unit tests for:

- manifest validation;
- ramp interpolation;
- surface containment;
- normal calculation;
- ground-height lookup;
- step-height validation;
- solid collision behavior;
- deterministic overlapping-surface selection.
### Server tests

Cover:

- authoritative movement on flat terrain;
- ascending and descending stairs;
- blocking excessive vertical changes;
- rejecting invalid/no-surface targets;
- ignoring client-provided
 height;
- preserving existing cast, crowd-control, death, and movement-state rules.

### Client/presentation tests

Cover:

- client/server movement parity;
- click-to-surface target generation;
- click indicators at resolved height;
- scene spawning and cleanup;
- no rendering systems in server mode.

### Validation commands

Run from `/Users/tacosalfornoh/Coding/Rust/BevyMMO`:

```sh
cargo test
cargo clippy -- -D warnings
```

For the server-specific footprint, also verify the documented production server build once the implementation is complete:

```sh
cargo build --release --no-default-features --features server,netcode,udp,replication --bin game
```

## Risks and mitigations

### Client/server divergence

**Risk**: The server and client calculate different heights or use different boundary rules.

**Mitigation**: Keep surface and movement resolution in `shared`; add parity tests before integrating visual polish.

### Visual/gameplay misalignment

**Risk**: A stair model appears walkable but its logical traversal volume is offset.

**Mitigation**: Store gameplay dimensions separately, add authoring validation, and render debug traversal volumes during development.

### Ambiguous overlapping surfaces

**Risk**: Multiple floors share the same `X/Z` range and the query returns a non-deterministic surface.

**Mitigation**: Define explicit surface precedence, sort or index deterministically, and test multi-level cases even if multi-level navigation is not initially supported.

### Unbounded scope

**Risk**: Stairs expand into physics, ladders, jumping, and full 3D navigation.

**Mitigation**: Ship ramps first. Keep ladders and dynamic traversal as separate future stories.

### Backward compatibility

**Risk**: Existing maps lack traversal fields or use the old flat-ground assumptions.

**Mitigation**: Use defaults or a manifest version migration and retain flat terrain behavior as the compatibility path.

## Open product decisions

These decisions should be confirmed before Slice 1 implementation:

1. Should ordinary stairs be automatic, or should the player press an interaction key to enter them?
2. Is the first release limited to straight stairs/ramps?
3. Will the game support jumping later, and should `MAX_STEP_HEIGHT` remain separate from jump height?
4. What player radius and maximum step height should be used for the first playable map?
5. Are multiple floors with overlapping `X/Z` ranges required in the first release?
6. Should spell targeting reuse the same walkable-surface intersection as movement, or remain restricted to the ground plane initially?
7. Should the editor display a debug traversal volume and surface normal while authoring?

## Quality gate

Before merging each slice:

1. Acceptance criteria are reviewed and approved before implementation.
2. The behavior test is written first and fails for the intended reason.
3. Minimum production code is implemented.
4. Mutation testing or equivalent targeted test-strength review checks likely surviving mutants.
5. Refactoring is limited to changes that improve the implemented slice.
6. `cargo test` passes.
7. `cargo clippy -- -D warnings` passes.
8. No unrelated code or formatting is changed.
9. No commit is created without explicit user approval.

## Completion criteria

The plan is complete when all slices are implemented, all acceptance criteria pass, the production server build succeeds, the first map contains a working stair example, and the plan is removed from `plans/` after the feature is accepted.

