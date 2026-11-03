//! 地形の絵を手続き的に描く（素材ファイルは使わない）

use yydrone_core::geo::*;
use yydrone_core::path::{FEAT_TEETH, FEAT_TRENCH};
use yydrone_core::rng::{fbm, hash2, value_noise};
use yydrone_core::world::World;

/// 地形の画像はタイルあたり 4 ピクセル
pub const SCALE: usize = 4;
pub const TW: usize = W * SCALE;
pub const TH: usize = H * SCALE;

#[inline]
pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let c = |v: f32| (v.clamp(0.0, 255.0)) as u32;
    c(r) | c(g) << 8 | c(b) << 16 | c(a) << 24
}

fn terrain_at(w: &World, fx: f32, fy: f32) -> Terrain {
    w.map.t(fx as i32, fy as i32)
}

fn pixel(w: &World, px: usize, py: usize) -> u32 {
    let (fx0, fy0) = ((px as f32 + 0.5) / SCALE as f32, (py as f32 + 0.5) / SCALE as f32);
    // 境界を少し揺らして、タイルの角を見せない
    let wob = 1.5;
    let nx = value_noise(fx0 * 0.7, fy0 * 0.7, 41) - 0.5 + (value_noise(fx0 * 2.1, fy0 * 2.1, 43) - 0.5) * 0.7;
    let ny = value_noise(fx0 * 0.7 + 9.0, fy0 * 0.7, 47) - 0.5 + (value_noise(fx0 * 2.1, fy0 * 2.1 + 5.0, 53) - 0.5) * 0.7;
    let (fx, fy) = (fx0 + nx * wob, fy0 + ny * wob);
    let t0 = terrain_at(w, fx0, fy0);
    // 川・橋は細いので揺らしすぎない
    let t = if matches!(t0, Terrain::Bridge) {
        t0
    } else {
        // 川を揺らさず、森と畑の境目だけ大きく揺らす
        let t1 = terrain_at(w, fx, fy);
        if t0 == Terrain::River || t1 == Terrain::River || t0 == Terrain::Urban || t1 == Terrain::Urban {
            terrain_at(w, fx0 + nx * 0.5, fy0 + ny * 0.5)
        } else {
            t1
        }
    };
    let grain = hash2(px as i32, py as i32, 7) - 0.5;
    let big = fbm(fx0 * 0.05, fy0 * 0.05, 5, 3);
    let (r, g, b) = match t {
        Terrain::Sea => {
            let d = fbm(fx0 * 0.03, fy0 * 0.03, 9, 2);
            (20.0 + d * 12.0, 42.0 + d * 22.0, 62.0 + d * 30.0)
        }
        Terrain::River => (40.0, 74.0, 98.0),
        Terrain::Bridge => (120.0, 112.0, 96.0),
        Terrain::Neutral => {
            let hatch = if (px + py) % 14 < 2 { 6.0 } else { 0.0 };
            (58.0 + hatch + big * 8.0, 60.0 + hatch + big * 8.0, 58.0 + hatch + big * 8.0)
        }
        Terrain::Forest => {
            let n = value_noise(fx0 * 3.0, fy0 * 3.0, 19);
            (44.0 + n * 22.0, 66.0 + n * 26.0, 40.0 + n * 16.0)
        }
        Terrain::Urban => {
            // 街区
            let (gx, gy) = (px % 5, py % 5);
            let block = hash2((px / 5) as i32, (py / 5) as i32, 31);
            if gx == 0 || gy == 0 {
                (82.0, 80.0, 76.0)
            } else {
                let v = 100.0 + block * 58.0;
                (v, v * 0.97, v * 0.92)
            }
        }
        Terrain::Marsh => {
            // 湿地：暗い緑に水たまりが光る
            let n = value_noise(fx0 * 2.6, fy0 * 2.6, 23);
            if n > 0.68 {
                (52.0, 84.0, 88.0)
            } else {
                (70.0 + n * 18.0, 88.0 + n * 18.0, 62.0 + n * 10.0)
            }
        }
        Terrain::Treeline => {
            let n = value_noise(fx0 * 4.0, fy0 * 4.0, 29);
            (52.0 + n * 20.0, 74.0 + n * 22.0, 44.0 + n * 14.0)
        }
        Terrain::Hill => {
            // 高地：褐色に等高線のような濃淡
            let h = fbm(fx0 * 0.45, fy0 * 0.45, 37, 3);
            let band = if (h * 22.0).fract() < 0.16 { -14.0 } else { 0.0 };
            (112.0 + h * 34.0 + band, 98.0 + h * 28.0 + band, 72.0 + h * 18.0 + band)
        }
        Terrain::Plain => {
            // 畑のつぎはぎ：黒土地帯らしい黄緑〜茶。区画は斜めに少し傾ける
            let (qx, qy) = (fx0 * 0.94 + fy0 * 0.34, fy0 * 0.94 - fx0 * 0.34);
            let (cx, cy) = ((qx / 1.3).floor() as i32, (qy / 0.9).floor() as i32);
            let f = hash2(cx, cy, 13);
            let base = big * 0.5 + 0.25;
            let tone = value_noise(fx0 * 0.12, fy0 * 0.12, 61);
            let (r0, g0, b0) = if f < 0.4 {
                (112.0, 116.0, 68.0)
            } else if f < 0.75 {
                (102.0, 112.0, 66.0)
            } else {
                (112.0, 104.0, 70.0)
            };
            (r0 + base * 22.0 + tone * 14.0, g0 + base * 18.0 + tone * 10.0, b0 + base * 8.0)
        }
    };
    rgba(r + grain * 8.0, g + grain * 8.0, b + grain * 8.0, 255.0)
}

