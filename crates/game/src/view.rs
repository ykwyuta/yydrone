//! 戦場の描画：部隊の絵・建物・弾・爆発

use crate::gfx::*;
use crate::Game;
use yydrone_core::defs::*;
use yydrone_core::geo::*;
use yydrone_core::world::*;

pub const SIDE_COL: [Col; 2] = [[0.36, 0.66, 1.0, 1.0], [1.0, 0.36, 0.30, 1.0]];
const DARK: Col = [0.05, 0.06, 0.07, 0.85];

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub grow: f32,
    pub col: [f32; 3],
    pub alpha: f32,
}

pub struct Streak {
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub life: f32,
    pub max: f32,
    pub w: f32,
    pub col: [f32; 3],
}

fn shade(c: Col, k: f32) -> Col {
    [c[0] * k, c[1] * k, c[2] * k, c[3]]
}

/// 部隊の絵。x が前方向。s は 1 が約 1 タイル
pub fn draw_unit_icon(g: &mut dyn Gfx, k: UK, x: f32, y: f32, ang: f32, s: f32, c: Col, t: f32) {
    const HULL: [(f32, f32); 4] = [(-0.62, -0.38), (0.62, -0.38), (0.62, 0.38), (-0.62, 0.38)];
    const HULL_S: [(f32, f32); 4] = [(-0.55, -0.3), (0.55, -0.3), (0.55, 0.3), (-0.55, 0.3)];
    const TRUCK: [(f32, f32); 4] = [(-0.7, -0.26), (0.7, -0.26), (0.7, 0.26), (-0.7, 0.26)];
    let hi = shade(c, 1.25);
    let lo = shade(c, 0.62);
    let (sn, cs) = ang.sin_cos();
    let at = |px: f32, py: f32| (x + (px * cs - py * sn) * s, y + (px * sn + py * cs) * s);
    let barrel = |g: &mut dyn Gfx, l: f32, w: f32, col: Col, ox: f32| {
        g.line(at(ox, 0.0), at(l, 0.0), w * s, col);
    };
    let dots = |g: &mut dyn Gfx, pts: &[(f32, f32)], r: f32, col: Col| {
        for p in pts {
            let q = at(p.0, p.1);
            g.circle(q.0, q.1, r * s, 6, DARK);
            g.circle(q.0, q.1, r * s * 0.72, 6, col);
        }
    };
    match k {
        UK::Tank => {
            g.poly(x, y, ang, s * 1.12, &HULL, DARK);
            g.poly(x, y, ang, s, &HULL, c);
            let q = at(-0.05, 0.0);
            g.circle(q.0, q.1, s * 0.27, 8, lo);
            barrel(g, 1.0, 0.11, DARK, 0.1);
        }
        UK::Ifv => {
            g.poly(x, y, ang, s * 1.12, &HULL_S, DARK);
            g.poly(x, y, ang, s, &HULL_S, c);
            let q = at(0.05, 0.0);
            g.circle(q.0, q.1, s * 0.16, 6, lo);
            barrel(g, 0.72, 0.07, DARK, 0.1);
        }
        UK::Howitzer => {
            g.poly(x, y, ang, s * 1.12, &HULL_S, DARK);
            g.poly(x, y, ang, s, &HULL_S, lo);
            g.poly(x, y, ang, s * 0.6, &HULL, c);
            barrel(g, 1.25, 0.1, DARK, 0.1);
        }
        UK::Mlrs | UK::Coastal => {
            g.poly(x, y, ang, s * 1.1, &TRUCK, DARK);
            g.poly(x, y, ang, s, &TRUCK, lo);
            for o in [-0.14f32, 0.0, 0.14] {
                g.line(at(-0.55, o), at(0.2, o), 0.1 * s, if k == UK::Coastal { [0.86, 0.9, 1.0, 1.0] } else { hi });
            }
        }
        UK::Shorad => {
            g.poly(x, y, ang, s * 1.12, &HULL_S, DARK);
            g.poly(x, y, ang, s, &HULL_S, lo);
            g.line(at(-0.1, -0.13), at(0.8, -0.13), 0.08 * s, hi);
            g.line(at(-0.1, 0.13), at(0.8, 0.13), 0.08 * s, hi);
        }
        UK::Sam => {
            g.poly(x, y, ang, s * 1.1, &TRUCK, DARK);
            g.poly(x, y, ang, s, &TRUCK, lo);
            g.line(at(-0.6, -0.12), at(0.5, -0.12), 0.15 * s, hi);
            g.line(at(-0.6, 0.12), at(0.5, 0.12), 0.15 * s, hi);
        }
        UK::Ew => {
            g.poly(x, y, ang, s * 1.12, &HULL_S, DARK);
            g.poly(x, y, ang, s, &HULL_S, c);
            let q = at(0.0, 0.0);
            g.ring(q.0, q.1, s * (0.28 + (t * 2.0).sin().abs() * 0.3), 0.05 * s, 12, hi);
            g.circle(q.0, q.1, s * 0.1, 6, DARK);
        }
        UK::Infantry => dots(g, &[(0.25, 0.0), (-0.2, -0.28), (-0.2, 0.28)], 0.2, c),
        UK::Engineer => {
            dots(g, &[(0.2, -0.2), (-0.2, 0.2)], 0.2, c);
            g.line(at(-0.35, -0.35), at(0.35, 0.35), 0.1 * s, hi);
        }
        UK::Atgm => {
            dots(g, &[(-0.2, -0.24), (-0.2, 0.24)], 0.2, c);
            g.line(at(-0.4, 0.0), at(0.65, 0.0), 0.14 * s, hi);
        }
        UK::FpvTeam => {
            dots(g, &[(-0.2, -0.24), (-0.2, 0.24)], 0.2, c);
            let q = at(0.32, 0.0);
            g.line((q.0 - 0.22 * s, q.1 - 0.22 * s), (q.0 + 0.22 * s, q.1 + 0.22 * s), 0.07 * s, hi);
            g.line((q.0 - 0.22 * s, q.1 + 0.22 * s), (q.0 + 0.22 * s, q.1 - 0.22 * s), 0.07 * s, hi);
        }
        UK::ReconDrone => {
            g.line(at(-0.5, 0.0), at(0.6, 0.0), 0.12 * s, c);
            g.line(at(0.1, -0.75), at(0.1, 0.75), 0.12 * s, c);
            g.line(at(-0.5, -0.25), at(-0.5, 0.25), 0.09 * s, c);
        }
        UK::BomberDrone => {
            for i in 0..6 {
                let a = i as f32 * 1.047;
                let q = at(a.cos() * 0.55, a.sin() * 0.55);
                g.line((x, y), q, 0.07 * s, lo);
                g.ring(q.0, q.1, 0.22 * s, 0.06 * s, 8, c);
            }
            g.circle(x, y, 0.22 * s, 8, c);
        }
        UK::ArmedUav => {
            g.line(at(-0.7, 0.0), at(0.65, 0.0), 0.14 * s, c);
            g.line(at(0.1, -1.0), at(0.1, 1.0), 0.13 * s, c);
            g.line(at(-0.7, -0.3), at(-0.7, 0.3), 0.09 * s, hi);
        }
        UK::LongDrone => {
            g.poly(x, y, ang, s, &[(0.8, 0.0), (-0.5, 0.7), (-0.35, 0.0), (-0.5, -0.7)], c);
        }
        UK::Heli => {
            g.poly(x, y, ang, s, &[(0.7, 0.0), (0.2, 0.22), (-0.3, 0.12), (-0.3, -0.12), (0.2, -0.22)], c);
            g.line(at(-0.3, 0.0), at(-0.95, 0.0), 0.09 * s, c);
            let a = t * 24.0;
            g.line((x + a.cos() * s * 0.9, y + a.sin() * s * 0.9), (x - a.cos() * s * 0.9, y - a.sin() * s * 0.9), 0.05 * s, [0.9, 0.95, 1.0, 0.75]);
            g.line((x - a.sin() * s * 0.9, y + a.cos() * s * 0.9), (x + a.sin() * s * 0.9, y - a.cos() * s * 0.9), 0.05 * s, [0.9, 0.95, 1.0, 0.75]);
        }
        UK::Jet => {
            g.poly(x, y, ang, s * 1.3, &[(0.9, 0.0), (0.05, 0.12), (-0.35, 0.72), (-0.55, 0.7), (-0.42, 0.08), (-0.75, 0.06), (-0.75, -0.06), (-0.42, -0.08), (-0.55, -0.7), (-0.35, -0.72), (0.05, -0.12)], c);
        }
        UK::Frigate => {
            g.poly(x, y, ang, s * 2.2, &[(0.9, 0.0), (0.45, 0.2), (-0.8, 0.2), (-0.8, -0.2), (0.45, -0.2)], lo);
            g.poly(x, y, ang, s * 1.1, &[(0.5, -0.16), (0.5, 0.16), (-0.8, 0.16), (-0.8, -0.16)], c);
        }
        UK::SeaDrone => {
            g.poly(x, y, ang, s * 1.2, &[(0.9, 0.0), (-0.4, 0.3), (-0.6, 0.0), (-0.4, -0.3)], c);
        }
        UK::Fpv => {
            g.line((x - 0.5 * s, y - 0.5 * s), (x + 0.5 * s, y + 0.5 * s), 0.16 * s, c);
            g.line((x - 0.5 * s, y + 0.5 * s), (x + 0.5 * s, y - 0.5 * s), 0.16 * s, c);
        }
        UK::Cruise => {
            g.line(at(-0.9, 0.0), at(0.9, 0.0), 0.2 * s, [0.92, 0.92, 0.95, 1.0]);
            g.line(at(-0.7, -0.35), at(-0.7, 0.35), 0.12 * s, c);
        }
    }
}

