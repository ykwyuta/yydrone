"""Prepare the regional fictional portraits (sources-v3) as 160px game assets."""
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
FOLDER = ROOT / "assets" / "command"
NAMES = [name for side in ['ua','ru'] for name in ([f'general_{side}_{i}' for i in range(9)] + [f'staff_{side}'])]

def main():
    preview = Image.new("RGB", (5*180,4*194), (30,35,39))
    draw = ImageDraw.Draw(preview)
    for i,name in enumerate(NAMES):
        image = Image.open(FOLDER / "sources-v3" / f"{name}.png").convert("RGBA")
        image = image.resize((160,160),Image.Resampling.LANCZOS)
        image.save(FOLDER / f"{name}.png")
        (FOLDER / f"{name}.rgba").write_bytes(image.tobytes())
        x,y=(i%5)*180,(i//5)*194
        preview.paste(image,(x+10,y+5),image)
        draw.text((x+8,y+174),name,fill=(230,232,235))
    preview.save(ROOT / "docs" / "command-portraits-preview.png")

if __name__ == "__main__":
    main()
