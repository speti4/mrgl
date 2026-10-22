"""Generates the application icon, a flat murloc head of our own drawing (fan art, no Blizzard artwork, D-005):
crates/mrgl-app/icons/icon.ico for Windows and icon.png for the window icon on the other targets.

One-off helper (Pillow), not part of the build: tauri-build embeds the committed .ico as the window exe's
application icon, the CLI's build script embeds the same .ico in mrgl.exe, and tauri's code generation wants the
.png as the window icon on every other target. Re-run after changing the drawing (`-I` ignores the user site, so
Pillow comes from uv):

    uv run --with pillow python -I scripts/make-icon.py

The drawing is geometry on a 256-unit square, rendered for every size on its own at four times the size and
scaled down (Pillow draws without anti-aliasing). Below 32 px the outline gets thicker and the details that
would only blur (the teeth, the eye highlights, the belly) are left out, so the face still reads at 16 px. The
colors are the window's theme tokens (crates/mrgl-app/ui/style.css): the dark theme's accent for the skin, the
light theme's accent for the fins, the dark warning amber for the crest, so the head stands out on a light and
on a dark title bar and taskbar alike.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ICONS = Path(__file__).resolve().parent.parent / "crates" / "mrgl-app" / "icons"
OUT = ICONS / "icon.ico"
PNG = ICONS / "icon.png"
# Every size the Windows shell asks an icon for: the title bar and the small views at 100 to 250 percent
# scaling (16, 20, 24, 32, 40), the taskbar and the medium views (32, 48, 64, 96), the large views (128, 256).
ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
PNG_SIZE = 512
SUPERSAMPLE = 4

# The theme tokens of style.css.
OUTLINE = (11, 23, 22, 255)  # --accent-fg, dark theme
SKIN = (63, 191, 179, 255)  # --accent, dark theme
FIN = (22, 128, 120, 255)  # --accent, light theme
CREST = (224, 168, 74, 255)  # --warn, dark theme
EYE = (244, 247, 246, 255)  # --bg, light theme
PUPIL = (20, 32, 31, 255)  # --bg, dark theme
MOUTH = (11, 23, 22, 255)  # --accent-fg, dark theme
TOOTH = (244, 247, 246, 255)
BELLY = (143, 214, 205, 255)  # the skin lightened toward the eye white


def draw_icon(size: int) -> Image.Image:
    """The murloc head at `size` pixels, square, on a transparent background."""
    small = size < 32
    scale = size * SUPERSAMPLE / 256
    canvas = round(size * SUPERSAMPLE)
    image = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    # One output pixel of outline at the small sizes, about 2.5 percent of the width above them.
    line = max(256 / size * 1.1, 7) if small else 7
    width = max(1, round(line * scale))

    def pt(x: float, y: float) -> tuple[float, float]:
        return (x * scale, y * scale)

    def box(cx: float, cy: float, rx: float, ry: float) -> tuple[float, float, float, float]:
        return (*pt(cx - rx, cy - ry), *pt(cx + rx, cy + ry))

    def polygon(points: list[tuple[float, float]], fill: tuple[int, int, int, int], outline: bool = True) -> None:
        corners = [pt(x, y) for x, y in points]
        draw.polygon(corners, fill=fill)
        if outline:
            # A line centred on the edge: Pillow's own polygon outline grows inward and eats a thin spike.
            draw.line([*corners, corners[0]], fill=OUTLINE, width=width, joint="curve")

    # The crest: three broad spikes standing up from the top of the head, behind it.
    polygon([(70, 104), (82, 30), (106, 72), (128, 10), (150, 72), (174, 30), (186, 104)], CREST)
    # The ear fins: a frill on each side, two points each, behind the head.
    polygon([(60, 108), (8, 76), (26, 132), (6, 180), (66, 178)], FIN)
    polygon([(196, 108), (248, 76), (230, 132), (250, 180), (190, 178)], FIN)

    # The head: one wide round shape, a little wider than tall.
    draw.ellipse(box(128, 150, 96, 88), fill=SKIN, outline=OUTLINE, width=width)
    if not small:
        # A lighter belly under the mouth, cut off by the head's outline.
        draw.chord(box(128, 170, 70, 62), 20, 160, fill=BELLY)

    # The mouth: a wide grin, the lower half of an ellipse, with teeth along its top edge.
    mouth = box(128, 150, 70, 58) if not small else box(128, 152, 72, 54)
    draw.chord(mouth, 0, 180, fill=MOUTH, outline=OUTLINE, width=width)
    if not small:
        top = 150
        for x in (76, 104, 152, 180):
            polygon([(x - 11, top), (x + 11, top), (x, top + 24)], TOOTH, outline=False)

    # The eyes: big and round, set high and wide, the pupils looking a little up and to the middle.
    eye_r = 38 if not small else 39
    pupil_r = 18 if not small else 19
    # Set a little further apart at the small sizes, so a strip of skin keeps the two eyes apart.
    spread = 0 if not small else 6
    for cx, look in ((86 - spread, 6), (170 + spread, -6)):
        cy = 94
        # At the small sizes an outline would turn the eye grey: the skin around it is contrast enough.
        if small:
            draw.ellipse(box(cx, cy, eye_r, eye_r), fill=EYE)
        else:
            draw.ellipse(box(cx, cy, eye_r, eye_r), fill=EYE, outline=OUTLINE, width=width)
        px, py = cx + look, cy - 4
        draw.ellipse(box(px, py, pupil_r, pupil_r), fill=PUPIL)
        if not small:
            draw.ellipse(box(px - 6, py - 7, 6, 6), fill=EYE)

    return image.resize((size, size), Image.LANCZOS)


def main() -> None:
    ICONS.mkdir(parents=True, exist_ok=True)
    frames = [draw_icon(size) for size in ICO_SIZES]
    # Each size is its own drawing: the ICO writer takes the frame whose size matches from append_images
    # instead of scaling the largest one down.
    frames[-1].save(
        OUT,
        format="ICO",
        sizes=[(size, size) for size in ICO_SIZES],
        append_images=frames[:-1],
    )
    draw_icon(PNG_SIZE).save(PNG, format="PNG")
    print(f"wrote {OUT} ({', '.join(str(size) for size in ICO_SIZES)} px) and {PNG} ({PNG_SIZE} px)")


if __name__ == "__main__":
    main()