impl Game {
    /// 爆発や曳光弾を world の出来事から作る
    pub fn absorb_fx(&mut self) {
        let Some(w) = self.world.as_mut() else { return };
        let fxs = std::mem::take(&mut w.fx);
        let viewer = self.viewer;
        for f in fxs {
            let visible = viewer.is_none_or(|v| {
                let i = tidx(f.x as i32, f.y as i32);
                let j = tidx(f.x2.clamp(0.0, W as f32 - 1.0) as i32, f.y2.clamp(0.0, H as f32 - 1.0) as i32);
                w.vis[v as usize][i] != 0 || w.vis[v as usize][j] != 0
            });
            if !visible || self.particles.len() > 2500 {
                continue;
            }
            let rng = &mut self.fx_rng;
            match f.kind {
                FxKind::Tracer => {
                    let col = if f.side == 0 { [0.75, 0.9, 1.0] } else { [1.0, 0.82, 0.55] };
                    let (w, life) = if f.r >= 1.0 { (0.16, 0.14) } else { (0.06, 0.08) };
                    self.streaks.push(Streak { a: (f.x, f.y), b: (f.x2, f.y2), life, max: life, w, col });
                }
                FxKind::Muzzle => {
                    self.particles.push(Particle { x: f.x, y: f.y, vx: 0.0, vy: 0.0, life: 0.14, max: 0.14, size: 0.7, grow: 0.0, col: [1.0, 0.85, 0.5], alpha: 0.9 });
                    self.particles.push(Particle { x: f.x, y: f.y, vx: rng.range(-0.3, 0.3), vy: rng.range(-0.5, -0.1), life: 1.4, max: 1.4, size: 0.5, grow: 0.8, col: [0.72, 0.72, 0.7], alpha: 0.35 });
                }
                FxKind::Launch => {
                    self.particles.push(Particle { x: f.x, y: f.y, vx: 0.0, vy: 0.0, life: 0.5, max: 0.5, size: 0.8, grow: 1.5, col: [0.9, 0.9, 0.85], alpha: 0.5 });
                }
                FxKind::Blast | FxKind::BigBlast | FxKind::MineBlast => {
                    let r = f.r.max(0.4);
                    self.particles.push(Particle { x: f.x, y: f.y, vx: 0.0, vy: 0.0, life: 0.16, max: 0.16, size: r * 0.8, grow: 2.0, col: [1.0, 0.92, 0.65], alpha: 0.9 });
                    let n = (5.0 + r * 7.0) as usize;
                    for _ in 0..n {
                        let a = rng.range(0.0, std::f32::consts::TAU);
                        let sp = rng.range(0.3, 2.6) * r;
                        let life = rng.range(0.25, 0.65);
                        self.particles.push(Particle { x: f.x, y: f.y, vx: a.cos() * sp, vy: a.sin() * sp, life, max: life, size: r * rng.range(0.25, 0.55), grow: 0.1, col: [1.0, rng.range(0.4, 0.75), 0.18], alpha: 0.9 });
                    }
                    let m = (2.0 + r * 3.0) as usize;
                    for _ in 0..m {
                        let life = rng.range(1.2, 2.8) * if f.kind == FxKind::BigBlast { 1.5 } else { 1.0 };
                        self.particles.push(Particle { x: f.x + rng.range(-0.4, 0.4) * r, y: f.y + rng.range(-0.4, 0.4) * r, vx: rng.range(-0.2, 0.25), vy: rng.range(-0.55, -0.15), life, max: life, size: r * rng.range(0.25, 0.5), grow: 0.28, col: [0.2, 0.19, 0.18], alpha: 0.34 });
                    }
                }
                FxKind::AirBurst | FxKind::Jam => {
                    let c = if f.kind == FxKind::Jam { [0.6, 0.85, 1.0] } else { [1.0, 0.75, 0.3] };
                    self.particles.push(Particle { x: f.x, y: f.y, vx: 0.0, vy: 0.0, life: 0.3, max: 0.3, size: f.r + 0.3, grow: 2.0, col: c, alpha: 0.85 });
                    for _ in 0..4 {
                        let life = rng.range(0.4, 0.9);
                        self.particles.push(Particle { x: f.x, y: f.y, vx: rng.range(-1.0, 1.0), vy: rng.range(-0.6, 1.4), life, max: life, size: 0.12, grow: 0.0, col: [0.4, 0.4, 0.4], alpha: 0.8 });
                    }
                }
            }
        }
    }

