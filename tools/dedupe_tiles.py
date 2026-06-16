#!/usr/bin/env python3
"""Dedupe + combine the top-down battle tilesets into one spritesheet.

The source art (assets/tiles/tileset_{1,2,3}.png) ships as three palette
variants of the SAME base tileset: the structural tiles (walls, pipes, room
corners, hazard objects, character sprites, UI icons) are pixel-identical
across all three sheets, while the floor field is recoloured per sheet. So
roughly half of every sheet is a duplicate of the others.

This tool slices each sheet into TILE x TILE cells, drops fully-uniform cells
(the solid-black void filler carries no art), dedupes the rest by exact pixel
content, and packs the unique tiles into a single grid spritesheet plus a JSON
manifest mapping every packed index back to its (source sheet, row, col). An
optional --preview renders the packed sheet with grid lines + indices so the
tiles can be categorised (floor / wall / corner / object / actor / icon).

Usage:
    python3 tools/dedupe_tiles.py [--tile 32] [--cols 16] \
        [--src assets/tiles] [--out assets/tiles/tileset_combined.png] \
        [--preview /tmp/tileset_combined_labeled.png] [--keep-uniform]
"""

from __future__ import annotations

import argparse
import glob
import hashlib
import json
import os

from PIL import Image, ImageDraw


def slice_tiles(img: Image.Image, tile: int):
    """Yield (row, col, tile_image) for every cell of an RGBA image."""
    w, h = img.size
    for row in range(h // tile):
        for col in range(w // tile):
            box = (col * tile, row * tile, (col + 1) * tile, (row + 1) * tile)
            yield row, col, img.crop(box)


def is_uniform(tile_img: Image.Image) -> bool:
    """True if every pixel is identical (solid colour / empty void filler)."""
    extrema = tile_img.getextrema()  # per-band (min, max); equal => uniform band
    return all(lo == hi for lo, hi in extrema)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--tile", type=int, default=32)
    ap.add_argument("--cols", type=int, default=16)
    ap.add_argument("--src", default="assets/tiles")
    ap.add_argument("--out", default="assets/tiles/tileset_combined.png")
    ap.add_argument("--preview", default=None)
    ap.add_argument(
        "--keep-uniform",
        action="store_true",
        help="keep solid-colour tiles instead of dropping them",
    )
    args = ap.parse_args()

    out_name = os.path.basename(args.out)
    sources = sorted(
        p
        for p in glob.glob(os.path.join(args.src, "*.png"))
        if os.path.basename(p) != out_name and "combined" not in os.path.basename(p)
    )
    if not sources:
        raise SystemExit(f"no source tilesets found under {args.src!r}")

    seen: dict[str, int] = {}
    uniques: list[Image.Image] = []
    manifest: list[dict] = []
    dropped_uniform = 0
    total_cells = 0

    for path in sources:
        sheet = Image.open(path).convert("RGBA")
        if sheet.width % args.tile or sheet.height % args.tile:
            raise SystemExit(
                f"{path} ({sheet.width}x{sheet.height}) is not a multiple of "
                f"tile size {args.tile}"
            )
        src = os.path.basename(path)
        for row, col, tile_img in slice_tiles(sheet, args.tile):
            total_cells += 1
            if not args.keep_uniform and is_uniform(tile_img):
                dropped_uniform += 1
                continue
            key = hashlib.sha1(tile_img.tobytes()).hexdigest()
            if key in seen:
                continue
            seen[key] = len(uniques)
            uniques.append(tile_img)
            manifest.append(
                {"index": len(uniques) - 1, "src": src, "row": row, "col": col}
            )

    count = len(uniques)
    cols = min(args.cols, count) if count else args.cols
    rows = (count + cols - 1) // cols if count else 0
    sheet_out = Image.new("RGBA", (cols * args.tile, rows * args.tile), (0, 0, 0, 0))
    for i, tile_img in enumerate(uniques):
        x = (i % cols) * args.tile
        y = (i // cols) * args.tile
        sheet_out.paste(tile_img, (x, y))

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    sheet_out.save(args.out)
    manifest_path = os.path.splitext(args.out)[0] + ".manifest.json"
    with open(manifest_path, "w") as fh:
        json.dump(
            {
                "tile_px": args.tile,
                "cols": cols,
                "rows": rows,
                "count": count,
                "sources": [os.path.basename(p) for p in sources],
                "tiles": manifest,
            },
            fh,
            indent=2,
        )

    if args.preview:
        scale = 2
        prev = sheet_out.resize(
            (sheet_out.width * scale, sheet_out.height * scale), Image.NEAREST
        ).convert("RGBA")
        draw = ImageDraw.Draw(prev)
        step = args.tile * scale
        for i in range(count):
            x = (i % cols) * step
            y = (i // cols) * step
            draw.rectangle([x, y, x + step - 1, y + step - 1], outline=(255, 0, 0, 180))
            draw.text((x + 2, y + 1), str(i), fill=(255, 255, 0, 255))
        prev.save(args.preview)

    print(f"sources         : {[os.path.basename(p) for p in sources]}")
    print(f"total cells     : {total_cells}")
    print(f"dropped uniform : {dropped_uniform}")
    print(f"unique tiles    : {count}  ->  {cols} cols x {rows} rows")
    print(f"spritesheet     : {args.out}")
    print(f"manifest        : {manifest_path}")
    if args.preview:
        print(f"labeled preview : {args.preview}")


if __name__ == "__main__":
    main()
