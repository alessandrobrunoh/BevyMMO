# BevyMMO World Exporter — Blender add-on
#
# Generates the `<map_id>.world.json` sidecar consumed by the engine's
# `load_world_json` loader (see crates/shared/src/world/loader.rs).
#
# The add-on walks the current scene, groups objects by their naming prefix
# (see docs/level-designer-guide.md §3) and serialises:
#   - `__bevymmo_map_meta`  -> top-level map metadata (map_id, bounds, ...)
#   - `WALKABLE_*`          -> surfaces with `walkable_mesh` + raycast heightfield
#   - `BLOCKING_*`          -> blockers with world-space transform + AABB shape
#   - `TRAVERSAL_*`         -> traversals linking two surfaces (data + helper points)
#   - objects carrying `bevymmo_kind` (or prefixed `PLACEABLE_`) -> manifest `props`
#     entries that the server dispatches into props / enemies / bosses /
#     NPCs / player spawn markers via the placeable catalog.
#
# Install: Edit > Preferences > Add-ons > Install... > pick this file.
# Usage:   File > Export > BevyMMO World (.world.json)
#          or  N-panel > BevyMMO > Export .world.json

from __future__ import annotations

bl_info = {
    "name": "BevyMMO World Exporter",
    "author": "Alessandro Bruno",
    "version": (1, 0, 0),
    "blender": (3, 6, 0),
    "location": "File > Export > BevyMMO World (.world.json) / N-panel > BevyMMO",
    "description": "Author the .world.json sidecar for a BevyMMO map directly from the scene.",
    "category": "Export",
}

import json
import math
import os
from dataclasses import dataclass, field
from typing import Optional

import bpy
from bpy.props import BoolProperty, IntProperty, StringProperty
from bpy_extras.io_utils import ExportHelper
from mathutils import Matrix, Vector

# Change of basis from Blender's Z-up to the engine's Y-up:
# (x, y, z) -> (x, z, -y). Identical to what Blender's own glTF exporter
# applies with "+Y Up" enabled, so the JSON and the GLB agree.
BLENDER_TO_ENGINE = Matrix(
    (
        (1.0, 0.0, 0.0, 0.0),
        (0.0, 0.0, 1.0, 0.0),
        (0.0, -1.0, 0.0, 0.0),
        (0.0, 0.0, 0.0, 1.0),
    )
)
BLENDER_TO_ENGINE_INV = BLENDER_TO_ENGINE.inverted()

# The engine estimates terrain slope with a fixed 1 m stencil
# (NORMAL_SAMPLE_OFFSET_M in crates/shared/src/world/manifest.rs). A
# heightfield coarser than that cannot describe anything the slope test can
# see, so the exporter warns above this cell size.
MAX_RECOMMENDED_CELL_M = 1.0

# ---------------------------------------------------------------------------
# Conventions — keep these in sync with docs/level-designer-guide.md and
# crates/shared/src/world/loader.rs (WALKABLE_NODE_PREFIX, WALKABLE_KIND_TAG).
# ---------------------------------------------------------------------------

META_NODE_NAME_PREFIX = "__bevymmo_map_meta"
# Doc reminder: keep in sync with crates/shared/src/world/loader.rs.
WALKABLE_PREFIX = "WALKABLE_"
BLOCKING_PREFIX = "BLOCKING_"
TRAVERSAL_PREFIX = "TRAVERSAL_"
PLACEABLE_PREFIX = "PLACEABLE_"
RESOURCE_PREFIX = "RESOURCE_"

# Custom-property key that tags an object as a placeable marker.
# Recognised values are the catalog KindIds authored in
# crates/shared/src/placeables_impl/ (e.g. "player_spawn", "mob_goblin",
# "boss_dragon", "npc_merchant", "prop_tree", ...).
KIND_PROP_KEY = "bevymmo_kind"
ID_PROP_KEY = "bevymmo_id"
TINT_PROP_KEY = "bevymmo_tint"
BLOCKS_MOVE_PROP_KEY = "bevymmo_blocks_move"

DEFAULT_WORLD_METRICS = {
    "player_radius": 0.35,
    "player_height": 1.7,
    "eye_height": 1.6,
    "max_step_height": 0.45,
    "max_walkable_slope_deg": 45.0,
}

MANIFEST_VERSION = 2


# ---------------------------------------------------------------------------
# Error reporting — collected per-export so the operator can show a single
# toast with everything the level designer needs to fix.
# ---------------------------------------------------------------------------


