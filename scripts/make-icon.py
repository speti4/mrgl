"""Generates the application icon, a flat murloc-eye glyph of our own (no Blizzard artwork, D-005):
crates/mrgl-app/icons/icon.ico for Windows and icon.png for the window icon on the other targets.

One-off helper (Pillow), not part of the build: tauri-build embeds the committed .ico as the Windows
application icon and tauri's code generation wants the .png as the window icon on every other target.
Re-run after changing the drawing (`-I` ignores the user site, so Pillow comes from uv):

    uv run --with pillow python -I scripts/make-icon.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

ICONS = Path(__file__).resolve().parent.parent / "crates" / "mrgl-app" / "icons"
OUT = ICONS / "icon.ico"
PNG = ICONS / "icon.png"
SIZE = 256
PNG_SIZE = 512
BACKGROUND = (22, 128, 120, 255)  # murloc teal
EYE = (245, 247, 240, 255)
PUPIL = (24, 36, 48, 255)
HIGHLIGHT = (255, 255, 255, 255)


def draw_icon(size: int) -> Image.Image:
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    radius = size * 0.22
    draw.rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=BACKGROUND)
    # One round eye, off centre, as a bubble on the water.
    cx, cy, r = size * 0.5, size * 0.5, size * 0.3
    draw.ellipse((cx - r, cy - r, cx + r, cy + r), fill=EYE)
    pr = r * 0.5
    draw.ellipse((cx - pr, cy - pr, cx + pr, cy + pr), fill=PUPIL)
    hr = pr * 0.35
    hx, hy = cx - pr * 0.4, cy - pr * 0.4
    draw.ellipse((hx - hr, hy - hr, hx + hr, hy + hr), fill=HIGHLIGHT)
    return image


def main() -> None:
    ICONS.mkdir(parents=True, exist_ok=True)
    base = draw_icon(SIZE)
    base.save(OUT, format="ICO", sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    draw_icon(PNG_SIZE).save(PNG, format="PNG")
    print(f"wrote {OUT} and {PNG}")


if __name__ == "__main__":
    main()
