# BevyMMO Authoring Template Builder — Blender script.
#
# Run from Blender's Scripting workspace (or `blender --python ...`) on a
# fresh scene to drop in the canonical BevyMMO gameplay authoring layer:
#   - `__bevymmo_map_meta` Empty (required by bevymmo_export_world.py)
#   - `WALKABLE_*` meshes (flat plates + ramps) so the height-aware movement
#     solver in crates/shared/src/world/collision.rs can resolve each level
#   - `BLOCKING_*` cubes around cliffs/borders so steep edges are walls
#   - `PLACEABLE_*` Empties for the player spawn, one demo enemy and the
#     dragon boss; the exporter serialises them into `manifest.props`, which
#     `spawn_placeables_on_map_load` consumes (see
#     crates/server/src/placeables/creatures.rs).
#
# Idempotent: re-running cleans up any previously spawned template objects
# (matched by name) before re-creating them, so you can iterate safely.
#
# IMPORTANT: the geometry positions and blocker sizes mirror the legacy
# `assets/models/world_map/walkable_world_map.blend` layout. If you change
# them, keep this file in sync with the engine's heightfield/movement
# expectations (see docs/level-designer-guide.md §3).

from __future__ import annotations

from typing import Iterable

import bmesh
import bpy
from mathutils import Vector

# ---------------------------------------------------------------------------
# Conventions — kept in sync with bevymmo_export_world.py
# ---------------------------------------------------------------------------

META_NODE_NAME = "__bevymmo_map_meta"
WALKABLE_PREFIX = "WALKABLE_"
BLOCKING_PREFIX = "BLOCKING_"
PLACEABLE_PREFIX = "PLACEABLE_"

KIND_PROP_KEY = "bevymmo_kind"
COLLISION_PROP_KEY = "bevymmo_collision"
RADIUS_PROP_KEY = "bevymmo_radius"
HEIGHT_PROP_KEY = "bevymmo_height"
HALF_EXTENTS_PROP_KEY = "bevymmo_half_extents"

# Template object names so the script is safely re-runnable.
TEMPLATE_OBJECT_NAMES = (
    META_NODE_NAME,
    "WALKABLE_ground",
    "WALKABLE_test_top",
    "WALKABLE_ramp_ground_to_test_top",
    "WALKABLE_ramp_crescent",
    "WALKABLE_mountain_01_top",
    "WALKABLE_ramp_castle_floor_1_to_2",
    "WALKABLE_ramp_castle_floor_2_to_3",
    "WALKABLE_ramp_castle_floor_3_to_4",
    "WALKABLE_castle_floor_1",
    "WALKABLE_castle_floor_2",
    "WALKABLE_castle_floor_3",
    "WALKABLE_castle_floor_4",
    "BLOCKING_test_top_back_cliff",
    "BLOCKING_test_top_left_cliff",
    "BLOCKING_test_top_right_cliff",
    "BLOCKING_mountain_01_back_cliff",
    "BLOCKING_mountain_01_left_cliff",
    "BLOCKING_mountain_01_right_cliff",
    "BLOCKING_blocker_mapboundary_north",
    "BLOCKING_blocker_mapboundary_south",
    "BLOCKING_blocker_mapboundary_east",
    "BLOCKING_blocker_mapboundary_west",
    "PLACEABLE_player_spawn",
    "PLACEABLE_mob_goblin",
    "PLACEABLE_boss_dragon",
)


# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------


def _delete_existing(objects: Iterable[str]) -> None:
    """Removes any previously authored template objects so re-runs are clean.

    Only deletes top-level scene objects that match the template name list,
    never touching other author content (lights, cameras, custom meshes).
    """
    for name in objects:
        obj = bpy.data.objects.get(name)
        if obj is None:
            continue
        bpy.data.objects.remove(obj, do_unlink=True)


# ---------------------------------------------------------------------------
# Custom property helpers — Blender stores them via `obj[key] = value`.
# We cast to plain Python types to keep glTF round-tripping consistent with
# bevymmo_export_world.py's `custom_prop` reader.
# ---------------------------------------------------------------------------


