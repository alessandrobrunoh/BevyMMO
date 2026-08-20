from __future__ import annotations

import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "ChatGPT Image Aug 19, 2026, 06_39_03 PM.png"
OUTPUT = ROOT / "assets/ui/extracted"

# Coordinates are (left, top, right, bottom), in source-image pixels.
COMPONENTS: list[tuple[str, tuple[int, int, int, int]]] = [
    ("window_left", (14, 16, 406, 407)),
    ("window_secondary", (427, 16, 672, 407)),
    ("header_center", (690, 16, 1111, 89)),
    ("header_right", (1188, 16, 1553, 88)),
    ("tab_active", (700, 115, 802, 158)),
    ("tab_inactive_01", (804, 115, 910, 158)),
    ("tab_inactive_02", (911, 115, 1017, 158)),
    ("tab_inactive_03", (1018, 115, 1116, 158)),
    ("content_panel", (695, 176, 1121, 403)),
    ("side_slot_01", (1135, 114, 1202, 177)),
    ("side_slot_02", (1135, 184, 1202, 247)),
    ("side_slot_03", (1135, 255, 1202, 317)),
    ("side_slot_04", (1135, 326, 1202, 388)),
    ("window_right", (1218, 99, 1557, 511)),
    ("window_right_scrollbar", (1510, 136, 1534, 480)),
    ("inventory_slot_01", (24, 435, 110, 517)),
    ("inventory_slot_02", (122, 435, 207, 517)),
    ("inventory_slot_03", (221, 435, 306, 517)),
    ("inventory_slot_04", (320, 435, 405, 517)),
    ("inventory_slot_05", (419, 435, 505, 517)),
    ("inventory_slot_06", (24, 528, 110, 610)),
    ("inventory_slot_07", (122, 528, 207, 610)),
    ("inventory_slot_08", (221, 528, 306, 610)),
    ("inventory_slot_09", (320, 528, 405, 610)),
    ("inventory_slot_10", (419, 528, 505, 610)),
    ("progress_bar_center", (557, 442, 1107, 476)),
    ("button_neutral_01", (556, 509, 686, 559)),
    ("button_blue_01", (708, 509, 848, 560)),
    ("button_blue_02", (871, 509, 1031, 560)),
    ("button_gold", (557, 587, 684, 634)),
    ("button_neutral_02", (708, 587, 846, 634)),
    ("button_red", (874, 587, 1004, 634)),
    ("icon_plus", (551, 658, 610, 719)),
    ("icon_settings", (635, 658, 694, 719)),
    ("icon_down", (718, 658, 777, 719)),
    ("icon_close", (800, 658, 859, 719)),
    ("icon_previous", (898, 658, 957, 719)),
    ("icon_next", (980, 658, 1041, 719)),
    ("text_field", (530, 756, 1055, 809)),
    ("progress_bar_lower", (553, 834, 1039, 851)),
    ("checkbox_checked", (565, 882, 609, 928)),
    ("checkbox_active", (631, 882, 675, 928)),
    ("checkbox_empty", (698, 882, 741, 928)),
    ("shield_round", (34, 711, 139, 856)),
    ("shield_square", (161, 723, 260, 870)),
    ("shield_banner_short", (286, 727, 356, 840)),
    ("shield_banner_tall", (379, 723, 473, 871)),
    ("slider_track_upper", (1158, 543, 1526, 577)),
    ("slider_track_lower", (1158, 596, 1526, 627)),
    ("slider_handle_upper", (1307, 542, 1347, 577)),
    ("slider_handle_lower", (1323, 602, 1408, 623)),
    ("vertical_meter_01", (1144, 674, 1177, 932)),
    ("vertical_meter_02", (1224, 674, 1258, 932)),
    ("vertical_meter_03", (1292, 677, 1320, 933)),
    ("icon_arrow_up", (1351, 717, 1394, 764)),
    ("icon_arrow_down", (1351, 850, 1394, 896)),
    ("decorative_gem", (810, 441, 844, 477)),
    ("decorative_gem_lower", (777, 821, 811, 854)),
    ("decorative_gem_right", (1308, 542, 1347, 578)),
]


def remove_outer_background(rgb: np.ndarray) -> np.ndarray:
    """Make the connected, near-black crop surround transparent.

    Flood-filling from all crop edges preserves dark panel interiors because
    those interiors are enclosed by their frames. The conservative tolerance
    avoids eating bronze/blue/red pixels at the component edges.
    """
    bgr = cv2.cvtColor(rgb, cv2.COLOR_RGB2BGR)
    h, w = bgr.shape[:2]
    mask = np.zeros((h + 2, w + 2), dtype=np.uint8)
    flood = np.zeros((h, w), dtype=np.uint8)
    flags = 4 | cv2.FLOODFILL_MASK_ONLY | (255 << 8)
    tolerance = (11, 11, 11)
    seeds = [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)]
    for x, y in seeds:
        if flood[y, x] == 0:
            cv2.floodFill(bgr.copy(), mask, (x, y), 0, tolerance, tolerance, flags)
            flood |= mask[1:-1, 1:-1]
    alpha = np.where(flood > 0, 0, 255).astype(np.uint8)
    rgba = np.dstack((rgb, alpha))
    return rgba


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    image = np.asarray(Image.open(SOURCE).convert("RGB"))
    manifest = []
    # The coordinates were measured on the 1567x965 display preview; map them
    # back to the original 1598x984 source before cropping.
    scale_x = image.shape[1] / 1567
    scale_y = image.shape[0] / 965
    for name, (left, top, right, bottom) in COMPONENTS:
        left = round(left * scale_x)
        top = round(top * scale_y)
        right = round(right * scale_x)
        bottom = round(bottom * scale_y)
        # Keep a generous safety margin so antialiased corners and shadows are
        # never clipped at the manually selected component boundary.
        padding = 5 if name.startswith("side_slot_") else 10
        source_left = max(0, left - padding)
        source_top = max(0, top - padding)
        source_right = min(image.shape[1], right + padding)
        source_bottom = min(image.shape[0], bottom + padding)
        crop = image[source_top:source_bottom, source_left:source_right]
        rgba = remove_outer_background(crop)
        # Dark UI fills can be connected to the source background by antialiasing.
        # Preserve the central fill for framed controls; decorative/icon crops stay
        # fully background-removed.
        if not name.startswith(("icon_", "decorative_")) and crop.shape[0] > 12 and crop.shape[1] > 12:
            rgba[4:-4, 4:-4, 3] = 255
        path = OUTPUT / f"{name}.png"
        Image.fromarray(rgba, "RGBA").save(path, optimize=True)
        manifest.append({
            "name": name,
            "file": str(path.relative_to(ROOT)),
            "source_rect": {"left": source_left, "top": source_top, "right": source_right, "bottom": source_bottom},
            "size": {"width": source_right - source_left, "height": source_bottom - source_top},
        })
    (OUTPUT / "manifest.json").write_text(json.dumps({
        "source": str(SOURCE.relative_to(ROOT)),
        "source_size": {"width": int(image.shape[1]), "height": int(image.shape[0])},
        "components": manifest,
    }, indent=2) + "\n")
    print(f"Extracted {len(manifest)} components to {OUTPUT}")


if __name__ == "__main__":
    main()
