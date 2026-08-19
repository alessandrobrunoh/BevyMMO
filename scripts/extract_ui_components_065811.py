from __future__ import annotations

import json
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "ChatGPT Image Aug 19, 2026, 06_58_11 PM.png"
OUTPUT = ROOT / "assets/ui/extracted_065811"

# Measured directly on the 1536x1024 source image.
COMPONENTS: list[tuple[str, tuple[int, int, int, int]]] = [
    ("panel_large_left", (14, 8, 469, 674)),
    ("panel_header", (488, 21, 978, 128)),
    ("ornate_line_header", (489, 129, 979, 183)),
    ("panel_row_01", (490, 192, 978, 313)),
    ("panel_row_02", (490, 315, 978, 429)),
    ("panel_row_active", (490, 432, 978, 551)),
    ("panel_row_04", (490, 555, 978, 685)),
    ("panel_bottom_left", (22, 701, 462, 787)),
    ("panel_bottom_center", (486, 701, 940, 789)),
    ("bar_neutral_left_01", (27, 805, 237, 856)),
    ("bar_neutral_right_01", (253, 805, 451, 856)),
    ("bar_blue_left_01", (27, 861, 237, 918)),
    ("bar_neutral_right_02", (253, 861, 451, 918)),
    ("bar_blue_left_02", (27, 925, 238, 985)),
    ("bar_blue_right_active", (253, 925, 455, 985)),
    ("bar_bottom_center", (488, 802, 939, 884)),
    ("slot_empty_01", (489, 901, 555, 982)),
    ("slot_empty_02", (576, 901, 641, 982)),
    ("slot_active", (659, 901, 724, 982)),
    ("slot_check_01", (745, 901, 810, 982)),
    ("slot_check_active", (827, 901, 893, 982)),
    ("ring_gold", (995, 18, 1119, 144)),
    ("ring_silver", (995, 145, 1119, 267)),
    ("ring_blue", (993, 265, 1120, 389)),
    ("plus_ring_gold", (990, 402, 1060, 479)),
    ("plus_ring_silver", (1058, 402, 1128, 479)),
    ("plus_ring_blue_01", (990, 480, 1060, 559)),
    ("plus_ring_blue_02", (1057, 480, 1128, 559)),
    ("plus_slot_01", (987, 559, 1060, 630)),
    ("plus_slot_02", (1062, 559, 1128, 630)),
    ("plus_slot_active", (1062, 633, 1128, 707)),
    ("plus_slot_03", (987, 633, 1060, 707)),
    ("button_close_neutral_01", (953, 742, 1030, 818)),
    ("button_close_neutral_02", (1050, 742, 1128, 818)),
    ("button_close_red", (1153, 742, 1231, 818)),
    ("button_close_blue", (1255, 742, 1333, 818)),
    ("banner_blue_large", (1154, 309, 1240, 435)),
    ("banner_green_large", (1248, 309, 1344, 435)),
    ("banner_purple_large", (1355, 309, 1454, 435)),
    ("banner_blue_small", (1154, 437, 1240, 548)),
    ("banner_purple_small", (1248, 437, 1339, 548)),
    ("shield_triangle_01", (1167, 558, 1242, 661)),
    ("shield_triangle_02", (1260, 558, 1333, 661)),
    ("corner_frame_top_left", (1138, 13, 1245, 116)),
    ("corner_frame_top_right", (1278, 12, 1418, 118)),
    ("corner_frame_right_top", (1425, 13, 1532, 120)),
    ("horizontal_line_right_01", (1237, 147, 1417, 202)),
    ("horizontal_line_right_02", (1240, 220, 1417, 274)),
    ("horizontal_line_right_03", (1145, 260, 1495, 297)),
    ("vertical_gem_sword", (1458, 307, 1518, 501)),
    ("vertical_meter_blue", (1338, 463, 1422, 943)),
    ("vertical_meter_dark", (1431, 467, 1497, 944)),
    ("vertical_gem_bottom_01", (1364, 841, 1417, 938)),
    ("vertical_gem_bottom_02", (1466, 841, 1515, 902)),
    ("triangle_control_01", (950, 838, 1020, 917)),
    ("triangle_control_02", (1046, 838, 1114, 917)),
    ("triangle_control_03", (1141, 838, 1212, 917)),
    ("triangle_control_04", (1238, 838, 1309, 917)),
    ("diamond_header_01", (211, 26, 272, 83)),
    ("diamond_header_02", (402, 27, 464, 85)),
    ("diamond_header_03", (705, 22, 765, 86)),
    ("diamond_header_04", (925, 28, 981, 83)),
    ("diamond_line_01", (700, 126, 773, 185)),
    ("diamond_line_02", (925, 126, 977, 184)),
    ("diamond_right_01", (1284, 143, 1350, 199)),
    ("diamond_right_02", (1283, 214, 1350, 272)),
    ("diamond_right_03", (1151, 256, 1205, 302)),
    ("diamond_right_04", (1457, 535, 1515, 594)),
    ("diamond_right_05", (1458, 709, 1516, 770)),
]


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    source = Image.open(SOURCE).convert("RGBA")
    if source.size != (1536, 1024):
        raise ValueError(f"Unexpected source size: {source.size}")

    manifest = []
    for name, (left, top, right, bottom) in COMPONENTS:
        padding = 1 if name.startswith(("plus_",)) else 4
        x0 = max(0, left - padding)
        y0 = max(0, top - padding)
        x1 = min(source.width, right + padding)
        y1 = min(source.height, bottom + padding)
        source.crop((x0, y0, x1, y1)).save(OUTPUT / f"{name}.png", optimize=True)
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
