import os
from PIL import Image, ImageFilter, ImageDraw, ImageEnhance

def save_raw_rgba(img, path):
    rgba = img.convert("RGBA")
    with open(path, "wb") as f:
        f.write(rgba.tobytes())

def fit_to_box(img, target_size, pad_ratio=0.06):
    target_w, target_h = target_size, target_size
    inner_w = int(target_w * (1.0 - pad_ratio * 2))
    inner_h = int(target_h * (1.0 - pad_ratio * 2))
    w, h = img.size
    ratio = min(inner_w / float(w), inner_h / float(h))
    new_w, new_h = max(1, int(w * ratio)), max(1, int(h * ratio))
    resized = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
    out = Image.new("RGBA", (target_w, target_h), (0, 0, 0, 0))
    paste_x = (target_w - new_w) // 2
    paste_y = (target_h - new_h) // 2
    out.paste(resized, (paste_x, paste_y), resized)
    return out

# 1. City: Capital (192x192) - Kyiv / Rostov
# Use yyempire's magnificent capital.png with modern command tower
cap_base = Image.open("references/yyempire/assets/map/capital.png").convert("RGBA")
base_img = Image.open("assets/map/military_base.png").convert("RGBA")
# Communication antenna on west bastion
mast = base_img.crop((52, 16, 76, 80)).resize((20, 55), Image.Resampling.LANCZOS)
cap_base.paste(mast, (50, 45), mast)
cap_192 = fit_to_box(cap_base, 192)
cap_192.save("assets/map/city_capital.png")
save_raw_rgba(cap_192, "assets/map/city_capital.rgba")

# 2. City: Major Town (192x192) - Kharkiv / Dnipro / Donetsk
town_base = Image.open("references/yyempire/assets/map/town.png").convert("RGBA")
town_base.paste(mast, (46, 52), mast)
town_192 = fit_to_box(town_base, 192)
town_192.save("assets/map/city_town.png")
save_raw_rgba(town_192, "assets/map/city_town.rgba")

# 3. City: Port (192x192) - Odesa / Sevastopol / Mariupol
# Airfield hangar + quay
port_base = Image.open("assets/map/airfield.png").convert("RGBA")
pdraw = ImageDraw.Draw(port_base)
# Subtle coastal water at bottom right
pdraw.polygon([(125, 145), (185, 110), (185, 185), (125, 185)], fill=(32, 65, 95, 230))
pdraw.line([(125, 145), (185, 110)], fill=(160, 180, 200, 255), width=2)
# Yellow port crane
pdraw.line([(145, 135), (145, 80)], fill=(235, 160, 25, 255), width=3)
pdraw.line([(130, 80), (165, 80)], fill=(235, 160, 25, 255), width=3)
port_192 = fit_to_box(port_base, 192)
port_192.save("assets/map/city_port.png")
save_raw_rgba(port_192, "assets/map/city_port.rgba")

# 4. City: Industrial (192x192) - Industrial center
work_base = Image.open("assets/map/drone_workshop.png").convert("RGBA")
wdraw = ImageDraw.Draw(work_base)
wdraw.rectangle([(136, 20), (144, 65)], fill=(155, 65, 55, 255), outline=(40, 40, 40, 255))
wdraw.rectangle([(136, 28), (144, 36)], fill=(240, 240, 240, 255))
wdraw.rectangle([(136, 44), (144, 52)], fill=(240, 240, 240, 255))
ind_192 = fit_to_box(work_base, 192)
ind_192.save("assets/map/city_industrial.png")
save_raw_rgba(ind_192, "assets/map/city_industrial.rgba")

print("Cities generated successfully!")
