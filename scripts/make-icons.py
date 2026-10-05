"""Draws the Tapka icon (the shapes of assets/tapka.svg) at every size the project needs:
icons/icon.ico for the exe, the installer and the tray, assets/logo.png for the README header,
assets/social.png for link previews. Run with: python scripts/make-icons.py"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
TILE, RIM, WHITE = "#364157", "#8ab4ff", "#ffffff"
T = [(62, 70), (194, 70), (194, 98), (143, 98), (143, 190), (113, 190), (113, 98), (62, 98)]


def icon(size: int) -> Image.Image:
    """The icon on a 256 grid, drawn 8 times larger and scaled down for smooth edges."""
    k = size * 8 / 256
    img = Image.new("RGBA", (size * 8, size * 8), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    box = lambda a, b: [a * k, a * k, b * k, b * k]
    d.rounded_rectangle(box(0, 256), radius=56 * k, fill=RIM)
    d.rounded_rectangle(box(14, 242), radius=42 * k, fill=TILE)
    pts = [(x * k, y * k) for x, y in T]
    d.polygon(pts, fill=WHITE)
    # The outline of the letter: 8 units wide with round joins
    d.line(pts + pts[:1], fill=WHITE, width=round(8 * k))
    for x, y in pts:
        d.ellipse([x - 4 * k, y - 4 * k, x + 4 * k, y + 4 * k], fill=WHITE)
    return img.resize((size, size), Image.LANCZOS)


def social() -> Image.Image:
    img = Image.new("RGB", (1280, 640), "#0e0f12")
    img.paste(icon(256), (150, 192), icon(256))
    d = ImageDraw.Draw(img)
    d.text((470, 215), "Tapka", font=ImageFont.truetype("segoeuib.ttf", 120), fill=WHITE)
    d.text((476, 370), "Touch panel for Windows 11", font=ImageFont.truetype("segoeui.ttf", 44), fill="#8b8f9a")
    return img


sizes = [16, 20, 24, 32, 40, 48, 64, 256]
images = [icon(s) for s in sizes]
images[-1].save(ROOT / "icons" / "icon.ico", format="ICO", append_images=images[:-1], sizes=[(s, s) for s in sizes])
(ROOT / "assets").mkdir(exist_ok=True)
icon(256).save(ROOT / "assets" / "logo.png")
social().save(ROOT / "assets" / "social.png")
