#!/usr/bin/env python3
"""Rescales a map — its GLB geometry and its `.world.json` — by a uniform factor.

Why this exists
---------------
map_02 was authored roughly twice life size: 19 m trees, 8 m rocks (up to 45 m),
a 360 m square with 40 m of relief. The player capsule is fixed at 1.7 m by
`WorldMetrics`, so the character reads as a speck and the camera has to sit far
enough back to make it worse. Shrinking the world is the same thing as growing
the character, and it is the one knob that fixes the proportions everywhere at
once.

**What scales**: terrain and prop geometry, map bounds, heightfield bounds and
heights, prop and blocker transforms, collision extents, traversal endpoints.

**What must not**: `world_metrics` — player radius, height, eye height,
`max_step_height`. Those describe the *character*, which is not shrinking, and
scaling them would undo the whole point. Slope limits are angles, likewise
untouched.

This is a post-processing step: a re-export from Blender overwrites both files
and has to be re-run. Once you settle on a factor, bake it into the `.blend`
instead (select all, `S`, apply transforms) and stop running this.

Usage:
    python3 scripts/scale_map.py map_02 0.5
    python3 scripts/scale_map.py map_02 0.5 --dry-run
"""

from __future__ import annotations

import argparse
import json
import shutil
import struct
import sys
from pathlib import Path

ASSETS = Path(__file__).resolve().parent.parent / "assets" / "maps"

SCALE_ROOT_NAME = "__bevymmo_scale_root"


def scale_glb(path: Path, factor: float, dry_run: bool) -> str:
    """Wraps every scene root in a scaling node.

    Inserting a parent rather than rewriting each node's transform keeps the
    authored values intact, so re-running with a different factor is a matter
    of replacing one node instead of unwinding arithmetic. An existing scale
    root from a previous run is reused.
    """
    data = path.read_bytes()
    if data[:4] != b"glTF":
        raise SystemExit(f"{path} is not a binary glTF file")

    json_len = struct.unpack("<I", data[12:16])[0]
    json_start, json_end = 20, 20 + json_len
    gltf = json.loads(data[json_start:json_end])

    scene = gltf["scenes"][gltf.get("scene", 0)]
    nodes = gltf["nodes"]

    existing = next(
        (i for i, n in enumerate(nodes) if n.get("name") == SCALE_ROOT_NAME), None
    )
    if existing is not None:
        nodes[existing]["scale"] = [factor, factor, factor]
        summary = f"reused existing scale root, now x{factor}"
    else:
        nodes.append(
            {
                "name": SCALE_ROOT_NAME,
                "scale": [factor, factor, factor],
                "children": list(scene["nodes"]),
            }
        )
        scene["nodes"] = [len(nodes) - 1]
        summary = f"inserted scale root x{factor} over {len(nodes[-1]['children'])} roots"

    if dry_run:
        return summary

    # The JSON chunk must stay 4-byte aligned, padded with spaces.
    payload = json.dumps(gltf, separators=(",", ":")).encode("utf-8")
    payload += b" " * (-len(payload) % 4)

    head = data[:12]
    rest = data[json_end:]
    chunk = struct.pack("<I", len(payload)) + b"JSON" + payload
    total = len(head) + len(chunk) + len(rest)
    path.write_bytes(
        head[:8] + struct.pack("<I", total) + chunk + rest
    )
    return summary


