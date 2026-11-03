import os
import sys
from PIL import Image, ImageFilter, ImageDraw

# 加工前の画像を置いたフォルダを環境変数で指定する
RAW_DIR = os.environ["YYDRONE_RAW_DIR"]

RAW_FILES = {
    "icon": os.path.join(RAW_DIR, "drone_front_app_icon_1793682491832.jpg"),
    "title": os.path.join(RAW_DIR, "drone_front_title_splash_1793682497422.jpg"),
    "tank": os.path.join(RAW_DIR, "mbt_drone_cage_sprite_1793682508083.jpg"),
    "fpv_drone": os.path.join(RAW_DIR, "rts_military_fpv_drone_sprite_1793682509439.jpg"),
    "infantry": os.path.join(RAW_DIR, "modern_tactical_infantry_rts_sprite_1793682519553.jpg"),
    "ifv": os.path.join(RAW_DIR, "ifv_unit_sprite_1793682553118.jpg"),
    "howitzer": os.path.join(RAW_DIR, "sph_howitzer_sprite_1793682578678.jpg"),
    "armed_uav": os.path.join(RAW_DIR, "armed_male_uav_sprite_1793682599457.jpg"),
    "military_base": os.path.join(RAW_DIR, "military_base_hq_1793682525550.jpg"),
    "airfield": os.path.join(RAW_DIR, "military_airfield_sprite_png_1793682584265.jpg"),
    "drone_workshop": os.path.join(RAW_DIR, "drone_workshop_sprite_png_1793682591180.jpg"),
    "trench_fort": os.path.join(RAW_DIR, "fortified_trench_strongpoint_1793682628700.jpg"),
}

os.makedirs("assets/units", exist_ok=True)
os.makedirs("assets/map", exist_ok=True)
os.makedirs("docs/images", exist_ok=True)

def remove_white_bg(img, thresh=240, feather=1.5):
    """Cleanly extracts sprites on solid white/near-white backgrounds."""
    img = img.convert("RGBA")
    w, h = img.size
    
    # 1. Detect background pixels by brightness and low saturation
    rgb = img.convert("RGB")
    data = list(rgb.getdata())
    
    # Simple flood fill from boundaries or threshold
    # To avoid cutting highlights inside the sprite, start from border
    # Create mask: 255 = foreground, 0 = background
    mask = Image.new("L", (w, h), 0)
    mask_draw = list(mask.getdata())
    
    # Detect candidate bg pixels
    is_bg_cand = []
    for r, g, b in data:
        br = (r + g + b) / 3.0
        diff = max(abs(r - br), abs(g - br), abs(b - br))
        if br >= thresh and diff < 16:
            is_bg_cand.append(True)
        else:
            is_bg_cand.append(False)
            
    # Breadth-first search from image edges to find connected background
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
                    
    # Foreground mask: 255 if not connected background
    fg_mask = Image.new("L", (w, h), 255)
    fg_pixels = fg_mask.load()
    for idx, is_bg in enumerate(visited):
        if is_bg:
            fg_pixels[idx % w, idx // w] = 0
            
    # Smooth alpha edges slightly
    if feather > 0:
        fg_mask = fg_mask.filter(ImageFilter.GaussianBlur(feather))
        
    img.putalpha(fg_mask)
    
    # Crop to non-transparent bounding box
    bbox = img.getbbox()
    if bbox:
        img = img.crop(bbox)
        
    return img

def fit_to_box(img, target_size, pad_ratio=0.08):
    """Scales img maintaining aspect ratio into target_size x target_size box."""
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
    """Saves raw 32-bit RGBA byte stream (u32 little endian: R, G, B, A)."""
    rgba = img.convert("RGBA")
    with open(path, "wb") as f:
        f.write(rgba.tobytes())

print("Processing App Icon...")
# Process Icon
icon_src = Image.open(RAW_FILES["icon"])
# Icon is already a square-ish badge, crop to 1:1 if needed
iw, ih = icon_src.size
dim = min(iw, ih)
icon_square = icon_src.crop(((iw - dim)//2, (ih - dim)//2, (iw + dim)//2, (ih + dim)//2))

# Save 512, 192, 180
icon_512 = icon_square.resize((512, 512), Image.Resampling.LANCZOS)
icon_512.save("assets/icon.png")
icon_512.save("web/icon-512.png")

icon_192 = icon_square.resize((192, 192), Image.Resampling.LANCZOS)
icon_192.save("web/icon-192.png")

icon_180 = icon_square.resize((180, 180), Image.Resampling.LANCZOS)
icon_180.save("web/apple-touch-icon.png")

# Windows ICO file with multiple sizes
icon_square.save("assets/app_icon.ico", format="ICO", sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print("App Icon done!")

print("Processing Title Art...")
# Process Title Image
title_src = Image.open(RAW_FILES["title"])
title_1600 = title_src.resize((1600, 900), Image.Resampling.LANCZOS)
title_1600.save("assets/title.jpg", quality=92)
title_1600.save("docs/images/title.jpg", quality=92)

# Also generate a 800x450 raw RGBA for in-game title screen background
title_800 = title_src.resize((800, 450), Image.Resampling.LANCZOS)
save_raw_rgba(title_800, "assets/title.rgba")
print("Title Art done!")

print("Processing Unit Sprites (160x160)...")
unit_names = ["tank", "fpv_drone", "infantry", "ifv", "howitzer", "armed_uav"]
for name in unit_names:
    src_path = RAW_FILES[name]
    print(f"  Extracting {name}...")
    src = Image.open(src_path)
    cut = remove_white_bg(src, thresh=242, feather=1.2)
    final_unit = fit_to_box(cut, 160)
    final_unit.save(f"assets/units/{name}.png")
    save_raw_rgba(final_unit, f"assets/units/{name}.rgba")
print("Unit Sprites done!")

print("Processing Map Building Sprites (192x192)...")
map_names = ["military_base", "airfield", "drone_workshop", "trench_fort"]
for name in map_names:
    src_path = RAW_FILES[name]
    print(f"  Extracting {name}...")
    src = Image.open(src_path)
    cut = remove_white_bg(src, thresh=242, feather=1.2)
    final_bld = fit_to_box(cut, 192)
    final_bld.save(f"assets/map/{name}.png")
    save_raw_rgba(final_bld, f"assets/map/{name}.rgba")
print("Map Sprites done!")

print("ALL ASSETS PROCESSED SUCCESSFULLY!")
