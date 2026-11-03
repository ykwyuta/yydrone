import os
from PIL import Image

def clean_and_defringe(img):
    img = img.convert("RGBA")
    w, h = img.size
    data = list(img.getdata())
    new_data = []
    
    for r, g, b, a in data:
        if a == 0:
            new_data.append((0, 0, 0, 0))
            continue
        
        min_c = min(r, g, b)
        sat = max(r, g, b) - min_c
        br = (r + g + b) / 3.0
        
        # 1. Solid or near-solid white background (whether outer or enclosed hole)
        if min_c >= 215 or (br >= 220 and sat <= 25):
            new_data.append((0, 0, 0, 0))
        # 2. Solid foreground
        elif min_c <= 155:
            new_data.append((r, g, b, a))
        # 3. Transition edge: defringe against white
        else:
            alpha = (215.0 - min_c) / (215.0 - 155.0)
            alpha = max(0.0, min(1.0, alpha))
            new_a = int(a * alpha)
            if new_a <= 8:
                new_data.append((0, 0, 0, 0))
            else:
                # Recover foreground: C_fg = (C - (1 - alpha)*255) / alpha
                dr = max(0, min(255, int((r - (1.0 - alpha) * 255.0) / alpha)))
                dg = max(0, min(255, int((g - (1.0 - alpha) * 255.0) / alpha)))
                db = max(0, min(255, int((b - (1.0 - alpha) * 255.0) / alpha)))
                new_data.append((dr, dg, db, new_a))
                
    out = Image.new("RGBA", (w, h))
    out.putdata(new_data)
    return out

def process_all():
    # Process units
    unit_dir = "assets/units"
    for f in sorted(os.listdir(unit_dir)):
        if not f.endswith(".png"): continue
        path = os.path.join(unit_dir, f)
        img = Image.open(path)
        cleaned = clean_and_defringe(img)
        cleaned.save(path)
        
        rgba_path = os.path.splitext(path)[0] + ".rgba"
        with open(rgba_path, "wb") as rf:
            rf.write(cleaned.convert("RGBA").tobytes())
            
        cnt = sum(1 for r,g,b,a in cleaned.getdata() if a > 30 and min(r,g,b) > 200)
        print(f"Unit {f:20s}: size={cleaned.size}, remaining white={cnt}, rgba={os.path.getsize(rgba_path)} bytes")
        
    # Process map
    map_dir = "assets/map"
    for f in sorted(os.listdir(map_dir)):
        if not f.endswith(".png"): continue
        path = os.path.join(map_dir, f)
        img = Image.open(path)
        cleaned = clean_and_defringe(img)
        cleaned.save(path)
        
        rgba_path = os.path.splitext(path)[0] + ".rgba"
        with open(rgba_path, "wb") as rf:
            rf.write(cleaned.convert("RGBA").tobytes())
            
        cnt = sum(1 for r,g,b,a in cleaned.getdata() if a > 30 and min(r,g,b) > 200)
        print(f"Map  {f:20s}: size={cleaned.size}, remaining white={cnt}, rgba={os.path.getsize(rgba_path)} bytes")

if __name__ == "__main__":
    process_all()
