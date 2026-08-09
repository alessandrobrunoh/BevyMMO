## Goal

Camera occlusion handling for the isometric MMO camera. Complex props (trees,
houses, …) are authored as two separate glTF nodes:

1. **Base** (`<Name>_Base`) — always rendered.
2. **Top** (`<Name>_Top`) — hidden when it blocks the line of sight between the
   game camera and the locally controlled player.

This matches the Albion Online / Diablo convention: no alpha sorting artifacts,
no overdraw cost, the trunk/walls stay visible while the canopy/roof vanishes.

## Asset Convention

The authoring rule lives in the `.glb`: any node whose `Name` ends with the
suffix `_Top` is treated as an occludable canopy/roof. The corresponding `_Base`
node is rendered unconditionally.

`assets/models/yggdrasil.glb` already conforms:

```
Yggdrasil (parent)
├── Yggdrasil_Base   (mesh 0)
└── Yggdrasil_Top    (mesh 1)
```

No editor step is required: when Bevy instantiates the scene, every glTF node
becomes an entity that keeps its `Name`, and the `tag_occludable_tops` system
promotes `_Top` nodes to `OccludableTop` automatically.

## Decision: Visibility Toggle (no alpha fade)

Two strategies were considered:

| Approach                       | Cost per frame                          | Visual       |
| ------------------------------ | --------------------------------------- | ------------ |
| `Visibility::Hidden` toggle    | Trivial — no asset mutation             | Hard cut     |
| `StandardMaterial` alpha blend | `ResMut<Assets<...>>` per occluder/frame | Soft fade    |

For an MMO with hundreds of props the alpha-blend path thrashes the material
asset registry and breaks depth sorting. We ship the **toggle** path as the
default. A future `OccludableFade` marker can opt specific hero assets
(boss arenas, special buildings) into the fade path without touching the
default.

## Architecture

Crate: `bevymmo_presentation` — pure client-side effect, runs only when
`GameScreen::InGame | Paused` and a `Controlled` player exists.

```
crates/presentation/src/scenes/base/
├── mod.rs        (registers the new systems)
├── occlusion.rs  (NEW — component, tagging, occlusion solver)
└── systems.rs    (existing camera/lifecycle systems)
```

### Component

```rust
#[derive(Component, Reflect, Default)]
pub struct OccludableTop;
```

A pure marker. No `radius` field — the radius is derived at runtime from
Bevy's `Aabb` (see [Radius auto-detection](#radius-auto-detection)).

### Systems

1. `tag_occludable_tops` — runs on `Added<Name>`. When a freshly instantiated
   scene node ends with `_Top`, it inserts `OccludableTop::default()` and
   `Visibility::default()`. Cheap and idempotent.
2. `update_camera_occlusion` — runs every `Update` frame. For each occluder:
   - Reads the world-space radius via [`occluder_world_radius`]
     (Aabb-derived, see below).
   - Rejects nodes whose distance from the camera is greater than
     `cam→player + radius` (early-exit cull, ~80% rejection).
   - Projects the occluder center onto the `camera → player` ray.
   - Hides it when the projection falls between camera and player AND the
     perpendicular distance is below `radius`, **or** when the player itself
     is within `radius` (standing under the canopy).
   - Otherwise restores `Visibility::Inherited`.

### Radius auto-detection

The occlusion test needs the world-space radius of each canopy/roof, but the
MMO ships props of very different sizes (a `tree_pine_small` is ~1.5 m, the
`yggdrasil` is tens of meters) and the artist cannot eyeball the in-game
radius while authoring in Blender. Hard-coding per kind (either in the
`#[props]` macro or as a default `f32` field) would couple the visual
structure to the catalog and require re-compilation for every tuning pass.

The radius is instead derived from Bevy's `Aabb` — the axis-aligned bounding
box that `bevy_render` automatically attaches to every entity with a `Mesh3d`.
The mapping is the **bounding-sphere radius** in world space:

```text
world_radius = (aabb.half_extents * global_transform.scale).length()
```

- **Pros**: zero configuration, works for any prop size, automatically
  accounts for the per-placement scale applied by the manifest and by the
  `#[props]` macro's `scale = (...)` attribute.
- **Fallback**: between scene instantiation and the first frame the asset is
  ready, `Aabb` is `None`. In that brief window the system falls back to
  [`DEFAULT_OCCLUDER_RADIUS`] (conservative, 2.0 m) to avoid spurious
  hide/show flicker while assets stream in.
- **Future override**: should an artist want a tighter or looser cutoff than
  the bounding sphere suggests, the natural place is the existing
  `extras.bevymmo` payload on the glTF node — same channel already used for
  `kind`/`collision`. Adding it later does not change the current contract.

### Edge Cases

- No `Controlled` player yet (login/connecting) → system early-returns.
- No `GameCamera` → early-return.
- Player standing under the canopy → still hidden (canonical behavior).
- Multiple `_Top` siblings under the same parent → each handled independently.

## Verification

### Automated Tests (`occlusion.rs`)

- Occluder on the camera→player segment is hidden.
- Occluder far off-axis stays visible.
- Occluder behind the player (w.r.t. the camera) stays visible.
- Player inside the occluder's radius hides it.
- `occlusion` no-ops when there is no `Controlled` player.
- `tag_occludable_tops` only tags `_Top`-suffixed names.
- `occluder_world_radius` returns the bounding-sphere radius scaled by the
  entity's global transform scale, and falls back to
  `DEFAULT_OCCLUDER_RADIUS` when `Aabb` is missing.

### Manual Verification

1. Place `yggdrasil.glb` in a test map.
2. Walk the controlled player behind the canopy (from the camera's POV).
3. Confirm the canopy (`Yggdrasil_Top`) disappears while the trunk
   (`Yggdrasil_Base`) stays fully opaque.
4. Walk under the canopy — confirm it also hides (so the character is visible).
5. Move away — confirm the canopy snaps back.