@dataclass
class ExportReport:
    warnings: list[str] = field(default_factory=list)
    errors: list[str] = field(default_factory=list)

    def warn(self, msg: str) -> None:
        self.warnings.append(msg)

    def error(self, msg: str) -> None:
        self.errors.append(msg)

    @property
    def ok(self) -> bool:
        return not self.errors


# ---------------------------------------------------------------------------
# Custom property helpers — Blender stores them as IDPropertyArray which is
# awkward to serialise. We coerce to plain Python primitives here so the
# JSON writer always sees native int/float/str/bool.
# ---------------------------------------------------------------------------


def _as_plain(value):
    """Coerces a Blender custom-property value into a JSON-friendly primitive."""
    if isinstance(value, (int, float, str, bool)):
        return value
    if hasattr(value, "__iter__"):
        # IDPropertyArray or Vector
        try:
            return [_as_plain(v) for v in value]
        except TypeError:
            return str(value)
    return str(value)


def custom_prop(obj, key: str, default=None):
    """Reads a single custom property from a Blender object."""
    if key in obj:
        return _as_plain(obj[key])
    return default


def custom_prop_float(obj, key: str, default: float) -> float:
    raw = custom_prop(obj, key, default)
    try:
        return float(raw)
    except (TypeError, ValueError):
        return default


def custom_prop_str(obj, key: str, default: str) -> str:
    raw = custom_prop(obj, key, default)
    if raw is None:
        return default
    return str(raw)


def custom_prop_int(obj, key: str, default: int) -> int:
    raw = custom_prop(obj, key, default)
    try:
        return int(float(raw))
    except (TypeError, ValueError):
        return default


# ---------------------------------------------------------------------------
# Meta node — must exist exactly once. We tolerate the trailing underscores
# variant the loader accepts too (see level-designer-guide.md §4).
# ---------------------------------------------------------------------------


def find_meta_node() -> Optional[bpy.types.Object]:
    for obj in bpy.context.scene.objects:
        if obj.name.startswith(META_NODE_NAME_PREFIX):
            return obj
    return None


def collect_map_bounds(meta: bpy.types.Object, report: ExportReport) -> dict:
    bounds = {
        "min_x": custom_prop_float(meta, "bevymmo_min_x", -20.0),
        "max_x": custom_prop_float(meta, "bevymmo_max_x", 20.0),
        "min_z": custom_prop_float(meta, "bevymmo_min_z", -20.0),
        "max_z": custom_prop_float(meta, "bevymmo_max_z", 20.0),
    }
    if bounds["min_x"] >= bounds["max_x"] or bounds["min_z"] >= bounds["max_z"]:
        report.error(
            f"Map bounds are inverted: {bounds}. "
            "Edit bevymmo_min_x/max_x/min_z/max_z on __bevymmo_map_meta."
        )
    return bounds


def collect_map_id(
    meta: bpy.types.Object, fallback_blend_path: str, report: ExportReport
) -> str:
    map_id = custom_prop_str(meta, "bevymmo_map_id", "")
    if not map_id:
        # Fall back to the .blend filename so the export never silently
        # produces an "untitled" id that breaks DB keys downstream.
        map_id = os.path.splitext(os.path.basename(fallback_blend_path))[0]
        report.warn(f"meta node has no bevymmo_map_id; falling back to '{map_id}'")
    return map_id


def collect_display_name(meta: bpy.types.Object, fallback: str) -> str:
    return custom_prop_str(
        meta, "bevymmo_display_name", fallback.replace("_", " ").title()
    )


# ---------------------------------------------------------------------------
# Walkable surfaces — mesh + heightfield extraction.
#
# Blender uses Z-up; the engine uses Y-up. Every coordinate that crosses the
# export boundary is converted via `_to_engine` so the engine receives
# vertex/heightfield/transform data it can consume verbatim as
# WalkableMeshData without further axis swaps.
#
# The heightfield is produced by raycasting straight down (-Z in Blender) on
# a regular grid covering the mesh's XY footprint. We raycast on the surface
# mesh itself (not the whole scene) so neighbouring geometry cannot pollute
# the samples.
# ---------------------------------------------------------------------------