def scale_manifest(manifest: dict, factor: float) -> dict:
    def scale_bounds(bounds: dict) -> dict:
        return {key: value * factor for key, value in bounds.items()}

    def scale_vec(values: list) -> list:
        return [v * factor for v in values]

    manifest["bounds"] = scale_bounds(manifest["bounds"])

    for surface in manifest.get("surfaces", []):
        if surface.get("bounds"):
            surface["bounds"] = scale_bounds(surface["bounds"])
        for key in ("height", "min_height", "max_height", "size"):
            if isinstance(surface.get(key), (int, float)):
                surface[key] = surface[key] * factor
        field = surface.get("heightfield")
        if field:
            field["bounds"] = scale_bounds(field["bounds"])
            field["heights"] = [h * factor for h in field["heights"]]
        mesh = surface.get("walkable_mesh")
        if mesh:
            mesh["vertices"] = [scale_vec(v) for v in mesh["vertices"]]

    for entry in list(manifest.get("props", [])) + list(manifest.get("blockers", [])):
        transform = entry.get("transform")
        if transform:
            transform["translation"] = scale_vec(transform["translation"])
            # `scale` stays: it multiplies the shape, which is scaled below.
        shape = entry.get("shape") or entry.get("collision")
        if isinstance(shape, dict):
            for variant, params in shape.items():
                if variant == "Box":
                    params["half_extents"] = scale_vec(params["half_extents"])
                elif variant in ("Cylinder", "Sphere"):
                    for key in ("radius", "height"):
                        if key in params:
                            params[key] = params[key] * factor

    for traversal in manifest.get("traversals", []):
        for key in ("start", "end"):
            if key in traversal:
                traversal[key] = scale_vec(traversal[key])
        if "width" in traversal:
            traversal["width"] = traversal["width"] * factor

    for point in manifest.get("test_route", []):
        for key in ("x", "z", "height"):
            if key in point:
                point[key] = point[key] * factor

    # `world_metrics` is deliberately untouched: it describes the character.
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("map_id")
    parser.add_argument("factor", type=float, help="e.g. 0.5 to halve the world")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--no-backup", action="store_true", help="skip the .orig copies"
    )
    args = parser.parse_args()

    if not 0.01 <= args.factor <= 100.0:
        raise SystemExit("factor must be between 0.01 and 100")

    glb_path = ASSETS / f"{args.map_id}.glb"
    json_path = ASSETS / f"{args.map_id}.world.json"
    for path in (glb_path, json_path):
        if not path.exists():
            raise SystemExit(f"missing {path}")

    manifest = json.loads(json_path.read_text())

    # The GLB reuses its scale root, but the manifest would be scaled a second
    # time — the two would silently disagree. Record what has been applied and
    # convert an absolute request into the delta that gets there.
    applied = float(manifest.get("bevymmo_applied_scale", 1.0))
    if abs(applied - args.factor) < 1e-9:
        print(f"already at x{args.factor}: nothing to do")
        return 0
    delta = args.factor / applied
    if applied != 1.0:
        print(f"already scaled x{applied}; applying x{delta:.4f} to reach x{args.factor}")

    before = manifest["bounds"]
    field = next(
        (s["heightfield"] for s in manifest["surfaces"] if s.get("heightfield")), None
    )
    relief_before = (max(field["heights"]) - min(field["heights"])) if field else 0.0

    if not args.dry_run and not args.no_backup:
        for path in (glb_path, json_path):
            backup = path.with_suffix(path.suffix + ".orig")
            if not backup.exists():
                shutil.copy2(path, backup)
                print(f"backed up {backup.name}")

    glb_summary = scale_glb(glb_path, args.factor, args.dry_run)
    scaled = scale_manifest(manifest, delta)
    scaled["bevymmo_applied_scale"] = args.factor

    print(f"glb  : {glb_summary}")
    print(
        f"world: {before['max_x'] - before['min_x']:.0f} m square"
        f" -> {scaled['bounds']['max_x'] - scaled['bounds']['min_x']:.0f} m"
    )
    print(f"relief: {relief_before:.1f} m -> {relief_before * delta:.1f} m")
    print(
        f"player stays {scaled['world_metrics']['player_height']:.2f} m tall"
        f" (world_metrics untouched)"
    )

    if args.dry_run:
        print("\n--dry-run: nothing written")
        return 0

    json_path.write_text(json.dumps(scaled, indent=2))
    print(f"\nwrote {json_path.name}")
    print("re-run generate_blockers_from_glb.py to rebuild collision at the new scale")
    return 0


if __name__ == "__main__":
    sys.exit(main())
