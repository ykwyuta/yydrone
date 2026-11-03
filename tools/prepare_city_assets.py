"""Prepare imagegen city originals for the game's 192px embedded textures.

Run from any directory: python tools/prepare_city_assets.py
Requires Pillow. Originals and their alpha channels are kept unchanged.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
MAP = ROOT / "assets" / "map"
CITIES = (
    "kyiv", "kharkiv", "odesa", "dnipro", "lviv", "rostov",
    "sevastopol", "mariupol", "donetsk", "bakhmut",
)
ALIASES = {"capital": "kyiv", "port": "odesa", "industrial": "mariupol", "town": "bakhmut"}
SIZE = 192
PADDING = 10


def prepare(city):
    source = Image.open(MAP / "city_sources" / f"{city}.png").convert("RGBA")
    alpha = source.getchannel("A")
    assert alpha.getextrema()[0] == 0, f"{city}: missing transparent background"
    bounds = alpha.getbbox()
    assert bounds is not None, f"{city}: empty image"
    sprite = source.crop(bounds)
    sprite.thumbnail((SIZE - 2 * PADDING, SIZE - 2 * PADDING), Image.Resampling.LANCZOS)
    result = Image.new("RGBA", (SIZE, SIZE))
    result.alpha_composite(sprite, ((SIZE - sprite.width) // 2, SIZE - PADDING - sprite.height))
    return result


def save(sprite, city):
    sprite.save(MAP / f"city_{city}.png")
    raw = sprite.tobytes()
    assert len(raw) == SIZE * SIZE * 4
    (MAP / f"city_{city}.rgba").write_bytes(raw)


def main():
    # Read every source before replacing assets, so missing files fail early.
    sprites = {city: prepare(city) for city in CITIES}
    for city, sprite in sprites.items():
        save(sprite, city)
    for alias, city in ALIASES.items():
        save(sprites[city], alias)
    preview = Image.new("RGB", (5 * 224, 2 * 232), (51, 61, 48))
    draw = ImageDraw.Draw(preview)
    for index, (city, sprite) in enumerate(sprites.items()):
        x, y = (index % 5) * 224, (index // 5) * 232
        preview.paste(sprite, (x + 16, y + 8), sprite)
        draw.text((x + 16, y + 207), city, fill=(240, 240, 225))
    preview.save(ROOT / "docs" / "city-assets-preview.png")
    print(f"Prepared {len(sprites)} city sprites and {len(ALIASES)} fallback aliases.")


if __name__ == "__main__":
    main()
