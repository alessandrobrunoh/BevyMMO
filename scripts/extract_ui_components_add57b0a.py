from __future__ import annotations

import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "add57b0a-04b6-4c40-913c-e9582ac704cd.png"
OUTPUT = ROOT / "assets/ui/extracted_add57b0a"

# Rectangles measured directly in the 1536x1024 source image.
COMPONENTS: list[tuple[str, tuple[int, int, int, int]]] = [
    ("logo_eivar_online", (42, 8, 449, 231)),
    ("panel_large_left", (34, 239, 412, 720)),
    ("panel_header_center", (482, 27, 959, 219)),
    ("panel_large_right", (1181, 26, 1512, 545)),
    ("button_settings", (986, 42, 1058, 113)),
    ("button_profile", (1082, 42, 1154, 113)),
    ("diamond_blue_large_01", (985, 129, 1060, 204)),
    ("diamond_blue_large_02", (1077, 129, 1152, 204)),
    ("ring_gold", (895, 213, 987, 309)),
    ("ring_silver", (989, 213, 1082, 309)),
    ("ring_blue", (1081, 213, 1177, 309)),
    ("ornate_line_long", (439, 240, 879, 282)),
    ("ornate_line_diamond", (570, 240, 706, 282)),
    ("ornate_line_short", (704, 240, 879, 282)),
    ("bar_blue_long_01", (430, 313, 700, 381)),
    ("bar_blue_long_02", (430, 390, 700, 458)),
    ("bar_gray_long", (430, 468, 700, 539)),
    ("bar_blue_short_01", (726, 316, 876, 373)),
    ("bar_blue_short_02", (726, 374, 876, 431)),
    ("bar_gray_short_01", (726, 432, 876, 488)),
    ("bar_gray_short_02", (726, 488, 876, 541)),
    ("banner_faction_white", (898, 325, 979, 449)),
    ("banner_faction_green", (984, 325, 1069, 449)),
    ("banner_faction_purple", (1077, 325, 1164, 449)),
    ("bar_blue_bottom", (36, 731, 376, 807)),
    ("ornate_line_bottom_long", (37, 811, 373, 854)),
    ("ornate_line_bottom_short", (40, 855, 344, 891)),
    ("diamond_blue_small_01", (46, 896, 100, 952)),
    ("diamond_blue_small_02", (122, 896, 177, 952)),
    ("diamond_blue_small_03", (202, 896, 259, 952)),
    ("diamond_gold_small_01", (46, 958, 101, 1013)),
    ("diamond_blue_small_04", (122, 958, 177, 1013)),
    ("diamond_blue_small_05", (202, 958, 259, 1013)),
    ("panel_square", (444, 553, 638, 732)),
    ("ring_panel_gold", (648, 553, 797, 706)),
    ("banner_large_blue", (850, 513, 1035, 740)),
    ("banner_tall_blue", (1051, 514, 1154, 813)),
    ("barrel_blue_gold", (1154, 568, 1218, 973)),
    ("barrel_blue", (1212, 570, 1260, 975)),
    ("world_island", (1316, 548, 1501, 759)),
    ("portal_platform", (1267, 898, 1512, 1016)),
    ("portal_beam", (1320, 760, 1499, 934)),
    ("button_triangle", (1268, 749, 1314, 796)),
    ("diamond_small_right_01", (1266, 570, 1315, 620)),
    ("diamond_small_right_02", (1266, 672, 1315, 722)),
    ("square_skill_01", (389, 781, 453, 846)),
    ("square_skill_02", (458, 781, 521, 846)),
    ("square_skill_03", (528, 781, 591, 846)),
    ("square_skill_04", (598, 781, 660, 846)),
    ("square_skill_05", (668, 781, 729, 846)),
    ("square_skill_06", (738, 781, 800, 846)),
    ("round_skill_01", (389, 852, 453, 916)),
    ("round_skill_02", (458, 852, 521, 916)),
    ("round_skill_03", (528, 852, 591, 916)),
    ("round_skill_04", (598, 852, 660, 916)),
    ("round_skill_05", (668, 852, 729, 916)),
    ("round_skill_06", (738, 852, 800, 916)),
    ("square_control_down", (389, 925, 453, 987)),
    ("square_control_empty_01", (458, 925, 521, 987)),
    ("square_control_empty_02", (528, 925, 591, 987)),
    ("square_control_check", (598, 925, 660, 987)),
    ("square_control_active", (668, 925, 729, 987)),
    ("panel_label_01", (830, 755, 1045, 849)),
    ("panel_label_02", (830, 876, 1045, 948)),
    ("ornament_top_center", (193, 6, 267, 111)),
]


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    source = Image.open(SOURCE).convert("RGBA")
    if source.size != (1536, 1024):
        raise ValueError(f"Unexpected source size: {source.size}")

    manifest = []
    padding = 4
    for name, (left, top, right, bottom) in COMPONENTS:
        x0 = max(0, left - padding)
        y0 = max(0, top - padding)
        x1 = min(source.width, right + padding)
        y1 = min(source.height, bottom + padding)
        crop = source.crop((x0, y0, x1, y1))
        crop.save(OUTPUT / f"{name}.png", optimize=True)
        manifest.append({
            "name": name,
            "file": str((OUTPUT / f"{name}.png").relative_to(ROOT)),
            "source_rect": {"left": x0, "top": y0, "right": x1, "bottom": y1},
            "size": {"width": x1 - x0, "height": y1 - y0},
        })

    (OUTPUT / "manifest.json").write_text(json.dumps({
        "source": str(SOURCE.relative_to(ROOT)),
        "source_size": {"width": source.width, "height": source.height},
        "alpha_preserved": True,
        "components": manifest,
    }, indent=2) + "\n")
    print(f"Extracted {len(manifest)} components to {OUTPUT}")


if __name__ == "__main__":
    main()