    pub fn step_particles(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.life -= dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vx *= 1.0 - 1.5 * dt;
            p.vy *= 1.0 - 1.5 * dt;
            p.size += p.grow * dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for s in &mut self.streaks {
            s.life -= dt;
        }
        self.streaks.retain(|s| s.life > 0.0);
    }

    /// 天候を画面に重ねる
    pub fn draw_weather(&mut self, g: &mut dyn Gfx) {
        let Some(w) = self.world.as_ref() else { return };
        let (sw, sh) = g.size();
        let t = self.anim;
        let hash = |i: u32, s: u32| yydrone_core::rng::hash2(i as i32, s as i32, 991);
        match w.weather {
            Weather::Clear => {}
            Weather::Fog => {
                g.rect(0.0, 0.0, sw, sh, [0.78, 0.8, 0.82, 0.3]);
                for i in 0..14u32 {
                    let (x0, y0, r) = (hash(i, 1) * sw, hash(i, 2) * sh, 120.0 + hash(i, 3) * 200.0);
                    let x = (x0 + t * (6.0 + hash(i, 4) * 10.0)) % (sw + 400.0) - 200.0;
                    g.circle(x, y0, r, 18, [0.85, 0.87, 0.89, 0.09]);
                }
            }
            Weather::Rain => {
                g.rect(0.0, 0.0, sw, sh, [0.2, 0.25, 0.32, 0.14]);
                for i in 0..160u32 {
                    let x = (hash(i, 1) * sw + t * 90.0) % sw;
                    let y = (hash(i, 2) * sh + t * (620.0 + hash(i, 3) * 200.0)) % sh;
                    g.line((x, y), (x - 3.0, y + 14.0), 1.0, [0.75, 0.84, 0.95, 0.35]);
                }
            }
            Weather::Snow => {
                g.rect(0.0, 0.0, sw, sh, [0.85, 0.88, 0.92, 0.1]);
                for i in 0..180u32 {
                    let sway = (t * 0.9 + hash(i, 4) * 6.0).sin() * 14.0;
                    let x = (hash(i, 1) * sw + sway + sw) % sw;
                    let y = (hash(i, 2) * sh + t * (34.0 + hash(i, 3) * 40.0)) % sh;
                    g.circle(x, y, 1.2 + hash(i, 5) * 1.4, 5, [1.0, 1.0, 1.0, 0.75]);
                }
            }
        }
    }

