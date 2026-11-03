import os
import math
from PIL import Image, ImageDraw, ImageFilter

def create_team_sprites():
    # 加工前の画像を置いたフォルダを環境変数で指定する
    src_path = os.path.join(os.environ["YYDRONE_RAW_DIR"], "modern_tactical_infantry_rts_sprite_1793682519553.jpg")
    src = Image.open(src_path).convert("RGB")
    w, h = src.size

    # Clean the source image with master alpha matting
    out_rgba = []
    for r, g, b in src.getdata():
        min_c = min(r, g, b)
        br = (r + g + b) / 3.0
        sat = max(r, g, b) - min_c
        if min_c >= 220 or (br >= 225 and sat <= 25):
            out_rgba.append((0, 0, 0, 0))
        elif min_c <= 155:
            out_rgba.append((r, g, b, 255))
        else:
            alpha = (220.0 - min_c) / (220.0 - 155.0)
            alpha = max(0.0, min(1.0, alpha))
            new_a = int(alpha * 255)
            if new_a <= 8:
                out_rgba.append((0, 0, 0, 0))
            else:
                dr = max(0, min(255, int((r - (1.0 - alpha) * 255.0) / alpha)))
                dg = max(0, min(255, int((g - (1.0 - alpha) * 255.0) / alpha)))
                db = max(0, min(255, int((b - (1.0 - alpha) * 255.0) / alpha)))
                out_rgba.append((dr, dg, db, new_a))

    full_clean = Image.new("RGBA", (w, h))
    full_clean.putdata(out_rgba)

    # Soldier crops
    s1 = full_clean.crop((60, 200, 420, 850))
    s1 = s1.crop(s1.getbbox())

    s2 = full_clean.crop((350, 120, 720, 880))
    s2 = s2.crop(s2.getbbox())

    s3 = full_clean.crop((650, 220, 980, 850))
    s3 = s3.crop(s3.getbbox())

    def fit_and_save(canvas, name):
        bbox = canvas.getbbox()
        if bbox:
            canvas = canvas.crop(bbox)
        
        target_sz = 160
        pad = 12
        inner = target_sz - pad * 2
        cw, ch = canvas.size
        scale = min(inner / cw, inner / ch)
        nw, nh = max(1, int(cw * scale)), max(1, int(ch * scale))
        
        res = canvas.resize((nw, nh), Image.Resampling.LANCZOS)
        out = Image.new("RGBA", (target_sz, target_sz), (0, 0, 0, 0))
        paste_x = (target_sz - nw) // 2
        paste_y = (target_sz - nh) // 2
        out.paste(res, (paste_x, paste_y), res)
        
        # Second-pass defringe on final 160x160 to guarantee 0 white fringe
        final_data = []
        for r, g, b, a in out.getdata():
            if a == 0:
                final_data.append((0, 0, 0, 0))
                continue
            min_c = min(r, g, b)
            sat = max(r, g, b) - min_c
            br = (r + g + b) / 3.0
            if min_c >= 215 or (br >= 220 and sat <= 25):
                final_data.append((0, 0, 0, 0))
            elif min_c <= 155:
                final_data.append((r, g, b, a))
            else:
                alpha = (215.0 - min_c) / (215.0 - 155.0)
                alpha = max(0.0, min(1.0, alpha))
                new_a = int(a * alpha)
                if new_a <= 8:
                    final_data.append((0, 0, 0, 0))
                else:
                    dr = max(0, min(255, int((r - (1.0 - alpha) * 255.0) / alpha)))
                    dg = max(0, min(255, int((g - (1.0 - alpha) * 255.0) / alpha)))
                    db = max(0, min(255, int((b - (1.0 - alpha) * 255.0) / alpha)))
                    final_data.append((dr, dg, db, new_a))
        out.putdata(final_data)
        
        png_path = f"assets/units/{name}.png"
        rgba_path = f"assets/units/{name}.rgba"
        out.save(png_path)
        with open(rgba_path, "wb") as f:
            f.write(out.convert("RGBA").tobytes())
        print(f"Saved {name}: {png_path} ({os.path.getsize(png_path)} bytes), {rgba_path} ({os.path.getsize(rgba_path)} bytes)")

    # 1. INFANTRY (歩兵 - Assault Fireteam)
    canvas_inf = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    canvas_inf.paste(s1, (60, 220), s1)
    canvas_inf.paste(s2, (350, 120), s2)
    canvas_inf.paste(s3, (650, 240), s3)
    fit_and_save(canvas_inf, "infantry")

    # 2. ATGM TEAM (対戦車班 - Javelin / Kornet Anti-Tank Missile Team)
    canvas_atgm = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    # Spotter on left
    s_spotter = s1.resize((int(s1.width * 0.88), int(s1.height * 0.88)), Image.Resampling.LANCZOS)
    canvas_atgm.paste(s_spotter, (120, 260), s_spotter)
    # Gunner in center-right
    s_gunner = s2.resize((int(s2.width * 0.92), int(s2.height * 0.92)), Image.Resampling.LANCZOS)
    canvas_atgm.paste(s_gunner, (380, 160), s_gunner)

    # Draw Heavy ATGM Missile Launcher (Javelin / Kornet tube on shoulder / tripod)
    draw_atgm = ImageDraw.Draw(canvas_atgm)
    # Tripod legs
    tripod_col = (38, 42, 38, 255)
    draw_atgm.line([(480, 480), (430, 680)], fill=tripod_col, width=12)
    draw_atgm.line([(480, 480), (510, 700)], fill=tripod_col, width=12)
    draw_atgm.line([(480, 480), (580, 660)], fill=tripod_col, width=12)
    # Missile Tube: angled thick cylinder pointing forward-up (towards top-left / center)
    tube_start = (340, 310)
    tube_end = (660, 480)
    # Main olive-drab launcher tube
    draw_atgm.line([tube_start, tube_end], fill=(58, 68, 52, 255), width=44)
    draw_atgm.line([tube_start, tube_end], fill=(74, 86, 66, 255), width=32)
    # Black shock-absorbing end rings
    cap1 = (320, 298)
    draw_atgm.line([cap1, (355, 318)], fill=(28, 30, 32, 255), width=52)
    draw_atgm.line([cap1, (355, 318)], fill=(45, 48, 52, 255), width=40)
    # Front aperture cap
    draw_atgm.ellipse([305, 280, 340, 315], fill=(22, 24, 25, 255), outline=(60, 65, 70, 255), width=3)
    # Rear exhaust bell
    draw_atgm.line([(640, 470), (680, 492)], fill=(32, 34, 36, 255), width=50)
    # Command Launch Unit (CLU) thermal optics module mounted on top
    draw_atgm.rounded_rectangle([420, 310, 520, 385], radius=8, fill=(35, 38, 42, 255), outline=(65, 70, 75, 255), width=4)
    # Optical lens aperture with blue AR-coating glass reflection
    draw_atgm.ellipse([430, 325, 465, 360], fill=(25, 75, 110, 255), outline=(45, 120, 160, 255), width=3)
    draw_atgm.ellipse([442, 332, 452, 342], fill=(120, 200, 255, 255))
    # Spare missile container on ground behind team
    draw_atgm.line([(220, 720), (380, 680)], fill=(48, 56, 44, 255), width=28)
    draw_atgm.line([(210, 723), (235, 716)], fill=(25, 26, 28, 255), width=32)
    draw_atgm.line([(365, 684), (390, 677)], fill=(25, 26, 28, 255), width=32)
    fit_and_save(canvas_atgm, "atgm")

    # 3. FPV TEAM (FPV班 - Tactical Drone Operator Squad)
    canvas_fpv = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    # Pilot seated/kneeling
    s_pilot = s2.resize((int(s2.width * 0.85), int(s2.height * 0.85)), Image.Resampling.LANCZOS)
    canvas_fpv.paste(s_pilot, (380, 220), s_pilot)
    # Spotter on left guarding
    s_spot = s3.resize((int(s3.width * 0.88), int(s3.height * 0.88)), Image.Resampling.LANCZOS)
    canvas_fpv.paste(s_spot, (160, 260), s_spot)

    draw_fpv = ImageDraw.Draw(canvas_fpv)
    # FPV Headset / Goggles on Pilot's head
    head_x, head_y = 515, 275
    draw_fpv.rounded_rectangle([head_x - 30, head_y - 12, head_x + 30, head_y + 12], radius=4, fill=(210, 215, 220, 255), outline=(50, 52, 55, 255), width=3)
    draw_fpv.line([head_x - 30, head_y, head_x - 45, head_y - 6], fill=(40, 42, 45, 255), width=4) # head strap
    draw_fpv.line([head_x + 30, head_y, head_x + 45, head_y - 6], fill=(40, 42, 45, 255), width=4)
    # Antenna on goggles
    draw_fpv.line([head_x - 18, head_y - 10, head_x - 22, head_y - 30], fill=(220, 40, 40, 255), width=4)
    draw_fpv.ellipse([head_x - 26, head_y - 38, head_x - 18, head_y - 30], fill=(220, 40, 40, 255))
    # Remote Controller (Transmitter) in Pilot's hands
    rc_x, rc_y = 485, 410
    draw_fpv.rounded_rectangle([rc_x - 45, rc_y - 25, rc_x + 45, rc_y + 25], radius=6, fill=(35, 36, 40, 255), outline=(65, 70, 75, 255), width=3)
    # Dual RC Antennas
    draw_fpv.line([rc_x - 25, rc_y - 25, rc_x - 35, rc_y - 70], fill=(25, 25, 28, 255), width=5)
    draw_fpv.line([rc_x + 25, rc_y - 25, rc_x + 35, rc_y - 70], fill=(25, 25, 28, 255), width=5)
    # Control Screen / Phone Display
    draw_fpv.rectangle([rc_x - 28, rc_y - 16, rc_x + 28, rc_y + 16], fill=(30, 95, 140, 255), outline=(15, 20, 25, 255), width=2)
    # Directional Antenna Mast on Tripod (Right of team)
    mast_x, mast_y = 750, 480
    draw_fpv.line([(mast_x, mast_y), (mast_x - 50, mast_y + 260)], fill=(45, 48, 52, 255), width=10)
    draw_fpv.line([(mast_x, mast_y), (mast_x + 10, mast_y + 270)], fill=(45, 48, 52, 255), width=10)
    draw_fpv.line([(mast_x, mast_y), (mast_x + 70, mast_y + 240)], fill=(45, 48, 52, 255), width=10)
    draw_fpv.line([(mast_x, mast_y), (mast_x, mast_y - 190)], fill=(65, 70, 75, 255), width=12) # telescoping mast
    # High-Gain Patch / Yagi Directional Antenna Panel
    panel_x, panel_y = mast_x - 15, mast_y - 210
    draw_fpv.rounded_rectangle([panel_x - 40, panel_y - 50, panel_x + 40, panel_y + 50], radius=8, fill=(65, 75, 68, 255), outline=(35, 40, 36, 255), width=5)
    draw_fpv.ellipse([panel_x - 15, panel_y - 15, panel_x + 15, panel_y + 15], fill=(180, 195, 185, 255))
    # 2 FPV Suicide Drones on Ground Launch Mat
    mat_x, mat_y = 480, 720
    draw_fpv.polygon([(mat_x - 110, mat_y), (mat_x + 90, mat_y - 45), (mat_x + 150, mat_y + 40), (mat_x - 50, mat_y + 85)], fill=(185, 120, 35, 255), outline=(130, 80, 20, 255))
    # Drone 1
    d1_x, d1_y = mat_x - 20, mat_y + 15
    draw_fpv.line([(d1_x - 30, d1_y - 20), (d1_x + 30, d1_y + 20)], fill=(30, 32, 35, 255), width=6)
    draw_fpv.line([(d1_x - 30, d1_y + 20), (d1_x + 30, d1_y - 20)], fill=(30, 32, 35, 255), width=6)
    draw_fpv.rounded_rectangle([d1_x - 16, d1_y - 8, d1_x + 16, d1_y + 8], radius=3, fill=(45, 50, 48, 255)) # RPG warhead
    draw_fpv.rounded_rectangle([d1_x - 12, d1_y - 14, d1_x + 8, d1_y - 8], radius=2, fill=(210, 180, 30, 255)) # battery
    for px, py in [(d1_x - 30, d1_y - 20), (d1_x + 30, d1_y + 20), (d1_x - 30, d1_y + 20), (d1_x + 30, d1_y - 20)]:
        draw_fpv.ellipse([px - 14, py - 6, px + 14, py + 6], fill=(50, 55, 60, 200))
    fit_and_save(canvas_fpv, "fpv_team")

    # 4. ENGINEER (工兵 - Combat Engineer Sapper Squad)
    canvas_eng = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    s_eng1 = s1.resize((int(s1.width * 0.90), int(s1.height * 0.90)), Image.Resampling.LANCZOS)
    canvas_eng.paste(s_eng1, (160, 240), s_eng1)
    s_eng2 = s2.resize((int(s2.width * 0.92), int(s2.height * 0.92)), Image.Resampling.LANCZOS)
    canvas_eng.paste(s_eng2, (440, 180), s_eng2)

    draw_eng = ImageDraw.Draw(canvas_eng)
    # Mine Detector wand in Engineer 1 hands sweeping ground
    wand_hand = (340, 480)
    wand_head = (240, 740)
    draw_eng.line([wand_hand, wand_head], fill=(42, 45, 48, 255), width=10)
    # Circular search head hovering over ground
    draw_eng.ellipse([wand_head[0] - 38, wand_head[1] - 16, wand_head[0] + 38, wand_head[1] + 16], fill=(30, 32, 35, 255), outline=(75, 80, 85, 255), width=4)
    # Entrenching Spade / Shovel on Engineer 2
    sh_top = (420, 280)
    sh_bot = (490, 520)
    draw_eng.line([sh_top, sh_bot], fill=(130, 95, 60, 255), width=12) # wooden shaft
    draw_eng.polygon([(sh_top[0] - 25, sh_top[1] - 40), (sh_top[0] + 15, sh_top[1] - 50), (sh_top[0] + 25, sh_top[1] - 10), (sh_top[0] - 15, sh_top[1] + 5)], fill=(50, 55, 60, 255), outline=(85, 90, 95, 255))
    # Fortification equipment on ground: Anti-tank mines & Sandbags
    # AT-Mine 1
    mine_x, mine_y = 520, 720
    draw_eng.ellipse([mine_x - 35, mine_y - 15, mine_x + 35, mine_y + 15], fill=(55, 65, 50, 255), outline=(32, 38, 30, 255), width=3)
    draw_eng.ellipse([mine_x - 12, mine_y - 6, mine_x + 12, mine_y + 6], fill=(35, 40, 32, 255))
    # Dragon's Tooth obstacle (concrete tetrahedron)
    dt_x, dt_y = 660, 680
    draw_eng.polygon([(dt_x, dt_y - 85), (dt_x - 55, dt_y + 35), (dt_x + 55, dt_y + 35)], fill=(125, 128, 125, 255), outline=(90, 92, 90, 255))
    draw_eng.polygon([(dt_x, dt_y - 85), (dt_x + 10, dt_y + 50), (dt_x + 55, dt_y + 35)], fill=(95, 98, 95, 255))
    fit_and_save(canvas_eng, "engineer")

if __name__ == "__main__":
    create_team_sprites()
