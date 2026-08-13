#!/usr/bin/env python3
"""Derives `blockers[]` for a map's `.world.json` from the decor meshes in its `.glb`.

Why this exists
---------------
Rocks and trees authored in Blender are plain glTF nodes: they carry no
`bevymmo_*` custom properties, so the world exporter classifies them as neither
props nor blockers and `CollisionGrid::build` ends up with zero obstacles. The
player then walks straight through every rock on the map.

The alternative is tagging all ~300 objects by hand in Blender. This script
takes the other route: the GLB already holds every decor node's world transform
and mesh bounds, which is exactly what a blocker needs, so the collision volumes
can be derived instead of authored.

Re-run it after every `.world.json` export. It only ever rewrites blockers whose
id starts with `AUTO_`; hand-authored ones are preserved.

Usage:
    python3 scripts/generate_blockers_from_glb.py map_02
    python3 scripts/generate_blockers_from_glb.py map_02 --dry-run
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
from pathlib import Path

import numpy as np

ASSETS = Path(__file__).resolve().parent.parent / "assets" / "maps"

AUTO_ID_PREFIX = "AUTO_"

# Substrings that mark a node as solid decor. Tree canopies are excluded on
# purpose: blocking a `_Top` would stop the player metres away from the trunk.
SOLID_NAME_PARTS = ("Rock",)
TRUNK_NAME_PARTS = ("Tree",)
TRUNK_REQUIRED_PART = "_Base"

# Obstacles shorter than this are left walkable. `WorldMetrics::max_step_height`
# is 0.45 m, so anything at or under it is something the player steps over
# rather than around; the ramp-edge pebbles on map_02 are all in this class and
# blocking them would fence off the ramps they decorate.
MIN_BLOCKER_HEIGHT_M = 1.0

# The terrain stepper already probes with a 0.45 m radius around the player
# (`STEP_COLLISION_RADIUS`), so the raw mesh AABB is shrunk slightly to keep the
# felt hitbox close to the visible silhouette instead of a halo around it.
SHRINK = 0.9

# A vertex counts as emerged only once it clears the terrain by this much.
# Below it the geometry is grazing the surface, not standing on it.
EMERGED_EPSILON_M = 0.3


def read_glb(path: Path) -> dict:
    return read_glb_full(path)[0]


def read_glb_full(path: Path):
    """Returns (gltf json, whole file bytes, offset of the BIN chunk payload)."""
    data = path.read_bytes()
    if data[:4] != b"glTF":
        raise SystemExit(f"{path} is not a binary glTF file")
    json_len = struct.unpack("<I", data[12:16])[0]
    gltf = json.loads(data[20 : 20 + json_len])
    # 12-byte header + 8-byte JSON chunk header + JSON + 8-byte BIN chunk header
    return gltf, data, 20 + json_len + 8


def heightfield_sampler(manifest: dict):
    """Bilinear ground-height lookup over the manifest's first heightfield."""
    surface = next(
        (s for s in manifest.get("surfaces", []) if s.get("heightfield")), None
    )
    if surface is None:
        return None
    field = surface["heightfield"]
    resolution = field["resolution"]
    side = resolution + 1
    bounds = field["bounds"]
    # Layout is X-major with Z varying fastest: index = x * stride + z.
    heights = np.array(field["heights"], dtype=float).reshape(side, side)
    cell_x = (bounds["max_x"] - bounds["min_x"]) / resolution
    cell_z = (bounds["max_z"] - bounds["min_z"]) / resolution

    def sample(x, z):
        lx = np.clip((np.asarray(x) - bounds["min_x"]) / cell_x, 0, resolution)
        lz = np.clip((np.asarray(z) - bounds["min_z"]) / cell_z, 0, resolution)
        x0 = np.floor(lx).astype(int)
        z0 = np.floor(lz).astype(int)
        x1 = np.minimum(x0 + 1, resolution)
        z1 = np.minimum(z0 + 1, resolution)
        fx = lx - x0
        fz = lz - z0
        return (heights[x0, z0] * (1 - fx) + heights[x1, z0] * fx) * (1 - fz) + (
            heights[x0, z1] * (1 - fx) + heights[x1, z1] * fx
        ) * fz

    return sample