    pub fn draw_world(&mut self, g: &mut dyn Gfx) {
        let (sw, sh) = g.size();
        let z = self.zoom;
        // 世界座標（タイル）→ NDC
        g.set_xform(2.0 * z / sw, -2.0 * z / sh, -2.0 * z * self.cam.0 / sw, 2.0 * z * self.cam.1 / sh);
        let (tt, ot) = (self.terrain_tex, self.overlay_tex);
        g.draw_textured(tt, 0.0, 0.0, W as f32, H as f32, [1.0; 4], true);
        g.draw_textured(ot, 0.0, 0.0, W as f32, H as f32, [1.0; 4], true);
        let Some(w) = self.world.as_ref() else { return };
        let (x0, y0) = (self.cam.0 - sw / 2.0 / z - 3.0, self.cam.1 - sh / 2.0 / z - 3.0);
        let (x1, y1) = (self.cam.0 + sw / 2.0 / z + 3.0, self.cam.1 + sh / 2.0 / z + 3.0);
        let in_view = |x: f32, y: f32| x >= x0 && x <= x1 && y >= y0 && y <= y1;
        let px = 1.0 / z;
        let viewer = self.viewer;
        let t = self.anim;
        // 地雷（自軍のものだけ見える）と橋
        if z >= 5.0 {
            let (xa, xb) = ((x0.max(0.0)) as usize, (x1.min(W as f32 - 1.0)) as usize);
            let (ya, yb) = ((y0.max(0.0)) as usize, (y1.min(H as f32 - 1.0)) as usize);
            for y in ya..=yb {
                for x in xa..=xb {
                    let m = w.mines[y * W + x];
                    if m != 0 && viewer.is_none_or(|v| v + 1 == m) {
                        let c = shade(SIDE_COL[m as usize - 1], 0.9);
                        g.circle(x as f32 + 0.5, y as f32 + 0.5, 0.18, 5, [0.0, 0.0, 0.0, 0.8]);
                        g.circle(x as f32 + 0.5, y as f32 + 0.5, 0.11, 5, c);
                    }
                }
            }
        }
        for b in &w.map.bridges {
            if b.hp > 0.0 && b.hp < b.max_hp && in_view(b.x, b.y) {
                let f = b.hp / b.max_hp;
                g.rect(b.x - 1.0, b.y - 1.4, b.x + 1.0, b.y - 1.1, DARK);
                g.rect(b.x - 1.0, b.y - 1.4, b.x - 1.0 + 2.0 * f, b.y - 1.1, [1.0, 0.75, 0.25, 1.0]);
            }
        }
        // 切られた線路
        for &(t, _) in w.rail_cuts.iter().step_by(3) {
            let (cx, cy) = ((t as usize % W) as f32 + 0.5, (t as usize / W) as f32 + 0.5);
            if in_view(cx, cy) && viewer.is_none_or(|v| w.vis[v as usize][t as usize] != 0) {
                let s = (0.45f32).max(4.0 * px);
                g.line((cx - s, cy - s), (cx + s, cy + s), 2.0 * px, [1.0, 0.35, 0.2, 0.95]);
                g.line((cx - s, cy + s), (cx + s, cy - s), 2.0 * px, [1.0, 0.35, 0.2, 0.95]);
            }
        }
        // 都市
        for c in &w.map.cities {
            if !in_view(c.x, c.y) {
                continue;
            }
            let col = SIDE_COL[c.owner as usize];
            let r = (0.35 + c.value as f32 * 0.09).max(5.0 * px);
            if z >= 3.0 {
                let csz = (1.8 + c.value as f32 * 0.4).min(3.8);
                // 都市の基盤に所属陣営カラーのリング
                g.circle(c.x, c.y, r * 1.5, 16, [0.0, 0.0, 0.0, 0.75]);
                g.circle(c.x, c.y, r * 1.2, 16, col);
                g.circle(c.x, c.y, r * 0.55, 12, [0.12, 0.14, 0.18, 0.9]);
                let tex = self.art.city_texture(g, c.value, c.port);
                g.draw_textured(
                    tex,
                    c.x - csz * 0.5,
                    c.y - csz * 0.68,
                    c.x + csz * 0.5,
                    c.y + csz * 0.32,
                    [1.0, 1.0, 1.0, 1.0],
                    true,
                );
            } else {
                g.circle(c.x, c.y, r * 1.3, 14, [0.0, 0.0, 0.0, 0.7]);
                g.circle(c.x, c.y, r, 14, col);
                g.circle(c.x, c.y, r * 0.45, 10, [1.0, 1.0, 1.0, 0.92]);
            }
        }
        // 建物
        for b in &w.blds {
            if !in_view(b.x, b.y) || viewer.is_some_and(|v| b.side != v && !b.seen[v as usize]) {
                continue;
            }
            let s = (0.95f32).max(9.0 * px);
            let c = SIDE_COL[b.side as usize];
            let a = if b.building > 0.0 { 0.45 } else { 1.0 };
            g.rect(b.x - s - px, b.y - s - px, b.x + s + px, b.y + s + px, [0.0, 0.0, 0.0, 0.85 * a]);
            g.rect(b.x - s, b.y - s, b.x + s, b.y + s, [c[0] * 0.5, c[1] * 0.5, c[2] * 0.5, a]);
            g.rect_outline(b.x - s, b.y - s, b.x + s, b.y + s, s * 0.14, [c[0], c[1], c[2], a]);
            if z >= 3.0 {
                if let Some(tex) = self.art.bld_texture(g, b.kind) {
                    let bsz = s * 2.8;
                    g.draw_textured(tex, b.x - bsz * 0.5, b.y - bsz * 0.65, b.x + bsz * 0.5, b.y + bsz * 0.35, [1.0, 1.0, 1.0, a], true);
                }
                // 絵の上から陣営色の枠を描き直す
                g.rect_outline(b.x - s, b.y - s, b.x + s, b.y + s, s * 0.14, [c[0], c[1], c[2], a]);
            }
            let f = (b.hp / b.max_hp).clamp(0.0, 1.0);
            if f < 0.999 {
                g.rect(b.x - s, b.y + s + 2.0 * px, b.x + s, b.y + s + 5.0 * px, DARK);
                g.rect(b.x - s, b.y + s + 2.0 * px, b.x - s + 2.0 * s * f, b.y + s + 5.0 * px, [0.95 - f * 0.6, 0.35 + f * 0.55, 0.25, 1.0]);
            }
            if self.sel_bld == Some(b.id) {
                g.rect_outline(b.x - s * 1.3, b.y - s * 1.3, b.x + s * 1.3, b.y + s * 1.3, s * 0.12, [0.5, 1.0, 0.5, 1.0]);
                if let Some(r) = b.rally {
                    g.line((b.x, b.y), r, 1.5 * px, [0.5, 1.0, 0.5, 0.65]);
                    g.ring(r.0, r.1, 5.0 * px, 1.5 * px, 12, [0.5, 1.0, 0.5, 0.9]);
                }
            }
        }
        // 地上・海上 → 飛行の順に重ねる
        let base = (0.62f32).max(9.5 * px);
        for pass in 0..2 {
            for u in &w.units {
                let d = def(u.kind);
                let air = d.pass == Pass::Air;
                if (pass == 1) != air || !in_view(u.x, u.y) {
                    continue;
                }
                if viewer.is_some_and(|v| !w.can_see(v, u)) {
                    continue;
                }
                let c = SIDE_COL[u.side as usize];
                let sel = self.selected.contains(&u.id);
                let s = base * match u.kind {
                    UK::Fpv => 0.55,
                    UK::Cruise => 0.9,
                    UK::Jet => 1.3,
                    UK::Heli | UK::ArmedUav => 1.15,
                    UK::Tank => 1.1,
                    UK::Infantry | UK::Atgm | UK::FpvTeam | UK::Engineer => 0.95,
                    _ => 1.0,
                };
                if air && !matches!(u.kind, UK::Fpv | UK::Cruise) {
                    // 地面の影
                    g.circle(u.x + 0.35 * s, u.y + 0.5 * s, 0.35 * s, 6, [0.0, 0.0, 0.0, 0.3]);
                }
                if sel {
                    g.ring(u.x, u.y, s * 1.05, s * 0.12, 16, [0.5, 1.0, 0.5, 0.95]);
                    if let Some(gl) = u.goal {
                        if !air || u.kind != UK::Fpv {
                            g.line((u.x, u.y), gl, 1.2 * px, [0.5, 1.0, 0.5, 0.35]);
                        }
                    }
                } else if u.auto && self.viewer == Some(u.side) {
                    g.circle(u.x + s * 0.8, u.y - s * 0.8, s * 0.17, 6, [1.0, 0.85, 0.35, 0.95]);
                }
                if u.dig > 0.3 && !sel && z > 6.0 {
                    g.ring(u.x, u.y, s * 0.95, s * 0.1, 10, [0.32, 0.23, 0.13, 0.85]);
                }
                let drawn_sprite = if z >= 3.5 {
                    if let Some(tex) = self.art.unit_texture(g, u.kind) {
                        let sz = s * 2.4;
                        // 絵は両軍共通なので、足元の円と外周の輪で陣営を示す
                        g.circle(u.x, u.y, s * 1.2, 16, [c[0], c[1], c[2], 0.4]);
                        g.draw_textured(tex, u.x - sz * 0.5, u.y - sz * 0.5, u.x + sz * 0.5, u.y + sz * 0.5, [1.0, 1.0, 1.0, 1.0], true);
                        g.ring(u.x, u.y, s * 1.2, s * 0.13, 16, c);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !drawn_sprite {
                    draw_unit_icon(g, u.kind, u.x, u.y, u.face, s, c, t + u.id as f32);
                }
                let f = u.hp / d.hp;
                if (f < 0.995 || sel) && !d.suicide {
                    let bw = s * 0.9;
                    g.rect(u.x - bw, u.y - s * 1.15, u.x + bw, u.y - s * 0.95, DARK);
                    g.rect(u.x - bw, u.y - s * 1.15, u.x - bw + 2.0 * bw * f.clamp(0.0, 1.0), u.y - s * 0.95, [1.0 - f * 0.7, 0.3 + f * 0.65, 0.25, 1.0]);
                }
                if u.flash > 3.0 && viewer.is_some_and(|v| v != u.side) {
                    g.ring(u.x, u.y, s * 1.6 + (t * 6.0).sin().abs() * s * 0.4, s * 0.1, 12, [1.0, 0.45, 0.2, 0.85]);
                }
            }
        }
        // 電子戦の範囲（選択中）
        for &id in &self.selected {
            if let Some(u) = w.unit(id) {
                let d = def(u.kind);
                if d.jam > 0.0 {
                    g.ring(u.x, u.y, d.jam, 2.0 * px, 48, [0.6, 0.85, 1.0, 0.5]);
                }
                if d.weapon.range > 2.0 && self.selected.len() <= 3 {
                    g.ring(u.x, u.y, d.weapon.range, 1.2 * px, 48, [1.0, 1.0, 1.0, 0.22]);
                }
            }
        }
        // 弾
        for p in &w.projs {
            if !in_view(p.x, p.y) {
                continue;
            }
            if viewer.is_some_and(|v| w.vis[v as usize][tidx(p.x as i32, p.y as i32)] == 0) {
                continue;
            }
            match p.kind {
                ProjKind::Shell | ProjKind::Bomb => {
                    // 放物線を見せる
                    let total = ((p.tx - p.sx).powi(2) + (p.ty - p.sy).powi(2)).sqrt().max(0.01);
                    let done = ((p.x - p.sx).powi(2) + (p.y - p.sy).powi(2)).sqrt() / total;
                    let h = (done * (1.0 - done) * 4.0) * total * 0.14;
                    g.circle(p.x, p.y, 2.0 * px, 5, [0.0, 0.0, 0.0, 0.35]);
                    g.circle(p.x, p.y - h, 2.2 * px, 6, [1.0, 0.9, 0.6, 1.0]);
                }
                ProjKind::Missile => {
                    let (dx, dy) = (p.tx - p.x, p.ty - p.y);
                    let l = (dx * dx + dy * dy).sqrt().max(0.01);
                    g.line((p.x - dx / l * 1.4, p.y - dy / l * 1.4), (p.x, p.y), 1.6 * px, [0.95, 0.95, 0.9, 0.55]);
                    g.circle(p.x, p.y, 2.2 * px, 6, [1.0, 0.75, 0.4, 1.0]);
                }
            }
        }
        // 曳光弾と爆発
        for s in &self.streaks {
            let a = s.life / s.max;
            g.line(s.a, s.b, s.w.max(1.0 * px), [s.col[0], s.col[1], s.col[2], a * 0.9]);
        }
        for p in &self.particles {
            if !in_view(p.x, p.y) {
                continue;
            }
            let a = (p.life / p.max) * p.alpha;
            g.circle(p.x, p.y, p.size.max(1.2 * px), 12, [p.col[0], p.col[1], p.col[2], a]);
        }
        // 衛星偵察の範囲
        for s in &w.sats {
            if viewer.is_none_or(|v| v == s.0) {
                g.ring(s.1, s.2, 24.0, 2.0 * px, 48, [0.6, 1.0, 0.8, 0.6]);
            }
        }
    }
}
