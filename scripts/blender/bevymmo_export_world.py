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
from mathutils import Vector

# ---------------------------------------------------------------------------
# Conventions — keep these in sync with docs/level-designer-guide.md and
# crates/shared/src/world/loader.rs (WALKABLE_NODE_PREFIX, WALKABLE_KIND_TAG).
# ---------------------------------------------------------------------------

META_NODE_NAME_PREFIX = "__bevymmo_map_meta"
WALKABLE_PREFIX = "WALKABLE_"
BLOCKING_PREFIX = "BLOCKING_"
TRAVERSAL_PREFIX = "TRAVERSAL_"

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
# Vertices/indices are emitted in world space (Y-up) so the engine can use
# them verbatim as WalkableMeshData.
#
# The heightfield is produced by raycasting straight down on a regular grid
# covering the mesh's XZ footprint. We raycast on the surface mesh itself
# (not the whole scene) so neighbouring geometry cannot pollute the samples.
# ---------------------------------------------------------------------------


def _world_triangles(
    obj: bpy.types.Object, depsgraph
) -> tuple[list[list[float]], list[int]]:
    """Returns (vertices, indices) of the mesh in world space.

    Triangles are flattened as `walkable_mesh` expects: vertices as a flat
    list of `[x, y, z]` and indices as a flat list of u32s in groups of 3.
    """
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    try:
        matrix = evaluated.matrix_world
        vertices = [
            [(matrix @ v.co).x, (matrix @ v.co).y, (matrix @ v.co).z]
            for v in mesh.vertices
        ]
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
    """Raycasts straight down on a (resolution+1)² grid to sample surface Y.

    `raycast_above` is a world Y safely above the mesh's highest vertex so
    the rays start outside the geometry. Cells that miss become the lowest
    sampled Y, mirroring the Rust helper in loader.rs.
    """
    side = resolution + 1
    dx = (bounds["max_x"] - bounds["min_x"]) / resolution
    dz = (bounds["max_z"] - bounds["min_z"]) / resolution
    if dx <= 0 or dz <= 0:
        report.error(
            f"WALKABLE_{obj.name} has zero-area bounds; cannot sample heightfield"
        )
        return {"resolution": resolution, "bounds": bounds, "heights": []}

    # World-space raycast on the scene: we honour modifiers via the depsgraph
    # and filter to `obj` only so neighbouring geometry cannot pollute samples.
    depsgraph = bpy.context.evaluated_depsgraph_get()
    scene = bpy.context.scene

    heights: list[Optional[float]] = [None] * (side * side)
    sampled_min = math.inf

    for gz in range(side):
        z = bounds["min_z"] + dz * gz
        for gx in range(side):
            x = bounds["min_x"] + dx * gx
            origin = Vector((x, raycast_above, z))
            # Cast in world space on the whole scene, then reject anything
            # that doesn't hit our surface.
            hit, _loc, _normal, hit_obj, _ = scene.ray_cast(
                depsgraph, origin, Vector((0, -1, 0))
            )
            if not hit or hit_obj != obj:
                continue
            # _loc is world-space intersection; Y is up.
            y = _loc.y
            heights[gx + gz * side] = y
            sampled_min = min(sampled_min, y)

    if sampled_min is math.inf:
        report.warn(
            f"WALKABLE_{obj.name}: no ray hit any cell — heightfield will be empty"
        )
        sampled_min = 0.0

    # Fill missed cells with the lowest sampled Y so queries stay finite.
    final_heights = [h if h is not None else sampled_min for h in heights]
    return {"resolution": resolution, "bounds": bounds, "heights": final_heights}


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
    kind = custom_prop_str(obj, "bevymmo_blocker_kind", "")
    if not kind:
        kind = custom_prop_str(obj, "bevymmo_kind", "box")
    if kind == "box":
        half_extents = custom_prop(obj, "bevymmo_half_extents")
        if half_extents is None:
            return None
        return {"type": "box", "half_extents": _as_vec3(half_extents)}
    if kind == "cylinder":
        radius = custom_prop_float(obj, "bevymmo_radius", 0.0)
        height = custom_prop_float(obj, "bevymmo_height", 0.0)
        return {"type": "cylinder", "radius": radius, "height": height}
    if kind == "sphere":
        radius = custom_prop_float(obj, "bevymmo_radius", 0.0)
        return {"type": "sphere", "radius": radius}
    return None


def _as_vec3(raw):
    if isinstance(raw, (list, tuple)) and len(raw) >= 3:
        return [float(raw[0]), float(raw[1]), float(raw[2])]
    if isinstance(raw, str):
        parts = [s.strip() for s in raw.split(",") if s.strip()]
        if len(parts) >= 3:
            try:
                return [float(parts[0]), float(parts[1]), float(parts[2])]
            except ValueError:
                return None
    return None


def _transform_to_dict(obj: bpy.types.Object) -> dict:
    loc, rot, scale = obj.matrix_world.decompose()
    # Euler YXZ in degrees — matches TransformData::rotation_deg.
    euler = rot.to_euler("YXZ")
    return {
        "translation": [loc.x, loc.y, loc.z],
        "rotation_deg": [
            math.degrees(euler.x),
            math.degrees(euler.y),
            math.degrees(euler.z),
        ],
        "scale": [scale.x, scale.y, scale.z],
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

        kind = shape["type"]
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
            start = list(bpy.data.objects[obj.name + "_start"].matrix_world.translation)
        if end is None and obj.name + "_end" in bpy.data.objects:
            end = list(bpy.data.objects[obj.name + "_end"].matrix_world.translation)

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

    manifest = {
        "version": MANIFEST_VERSION,
        "map_id": map_id,
        "display_name": display_name,
        "bounds": bounds,
        "world_metrics": dict(DEFAULT_WORLD_METRICS),
        "surfaces": surfaces,
        "traversals": traversals,
        "blockers": blockers,
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
            "Needed for exact point-in-triangle ground queries; turn off for smaller shipping JSON."
        ),
        default=True,
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
