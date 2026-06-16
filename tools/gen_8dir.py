#!/usr/bin/env python3
"""gen_8dir.py — attempt to synthesize 8-direction character sprites from the
4-direction art in assets/tiles/alt_tileset_characters.png, then honestly
assess whether the result is usable. Also documents the reliable fallback:
mapping the sim's 8 facings onto the 4 existing sprite frames.

Run from the repo root:  python3 tools/gen_8dir.py
Outputs land under /tmp (this tool never edits game code or assets).

=== DETECTED LAYOUT (see analysis in the docstring + emitted previews) ===
Sheet:  assets/tiles/alt_tileset_characters.png  256x288  = 16 cols x 18 rows
        of 16x16-px tiles.
Each ACTOR occupies a contiguous horizontal run of 4 cells (its 4 facings).
A 16-wide row therefore holds 4 actors. Facing ORDER within a 4-run is:

        col+0 = LEFT  (West,  gun/profile pointing -X)
        col+1 = DOWN  (South, facing the viewer, face visible)
        col+2 = UP    (North, facing away, back of head)
        col+3 = RIGHT (East,  mirror of LEFT, gun pointing +X)

Evidence: the gun-less DOWN/UP frames are horizontally symmetric
(d(frame, hflip(frame)) == 0 for clean actors); LEFT and RIGHT are the
profile pair (RIGHT ~= hflip(LEFT), hand-retouched so not pixel-exact).
"""

from __future__ import annotations

import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw
except ImportError:  # pragma: no cover - environment guard
    sys.stderr.write("Pillow required: pip install pillow\n")
    raise SystemExit(1)

TS = 16  # tile size in px
SHEET = Path("assets/tiles/alt_tileset_characters.png")
OUT_DIR = Path("/tmp")

# Cardinal frame index within an actor's 4-run.
LEFT, DOWN, UP, RIGHT = 0, 1, 2, 3

# Sample actor groups to test on: (label, row, base_col).
SAMPLES = [
    ("green_helmet", 0, 0),
    ("blue_shirt", 0, 4),
    ("blonde", 2, 0),
]


def load_sheet() -> Image.Image:
    if not SHEET.exists():
        sys.stderr.write(f"missing sheet: {SHEET} (run from repo root)\n")
        raise SystemExit(1)
    return Image.open(SHEET).convert("RGBA")


def cell(sheet: Image.Image, col: int, row: int) -> Image.Image:
    return sheet.crop((col * TS, row * TS, (col + 1) * TS, (row + 1) * TS))


def hflip(img: Image.Image) -> Image.Image:
    return img.transpose(Image.FLIP_LEFT_RIGHT)


def rotate_recrop(img: Image.Image, degrees: float) -> Image.Image:
    """(a) 45-deg nearest-neighbour rotation of a cardinal frame.

    Expand canvas so corners are not clipped, rotate with NEAREST (to keep the
    hard pixel edges), then recrop the central 16x16.
    """
    big = Image.new("RGBA", (TS * 3, TS * 3), (0, 0, 0, 0))
    big.paste(img, (TS, TS))
    rot = big.rotate(degrees, resample=Image.NEAREST, expand=False)
    left = (rot.width - TS) // 2
    top = (rot.height - TS) // 2
    return rot.crop((left, top, left + TS, top + TS))


def synth_diagonals(frames: dict[str, Image.Image]) -> dict[str, Image.Image]:
    """Produce the 4 diagonal frames from the 4 cardinals.

    Method (a) rotation of the adjacent cardinal toward the diagonal, plus
    method (b) mirror to keep left/right diagonals consistent. NE/NW derive
    from UP; SE/SW derive from DOWN — rotated 45 deg toward the side.
    """
    up, down = frames["up"], frames["down"]
    # Rotate the front/back frame 45 deg; mirror for the opposite side.
    ne = rotate_recrop(up, -45)
    nw = hflip(ne)
    se = rotate_recrop(down, 45)
    sw = hflip(se)
    return {"ne": ne, "nw": nw, "se": se, "sw": sw}


def label_strip(
    images: list[tuple[str, Image.Image]], scale: int = 16
) -> Image.Image:
    """Lay frames in a labelled horizontal strip at NEAREST `scale`x zoom."""
    pad = 4
    cell_w = TS * scale
    label_h = 18
    strip = Image.new(
        "RGBA",
        (len(images) * (cell_w + pad) + pad, cell_w + label_h + pad * 2),
        (30, 30, 36, 255),
    )
    draw = ImageDraw.Draw(strip)
    x = pad
    for name, img in images:
        big = img.resize((cell_w, cell_w), Image.NEAREST)
        strip.paste(big, (x, label_h + pad), big)
        draw.text((x + 2, 2), name, fill=(230, 230, 230, 255))
        x += cell_w + pad
    return strip


