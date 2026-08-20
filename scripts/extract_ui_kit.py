from __future__ import annotations

import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/ui/source/input_kit.jpg"
OUTPUT = ROOT / "assets/ui/extracted_kit"

# Coordinates are (left, top, right, bottom) in source-image pixels, measured
# on the 1536x1024 JPEG (Canny + residual components, then checked against a
# 64px grid so gold/blue corner diamonds and glow are not clipped).
COMPONENTS: list[tuple[str, tuple[int, int, int, int]]] = [
    # Left column: user-input bars (icon is baked into the left cap).
    ("input_gold", (54, 31, 717, 153)),
    ("input_gold_hover", (53, 169, 714, 291)),
    ("input_blue", (48, 305, 719, 429)),
    ("input_silver", (50, 446, 717, 568)),
    # Right column: password bars (lock + eye baked in).
    ("input_password_gold", (819, 31, 1484, 153)),
    ("input_password_gold_hover", (817, 169, 1485, 291)),
    ("input_password_blue", (816, 305, 1486, 428)),
    ("input_password_silver", (816, 443, 1487, 568)),
    # Square icon buttons.
    ("icon_user_gold", (65, 606, 193, 730)),
    ("icon_user_blue", (242, 604, 379, 732)),
    ("icon_user_silver", (429, 605, 562, 730)),
    ("icon_lock_gold", (64, 760, 193, 885)),
    ("icon_lock_blue", (242, 760, 383, 888)),
    ("icon_lock_silver", (427, 760, 562, 886)),
    # Vertical divider sticks.
    ("divider_gold", (632, 603, 648, 751)),
    ("divider_gold_02", (712, 605, 734, 753)),
    ("divider_blue", (802, 605, 823, 753)),
    ("divider_silver", (895, 602, 912, 752)),
    # Eye icons (gold, gold-glow, blue, silver slashed).
    ("icon_eye_gold", (983, 635, 1077, 699)),
    ("icon_eye_gold_glow", (1108, 635, 1206, 700)),
    ("icon_eye_blue", (1236, 631, 1336, 699)),
    ("icon_eye_off_silver", (1366, 627, 1457, 705)),
    # Slash-square icons.
    ("icon_slash_gold", (984, 752, 1065, 836)),
    ("icon_slash_gold_glow", (1113, 751, 1197, 832)),
    ("icon_slash_blue", (1243, 751, 1320, 833)),
    ("icon_slash_silver", (1368, 752, 1448, 835)),
    # Checkboxes, including the extra blue-glow pair.
    ("checkbox_gold_empty", (640, 886, 728, 973)),
    ("checkbox_gold_checked", (750, 886, 835, 972)),
    ("checkbox_blue_empty", (861, 886, 949, 973)),
    ("checkbox_blue_checked", (969, 887, 1055, 972)),
    ("checkbox_silver_empty", (1080, 887, 1163, 968)),
    ("checkbox_silver_checked", (1188, 887, 1273, 970)),
    ("checkbox_blue_glow_empty", (1304, 883, 1385, 967)),
    ("checkbox_blue_glow_checked", (1406, 883, 1489, 968)),
]


def padding_for(name: str) -> int:
    if name.startswith("input_") or name.endswith("_blue") or "glow" in name:
        return 12
    if name.startswith("divider_"):
        return 8
    return 10


def is_framed(name: str) -> bool:
    """Ornaments whose interior fill must stay opaque if flood-fill leaks."""
    return name.startswith(
        ("input_", "icon_user_", "icon_lock_", "checkbox_", "icon_slash_")
    )