def _to_engine(v) -> list[float]:
    """Converts a Blender Z-up position to an engine Y-up position.

    Mapping: blender (x, y, z) -> engine (x, z, **-y**).

    The negated Z is not cosmetic: it is the exact conversion Blender's own
    glTF exporter applies when "+Y Up" is enabled (the default). Dropping the
    minus sign mirrors the whole map north/south relative to the rendered
    `.glb`, so collision, spawns and props end up on the opposite side of the
    terrain they were authored on.
    """
    return [v[0], v[2], -v[1]]


def _to_engine_vec(v) -> Vector:
    """Same as `_to_engine` but returns a mathutils.Vector for raycast math."""
    return Vector((v[0], v[2], -v[1]))


def _engine_z_to_blender_y(z: float) -> float:
    """Inverse of the Z mapping, for driving Blender-space raycasts from an
    engine-space grid."""
    return -z


def _world_triangles(
    obj: bpy.types.Object, depsgraph
) -> tuple[list[list[float]], list[int]]:
    """Returns (vertices, indices) of the mesh in engine Y-up world space.

    Triangles are flattened as `walkable_mesh` expects: vertices as a flat
    list of `[x, y, z]` (Y-up) and indices as a flat list of u32s in groups
    of 3.
    """
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    try:
        matrix = evaluated.matrix_world
        vertices = [_to_engine(matrix @ v.co) for v in mesh.vertices]
        indices: list[int] = []
        for poly in mesh.polygons:
            if len(poly.vertices) < 3:
                continue
            # Fan-triangulate n-gons into triangles around the first vertex.
            for i in range(1, len(poly.vertices) - 1):
                indices.extend(
                    (
                        int(poly.vertices[0]),
                        int(poly.vertices[i]),
                        int(poly.vertices[i + 1]),
                    )
                )
        return vertices, indices
    finally:
        evaluated.to_mesh_clear()


def _world_xz_bounds(vertices: list[list[float]]) -> dict:
    """Computes the engine XZ ground-plane bounds from converted vertices.

    Vertices are already in engine Y-up (see `_to_engine`), so X = v[0] and
    Z = v[2] are the horizontal axes.
    """
    xs = [v[0] for v in vertices]
    zs = [v[2] for v in vertices]
    return {
        "min_x": min(xs),
        "max_x": max(xs),
        "min_z": min(zs),
        "max_z": max(zs),
    }


def _build_heightfield(
    obj: bpy.types.Object,
    bounds: dict,
    resolution: int,
    raycast_above: float,
    report: ExportReport,
) -> dict:
    """Raycasts straight down (-Z in Blender) on a (resolution+1)² grid.

    `bounds` are in **engine** XZ; engine X is Blender X and engine Z is
    Blender -Y (see `_to_engine`). We raycast from above in Blender Z and
    store the hit Z as the engine Y height. Cells that miss are interpolated
    from their neighbours by `_fill_missing_heights`.
    """
    side = resolution + 1
    dx = (bounds["max_x"] - bounds["min_x"]) / resolution
    dz = (bounds["max_z"] - bounds["min_z"]) / resolution
    if dx <= 0 or dz <= 0:
        report.error(
            f"WALKABLE_{obj.name} has zero-area bounds; cannot sample heightfield"
        )
        return {"resolution": resolution, "bounds": bounds, "heights": []}

    # Raycast against the evaluated object so modifiers (subdivision,
    # displacement, ...) are taken into account, exactly like the GLB the
    # player sees.
    depsgraph = bpy.context.evaluated_depsgraph_get()
    target = obj.evaluated_get(depsgraph)

    cell_m = max(dx, dz)
    if cell_m > MAX_RECOMMENDED_CELL_M:
        report.warn(
            f"WALKABLE_{obj.name}: heightfield cell is {cell_m:.2f} m "
            f"(resolution={resolution} over {bounds['max_x'] - bounds['min_x']:.0f} m). "
            f"The engine estimates slopes with a {MAX_RECOMMENDED_CELL_M:.1f} m stencil, so terrain "
            "detail narrower than one cell (paths, switchbacks, ledges) is lost and "
            "movement will disagree with what the player sees. Raise the resolution "
            f"to at least {math.ceil((bounds['max_x'] - bounds['min_x']) / MAX_RECOMMENDED_CELL_M)}."
        )

    matrix = target.matrix_world
    matrix_inv = matrix.inverted()
    # Direction must be transformed without translation, hence the 3x3 part.
    down_local = (matrix_inv.to_3x3() @ Vector((0.0, 0.0, -1.0))).normalized()

    heights: list[Optional[float]] = [None] * (side * side)
    misses = 0

    for gz in range(side):
        # Engine Z maps to Blender -Y on the ground plane.
        b_y = _engine_z_to_blender_y(bounds["min_z"] + dz * gz)
        for gx in range(side):
            b_x = bounds["min_x"] + dx * gx
            # Raycast straight down in Blender Z from above the mesh.
            #
            # This casts against `obj` alone, not the scene: `scene.ray_cast`
            # returns whichever object is hit first, so every tree, rock or
            # prop standing on the terrain used to shadow the ground below it
            # and the sample was discarded. On a decorated map that punched
            # hundreds of full-depth pits into the collision surface.
            origin_local = matrix_inv @ Vector((b_x, b_y, raycast_above))
            hit, location, _normal, _index = target.ray_cast(origin_local, down_local)
            if not hit:
                misses += 1
                continue
            # Blender Z is the height -> store as engine Y.
            height_z = (matrix @ location).z
            # Match the Rust sampler's layout: index = x * stride + z,
            # where stride = resolution + 1 and z varies fastest. The
            # previous `gx + gz * side` form silently transposed the
            # heightfield on asymmetric maps, making slopes appear along
            # the wrong axis.
            heights[gx * side + gz] = height_z

    if misses:
        report.warn(
            f"WALKABLE_{obj.name}: {misses}/{side * side} heightfield samples missed the mesh "
            "(holes in the surface, or the grid extends past the geometry); "
            "filled from neighbouring samples."
        )
    final_heights = _fill_missing_heights(heights, side, report, obj.name)
    return {"resolution": resolution, "bounds": bounds, "heights": final_heights}