def _set_custom_props(obj: bpy.types.Object, **props) -> None:
    for key, value in props.items():
        obj[key] = value


# ---------------------------------------------------------------------------
# Mesh builders
# ---------------------------------------------------------------------------


def _add_flat_plate(
    name: str, center: Vector, size_x: float, size_z: float, height: float
) -> bpy.types.Object:
    """Creates a flat quad on the XZ plane at the given height.

    The exporter derives the surface bounds from the world-space vertex AABB,
    so the geometry must match the intended gameplay footprint exactly.
    """
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()

    half_x = size_x * 0.5
    half_z = size_z * 0.5
    v0 = bm.verts.new((-half_x, height, -half_z))
    v1 = bm.verts.new((half_x, height, -half_z))
    v2 = bm.verts.new((half_x, height, half_z))
    v3 = bm.verts.new((-half_x, height, half_z))
    bm.faces.new((v0, v1, v2, v3))

    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.location = center
    return obj


def _add_ramp(
    name: str, start: Vector, end: Vector, width: float, thickness: float = 0.1
) -> bpy.types.Object:
    """Builds a sloped quad connecting `start` (lower) to `end` (upper).

    The quad is authored in world space: we generate the four corner vertices
    directly at their final positions so the exported heightfield (which
    raycasts the evaluated mesh) sees a clean linear slope.
    """
    start_v = Vector(start)
    end_v = Vector(end)
    direction = end_v - start_v
    if direction.length < 1e-4:
        raise ValueError(f"ramp {name} has zero length")

    # Perpendicular in the XZ plane (Y stays up).
    perp = Vector((-direction.z, 0.0, direction.x)).normalized() * (width * 0.5)

    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()

    corners = [
        start_v - perp,
        start_v + perp,
        end_v + perp,
        end_v - perp,
    ]
    verts = [bm.verts.new(c) for c in corners]
    bm.faces.new(verts)

    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    # Geometry is already in world coords, so keep the object origin at 0,0,0.
    obj.location = Vector((0.0, 0.0, 0.0))
    return obj


def _add_box_blocker(name: str, center: Vector, half_extents) -> bpy.types.Object:
    """Creates a MESH cube matching the blocker AABB.

    `bevymmo_export_world.py` derives the blocker half-extents from the mesh
    AABB when no explicit shape props are set, so we set both: the geometry
    and the explicit `bevymmo_collision`/`bevymmo_half_extents` props. Keeping
    them in sync avoids silent drift if an author rescales only one.
    """
    bpy.ops.mesh.primitive_cube_add(size=2.0, location=center)
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = (half_extents[0], half_extents[1], half_extents[2])

    half_str = ",".join(str(v) for v in half_extents)
    _set_custom_props(
        obj,
        **{
            COLLISION_PROP_KEY: "box",
            HALF_EXTENTS_PROP_KEY: half_str,
            "bevymmo_blocks_move": True,
        },
    )
    return obj


def _add_empty_placeable(name: str, position: Vector, kind: str) -> bpy.types.Object:
    """Adds an Empty (Plain Axes) tagged as a placeable spawn marker.

    Invisible in-game — only its transform and `bevymmo_kind` reach the
    engine via the exporter's `collect_props`.
    """
    bpy.ops.object.empty_add(type="PLAIN_AXES", location=position)
    obj = bpy.context.active_object
    obj.name = name
    _set_custom_props(obj, **{KIND_PROP_KEY: kind})
    return obj


# ---------------------------------------------------------------------------
# Top-level builder
# ---------------------------------------------------------------------------


def _create_meta_node() -> bpy.types.Object:
    """Drops the required `__bevymmo_map_meta` Empty with sane defaults.

    The exporter rejects the scene without it. Bounds mirror the legacy
    `walkable_world_map` layout so the template can be exported as-is.
    """
    bpy.ops.object.empty_add(type="PLAIN_AXES", location=(0.0, 0.0, 0.0))
    obj = bpy.context.active_object
    obj.name = META_NODE_NAME
    _set_custom_props(
        obj,
        **{
            "bevymmo_map_id": "starting_village",
            "bevymmo_display_name": "Starting Village",
            "bevymmo_min_x": -15.0,
            "bevymmo_max_x": 30.0,
            "bevymmo_min_z": -26.0,
            "bevymmo_max_z": 15.0,
        },
    )
    return obj


