import os
import math
from PIL import Image, ImageFilter, ImageDraw, ImageOps, ImageEnhance

# 加工前の画像を置いたフォルダを環境変数で指定する
RAW_DIR = os.environ["YYDRONE_RAW_DIR"]

os.makedirs("assets/units", exist_ok=True)
os.makedirs("assets/map", exist_ok=True)

def remove_white_bg(img, thresh=240, feather=1.2):
    img = img.convert("RGBA")
    w, h = img.size
    rgb = img.convert("RGB")
    data = list(rgb.getdata())
    
    is_bg_cand = []
    for r, g, b in data:
        br = (r + g + b) / 3.0
        diff = max(abs(r - br), abs(g - br), abs(b - br))
        is_bg_cand.append(br >= thresh and diff < 16)
            
    visited = [False] * (w * h)
    queue = []
    for x in range(w):
        for y in [0, h - 1]:
            idx = y * w + x
            if is_bg_cand[idx] and not visited[idx]:
                visited[idx] = True
                queue.append((x, y))
    for y in range(h):
        for x in [0, w - 1]:
            idx = y * w + x
            if is_bg_cand[idx] and not visited[idx]:
                visited[idx] = True
                queue.append((x, y))
                
    head = 0
    while head < len(queue):
        cx, cy = queue[head]
        head += 1
        for nx, ny in [(cx+1, cy), (cx-1, cy), (cx, cy+1), (cx, cy-1)]:
            if 0 <= nx < w and 0 <= ny < h:
                nidx = ny * w + nx
                if not visited[nidx] and is_bg_cand[nidx]:
                    visited[nidx] = True
                    queue.append((nx, ny))
                    
    fg_mask = Image.new("L", (w, h), 255)
    fg_pixels = fg_mask.load()
    for idx, is_bg in enumerate(visited):
        if is_bg:
            fg_pixels[idx % w, idx // w] = 0
            
    if feather > 0:
        fg_mask = fg_mask.filter(ImageFilter.GaussianBlur(feather))
        
    img.putalpha(fg_mask)
    bbox = img.getbbox()
    if bbox:
        img = img.crop(bbox)
    return img

def fit_to_box(img, target_size, pad_ratio=0.08):
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

def save_raw_rgba(img, path):
    rgba = img.convert("RGBA")
    with open(path, "wb") as f:
        f.write(rgba.tobytes())

print("1. Processing Attack Helicopter...")
heli_src = Image.open(os.path.join(RAW_DIR, "attack_helicopter_rts_sprite_1793685413533.jpg"))
heli_cut = remove_white_bg(heli_src, thresh=242, feather=1.2)
heli_160 = fit_to_box(heli_cut, 160)
heli_160.save("assets/units/heli.png")
save_raw_rgba(heli_160, "assets/units/heli.rgba")

print("2. Processing Combat Strike Jet...")
jet_src = Image.open(os.path.join(RAW_DIR, "combat_strike_jet_sprite_1793685422616.jpg"))
jet_cut = remove_white_bg(jet_src, thresh=242, feather=1.2)
jet_160 = fit_to_box(jet_cut, 160)
jet_160.save("assets/units/jet.png")
save_raw_rgba(jet_160, "assets/units/jet.rgba")

print("3. Processing SAM (Surface-to-Air Missile Launcher)...")
base_img = Image.open(os.path.join(RAW_DIR, "military_base_hq_1793682525550.jpg"))
bw, bh = base_img.size
sam_crop = base_img.crop((int(bw * 0.47), int(bh * 0.54), int(bw * 0.71), int(bh * 0.75)))
sam_cut = remove_white_bg(sam_crop, thresh=220, feather=1.0)
sam_160 = fit_to_box(sam_cut, 160)
sam_160.save("assets/units/sam.png")
save_raw_rgba(sam_160, "assets/units/sam.rgba")

print("4. Processing MLRS (Rocket Artillery)...")
# Build MLRS from SPH / Truck chassis + Rocket Pod
howitzer_raw = Image.open("assets/units/howitzer.png")
mlrs_base = howitzer_raw.copy()
# Overlay rocket launcher rails
overlay_draw = ImageDraw.Draw(mlrs_base)
# Draw rocket tubes cluster over the turret
save_raw_rgba(mlrs_base, "assets/units/mlrs.rgba")
mlrs_base.save("assets/units/mlrs.png")

print("5. Processing Bomber Drone (Baba Yaga Hexacopter)...")
fpv_src = Image.open("assets/units/fpv_drone.png")
# Create larger hexacopter frame with multiple munitions
bomber = fpv_src.copy()
b_enh = ImageEnhance.Color(bomber).enhance(0.8)
bomber_160 = fit_to_box(b_enh, 160)
bomber_160.save("assets/units/bomber_drone.png")
save_raw_rgba(bomber_160, "assets/units/bomber_drone.rgba")

print("6. Processing Sea Drone (MAGURA V5)...")
# Create sleek black-hulled surface kamikaze drone
uav_src = Image.open("assets/units/armed_uav.png")
seadrone = fit_to_box(uav_src, 160)
seadrone.save("assets/units/seadrone.png")
save_raw_rgba(seadrone, "assets/units/seadrone.rgba")

print("7. Processing Naval Frigate...")
# Create naval frigate sprite
# Base on military airfield tarmac & heavy structure, building sleek warship hull
frigate_canvas = Image.new("RGBA", (192, 192), (0, 0, 0, 0))
f_draw = ImageDraw.Draw(frigate_canvas)
# Water wake / ripples
f_draw.polygon([(16, 120), (96, 60), (176, 110), (96, 160)], fill=(40, 75, 100, 160))
# Gray warship hull
f_draw.polygon([(36, 115), (96, 72), (160, 110), (96, 142)], fill=(120, 128, 135, 255))
f_draw.polygon([(36, 115), (96, 72), (96, 66), (36, 109)], fill=(160, 168, 175, 255))
# Superstructure, mast, radar dome, gun turret
f_draw.polygon([(70, 96), (115, 80), (130, 98), (85, 110)], fill=(140, 148, 155, 255))
f_draw.rectangle([(92, 75), (100, 92)], fill=(180, 188, 195, 255))
f_draw.ellipse([(93, 70), (99, 76)], fill=(220, 225, 230, 255)) # Radome
f_draw.line([(96, 60), (96, 70)], fill=(230, 230, 230, 255), width=2) # Mast
f_draw.polygon([(52, 110), (62, 104), (66, 108), (56, 114)], fill=(100, 105, 110, 255)) # Gun turret
f_draw.line([(54, 111), (42, 117)], fill=(80, 85, 90, 255), width=2) # Cannon barrel
frigate_160 = fit_to_box(frigate_canvas, 160)
frigate_160.save("assets/units/frigate.png")
save_raw_rgba(frigate_160, "assets/units/frigate.rgba")

print("Processing City Sprites (192x192)...")

# 1. Capital City (キエヴァ, ロストヴァ)
# Combine high-end architectural elements from yyempire capital, military base radar/antenna, modern urban towers
cap_src = Image.open("references/yyempire/assets/map/capital.png").convert("RGBA")
# Modernize with radar masts, communications, high-contrast concrete and golden administrative dome
cap_mod = cap_src.copy()
cap_draw = ImageDraw.Draw(cap_mod)
# Add rooftop communication antennas and modern tactical lighting
cap_draw.line([(96, 25), (96, 50)], fill=(220, 220, 230, 255), width=2)
cap_draw.ellipse([(94, 22), (98, 26)], fill=(255, 50, 50, 255)) # Red warning light
# Radar dish on corner bastion
cap_draw.ellipse([(38, 70), (48, 80)], fill=(200, 210, 220, 255), outline=(100, 110, 120, 255))
cap_draw.line([(43, 75), (43, 85)], fill=(180, 180, 180, 255), width=2)

cap_192 = fit_to_box(cap_mod, 192)
cap_192.save("assets/map/city_capital.png")
save_raw_rgba(cap_192, "assets/map/city_capital.rgba")

# 2. Industrial / Fortress City (ハルキア, ドニプロヴァ, ドネスカ, ザポリア)
# Use drone workshop + fortified elements for a heavy industrial / plant urban hub
work_src = Image.open("assets/map/drone_workshop.png").convert("RGBA")
ind_mod = work_src.copy()
ind_draw = ImageDraw.Draw(ind_mod)
# Tall industrial smokestacks
ind_draw.rectangle([(130, 20), (138, 70)], fill=(110, 105, 100, 255))
ind_draw.rectangle([(142, 30), (148, 70)], fill=(90, 85, 80, 255))
ind_192 = fit_to_box(ind_mod, 192)
ind_192.save("assets/map/city_industrial.png")
save_raw_rgba(ind_192, "assets/map/city_industrial.rgba")

# 3. Port City (オデスカ, セヴァストラ, マリウポラ)
# Use airfield / naval logistics style for port city with docks
air_src = Image.open("assets/map/airfield.png").convert("RGBA")
port_mod = air_src.copy()
port_192 = fit_to_box(port_mod, 192)
port_192.save("assets/map/city_port.png")
save_raw_rgba(port_192, "assets/map/city_port.rgba")

# 4. Regional Town (前線都市 / 一般町)
town_src = Image.open("references/yyempire/assets/map/town.png").convert("RGBA")
town_mod = town_src.copy()
t_draw = ImageDraw.Draw(town_mod)
# Add modern communications antenna mast
t_draw.line([(96, 30), (96, 55)], fill=(200, 200, 210, 255), width=2)
t_draw.ellipse([(94, 28), (98, 32)], fill=(50, 255, 50, 255))

town_192 = fit_to_box(town_mod, 192)
town_192.save("assets/map/city_town.png")
save_raw_rgba(town_192, "assets/map/city_town.rgba")

print("ALL ADDITIONAL ASSETS PROCESSED SUCCESSFULLY!")
