import os
from PIL import Image, ImageFilter, ImageDraw, ImageEnhance, ImageChops

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

# 1. Perfectly cut out SAM vehicle
sam_raw = Image.open("assets/units/sam_test.png").convert("RGBA")
sw, sh = sam_raw.size
# Draw precise mask around SAM truck and missile rack
mask = Image.new("L", (sw, sh), 0)
mdraw = ImageDraw.Draw(mask)
sam_poly = [
    (68, 175), (96, 125), (120, 132), (125, 52), (145, 50),
    (158, 62), (180, 58), (242, 92), (250, 115), (235, 140),
    (240, 155), (220, 185), (175, 195), (125, 235), (85, 235),
    (65, 195)
]
mdraw.polygon(sam_poly, fill=255)
mask = mask.filter(ImageFilter.GaussianBlur(1.0))
sam_raw.putalpha(mask)
sam_clean = fit_to_box(sam_raw, 160)
sam_clean.save("assets/units/sam.png")
save_raw_rgba(sam_clean, "assets/units/sam.rgba")
print("Clean SAM saved!")

# 2. Modern Capital Metropolis (Kyiv / Rostov style)
# High resolution blend of fortified citadel + modern communications & radar
cap_base = Image.open("references/yyempire/assets/map/capital.png").convert("RGBA")
# Color grade towards modern slate stone & military concrete
cap_enh = ImageEnhance.Color(cap_base).enhance(0.7)
cap_enh = ImageEnhance.Brightness(cap_enh).enhance(0.95)

# Add military base communication mast to capital corner
base_img = Image.open("assets/map/military_base.png").convert("RGBA")
# Extract communication tower
mast = base_img.crop((52, 16, 76, 80)).resize((22, 60), Image.Resampling.LANCZOS)
cap_enh.paste(mast, (50, 48), mast)

# Add central radar dome & antenna to the central building roof
cdraw = ImageDraw.Draw(cap_enh)
# Rooftop radar dome
cdraw.ellipse([(90, 50), (102, 62)], fill=(230, 235, 240, 255), outline=(100, 110, 120, 255))
cdraw.line([(96, 44), (96, 50)], fill=(220, 220, 220, 255), width=2)
cdraw.ellipse([(94, 42), (98, 46)], fill=(255, 40, 40, 255)) # Red blinking light

# Helipad in courtyard
cdraw.ellipse([(85, 96), (107, 108)], outline=(240, 215, 50, 255), width=2)
cdraw.text((93, 97), "H", fill=(240, 215, 50, 255))

cap_final = fit_to_box(cap_enh, 192)
cap_final.save("assets/map/city_capital.png")
save_raw_rgba(cap_final, "assets/map/city_capital.rgba")
print("city_capital done!")

# 3. Modern Fortress Town (Frontline Cities: Bakhmut / Kramatorsk / Izyum)
town_base = Image.open("references/yyempire/assets/map/town.png").convert("RGBA")
town_enh = ImageEnhance.Color(town_base).enhance(0.75)
town_enh.paste(mast, (46, 55), mast)
tdraw = ImageDraw.Draw(town_enh)
# Sandbag revetments & modern radio beacon
tdraw.ellipse([(94, 45), (98, 49)], fill=(40, 255, 40, 255))
town_final = fit_to_box(town_enh, 192)
town_final.save("assets/map/city_town.png")
save_raw_rgba(town_final, "assets/map/city_town.rgba")
print("city_town done!")

# 4. Heavy Industrial City (Kharkiv / Dnipro / Donetsk)
# Blend of drone workshop + factory components with fortified perimeters
work_img = Image.open("assets/map/drone_workshop.png").convert("RGBA")
ind_base = work_img.copy()
idraw = ImageDraw.Draw(ind_base)
# Heavy factory smokestacks & refinery pipes
idraw.rectangle([(136, 18), (144, 65)], fill=(160, 70, 60, 255), outline=(50, 50, 50, 255))
idraw.rectangle([(136, 26), (144, 34)], fill=(240, 240, 240, 255))
idraw.rectangle([(136, 42), (144, 50)], fill=(240, 240, 240, 255))
# Smoke
idraw.ellipse([(132, 6), (148, 20)], fill=(180, 180, 185, 140))
# Railway lines on foreground
idraw.line([(25, 160), (160, 145)], fill=(140, 130, 110, 220), width=2)
ind_final = fit_to_box(ind_base, 192)
ind_final.save("assets/map/city_industrial.png")
save_raw_rgba(ind_final, "assets/map/city_industrial.rgba")
print("city_industrial done!")

# 5. Port City (Odesa / Sevastopol / Mariupol)
air_img = Image.open("assets/map/airfield.png").convert("RGBA")
port_base = air_img.copy()
pdraw = ImageDraw.Draw(port_base)
# Add deep water quayside at bottom right
pdraw.polygon([(110, 150), (185, 110), (185, 185), (110, 185)], fill=(35, 70, 95, 255))
pdraw.line([(110, 150), (185, 110)], fill=(180, 190, 200, 255), width=3) # Pier edge
# Gantry crane
pdraw.line([(135, 140), (135, 95)], fill=(235, 150, 30, 255), width=3)
pdraw.line([(120, 95), (160, 95)], fill=(235, 150, 30, 255), width=3)
port_final = fit_to_box(port_base, 192)
port_final.save("assets/map/city_port.png")
save_raw_rgba(port_final, "assets/map/city_port.rgba")
print("city_port done!")

print("All enhanced city & SAM sprites successfully created!")