def remove_outer_background(rgb: np.ndarray, name: str) -> np.ndarray:
    """Make the connected crop surround transparent.

    The source is a brown→blue gradient rather than near-black, so every edge
    pixel is used as a flood seed (corners alone do not reach the far side of
    the gradient). The conservative tolerance avoids eating bronze/blue pixels
    at the component edges.
    """
    bgr = cv2.cvtColor(rgb, cv2.COLOR_RGB2BGR)
    h, w = bgr.shape[:2]
    flood = np.zeros((h, w), dtype=np.uint8)
    flags = 4 | cv2.FLOODFILL_MASK_ONLY | (255 << 8)
    # Silver ornaments sit close to the grey-blue gradient; a slightly
    # tighter fill keeps their corner brackets from being eaten.
    # Inputs sit on a brown/blue wash; a higher tolerance is needed to
    # punch that halo without eating the bronze frame (checked visually).
    if name.startswith("input_"):
        lo = 28
    elif "silver" in name:
        lo = 10
    else:
        lo = 12
    tolerance = (lo, lo, lo)
    seeds: list[tuple[int, int]] = []
    step = 2
    for x in range(0, w, step):
        seeds.append((x, 0))
        seeds.append((x, h - 1))
    for y in range(0, h, step):
        seeds.append((0, y))
        seeds.append((w - 1, y))
    for x, y in seeds:
        if flood[y, x] != 0:
            continue
        mask = np.zeros((h + 2, w + 2), dtype=np.uint8)
        cv2.floodFill(bgr.copy(), mask, (x, y), 0, tolerance, tolerance, flags)
        flood |= mask[1:-1, 1:-1]
    alpha = np.where(flood > 0, 0, 255).astype(np.uint8)
    return np.dstack((rgb, alpha))


def force_keep_interior(rgba: np.ndarray, name: str, padding: int) -> None:
    """Preserve the central fill for framed controls.

    Inset past the outer padding *and* the diamond/glow cutouts so the keep
    rectangle cannot paint a rectangular halo around pointed ornaments.
    """
    h, w = rgba.shape[:2]
    if name.startswith("input_"):
        inset_x = padding + 44
        # The bar is much shorter than the padded crop; a small Y inset
        # would paint the brown sheet wash back over the punched halo.
        inset_y = padding + 28
    else:
        inset_x = inset_y = padding + 12
    if h <= 2 * inset_y + 8 or w <= 2 * inset_x + 8:
        return
    rgba[inset_y : h - inset_y, inset_x : w - inset_x, 3] = 255


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    image = np.asarray(Image.open(SOURCE).convert("RGB"))
    print(f"source {SOURCE.relative_to(ROOT)} {image.shape[1]}x{image.shape[0]}")
    # Sparse row sample so the chosen Y bands can be re-checked without a GUI.
    for y in (40, 100, 180, 240, 330, 390, 470, 530, 640, 700, 800, 860, 930):
        row = image[y]
        gold = int(((row[:, 0] > 130) & (row[:, 1] > 90) & (row[:, 0] > row[:, 2] + 25)).sum())
        print(f"  y={y:4d} mean={row.mean():6.1f} gold={gold:4d}")

    manifest = []
    for name, (left, top, right, bottom) in COMPONENTS:
        padding = padding_for(name)
        source_left = max(0, left - padding)
        source_top = max(0, top - padding)
        source_right = min(image.shape[1], right + padding)
        source_bottom = min(image.shape[0], bottom + padding)
        crop = image[source_top:source_bottom, source_left:source_right]
        rgba = remove_outer_background(crop, name)
        if is_framed(name):
            force_keep_interior(rgba, name, padding)

        opaque = int((rgba[:, :, 3] > 0).sum())
        total = int(rgba.shape[0] * rgba.shape[1])
        if opaque < total * 0.08:
            raise SystemExit(
                f"{name}: crop looks empty ({opaque}/{total} opaque) — check the box"
            )

        path = OUTPUT / f"{name}.png"
        Image.fromarray(rgba).save(path, optimize=True)
        manifest.append(
            {
                "name": name,
                "file": str(path.relative_to(ROOT)),
                "source_rect": {
                    "left": source_left,
                    "top": source_top,
                    "right": source_right,
                    "bottom": source_bottom,
                },
                "size": {
                    "width": source_right - source_left,
                    "height": source_bottom - source_top,
                },
                "opaque_pixels": opaque,
            }
        )
        print(
            f"  {name:28s} {source_right - source_left:4d}x{source_bottom - source_top:<4d} "
            f"opaque={opaque / total:5.1%}"
        )

    (OUTPUT / "manifest.json").write_text(
        json.dumps(
            {
                "source": str(SOURCE.relative_to(ROOT)),
                "source_size": {
                    "width": int(image.shape[1]),
                    "height": int(image.shape[0]),
                },
                "components": manifest,
            },
            indent=2,
        )
        + "\n"
    )
    print(f"Extracted {len(manifest)} components to {OUTPUT}")


if __name__ == "__main__":
    main()