def _fill_missing_heights(
    heights: list[Optional[float]], side: int, report: ExportReport, obj_name: str
) -> list[float]:
    """Fills `None` samples by flood-filling from their nearest valid neighbours.

    The previous behaviour — substituting the surface's global minimum — turned
    every missed sample into a full-depth hole. A cell missed on a 40 m summit
    became a 40 m shaft the player fell into; a cell missed on a slope became a
    vertical wall the player could not walk past. Averaging the known
    neighbours keeps the surface continuous, which is what movement needs.
    """
    filled = list(heights)
    if all(h is None for h in filled):
        report.warn(f"WALKABLE_{obj_name}: no ray hit any cell — heightfield is flat 0")
        return [0.0] * (side * side)

    while True:
        pending = [i for i, h in enumerate(filled) if h is None]
        if not pending:
            break
        progressed = False
        for i in pending:
            gx, gz = divmod(i, side)
            neighbours = []
            for ox, oz in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                nx, nz = gx + ox, gz + oz
                if 0 <= nx < side and 0 <= nz < side:
                    value = filled[nx * side + nz]
                    if value is not None:
                        neighbours.append(value)
            if neighbours:
                filled[i] = sum(neighbours) / len(neighbours)
                progressed = True
        if not progressed:
            break

    return [h if h is not None else 0.0 for h in filled]


def collect_surfaces(resolution: int, report: ExportReport) -> list[dict]:
    surfaces: list[dict] = []
    depsgraph = bpy.context.evaluated_depsgraph_get()

    for obj in bpy.context.scene.objects:
        if not obj.name.startswith(WALKABLE_PREFIX):
            continue
        if obj.type != "MESH":
            report.warn(f"WALKABLE_{obj.name} is not a MESH (type={obj.type}); skipped")
            continue

        vertices, indices = _world_triangles(obj, depsgraph)
        if not vertices or len(indices) < 3:
            report.warn(f"WALKABLE_{obj.name} has no triangles; skipped")
            continue

        bounds = _world_xz_bounds(vertices)
        # Vertices are engine Y-up: v[1] = height (Blender Z).
        # raycast_above needs to be above the mesh in Blender Z, which equals
        # the max engine Y, so max_y + 10 is correct.
        max_y = max(v[1] for v in vertices)
        min_y = min(v[1] for v in vertices)
        raycast_above = max_y + 10.0

        heightfield = _build_heightfield(obj, bounds, resolution, raycast_above, report)

        surface = {
            "id": obj.name,
            "kind": "mesh",
            "object": obj.name,
            "bounds": bounds,
            "min_height": min_y,
            "max_height": max_y,
            "heightfield": heightfield,
            "walkable_mesh": {"vertices": vertices, "indices": indices},
            # Authors can override per-surface via bevymmo_max_slope_deg.
            "max_slope_deg": custom_prop(obj, "bevymmo_max_slope_deg"),
        }
        # Strip null entries so the JSON stays compact and matches the
        # serde(default) Rust side, which tolerates their absence.
        surface = {k: v for k, v in surface.items() if v is not None}
        surfaces.append(surface)

    return surfaces