pub fn paint_all(w: &World) -> Vec<u32> {
    let mut buf = vec![0u32; TW * TH];
    for py in 0..TH {
        for px in 0..TW {
            buf[py * TW + px] = pixel(w, px, py);
        }
    }
    // 道路：中心線を細く描く
    for line in &w.map.road_lines {
        for s in line.windows(2) {
            let (dx, dy) = (s[1].0 - s[0].0, s[1].1 - s[0].1);
            let n = ((dx.abs().max(dy.abs())) * SCALE as f32 * 2.0).ceil().max(1.0) as usize;
            for k in 0..n {
                let t = k as f32 / n as f32;
                let (x, y) = (s[0].0 + dx * t, s[0].1 + dy * t);
                let (px, py) = ((x * SCALE as f32) as usize, (y * SCALE as f32) as usize);
                if px + 1 >= TW || py + 1 >= TH || !(w.map.t(x as i32, y as i32).land() || w.map.t(x as i32, y as i32) == Terrain::Marsh) {
                    continue;
                }
                for (ox, oy, a) in [(0, 0, 0.8), (1, 0, 0.3), (0, 1, 0.3)] {
                    let p = &mut buf[(py + oy) * TW + px + ox];
                    *p = blend(*p, (182, 170, 134), a);
                }
            }
        }
    }
    // 鉄道：黒い線に枕木
    for line in &w.map.rail_lines {
        let mut step = 0;
        for s in line.windows(2) {
            let (dx, dy) = (s[1].0 - s[0].0, s[1].1 - s[0].1);
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let n = (len * SCALE as f32 * 2.0).ceil().max(1.0) as usize;
            for k in 0..n {
                let t = k as f32 / n as f32;
                let (x, y) = (s[0].0 + dx * t, s[0].1 + dy * t);
                let (px, py) = ((x * SCALE as f32) as usize, (y * SCALE as f32) as usize);
                step += 1;
                if px < 2 || py < 2 || px + 2 >= TW || py + 2 >= TH || !(w.map.t(x as i32, y as i32).land() || matches!(w.map.t(x as i32, y as i32), Terrain::Marsh | Terrain::Bridge)) {
                    continue;
                }
                let p = &mut buf[py * TW + px];
                *p = blend(*p, (28, 26, 30), 0.92);
                // 枕木：線路に直角の短い線
                if step % 7 == 0 {
                    let (nx, ny) = (-dy / len, dx / len);
                    for o in [-1.5f32, 1.5] {
                        let (qx, qy) = ((px as f32 + nx * o) as usize, (py as f32 + ny * o) as usize);
                        let q = &mut buf[qy * TW + qx];
                        *q = blend(*q, (28, 26, 30), 0.8);
                    }
                }
            }
        }
    }
    // 海岸の明るい帯
    let sea = |x: i32, y: i32| w.map.t(x, y) == Terrain::Sea;
    for y in 1..H as i32 - 1 {
        for x in 1..W as i32 - 1 {
            if !sea(x, y) {
                continue;
            }
            if sea(x + 1, y) && sea(x - 1, y) && sea(x, y + 1) && sea(x, y - 1) {
                continue;
            }
            for py in 0..SCALE {
                for px in 0..SCALE {
                    let p = &mut buf[(y as usize * SCALE + py) * TW + x as usize * SCALE + px];
                    *p = blend(*p, (70, 110, 130), 0.25);
                }
            }
        }
    }
    for i in 0..W * H {
        if w.feat[i] != 0 {
            paint_features(w, &mut buf, i);
        }
    }
    buf
}

