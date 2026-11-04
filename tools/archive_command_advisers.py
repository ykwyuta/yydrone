"""Preserve earlier portraits as future military-adviser candidates.

Original files remain in sources/ and sources-v2/; this makes named preview
copies and a catalog without deleting or overwriting those originals.
"""
import json
import hashlib
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
FOLDER = ROOT / "assets" / "command"

def main():
    records = json.loads((ROOT / "docs/command-adviser-prompts.json").read_text(encoding="utf-8"))["portraits"]
    archive = FOLDER / "advisers"
    archive.mkdir(exist_ok=True)
    preview = Image.new("RGB",(7*172,4*190),(30,35,39))
    draw = ImageDraw.Draw(preview)
    catalog = []
    for index, record in enumerate(records):
        source = FOLDER / record["series"] / (record["id"] + ".png")
        if not source.exists():
            raise FileNotFoundError(source)
        name = f"adviser_{index+1:02}"
        image = Image.open(source).convert("RGBA").resize((160,160),Image.Resampling.LANCZOS)
        target = archive / f"{name}.png"
        image.save(target)
        catalog.append({"id":name,"role":"軍事顧問候補（将来用・現在の将軍名簿には未登録）","portrait":target.relative_to(ROOT).as_posix(),"original":source.relative_to(ROOT).as_posix(),"original_sha256":hashlib.sha256(source.read_bytes()).hexdigest(),"prompt_manifest":"docs/command-adviser-prompts.json","prompt_id":record["series"]+"/"+record["id"]})
        x,y=index%7*172,index//7*190
        preview.paste(image,(x+6,y+4),image)
        draw.text((x+8,y+170),name,fill=(230,232,235))
    (archive / "catalog.json").write_text(json.dumps(catalog,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    preview.save(ROOT / "docs/command-advisers-preview.png")
    print(f"Preserved {len(catalog)} military-adviser portraits; original files retained.")

if __name__ == "__main__":
    main()