# ---------------------------------------------------------------------------
# Blockers — read the world transform and shape. Authors can either bake
# the shape into a MESH (we read its AABB) or declare it via custom props
# (bevymmo_kind + bevymmo_radius/half_extents/height).
# ---------------------------------------------------------------------------


def _shape_from_props(obj: bpy.types.Object) -> Optional[dict]:
    """Builds a `CollisionShape` payload for a blocker from its custom props.

    The Rust `CollisionShape` is an externally tagged serde enum, so the only
    accepted encoding is a single-key map naming the variant — the same one
    `_collision_shape_from_props` already emits for props. The previous
    `{"type": "box", ...}` form was not a shape serde could read at all: it
    made the whole `.world.json` fail to deserialize, so any map that declared
    even one blocker refused to load.
    """
    kind = custom_prop_str(obj, "bevymmo_blocker_kind", "")
    if not kind:
        kind = custom_prop_str(obj, "bevymmo_kind", "box")
    if kind == "box":
        half_extents = custom_prop(obj, "bevymmo_half_extents")
        if half_extents is None:
            return None
        extents = _as_vec3(half_extents)
        if extents is None:
            return None
        # Half-extents are sizes, not positions: the axis swap reorders them
        # but the sign flip on Z must not leak through.
        return {"Box": {"half_extents": [abs(e) for e in extents]}}
    if kind == "cylinder":
        radius = custom_prop_float(obj, "bevymmo_radius", 0.0)
        height = custom_prop_float(obj, "bevymmo_height", 0.0)
        return {"Cylinder": {"radius": radius, "height": height}}
    if kind == "sphere":
        radius = custom_prop_float(obj, "bevymmo_radius", 0.0)
        return {"Sphere": {"radius": radius}}
    return None


def _as_vec3(raw, to_engine: bool = True):
    """Parses a 3-float array/string from a custom property.

    When `to_engine` is True (default), applies the Z-up -> Y-up swap so
    values authored in Blender's native orientation reach the engine in
    its own coordinate system.
    """
    parsed = None
    if isinstance(raw, (list, tuple)) and len(raw) >= 3:
        parsed = [float(raw[0]), float(raw[1]), float(raw[2])]
    elif isinstance(raw, str):
        parts = [s.strip() for s in raw.split(",") if s.strip()]
        if len(parts) >= 3:
            try:
                parsed = [float(parts[0]), float(parts[1]), float(parts[2])]
            except ValueError:
                return None
    if parsed is None:
        return None
    return _to_engine(parsed) if to_engine else parsed


def _transform_to_dict(obj: bpy.types.Object) -> dict:
    # Rebase the whole matrix into engine space instead of permuting the
    # decomposed euler by hand: with the negated Z (see `_to_engine`) a
    # component swap no longer describes the same rotation, and props would
    # come out mirrored even when their position was right.
    engine_matrix = BLENDER_TO_ENGINE @ obj.matrix_world @ BLENDER_TO_ENGINE_INV
    loc, rot, scale = engine_matrix.decompose()
    engine_loc = [loc.x, loc.y, loc.z]
    # Euler YXZ in degrees — matches TransformData::rotation_deg, which the
    # renderer reads back as Quat::from_euler(YXZ, rot[1], rot[0], rot[2]).
    euler = rot.to_euler("YXZ")
    engine_rot = [math.degrees(euler.x), math.degrees(euler.y), math.degrees(euler.z)]
    engine_scale = [abs(scale.x), abs(scale.y), abs(scale.z)]
    return {
        "translation": engine_loc,
        "rotation_deg": engine_rot,
        "scale": engine_scale,
    }


