"""Convert imagegen equipment originals into embedded RGBA textures.

python tools/prepare_equipment_assets.py
Use --available to prepare only sources that have finished generating.
"""

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs" / "equipment-prompts.json"


def prepare(asset):
    folder = ROOT / "assets" / asset["directory"]
    source = folder / "equipment_sources" / f"{asset['id']}.png"
    image = Image.open(source).convert("RGBA")
    alpha = image.getchannel("A")
    if alpha.getextrema() != (0, 255):
        raise ValueError(f"{source}: expected opaque subject and transparent background")
    bounds = alpha.getbbox()
    if bounds is None:
        raise ValueError(f"{source}: empty image")
    size = 192 if asset["directory"] == "map" else 160
    padding = 10
    sprite = image.crop(bounds)
    sprite.thumbnail((size - padding * 2, size - padding * 2), Image.Resampling.LANCZOS)
    result = Image.new("RGBA", (size, size))
    result.alpha_composite(sprite, ((size - sprite.width) // 2, (size - sprite.height) // 2))
    return folder / asset["id"], result


def save(path, image):
    image.save(path.with_suffix(".png"))
    path.with_suffix(".rgba").write_bytes(image.tobytes())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--available", action="store_true")
    args = parser.parse_args()
    assets = json.loads(MANIFEST.read_text(encoding="utf-8"))["assets"]
    if args.available:
        assets = [a for a in assets if (ROOT / "assets" / a["directory"] / "equipment_sources" / f"{a['id']}.png").exists()]
    # Validate all selected sources before replacing any game texture.
    prepared = [prepare(a) for a in assets]
    for path, image in prepared:
        save(path, image)
    # Keep legacy filenames representative of their own weapon category too.
    for alias, source in {"mlrs": "mlrs_ua", "sam": "sam_ua", "jet": "jet_ua", "bomber_drone": "bomber_drone_ua"}.items():
        for path, image in prepared:
            if path.name == source:
                save(path.with_name(alias), image)
    columns, cell_w, cell_h = 6, 208, 220
    rows = max(1, (len(prepared) + columns - 1) // columns)
    preview = Image.new("RGB", (columns * cell_w, rows * cell_h), (51, 61, 48))
    draw = ImageDraw.Draw(preview)
    for index, (path, image) in enumerate(prepared):
        x, y = (index % columns) * cell_w, (index // columns) * cell_h
        preview.paste(image, (x + (cell_w - image.width) // 2, y + (192 - image.height) // 2), image)
        draw.text((x + 8, y + 198), path.name, fill=(240, 240, 225))
    preview.save(ROOT / "docs" / "equipment-assets-preview.png")
    print(f"Prepared {len(prepared)} generated equipment assets.")


if __name__ == "__main__":
    main()