#[inline]
fn blend(p: u32, c: (u8, u8, u8), a: f32) -> u32 {
    let (r, g, b) = ((p & 255) as f32, (p >> 8 & 255) as f32, (p >> 16 & 255) as f32);
    rgba(r + (c.0 as f32 - r) * a, g + (c.1 as f32 - g) * a, b + (c.2 as f32 - b) * a, 255.0)
}

/// タイル1枚を描き直す（橋の破壊など）
pub fn repaint_tile(w: &World, buf: &mut [u32], i: usize) {
    let (x, y) = (i % W, i / W);
    // 地形が変わるのは橋の破壊と浮橋だけ（他は道路を消さないよう上に描くだけ）
    if matches!(w.map.terrain[i], Terrain::River | Terrain::Sea | Terrain::Bridge) {
        for py in 0..SCALE {
            for px in 0..SCALE {
                buf[(y * SCALE + py) * TW + x * SCALE + px] = pixel(w, x * SCALE + px, y * SCALE + py);
            }
        }
    }
    paint_features(w, buf, i);
}

/// 地形が変わったタイルを描き直す（氾濫など）
pub fn repaint_full(w: &World, buf: &mut [u32], i: usize) {
    let (x, y) = (i % W, i / W);
    for py in 0..SCALE {
        for px in 0..SCALE {
            buf[(y * SCALE + py) * TW + x * SCALE + px] = pixel(w, x * SCALE + px, y * SCALE + py);
        }
    }
    paint_features(w, buf, i);
}

fn paint_features(w: &World, buf: &mut [u32], i: usize) {
    let (x, y) = (i % W, i / W);
    let f = w.feat[i];
    if f & FEAT_TRENCH != 0 {
        // 塹壕：ジグザグの暗い線
        const ZIG: [(usize, usize); 7] = [(0, 1), (1, 1), (1, 2), (2, 2), (2, 1), (3, 1), (3, 2)];
        for (px, py) in ZIG {
            let p = &mut buf[(y * SCALE + py) * TW + x * SCALE + px];
            *p = blend(*p, (38, 26, 16), 0.9);
        }
        for px in 0..SCALE {
            let p = &mut buf[(y * SCALE + 3) * TW + x * SCALE + px];
            *p = blend(*p, (112, 88, 60), 0.35);
        }
    }
    if f & FEAT_TEETH != 0 {
        for (px, py) in [(0, 0), (2, 0), (1, 2), (3, 2)] {
            let p = &mut buf[(y * SCALE + py) * TW + x * SCALE + px];
            *p = blend(*p, (214, 210, 196), 0.95);
        }
    }
}

/// 砲撃跡を地形に焼き付ける。書き換えた範囲を返す
pub fn stamp_scar(buf: &mut [u32], x: f32, y: f32, r: f32, seed: u32) -> (u32, u32, u32, u32) {
    let s = SCALE as f32;
    let (cx, cy, rp) = (x * s, y * s, (r * s).max(1.5));
    let x0 = ((cx - rp - 1.0).max(0.0)) as u32;
    let y0 = ((cy - rp - 1.0).max(0.0)) as u32;
    let x1 = ((cx + rp + 2.0).min(TW as f32 - 1.0)) as u32;
    let y1 = ((cy + rp + 2.0).min(TH as f32 - 1.0)) as u32;
    for py in y0..y1 {
        for px in x0..x1 {
            let (dx, dy) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
            let d = (dx * dx + dy * dy).sqrt() / rp;
            if d >= 1.0 {
                continue;
            }
            let n = hash2(px as i32, py as i32, seed);
            let a = (1.0 - d) * 0.55 * (0.5 + n);
            let p = &mut buf[py as usize * TW + px as usize];
            *p = blend(*p, (34, 30, 26), a.min(0.8));
        }
    }
    (x0, y0, x1 - x0, y1 - y0)
}

