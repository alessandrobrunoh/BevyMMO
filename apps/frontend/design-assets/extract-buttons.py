#!/usr/bin/env python3
"""Extract the approved Eivar button sheet into transparent runtime assets.

Run from apps/frontend with:
    python3 design-assets/extract-buttons.py
"""

from __future__ import annotations

import sys
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw


@dataclass(frozen=True)
class Sprite:
    name: str
    box: tuple[int, int, int, int]
    shape: str


ROW_BANDS = {
    "large-gold": (0, 167),
    "large-blue": (167, 307),
    "wide-dark": (307, 401),
    "compact-blue": (401, 491),
    "compact-green": (491, 566),
    "compact-red": (566, 645),
    "compact-gold": (645, 730),
    "square": (730, 842),
    "circle": (842, 955),
    "navigation": (955, 1086),
}

LARGE_COLUMNS = (0, 414, 814, 1153, 1448)
COMPACT_COLUMNS = (0, 300, 580, 860, 1140, 1448)
ICON_COLUMNS = (0, 110, 194, 278, 370, 462, 548, 635, 727, 817, 900, 986, 1076, 1168, 1252, 1338, 1448)
NAVIGATION_COLUMNS = (90, 197, 303, 410, 508, 592, 676, 772, 855, 939, 1033, 1135, 1242, 1340)


def cells(bounds: tuple[int, ...]) -> Iterable[tuple[int, int]]:
    return zip(bounds, bounds[1:])


def build_manifest() -> list[Sprite]:
    sprites: list[Sprite] = []

    for family in ("large-gold", "large-blue", "wide-dark"):
        y0, y1 = ROW_BANDS[family]
        for state, (x0, x1) in zip(("default", "hover", "pressed", "disabled"), cells(LARGE_COLUMNS)):
            sprites.append(Sprite(f"{family}-{state}", (x0, y0, x1, y1), "bar"))

    for family in ("compact-blue", "compact-green", "compact-red", "compact-gold"):
        y0, y1 = ROW_BANDS[family]
        states = ("default", "hover", "pressed", "emblem", "disabled")
        for state, (x0, x1) in zip(states, cells(COMPACT_COLUMNS)):
            sprites.append(Sprite(f"{family}-{state}", (x0, y0, x1, y1), "bar"))

    icon_states = ("default", "hover", "blank", "disabled")
    icon_cells = list(cells(ICON_COLUMNS))
    for shape in ("square", "circle"):
        y0, y1 = ROW_BANDS[shape]
        for tone_index, tone in enumerate(("blue", "green", "red", "gold")):
            for state_index, state in enumerate(icon_states):
                x0, x1 = icon_cells[tone_index * 4 + state_index]
                sprites.append(Sprite(f"{shape}-{tone}-{state}", (x0, y0, x1, y1), shape))

    y0, y1 = ROW_BANDS["navigation"]
    nav_cells = list(cells(NAVIGATION_COLUMNS))
    for state, (x0, x1) in zip(("default", "hover", "disabled"), nav_cells[:3]):
        sprites.append(Sprite(f"arrow-left-{state}", (x0, y0, x1, y1), "square"))
    for index, (x0, x1) in enumerate(nav_cells[3:10], start=1):
        sprites.append(Sprite(f"diamond-{index}", (x0, y0, x1, y1), "diamond"))
    for state, (x0, x1) in zip(("default", "hover", "disabled"), nav_cells[10:13]):
        sprites.append(Sprite(f"arrow-right-{state}", (x0, y0, x1, y1), "square"))

    return sprites


def content_box(image: Image.Image, cell: tuple[int, int, int, int], padding: int = 6) -> tuple[int, int, int, int]:
    crop = image.crop(cell)
    mask = crop.convert("L").point([0] * 11 + [255] * 245)
    bounds = mask.getbbox()
    if bounds is None:
        raise ValueError(f"No visible pixels found in cell {cell}")

    left = max(cell[0] + bounds[0] - padding, 0)
    top = max(cell[1] + bounds[1] - padding, 0)
    right = min(cell[0] + bounds[2] + padding, image.width)
    bottom = min(cell[1] + bounds[3] + padding, image.height)
    return left, top, right, bottom


def core_mask(size: tuple[int, int], shape: str) -> Image.Image:
    width, height = size
    mask = Image.new("L", size, 0)
    draw = ImageDraw.Draw(mask)

    if shape == "circle":
        draw.ellipse((width * 0.12, height * 0.12, width * 0.88, height * 0.88), fill=255)
    elif shape == "square":
        draw.rectangle((width * 0.14, height * 0.14, width * 0.86, height * 0.86), fill=255)
    elif shape == "diamond":
        draw.polygon(
            ((width * 0.5, height * 0.08), (width * 0.92, height * 0.5), (width * 0.5, height * 0.92), (width * 0.08, height * 0.5)),
            fill=255,
        )
    else:
        draw.polygon(
            (
                (width * 0.10, height * 0.14),
                (width * 0.90, height * 0.14),
                (width * 0.98, height * 0.50),
                (width * 0.90, height * 0.86),
                (width * 0.10, height * 0.86),
                (width * 0.02, height * 0.50),
            ),
            fill=255,
        )

    return mask


def remove_black_background(image: Image.Image, shape: str) -> Image.Image:
    rgb = image.convert("RGB")
    mask = core_mask(rgb.size, shape)
    source = list(rgb.getdata())
    protected = list(mask.getdata())
    output: list[tuple[int, int, int, int]] = []

    for (red, green, blue), is_core in zip(source, protected):
        strongest = max(red, green, blue)
        if is_core:
            output.append((red, green, blue, 255))
            continue
        if strongest <= 4:
            output.append((0, 0, 0, 0))
            continue

        alpha = min(255, strongest * 2)
        output.append(
            (
                min(255, round(red * 255 / alpha)),
                min(255, round(green * 255 / alpha)),
                min(255, round(blue * 255 / alpha)),
                alpha,
            )
        )

    rgba = Image.new("RGBA", rgb.size)
    rgba.putdata(output)
    return rgba


def main() -> None:
    source = (
        Path(sys.argv[1])
        if len(sys.argv) > 1
        else Path(__file__).resolve().parent / "sheets/buttons.png"
    )
    destination = Path(__file__).resolve().parents[1] / "public/assets/ui/buttons"
    destination.mkdir(parents=True, exist_ok=True)

    sheet = Image.open(source).convert("RGB")
    if sheet.size != (1448, 1086):
        raise ValueError(f"Expected a 1448x1086 source sheet, received {sheet.size[0]}x{sheet.size[1]}")

    manifest = build_manifest()
    for sprite in manifest:
        crop = sheet.crop(content_box(sheet, sprite.box))
        remove_black_background(crop, sprite.shape).save(destination / f"{sprite.name}.png", optimize=True)

    print(f"Extracted {len(manifest)} button assets to {destination}")


if __name__ == "__main__":
    main()
