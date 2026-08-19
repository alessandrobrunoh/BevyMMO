"""Crop the three stacked rings from ``assets/ui/source/spell_rings.png``.

The source is 88x240 RGBA: gold, silver/gray, then blue. Dark fill outside
and inside each ring becomes transparent; the metal itself stays opaque.
"""

from __future__ import annotations

from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/ui/source/spell_rings.png"
OUTPUT = ROOT / "assets/ui/extracted_kit"

FILL_TOLERANCE = 12
CROP_PAD = 4
SQUARE_PAD = 3
BRIGHT_THRESHOLD = 50

NAMES = ("spell_ring_gold", "spell_ring_silver", "spell_ring_blue")


def detect_ring_bands(rgb: np.ndarray) -> list[tuple[int, int]]:
    bright = rgb.max(axis=2) > BRIGHT_THRESHOLD
    rows = np.where(bright.any(axis=1))[0]
    if len(rows) == 0:
        raise ValueError("No rings found in source image")
    groups: list[tuple[int, int]] = []
    start = prev = int(rows[0])
    for raw_y in rows[1:]:
        y = int(raw_y)
        if y > prev + 3:
            groups.append((start, prev))
            start = y
        prev = y
    groups.append((start, prev))
    if len(groups) != 3:
        raise ValueError(f"Expected 3 stacked rings, found {len(groups)}: {groups}")
    return groups


def flood(seeds: list[tuple[int, int]], allowed: np.ndarray) -> np.ndarray:
    height, width = allowed.shape
    visited = np.zeros((height, width), dtype=bool)
    queue: deque[tuple[int, int]] = deque()
    for x, y in seeds:
        if 0 <= x < width and 0 <= y < height and allowed[y, x] and not visited[y, x]:
            visited[y, x] = True
            queue.append((x, y))
    while queue:
        x, y = queue.popleft()
        for dx, dy in (
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ):
            nx, ny = x + dx, y + dy
            if 0 <= nx < width and 0 <= ny < height and allowed[ny, nx] and not visited[ny, nx]:
                visited[ny, nx] = True
                queue.append((nx, ny))
    return visited


def largest_component(mask: np.ndarray) -> np.ndarray:
    height, width = mask.shape
    seen = np.zeros((height, width), dtype=bool)
    best: list[tuple[int, int]] = []
    for y in range(height):
        for x in range(width):
            if not mask[y, x] or seen[y, x]:
                continue
            queue: deque[tuple[int, int]] = deque([(x, y)])
            seen[y, x] = True
            cells = [(x, y)]
            while queue:
                cx, cy = queue.popleft()
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    nx, ny = cx + dx, cy + dy
                    if 0 <= nx < width and 0 <= ny < height and mask[ny, nx] and not seen[ny, nx]:
                        seen[ny, nx] = True
                        queue.append((nx, ny))
                        cells.append((nx, ny))
            if len(cells) > len(best):
                best = cells
    out = np.zeros((height, width), dtype=bool)
    for x, y in best:
        out[y, x] = True
    return out


def extract_ring(src: np.ndarray, y0: int, y1: int) -> np.ndarray:
    band = src[y0 : y1 + 1]
    bright = band[:, :, :3].max(axis=2) > 45
    ys, xs = np.where(bright)
    left, right = int(xs.min()), int(xs.max())
    top, bottom = int(ys.min() + y0), int(ys.max() + y0)
    x0 = max(0, left - CROP_PAD)
    x1 = min(src.shape[1], right + CROP_PAD + 1)
    yy0 = max(0, top - CROP_PAD)
    yy1 = min(src.shape[0], bottom + CROP_PAD + 1)
    crop = src[yy0:yy1, x0:x1].copy()
    rgb = crop[:, :, :3].astype(np.int16)
    height, width = rgb.shape[:2]
    outer = rgb[1, 1]
    inner = rgb[height // 2, width // 2]
    fill_like = (np.abs(rgb - outer).max(axis=2) <= FILL_TOLERANCE) | (
        np.abs(rgb - inner).max(axis=2) <= FILL_TOLERANCE
    )
    seeds: list[tuple[int, int]] = []
    for x in range(width):
        seeds.append((x, 0))
        seeds.append((x, height - 1))
    for y in range(height):
        seeds.append((0, y))
        seeds.append((width - 1, y))
    seeds.append((width // 2, height // 2))
    transparent = flood(seeds, fill_like)
    opaque = largest_component(~transparent)
    ys, xs = np.where(opaque)
    left, right = int(xs.min()), int(xs.max())
    top, bottom = int(ys.min()), int(ys.max())
    left = max(0, left - SQUARE_PAD)
    right = min(width - 1, right + SQUARE_PAD)
    top = max(0, top - SQUARE_PAD)
    bottom = min(height - 1, bottom + SQUARE_PAD)
    box_w, box_h = right - left + 1, bottom - top + 1
    side = max(box_w, box_h)
    canvas = np.zeros((side, side, 4), dtype=np.uint8)
    ox = (side - box_w) // 2
    oy = (side - box_h) // 2
    sub = crop[top : bottom + 1, left : right + 1].copy()
    sub[:, :, 3] = np.where(opaque[top : bottom + 1, left : right + 1], 255, 0).astype(np.uint8)
    canvas[oy : oy + box_h, ox : ox + box_w] = sub
    return canvas


def verify(name: str, rgba: np.ndarray) -> None:
    alpha = rgba[:, :, 3]
    height, width = alpha.shape
    cy, cx = height // 2, width // 2
    if alpha[cy, cx] != 0:
        raise SystemExit(f"{name}: center is not transparent")
    corners = (alpha[0, 0], alpha[0, -1], alpha[-1, 0], alpha[-1, -1])
    if any(corners):
        raise SystemExit(f"{name}: outside corners are not transparent")
    if alpha[0].max() or alpha[-1].max() or alpha[:, 0].max() or alpha[:, -1].max():
        raise SystemExit(f"{name}: ring is clipped at the image edge")
    ys, xs = np.where(alpha == 255)
    radii = np.sqrt((xs - cx) ** 2 + (ys - cy) ** 2)
    r_in, r_out = np.percentile(radii, [5, 95])
    missing = 0
    for i in range(72):
        ang = 2.0 * np.pi * i / 72.0
        hit = False
        for radius in np.linspace(r_in - 2.0, r_out + 2.0, 24):
            x = int(round(cx + radius * np.cos(ang)))
            y = int(round(cy + radius * np.sin(ang)))
            if 0 <= x < width and 0 <= y < height and alpha[y, x] == 255:
                hit = True
                break
        if not hit:
            missing += 1
    if missing:
        raise SystemExit(f"{name}: incomplete circle ({missing}/72 rays miss the ring)")


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    src = np.asarray(Image.open(SOURCE).convert("RGBA"))
    if src.shape[0] != 240 or src.shape[1] != 88:
        raise SystemExit(f"Unexpected source size: {src.shape[1]}x{src.shape[0]} (expected 88x240)")
    bands = detect_ring_bands(src[:, :, :3])
    for name, (y0, y1) in zip(NAMES, bands):
        rgba = extract_ring(src, y0, y1)
        verify(name, rgba)
        path = OUTPUT / f"{name}.png"
        Image.fromarray(rgba).save(path, optimize=True)
        print(f"{name}: {rgba.shape[1]}x{rgba.shape[0]} -> {path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