/// 領土色・霧・季節をまとめたタイル解像度の重ね絵
pub fn paint_overlay(w: &World, viewer: Option<u8>, buf: &mut [u32]) {
    const TINT: [(f32, f32, f32); 2] = [(52.0, 122.0, 226.0), (222.0, 60.0, 52.0)];
    let winter = w.winter();
    let mud = w.mud();
    let leaf = w.leaf_on();
    let (_, month, _) = w.date();
    let dry = (7..=8).contains(&month);
    for y in 0..H {
        for x in 0..W {
            let i = y * W + x;
            let o = w.owner[i];
            if o < 0 {
                buf[i] = if w.map.terrain[i] == Terrain::Neutral { rgba(10.0, 10.0, 14.0, 90.0) } else { 0 };
                continue;
            }
            let (mut r, mut g, mut b) = TINT[o as usize];
            let mut a = 34.0;
            // 前線の縁は濃く
            let (xi, yi) = (x as i32, y as i32);
            let mut edge = false;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                if in_map(xi + dx, yi + dy) {
                    let o2 = w.owner[tidx(xi + dx, yi + dy)];
                    if o2 >= 0 && o2 != o {
                        edge = true;
                    }
                }
            }
            if edge {
                a = 190.0;
                r = r * 0.7 + 255.0 * 0.3;
                g = g * 0.7 + 255.0 * 0.3;
                b = b * 0.7 + 255.0 * 0.3;
            } else {
                // 季節の色：領土の色と混ぜて、どちらの土地か分かるようにする
                let t = w.map.terrain[i];
                let leafy = matches!(t, Terrain::Forest | Terrain::Treeline);
                let tint = if mud {
                    // 泥濘が雪より優先（2022年の開戦時は2月でも泥濘）
                    if matches!(t, Terrain::Plain | Terrain::Treeline | Terrain::Marsh) {
                        Some((64.0, 46.0, 28.0, 92.0))
                    } else {
                        None
                    }
                } else if winter {
                    // 雪。森は枝が見えて暗い
                    if leafy {
                        Some((150.0, 150.0, 150.0, 70.0))
                    } else if matches!(t, Terrain::Plain | Terrain::Hill | Terrain::Marsh | Terrain::Urban) {
                        Some((232.0, 238.0, 244.0, 105.0))
                    } else {
                        None
                    }
                } else if leafy && !leaf {
                    // 落葉した木々
                    Some((110.0, 84.0, 58.0, 84.0))
                } else if dry && t == Terrain::Plain {
                    // 真夏のステップは乾いて黄色い
                    Some((190.0, 164.0, 92.0, 56.0))
                } else {
                    None
                };
                if let Some((sr, sg, sb, sa)) = tint {
                    r = r * 0.3 + sr * 0.7;
                    g = g * 0.3 + sg * 0.7;
                    b = b * 0.3 + sb * 0.7;
                    a = sa;
                }
            }
            // 占領が進んでいるタイルは敵の色に寄せる
            if !edge && w.cap[i] > 0 {
                let t = TINT[1 - o as usize];
                let k = w.cap[i] as f32 / 100.0;
                r += (t.0 - r) * k;
                g += (t.1 - g) * k;
                b += (t.2 - b) * k;
                a += 80.0 * k;
            }
            // 霧：敵地で見えていない場所は暗くする
            if let Some(v) = viewer {
                if o != v as i8 && w.vis[v as usize][i] == 0 && !edge {
                    let k = 0.62;
                    r *= 1.0 - k;
                    g *= 1.0 - k;
                    b *= 1.0 - k;
                    a = 120.0;
                }
            }
            buf[i] = rgba(r, g, b, a);
        }
    }
}