def collect_blockers(report: ExportReport) -> list[dict]:
    blockers: list[dict] = []

    for obj in bpy.context.scene.objects:
        if not obj.name.startswith(BLOCKING_PREFIX):
            continue

        shape = _shape_from_props(obj)
        if shape is None:
            report.warn(
                f"BLOCKING_{obj.name} has no shape (need bevymmo_blocker_kind + dims); skipped"
            )
            continue

        # `shape` is a single-key map naming the serde variant ("Box",
        # "Cylinder", "Sphere"); `BlockerKind` accepts the lowercase spelling.
        kind = next(iter(shape)).lower()
        if kind not in ("box", "cylinder"):
            # `BlockerKind` has no Sphere variant, so emitting one would make
            # the whole manifest fail to load rather than just this blocker.
            report.warn(
                f"BLOCKING_{obj.name} uses shape {kind!r}, which blockers do not "
                "support (box or cylinder only); skipped"
            )
            continue
        blocker = {
            "id": obj.name,
            "kind": kind,
            "object": obj.name,
            "transform": _transform_to_dict(obj),
            "shape": shape,
            "blocks_movement": True,
        }
        blockers.append(blocker)

    return blockers


# ---------------------------------------------------------------------------
# Traversals — data-only. Authors can either:
#  - drop a TRAVERSAL_* empty with bevymmo_* props, or
#  - use a curve with start/end points; we sample the endpoints and width.
# Here we go with the simpler custom-property recipe so the artist stays
# in control of the exact start/end coordinates.
# ---------------------------------------------------------------------------


def collect_traversals(report: ExportReport) -> list[dict]:
    traversals: list[dict] = []

    for obj in bpy.context.scene.objects:
        if not obj.name.startswith(TRAVERSAL_PREFIX):
            continue

        kind = custom_prop_str(obj, "bevymmo_traversal_kind", "stairs")
        start_surface = custom_prop_str(obj, "bevymmo_start_surface", "")
        end_surface = custom_prop_str(obj, "bevymmo_end_surface", "")
        width = custom_prop_float(obj, "bevymmo_width", 1.0)

        # Endpoints: prefer bevymmo_start/end arrays; fall back to a second
        # linked Empty named `<obj>_start` / `<obj>_end` if present.
        start = _as_vec3(custom_prop(obj, "bevymmo_start"))
        end = _as_vec3(custom_prop(obj, "bevymmo_end"))
        if start is None and obj.name + "_start" in bpy.data.objects:
            start = _to_engine(
                bpy.data.objects[obj.name + "_start"].matrix_world.translation
            )
        if end is None and obj.name + "_end" in bpy.data.objects:
            end = _to_engine(
                bpy.data.objects[obj.name + "_end"].matrix_world.translation
            )

        if start is None or end is None:
            report.warn(
                f"TRAVERSAL_{obj.name} missing start/end (set bevymmo_start / bevymmo_end); skipped"
            )
            continue
        if width <= 0:
            report.warn(f"TRAVERSAL_{obj.name} width <= 0; skipped")
            continue

        entry = {
            "id": obj.name,
            "kind": kind,
            "start": list(start),
            "end": list(end),
            "width": width,
        }
        if start_surface:
            entry["start_surface"] = start_surface
        if end_surface:
            entry["end_surface"] = end_surface
        traversals.append(entry)

    return traversals


# ---------------------------------------------------------------------------
# Placeables (props + spawn markers)
# ---------------------------------------------------------------------------

# Object name prefixes that are already consumed by the dedicated collectors
# above; we skip them here so a placeable never gets duplicated.
_RESERVED_PREFIXES = (
    META_NODE_NAME_PREFIX,
    WALKABLE_PREFIX,
    BLOCKING_PREFIX,
    TRAVERSAL_PREFIX,
)


def _collision_shape_from_props(obj: bpy.types.Object) -> Optional[dict]:
    """Builds a serialised CollisionShape from bevymmo_collision* custom props.

    Shape names follow `CollisionShape` variants in
    crates/shared/src/world/shapes.rs. Returns ``None`` when no shape is
    declared, leaving the prop passable for movement.
    """
    kind = custom_prop_str(obj, "bevymmo_collision", "").strip().lower()
    if not kind or kind == "none":
        return None

    if kind == "sphere":
        radius = custom_prop_float(obj, "bevymmo_radius", 0.5)
        return {"Sphere": {"radius": radius}}
    if kind == "cylinder":
        radius = custom_prop_float(obj, "bevymmo_radius", 0.5)
        height = custom_prop_float(obj, "bevymmo_height", 2.0)
        return {"Cylinder": {"radius": radius, "height": height}}
    if kind == "box":
        raw = custom_prop(obj, "bevymmo_half_extents")
        if raw is None:
            half_extents = [0.5, 0.5, 0.5]
        elif isinstance(raw, (list, tuple)):
            half_extents = list(raw)
            if len(half_extents) < 3:
                half_extents += [0.5] * (3 - len(half_extents))
        else:
            # Comma-separated string or scalar
            try:
                parts = [float(p.strip()) for p in str(raw).split(",")]
            except ValueError:
                parts = [0.5, 0.5, 0.5]
            half_extents = (parts + [0.5, 0.5, 0.5])[:3]
        return {"Box": {"half_extents": half_extents[:3]}}

    return None