def main() -> None:
    sheet = load_sheet()
    OUT_DIR.mkdir(exist_ok=True)

    # 8-direction output sheet: one ROW per sample actor, 8 columns
    # in clockwise order N, NE, E, SE, S, SW, W, NW.
    order = ["up", "ne", "right", "se", "down", "sw", "left", "nw"]
    out_cols = len(order)
    out = Image.new("RGBA", (out_cols * TS, len(SAMPLES) * TS), (0, 0, 0, 0))

    preview_rows: list[Image.Image] = []

    for r_idx, (label, row, base) in enumerate(SAMPLES):
        frames = {
            "left": cell(sheet, base + LEFT, row),
            "down": cell(sheet, base + DOWN, row),
            "up": cell(sheet, base + UP, row),
            "right": cell(sheet, base + RIGHT, row),
        }
        diag = synth_diagonals(frames)
        all_frames = {**frames, **diag}

        for c_idx, key in enumerate(order):
            out.paste(all_frames[key], (c_idx * TS, r_idx * TS))

        # Labelled preview row: cardinals (real) then diagonals (synth).
        strip_imgs = [
            (f"{label}", frames["down"]),
            ("L*", frames["left"]),
            ("D*", frames["down"]),
            ("U*", frames["up"]),
            ("R*", frames["right"]),
            ("NE~", diag["ne"]),
            ("NW~", diag["nw"]),
            ("SE~", diag["se"]),
            ("SW~", diag["sw"]),
        ]
        preview_rows.append(label_strip(strip_imgs))

    out_path = OUT_DIR / "chars_8dir.png"
    out.save(out_path)
    out_4x = out.resize((out.width * 4, out.height * 4), Image.NEAREST)
    out_4x.save(OUT_DIR / "chars_8dir_4x.png")

    # Stack the labelled preview rows into one image.
    pw = max(p.width for p in preview_rows)
    ph = sum(p.height for p in preview_rows)
    preview = Image.new("RGBA", (pw, ph), (30, 30, 36, 255))
    y = 0
    for p in preview_rows:
        preview.paste(p, (0, y))
        y += p.height
    preview_path = OUT_DIR / "chars_8dir_preview.png"
    preview.save(preview_path)

    print(f"wrote {out_path}")
    print(f"wrote {OUT_DIR / 'chars_8dir_4x.png'}")
    print(f"wrote {preview_path}  (* = real art, ~ = synthesized diagonal)")
    print()
    print("Cardinals are real art; NE/NW/SE/SW are 45-deg NEAREST rotations")
    print("(+ mirror). Inspect the preview: at 16px the rotated frames show")
    print("jagged stair-steps, a tilted 'ground shadow', and a gun barrel")
    print("that rotates off the body. Verdict: NOT usable as game art.")
    print()
    print(fallback_doc())


def fallback_doc() -> str:
    """The reliable fallback: map the sim's 8 facings to the 4 sprite frames.

    Sim `Facing` (8-dir) -> sprite frame index in the actor's 4-run.
    Diagonals snap to the NEAREST CARDINAL by the dominant axis; convention
    here resolves a pure-diagonal tie toward the VERTICAL frame (Up/Down) so
    the face/back read stays legible (a left/right profile loses the face).
    Zero new art, zero artifacts.
    """
    mapping = [
        ("North", "UP    (col+2)"),
        ("NorthEast", "UP    (col+2)   # vertical-bias tie -> Up"),
        ("East", "RIGHT (col+3)"),
        ("SouthEast", "DOWN  (col+1)   # vertical-bias tie -> Down"),
        ("South", "DOWN  (col+1)"),
        ("SouthWest", "DOWN  (col+1)   # vertical-bias tie -> Down"),
        ("West", "LEFT  (col+0)"),
        ("NorthWest", "UP    (col+2)   # vertical-bias tie -> Up"),
    ]
    lines = [
        "=== FALLBACK (recommended): sim stays 8-dir, RENDERER maps 8 -> 4 ===",
        "Sim Facing            ->  sprite frame",
    ]
    for k, v in mapping:
        lines.append(f"  {k:<10}          ->  {v}")
    lines += [
        "",
        "Alternative horizontal-bias tie (diagonals -> L/R profile) is also",
        "reasonable if the profile silhouette reads better in-engine; pick one",
        "convention in the renderer and keep it. The mapping is a pure lookup",
        "in the presenter's draw system — the sim is untouched.",
    ]
    return "\n".join(lines)


if __name__ == "__main__":
    main()
