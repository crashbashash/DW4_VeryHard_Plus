#!/usr/bin/env python3
"""Regenerate the app icon from the committed source artwork.

``crates/dw4vhp-gui/icons/source.png`` is the reproducible input: a square,
1024x1024, RGBA image of the app's own artwork (a yellow crown over a dark
"Very Hard Plus" badge — the crown is the rarity-5 mark the patcher writes, so
it is artwork, not game data). This script reads it and writes the single icon
``eframe`` embeds:

    crates/dw4vhp-gui/icons/icon.png   256x256 RGBA

``eframe``'s ``IconData`` takes exactly one image, and 256x256 is the size its
own docs recommend, so no other sizes are generated. Run it by hand only when
the artwork changes, then commit both PNGs:

    python3 tools/gen_icon.py

The source is drawn deterministically on a first run (so it is reproducible
from this file alone); on every run it is read back from ``source.png`` and
downscaled, never regenerated over a committed file.
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw  # type: ignore[import-not-found]

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "crates" / "dw4vhp-gui" / "icons"
SOURCE = ICONS / "source.png"
ICON = ICONS / "icon.png"

# The committed source artwork is square and at least this large.
SOURCE_SIZE = 1024
# The one size eframe's IconData takes.
ICON_SIZE = 256
# Draw at 4x and downscale, so the crown and plus edges anti-alias cleanly.
SUPERSAMPLE = 4

# A dark navy "game UI" background, gold crown, gold plus.
BG_TOP = (43, 58, 103, 255)  # #2b3a67
BG_BOTTOM = (22, 28, 51, 255)  # #161c33
GOLD = (232, 185, 35, 255)  # #e8b923
GOLD_EDGE = (184, 135, 15, 255)  # #b8870f


def draw_source() -> Image.Image:
    """Return the 1024x1024 RGBA source artwork, drawn deterministically."""
    size = SOURCE_SIZE * SUPERSAMPLE
    s = float(size)

    def p(x: float, y: float) -> tuple[float, float]:
        return (x * SUPERSAMPLE, y * SUPERSAMPLE)

    # Vertical-gradient background, clipped to a rounded square.
    gradient = Image.new("RGBA", (size, size))
    gdraw = ImageDraw.Draw(gradient)
    for y in range(size):
        t = y / (s - 1)
        colour = tuple(round(a + (b - a) * t) for a, b in zip(BG_TOP, BG_BOTTOM))
        gdraw.line([(0, y), (size, y)], fill=colour)

    mask = Image.new("L", (size, size), 0)
    margin = 48 * SUPERSAMPLE
    radius = 220 * SUPERSAMPLE
    ImageDraw.Draw(mask).rounded_rectangle(
        [margin, margin, size - margin, size - margin], radius=radius, fill=255
    )

    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    img.paste(gradient, (0, 0), mask)
    draw = ImageDraw.Draw(img)

    # A five-point crown (the rarity-5 "yellow crown" the patcher writes).
    crown = [
        p(160, 720),
        p(160, 600),
        p(210, 600),
        p(280, 360),
        p(350, 600),
        p(430, 600),
        p(512, 240),
        p(594, 600),
        p(674, 600),
        p(744, 360),
        p(814, 600),
        p(864, 600),
        p(864, 720),
    ]
    draw.polygon(crown, fill=GOLD)
    draw.line(crown + [crown[0]], fill=GOLD_EDGE, width=6 * SUPERSAMPLE, joint="curve")

    # The "Plus" below the crown.
    vertical = [p(482, 745), p(542, 745), p(542, 885), p(482, 885)]
    horizontal = [p(442, 785), p(582, 785), p(582, 845), p(442, 845)]
    draw.polygon(vertical, fill=GOLD)
    draw.polygon(horizontal, fill=GOLD)
    for bar in (vertical, horizontal):
        draw.line(bar + [bar[0]], fill=GOLD_EDGE, width=6 * SUPERSAMPLE, joint="curve")

    return img.resize((SOURCE_SIZE, SOURCE_SIZE), Image.LANCZOS)


def main() -> int:
    if not SOURCE.is_file():
        print(f"source artwork missing, drawing it deterministically: {SOURCE}")
        draw_source().save(SOURCE)

    source = Image.open(SOURCE).convert("RGBA")
    if source.width != source.height:
        print(f"source must be square, got {source.size}", file=sys.stderr)
        return 1
    if source.width < SOURCE_SIZE:
        print(
            f"source should be at least {SOURCE_SIZE}px, got {source.width}",
            file=sys.stderr,
        )
        return 1

    icon = source.resize((ICON_SIZE, ICON_SIZE), Image.LANCZOS)
    icon.save(ICON)
    print(f"wrote {ICON.relative_to(ROOT)} ({ICON_SIZE}x{ICON_SIZE})")
    print(f"source: {SOURCE.relative_to(ROOT)} ({source.width}x{source.height})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