def collect_props(report: ExportReport) -> list[dict]:
    """Collects authored placeable entries (props + spawn markers).

    An object is treated as a placeable when it carries the `bevymmo_kind`
    custom property OR its name starts with the ``PLACEABLE_`` prefix. This
    covers visual props (e.g. ``prop_tree``), AI spawns (``mob_goblin``,
    ``boss_dragon``) and the invisible player spawn marker
    (``player_spawn``) — the engine dispatches by category in
    ``crates/server/src/placeables/creatures.rs::spawn_placeables_on_map_load``.

    Reserved nodes (meta / walkable / blocking / traversal) are skipped to
    avoid double-counting.
    """
    props: list[dict] = []

    for obj in bpy.context.scene.objects:
        if obj.name.startswith(_RESERVED_PREFIXES):
            continue

        kind = custom_prop_str(obj, KIND_PROP_KEY, "")
        is_placeable_prefixed = obj.name.startswith(PLACEABLE_PREFIX)
        is_resource_prefixed = obj.name.startswith(RESOURCE_PREFIX)
        if not kind and not is_placeable_prefixed and not is_resource_prefixed:
            continue

        if is_resource_prefixed and not kind:
            report.warn(
                f"{obj.name} uses RESOURCE_ prefix but has no {KIND_PROP_KEY}; skipped"
            )
            continue

        if not kind:
            # PLACEABLE_<kind> naming fallback so authors can skip custom props.
            kind = obj.name[len(PLACEABLE_PREFIX) :]
            if not kind:
                report.warn(f"{obj.name} uses PLACEABLE_ prefix with no kind; skipped")
                continue

        prop_id = custom_prop_str(obj, ID_PROP_KEY, "") or obj.name
        tint_raw = custom_prop(obj, TINT_PROP_KEY)
        tint = None
        if tint_raw is not None:
            if isinstance(tint_raw, (list, tuple)) and len(tint_raw) >= 3:
                tint = [float(tint_raw[0]), float(tint_raw[1]), float(tint_raw[2])]
            else:
                report.warn(f"{obj.name} has invalid {TINT_PROP_KEY}; ignored")

        entry = {
            "id": prop_id,
            "kind": kind,
            "transform": _transform_to_dict(obj),
            "blocks_movement": bool(custom_prop(obj, BLOCKS_MOVE_PROP_KEY, False)),
        }
        if tint is not None:
            entry["tint"] = tint
        collision = _collision_shape_from_props(obj)
        if collision is not None:
            entry["collision"] = collision

        props.append(entry)

    return props


# ---------------------------------------------------------------------------
# Top-level manifest assembly.
# ---------------------------------------------------------------------------


def build_manifest(
    blend_path: str,
    resolution: int,
    include_walkable_mesh: bool,
    report: ExportReport,
) -> Optional[dict]:
    meta = find_meta_node()
    if meta is None:
        report.error(
            "Missing __bevymmo_map_meta empty. "
            "Add it (Add > Empty > Plain Axes) and set bevymmo_* custom properties."
        )
        return None

    map_id = collect_map_id(meta, blend_path, report)
    display_name = collect_display_name(meta, map_id)
    bounds = collect_map_bounds(meta, report)

    surfaces = collect_surfaces(resolution, report)
    if not include_walkable_mesh:
        # The heightfield alone is enough for runtime queries; shedding the
        # mesh keeps the JSON small for shipping maps.
        for surface in surfaces:
            surface.pop("walkable_mesh", None)

    blockers = collect_blockers(report)
    traversals = collect_traversals(report)
    props = collect_props(report)

    manifest = {
        "version": MANIFEST_VERSION,
        "map_id": map_id,
        "display_name": display_name,
        "bounds": bounds,
        "world_metrics": dict(DEFAULT_WORLD_METRICS),
        "surfaces": surfaces,
        "traversals": traversals,
        "blockers": blockers,
        "props": props,
        "test_route": [],
        "test_checklist": [],
        "mountain_switchback_test": None,
        "distant_plateau_test": None,
    }
    return manifest