def _create_walkables() -> list[bpy.types.Object]:
    """Recreates the 12 walkable surfaces of the legacy map.

    Layout mirrors `assets/models/world_map/walkable_world_map.world.json` so
    the height-aware movement query resolves each level. Each flat plate sits
    at its authored `height`; ramps interpolate between two flat levels.
    """
    out: list[bpy.types.Object] = []

    # Ground plane — covers the whole playable footprint.
    out.append(
        _add_flat_plate(
            "WALKABLE_ground",
            Vector((7.5, 0.0, -5.0)),
            72.0,
            70.0,
            0.0,
        )
    )

    # test_top — raised plateau at y=4.
    out.append(
        _add_flat_plate(
            "WALKABLE_test_top",
            Vector((0.0, 4.0, 12.0)),
            12.85,
            8.0,
            4.0,
        )
    )

    # Ramps from ground (y=0) up to test_top (y=4).
    out.append(
        _add_ramp(
            "WALKABLE_ramp_ground_to_test_top",
            Vector((-2.0, 0.0, 0.0)),
            Vector((-2.0, 4.0, 8.0)),
            width=8.0,
        )
    )
    out.append(
        _add_ramp(
            "WALKABLE_ramp_crescent",
            Vector((10.0, 0.0, 2.5)),
            Vector((10.0, 4.0, 9.7)),
            width=14.0,
        )
    )

    # mountain_01_top — small summit at y=4.
    out.append(
        _add_flat_plate(
            "WALKABLE_mountain_01_top",
            Vector((4.0, 4.0, 0.0)),
            4.0,
            5.2,
            4.0,
        )
    )

    # Castle floors — stacked at increasing heights.
    castle_center = Vector((-16.0, 0.0, -8.0))
    castle_size = (8.8, 8.8)
    out.append(
        _add_flat_plate(
            "WALKABLE_castle_floor_1",
            castle_center,
            castle_size[0],
            castle_size[1],
            0.01,
        )
    )
    out.append(
        _add_flat_plate(
            "WALKABLE_castle_floor_2",
            castle_center,
            castle_size[0],
            castle_size[1],
            3.52,
        )
    )
    out.append(
        _add_flat_plate(
            "WALKABLE_castle_floor_3",
            castle_center,
            castle_size[0],
            castle_size[1],
            7.03,
        )
    )
    out.append(
        _add_flat_plate(
            "WALKABLE_castle_floor_4",
            castle_center,
            castle_size[0],
            castle_size[1],
            10.54,
        )
    )

    # Ramps linking the castle floors (each segment covers one storey).
    out.append(
        _add_ramp(
            "WALKABLE_ramp_castle_floor_1_to_2",
            Vector((-12.5, 0.05, -12.0)),
            Vector((-12.5, 3.55, -6.0)),
            width=2.0,
        )
    )
    out.append(
        _add_ramp(
            "WALKABLE_ramp_castle_floor_2_to_3",
            Vector((-12.5, 3.55, -11.9)),
            Vector((-12.5, 7.05, -5.9)),
            width=2.0,
        )
    )
    out.append(
        _add_ramp(
            "WALKABLE_ramp_castle_floor_3_to_4",
            Vector((-12.5, 7.05, -11.8)),
            Vector((-12.5, 10.55, -5.8)),
            width=2.0,
        )
    )

    return out