def node_matrix(node: dict) -> np.ndarray:
    """Local transform of a glTF node, as a 4x4 row-major matrix."""
    if "matrix" in node:
        # glTF stores matrices column-major.
        return np.array(node["matrix"], dtype=float).reshape(4, 4).T

    matrix = np.eye(4)
    if "scale" in node:
        matrix = matrix @ np.diag([*node["scale"], 1.0])
    if "rotation" in node:
        x, y, z, w = node["rotation"]
        rotation = np.eye(4)
        rotation[:3, :3] = [
            [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
        ]
        matrix = rotation @ matrix
    if "translation" in node:
        translation = np.eye(4)
        translation[:3, 3] = node["translation"]
        matrix = translation @ matrix
    return matrix


def world_matrices(gltf: dict) -> dict[int, np.ndarray]:
    """World transform per node index, resolved by walking the scene graph."""
    nodes = gltf["nodes"]
    out: dict[int, np.ndarray] = {}

    def walk(index: int, parent: np.ndarray) -> None:
        matrix = parent @ node_matrix(nodes[index])
        out[index] = matrix
        for child in nodes[index].get("children", []):
            walk(child, matrix)

    for root in gltf["scenes"][gltf.get("scene", 0)]["nodes"]:
        walk(root, np.eye(4))
    return out


def is_solid_decor(name: str) -> bool:
    if any(part in name for part in SOLID_NAME_PARTS):
        return True
    return (
        any(part in name for part in TRUNK_NAME_PARTS) and TRUNK_REQUIRED_PART in name
    )


_COMPONENT_DTYPES = {5120: "i1", 5121: "u1", 5122: "i2", 5123: "u2", 5125: "u4", 5126: "f4"}
_TYPE_COUNTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def read_accessor(gltf: dict, blob: bytes, bin_offset: int, index: int) -> np.ndarray:
    """Reads an accessor into an (count, components) array, honouring byteStride."""
    accessor = gltf["accessors"][index]
    view = gltf["bufferViews"][accessor["bufferView"]]
    dtype = np.dtype("<" + _COMPONENT_DTYPES[accessor["componentType"]])
    components = _TYPE_COUNTS[accessor["type"]]
    start = bin_offset + view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    stride = view.get("byteStride")

    if stride and stride != dtype.itemsize * components:
        rows = []
        for i in range(accessor["count"]):
            row = np.frombuffer(
                blob, dtype=dtype, count=components, offset=start + i * stride
            )
            rows.append(row)
        return np.array(rows)
    flat = np.frombuffer(
        blob, dtype=dtype, count=accessor["count"] * components, offset=start
    )
    return flat.reshape(accessor["count"], components)


def world_vertices(gltf: dict, blob: bytes, bin_offset: int, node: dict, matrix: np.ndarray):
    """Every vertex of a node's mesh, in world space."""
    chunks = []
    for primitive in gltf["meshes"][node["mesh"]]["primitives"]:
        local = read_accessor(
            gltf, blob, bin_offset, primitive["attributes"]["POSITION"]
        ).astype(float)
        homogeneous = np.hstack([local, np.ones((len(local), 1))])
        chunks.append((matrix @ homogeneous.T)[:3].T)
    if not chunks:
        return None
    return np.concatenate(chunks)


def emerged_aabb(vertices: np.ndarray, ground_at, epsilon: float):
    """AABB of the part of a mesh that actually stands above the terrain.

    Decor rocks on map_02 are outcrops: 67% of a typical rock's volume is
    buried, and 92 of them are more than 80% under the surface. Bounding the
    whole mesh therefore produced blockers up to 40 m across for a boulder
    showing a one-metre tip — the player was stopped a dozen metres from
    anything visible. Only the emerged vertices define something to collide
    with.

    Returns `None` when nothing meaningful sticks out.
    """
    ground = ground_at(vertices[:, 0], vertices[:, 2])
    above = vertices[vertices[:, 1] > ground + epsilon]
    if len(above) < 3:
        return None
    return above.min(axis=0), above.max(axis=0)


def build_blockers(gltf: dict, blob: bytes, bin_offset: int, ground_at) -> tuple[list[dict], dict]:
    matrices = world_matrices(gltf)
    blockers: list[dict] = []
    stats = {"candidates": 0, "too_short": 0, "no_bounds": 0, "fully_buried": 0}

    for index, node in enumerate(gltf["nodes"]):
        name = node.get("name", "")
        if "mesh" not in node or not is_solid_decor(name):
            continue
        stats["candidates"] += 1

        vertices = world_vertices(gltf, blob, bin_offset, node, matrices[index])
        if vertices is None or len(vertices) == 0:
            stats["no_bounds"] += 1
            continue

        if ground_at is None:
            lo, hi = vertices.min(axis=0), vertices.max(axis=0)
        else:
            bounds = emerged_aabb(vertices, ground_at, EMERGED_EPSILON_M)
            if bounds is None:
                stats["fully_buried"] += 1
                continue
            lo, hi = bounds
            # The emerged cap starts at the surface, not at the mesh's lowest
            # vertex: extend it down so the box still meets the ground.
            lo = lo.copy()
            lo[1] = min(lo[1], float(ground_at(np.array([(lo[0] + hi[0]) / 2]),
                                               np.array([(lo[2] + hi[2]) / 2]))[0]))

        if hi[1] - lo[1] < MIN_BLOCKER_HEIGHT_M:
            stats["too_short"] += 1
            continue

        center = (lo + hi) / 2.0
        half = (hi - lo) / 2.0 * SHRINK

        blockers.append(
            {
                "id": f"{AUTO_ID_PREFIX}{name}",
                "kind": "box",
                "object": name,
                "transform": {
                    "translation": [round(float(v), 4) for v in center],
                    "rotation_deg": [0.0, 0.0, 0.0],
                    "scale": [1.0, 1.0, 1.0],
                },
                "shape": {"Box": {"half_extents": [round(float(v), 4) for v in half]}},
                "blocks_movement": True,
            }
        )

    return blockers, stats


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("map_id", help="e.g. map_02")
    parser.add_argument(
        "--dry-run", action="store_true", help="report what would change, write nothing"
    )
    args = parser.parse_args()

    glb_path = ASSETS / f"{args.map_id}.glb"
    json_path = ASSETS / f"{args.map_id}.world.json"
    for path in (glb_path, json_path):
        if not path.exists():
            raise SystemExit(f"missing {path}")

    gltf, blob, bin_offset = read_glb_full(glb_path)
    manifest = json.loads(json_path.read_text())

    ground_at = heightfield_sampler(manifest)
    if ground_at is None:
        print("no heightfield in the manifest: bounding whole meshes instead")
    generated, stats = build_blockers(gltf, blob, bin_offset, ground_at)
    authored = [
        b for b in manifest.get("blockers", []) if not b["id"].startswith(AUTO_ID_PREFIX)
    ]
    manifest["blockers"] = authored + generated

    print(f"decor nodes matched : {stats['candidates']}")
    print(f"  skipped, entirely below the terrain            : {stats['fully_buried']}")
    print(f"  skipped, emerged part shorter than {MIN_BLOCKER_HEIGHT_M} m      : {stats['too_short']}")
    print(f"  skipped, no mesh bounds                        : {stats['no_bounds']}")
    print(f"blockers generated  : {len(generated)}")
    print(f"blockers hand-authored, kept : {len(authored)}")

    if generated:
        halves = np.array([b["shape"]["Box"]["half_extents"] for b in generated])
        print(
            "half-extents (m): horizontal median "
            f"{np.median(halves[:, [0, 2]]):.2f}, max {halves[:, [0, 2]].max():.2f}; "
            f"vertical median {np.median(halves[:, 1]):.2f}"
        )

    if args.dry_run:
        print("\n--dry-run: nothing written")
        return 0

    json_path.write_text(json.dumps(manifest, indent=2))
    print(f"\nwrote {json_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