# ---------------------------------------------------------------------------
# Operator + UI registration.
# ---------------------------------------------------------------------------


class BEVYMMO_OT_export_world(bpy.types.Operator, ExportHelper):
    """Export the current scene as a BevyMMO `.world.json` sidecar.

    Run this after editing the .blend. It writes gameplay data (walkable
    surfaces, blockers, traversals, world metrics) next to the GLB so the
    engine can load the map without further hand-edits.
    """

    bl_idname = "export_scene.bevymmo_world_json"
    bl_label = "Export BevyMMO World (.world.json)"
    bl_options = {"REGISTER", "UNDO"}

    filename_ext = ".world.json"
    filter_glob: StringProperty(default="*.world.json", options={"HIDDEN"})

    heightfield_resolution: IntProperty(
        name="Heightfield Resolution",
        description="Cells per side for each WALKABLE_* heightfield. (resolution+1)² samples per surface.",
        default=32,
        min=1,
        max=256,
    )
    include_walkable_mesh: BoolProperty(
        name="Include walkable_mesh",
        description=(
            "Also export the full triangle mesh per surface (vertices + indices). "
            "OFF by default and normally best left off: when present, the engine "
            "ignores the heightfield and resolves every ground query with a linear "
            "scan over all triangles (SurfaceQuery::resolve_triangle_mesh), which "
            "costs O(triangles) per query and returns the first matching triangle "
            "rather than the highest. Only enable it for debugging a surface whose "
            "heightfield is suspect."
        ),
        default=False,
    )

    def execute(self, context):
        report = ExportReport()
        blend_path = bpy.data.filepath or self.filepath
        manifest = build_manifest(
            blend_path=blend_path,
            resolution=self.heightfield_resolution,
            include_walkable_mesh=self.include_walkable_mesh,
            report=report,
        )

        if not report.ok or manifest is None:
            for err in report.errors:
                self.report({"ERROR"}, err)
            return {"CANCELLED"}

        # Default filename: <map_id>.world.json, mirroring the GLB convention.
        target = self.filepath
        if os.path.isdir(target) or not target.endswith(".world.json"):
            target = os.path.join(target, f"{manifest['map_id']}.world.json")

        try:
            with open(target, "w", encoding="utf-8") as f:
                json.dump(manifest, f, indent=2)
        except OSError as exc:
            self.report({"ERROR"}, f"Failed to write {target}: {exc}")
            return {"CANCELLED"}

        surface_count = len(manifest["surfaces"])
        blocker_count = len(manifest["blockers"])
        self.report(
            {"INFO"},
            f"Exported {manifest['map_id']}: "
            f"{surface_count} surface(s), {blocker_count} blocker(s) -> {target}",
        )
        for warn in report.warnings:
            self.report({"WARNING"}, warn)

        # Tag the file path so the N-panel can show it next time.
        context.scene.bevymmo_last_export_path = target
        return {"FINISHED"}


class BEVYMMO_PT_panel(bpy.types.Panel):
    bl_idname = "BEVYMMO_PT_panel"
    bl_label = "BevyMMO"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "BevyMMO"

    def draw(self, context):
        layout = self.layout
        col = layout.column(align=True)
        col.operator(
            "export_scene.bevymmo_world_json",
            text="Export .world.json",
            icon="EXPORT",
        )
        last = getattr(context.scene, "bevymmo_last_export_path", "")
        if last:
            col.label(text=f"Last: {os.path.basename(last)}", icon="FILE_TICK")


_CLASSES = (
    BEVYMMO_OT_export_world,
    BEVYMMO_PT_panel,
)


def menu_func_export(self, _context):
    self.layout.operator(
        "export_scene.bevymmo_world_json",
        text="BevyMMO World (.world.json)",
    )


def register():
    for cls in _CLASSES:
        bpy.utils.register_class(cls)
    bpy.types.TOPBAR_MT_file_export.append(menu_func_export)
    bpy.types.Scene.bevymmo_last_export_path = bpy.props.StringProperty(
        name="Last BevyMMO export path",
        default="",
        options={"HIDDEN"},
    )


def unregister():
    bpy.types.TOPBAR_MT_file_export.remove(menu_func_export)
    for cls in reversed(_CLASSES):
        bpy.utils.unregister_class(cls)
    del bpy.types.Scene.bevymmo_last_export_path


if __name__ == "__main__":
    register()