def _create_blockers() -> list[bpy.types.Object]:
    """Builds the 10 blockers of the legacy map.

    Cliff boxes prevent the player from walking off the raised plateaus
    onto steep terrain; boundary boxes wrap the playable area.
    """
    out: list[bpy.types.Object] = []

    # test_top cliffs (plateau at y=4 → box centered at y=4).
    out.append(
        _add_box_blocker(
            "BLOCKING_test_top_back_cliff",
            Vector((0.0, 4.0, 16.25)),
            (4.25, 2.5, 0.25),
        )
    )
    out.append(
        _add_box_blocker(
            "BLOCKING_test_top_left_cliff",
            Vector((-6.675, 4.0, 12.0)),
            (0.25, 2.5, 8.0),
        )
    )
    out.append(
        _add_box_blocker(
            "BLOCKING_test_top_right_cliff",
            Vector((6.675, 4.0, 12.0)),
            (0.25, 2.5, 8.0),
        )
    )

    # mountain_01 cliffs (summit at y=4).
    out.append(
        _add_box_blocker(
            "BLOCKING_mountain_01_back_cliff",
            Vector((4.0, 4.0, 2.85)),
            (2.5, 3.0, 0.25),
        )
    )
    out.append(
        _add_box_blocker(
            "BLOCKING_mountain_01_left_cliff",
            Vector((1.75, 4.0, 0.0)),
            (0.25, 3.0, 4.0),
        )
    )
    out.append(
        _add_box_blocker(
            "BLOCKING_mountain_01_right_cliff",
            Vector((6.25, 4.0, 0.0)),
            (0.25, 3.0, 4.0),
        )
    )

    # World boundary walls.
    out.append(
        _add_box_blocker(
            "BLOCKING_blocker_mapboundary_north",
            Vector((7.5, 0.0, 15.25)),
            (15.25, 1.5, 0.25),
        )
    )
    out.append(
        _add_box_blocker(
            "BLOCKING_blocker_mapboundary_south",
            Vector((7.5, 0.0, -26.25)),
            (15.25, 1.5, 0.25),
        )
    )
    out.append(
        _add_box_blocker(
            "BLOCKING_blocker_mapboundary_east",
            Vector((30.25, 0.0, -5.5)),
            (0.25, 1.5, 15.25),
        )
    )
    out.append(
        _add_box_blocker(
            "BLOCKING_blocker_mapboundary_west",
            Vector((-15.25, 0.0, -5.5)),
            (0.25, 1.5, 15.25),
        )
    )

    return out


def _create_placeables() -> list[bpy.types.Object]:
    """Drops the three gameplay spawn markers requested.

    The exporter maps these to the engine's catalog dispatch in
    `spawn_placeables_on_map_load`:
      - `player_spawn` → recorded in `PlayerSpawnPoints`, used by the join
        handler to place brand-new players (see network/server.rs).
      - `mob_goblin` → spawns a Goblin enemy via `spawn_creature`.
      - `boss_dragon` → spawns the dragon boss via `spawn_boss`.
    """
    out: list[bpy.types.Object] = []

    # Player spawn: clear spot on the ground near the centre.
    out.append(
        _add_empty_placeable(
            "PLACEABLE_player_spawn",
            Vector((0.0, 0.0, -10.0)),
            "player_spawn",
        )
    )

    # Demo enemy: in front of the mountain summit ramp.
    out.append(
        _add_empty_placeable(
            "PLACEABLE_mob_goblin",
            Vector((4.0, 0.0, 6.0)),
            "mob_goblin",
        )
    )

    # Boss: at the top of the castle tower (floor 4).
    out.append(
        _add_empty_placeable(
            "PLACEABLE_boss_dragon",
            Vector((-16.0, 10.54, -8.0)),
            "boss_dragon",
        )
    )

    return out


def build_template() -> None:
    """Recreates the full authoring layer in the current scene.

    Safe to call multiple times: previous template objects are removed first.
    Reports a summary so the operator output confirms what landed.
    """
    _delete_existing(TEMPLATE_OBJECT_NAMES)

    meta = _create_meta_node()
    walkables = _create_walkables()
    blockers = _create_blockers()
    placeables = _create_placeables()

    # Deselect everything so the user starts from a clean selection state.
    for obj in bpy.context.selected_objects:
        obj.select_set(False)

    print(
        "[bevymmo_spawn_template] created "
        f"1 meta + {len(walkables)} walkables + "
        f"{len(blockers)} blockers + {len(placeables)} placeables"
    )


if __name__ == "__main__":
    # When run from Blender's Scripting tab, `__name__` is "__main__".
    build_template()
