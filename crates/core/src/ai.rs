//! 将軍 AI：コンピュータ陣営と、プレイヤーが委任した部隊を動かす

use crate::defs::*;
use crate::geo::*;
use crate::path::{FEAT_TEETH, FEAT_TRENCH};
use crate::world::*;

const CELL: usize = 8;
const CW: usize = W / CELL + 1;
const CH: usize = H / CELL + 1;

#[derive(Default)]
pub struct Plan {
    /// 主攻の目標（都市番号）
    pub axes: Vec<usize>,
    pub axis_t: f32,
    pub build_t: f32,
    pub buy_t: f32,
    pub power_t: f32,
    /// 史実の出来事で決まった主攻（都市番号）とその期限の日
    pub forced: Vec<usize>,
    pub forced_until: u32,
}

#[derive(Clone, Copy, Default)]
struct Cell {
    own: u16,
    foe: u16,
    own_str: f32,
    foe_str: f32,
    claimed: f32,
}

impl Cell {
    fn is_front(&self) -> bool {
        self.own > 0 && self.foe > 0
    }
}

fn combat_strength(k: UK) -> f32 {
    match k {
        UK::Infantry => 1.0,
        UK::Atgm | UK::FpvTeam | UK::Engineer => 1.0,
        UK::Ifv => 2.0,
        UK::Tank => 3.5,
        _ => 0.0,
    }
}

impl World {
    pub fn ai_tick(&mut self, side: u8) {
        let s = side as usize;
        let is_ai = self.sides[s].ai;
        let cells = self.scan_cells(side);
        let fronts: Vec<(usize, usize)> = (0..CH).flat_map(|y| (0..CW).map(move |x| (x, y))).filter(|&(x, y)| cells[y * CW + x].is_front()).collect();
        self.ai_axes(side, &fronts);
        if is_ai || self.sides[s].auto_buy {
            self.ai_build(side, &fronts);
            self.ai_buy(side);
        }
        if is_ai {
            self.ai_powers(side);
        }
        if !self.is_frozen(side) {
            self.ai_units(side, cells, &fronts);
        }
    }

    fn scan_cells(&self, side: u8) -> Vec<Cell> {
        let mut cells = vec![Cell::default(); CW * CH];
        for y in 0..H {
            for x in 0..W {
                let o = self.owner[y * W + x];
                if o < 0 {
                    continue;
                }
                let c = &mut cells[y / CELL * CW + x / CELL];
                if o == side as i8 {
                    c.own += 1;
                } else {
                    c.foe += 1;
                }
            }
        }
        for u in &self.units {
            if u.dead {
                continue;
            }
            let st = combat_strength(u.kind);
            if st <= 0.0 {
                continue;
            }
            let c = &mut cells[u.y as usize / CELL * CW + u.x as usize / CELL];
            if u.side == side {
                c.own_str += st;
            } else if self.can_see(side, u) {
                c.foe_str += st;
            }
        }
        cells
    }

    /// 主攻の方向を時々選び直す
    fn ai_axes(&mut self, side: u8, fronts: &[(usize, usize)]) {
        let s = side as usize;
        // 史実の主攻が決まっている間はそれに従う
        if self.day() < self.sides[s].plan.forced_until {
            let forced: Vec<usize> = self.sides[s].plan.forced.iter().copied().filter(|&c| self.map.cities[c].owner != side).collect();
            if !forced.is_empty() {
                self.sides[s].plan.axes = forced;
                return;
            }
        }
        self.sides[s].plan.axis_t -= 1.0;
        if self.sides[s].plan.axis_t > 0.0 && !self.sides[s].plan.axes.is_empty() {
            return;
        }
        self.sides[s].plan.axis_t = self.rng.range(35.0, 70.0);
        let mut scored: Vec<(f32, usize)> = Vec::new();
        for (ci, c) in self.map.cities.iter().enumerate() {
            if c.owner == side {
                continue;
            }
            let mut dist = 1e9f32;
            for &(fx, fy) in fronts {
                let (cx, cy) = ((fx * CELL) as f32 + 4.0, (fy * CELL) as f32 + 4.0);
                dist = dist.min(((cx - c.x).powi(2) + (cy - c.y).powi(2)).sqrt());
            }
            let score = (c.value as f32 + 1.0) / (dist + 15.0) * self.rng.range(0.7, 1.3);
            scored.push((score, ci));
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        self.sides[s].plan.axes = scored.iter().take(3).map(|x| x.1).collect();
    }

    fn ai_build(&mut self, side: u8, fronts: &[(usize, usize)]) {
        let s = side as usize;
        self.sides[s].plan.build_t -= 1.0;
        if self.sides[s].plan.build_t > 0.0 {
            return;
        }
        self.sides[s].plan.build_t = 5.0;
        let count = |w: &World, k: BK| w.blds.iter().filter(|b| b.side == side && b.kind == k).count();
        let cities = self.map.cities.iter().filter(|c| c.owner == side).count();
        let want: [(BK, usize); 6] = [
            (BK::Base, 3 + cities / 6),
            (BK::Workshop, 2),
            (BK::Airfield, 1 + cities / 10),
            (BK::Factory, 2 + cities / 6),
            (BK::Depot, 3 + fronts.len() / 8),
            (BK::Port, 1),
        ];
        for (k, n) in want {
            let critical = count(self, k) == 0 && matches!(k, BK::Base | BK::Workshop | BK::Airfield | BK::Depot | BK::Factory);
            if count(self, k) >= n || self.sides[s].funds < bdef(k).cost + if critical { 0.0 } else { 200.0 } {
                continue;
            }
            let pos = match k {
                BK::Port => self.map.cities.iter().find(|c| c.owner == side && c.port).map(|c| (c.x, c.y)),
                BK::Depot => self.spot_near_front(side, fronts, 14.0),
                BK::Workshop => self.spot_near_front(side, fronts, 22.0),
                BK::Base => self.spot_near_front(side, fronts, 30.0),
                _ => self.spot_near_front(side, fronts, 45.0),
            };
            let Some((x, y)) = pos else { continue };
            for _ in 0..12 {
                let (dx, dy) = (self.rng.range(-7.0, 7.0), self.rng.range(-7.0, 7.0));
                if self.place_building(side, k, x + dx, y + dy).is_ok() {
                    break;
                }
            }
            break;
        }
    }

    fn spot_near_front(&mut self, side: u8, fronts: &[(usize, usize)], back: f32) -> Option<(f32, f32)> {
        if fronts.is_empty() {
            return self.map.cities.iter().find(|c| c.owner == side).map(|c| (c.x, c.y));
        }
        for _ in 0..12 {
            let f = fronts[self.rng.below(fronts.len() as u32) as usize];
            let (fx, fy) = ((f.0 * CELL) as f32 + 4.0, (f.1 * CELL) as f32 + 4.0);
            // 前線から自国の都市の方向へ下がる
            let near = self.map.cities.iter().filter(|c| c.owner == side).min_by(|a, b| {
                let da = (a.x - fx).powi(2) + (a.y - fy).powi(2);
                let db = (b.x - fx).powi(2) + (b.y - fy).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });
            let Some(c) = near else { continue };
            let (dx, dy) = (c.x - fx, c.y - fy);
            let l = (dx * dx + dy * dy).sqrt().max(1.0);
            let (px, py) = (fx + dx / l * back, fy + dy / l * back);
            if in_map(px as i32, py as i32) && self.owner[tidx(px as i32, py as i32)] == side as i8 {
                return Some((px, py));
            }
        }
        None
    }

    fn ai_buy(&mut self, side: u8) {
        let s = side as usize;
        self.sides[s].plan.buy_t -= 1.0;
        if self.sides[s].plan.buy_t > 0.0 {
            return;
        }
        self.sides[s].plan.buy_t = 2.0;
        // 重要な建物を失っていたら、再建の資金を残す
        for k in [BK::Base, BK::Workshop, BK::Factory, BK::Airfield] {
            if !self.blds.iter().any(|b| b.side == side && b.kind == k) && self.sides[s].funds < bdef(k).cost + 100.0 {
                return;
            }
        }
        let mut have = [0u32; UK_COUNT];
        for u in &self.units {
            if u.side == side && !u.dead {
                have[u.kind as usize] += 1;
            }
        }
        for b in &self.blds {
            if b.side == side {
                for q in &b.queue {
                    have[*q as usize] += 1;
                }
            }
        }
        // 目標の編成（比率と上限）
        let mix: [(UK, f32, u32); 19] = [
            (UK::Infantry, 12.0, 90),
            (UK::Tank, 5.0, 40),
            (UK::Ifv, 5.0, 40),
            (UK::FpvTeam, 4.0, 24),
            (UK::Atgm, 3.0, 20),
            (UK::Engineer, 1.0, 6),
            (UK::Howitzer, 3.0, 22),
            (UK::Mlrs, 0.8, 7),
            (UK::Shorad, 1.8, 12),
            (UK::Sam, 0.7, 6),
            (UK::Ew, 0.8, 6),
            (UK::ReconDrone, 2.0, 12),
            (UK::BomberDrone, 1.0, 8),
            (UK::ArmedUav, 0.5, 4),
            (UK::LongDrone, 0.8, 6),
            (UK::Heli, 0.5, 5),
            (UK::Jet, 0.5, 5),
            if side == 0 { (UK::Coastal, 0.2, 2) } else { (UK::Frigate, 0.2, 3) },
            if side == 0 { (UK::SeaDrone, 0.3, 3) } else { (UK::Infantry, 0.0, 0) },
        ];
        for _ in 0..6 {
            let mut best: Option<(f32, UK)> = None;
            for (k, ratio, cap) in mix {
                if ratio <= 0.0 || have[k as usize] >= cap {
                    continue;
                }
                let need = (have[k as usize] as f32 + 1.0) / ratio;
                if best.map_or(true, |b| need < b.0) {
                    best = Some((need, k));
                }
            }
            let Some((_, k)) = best else { break };
            if self.sides[s].funds < unit_cost(side, k) + 30.0 {
                break;
            }
            match self.queue_unit(side, k) {
                Ok(()) => have[k as usize] += 1,
                Err(_) => {
                    // 建物がないなど：今は諦めて他を見るため上限扱いにする
                    have[k as usize] += 1000;
                }
            }
        }
    }

    fn ai_powers(&mut self, side: u8) {
        let s = side as usize;
        self.sides[s].plan.power_t -= 1.0;
        if self.sides[s].plan.power_t > 0.0 {
            return;
        }
        self.sides[s].plan.power_t = 15.0;
        // 見つけた敵の建物に巡航ミサイル
        if self.sides[s].funds > 900.0 {
            let tgt = self.blds.iter().find(|b| b.side != side && b.seen[s]).map(|b| (b.x, b.y));
            if let Some(p) = tgt {
                let _ = self.use_power(side, Power::Cruise, p.0, p.1);
            }
        }
        // 資金に余裕があれば動員
        if self.sides[s].funds > 1400.0 {
            let c = self.sides[s].plan.axes.first().map(|&c| (self.map.cities[c].x, self.map.cities[c].y));
            if let Some(p) = c {
                let _ = self.use_power(side, Power::Mobilize, p.0, p.1);
            }
        }
    }

    fn ai_units(&mut self, side: u8, mut cells: Vec<Cell>, fronts: &[(usize, usize)]) {
        if fronts.is_empty() {
            return;
        }
        let s = side as usize;
        let axes: Vec<(f32, f32)> = self.sides[s].plan.axes.iter().map(|&c| (self.map.cities[c].x, self.map.cities[c].y)).collect();
        let ids: Vec<u32> = self.units.iter().filter(|u| u.side == side && u.auto && !u.dead).map(|u| u.id).collect();
        for id in ids {
            let Some(i) = self.unit_index(id) else { continue };
            let k = self.units[i].kind;
            let u = &mut self.units[i];
            u.ai_t -= 1.0;
            if u.ai_t > 0.0 {
                continue;
            }
            u.ai_t = self.rng.range(7.0, 13.0);
            match k {
                UK::Infantry | UK::Tank | UK::Ifv | UK::Atgm | UK::FpvTeam => self.ai_line_unit(i, &mut cells, fronts, &axes),
                UK::Engineer => self.ai_engineer(i, fronts),
                UK::Howitzer | UK::Mlrs | UK::Coastal | UK::Shorad | UK::Sam | UK::Ew => self.ai_support(i, fronts, &axes),
                UK::ReconDrone | UK::BomberDrone | UK::ArmedUav | UK::Heli => self.ai_flyer(i, fronts),
                UK::LongDrone => self.ai_long_strike(i),
                UK::Jet => self.ai_jet(i),
                UK::Frigate => self.ai_frigate(i),
                UK::SeaDrone => self.ai_sea_drone(i),
                UK::Fpv | UK::Cruise => {}
            }
        }
    }

    /// 前線の部隊：戦力の足りない前線へ向かい、敵が薄ければ押し上げる
    fn ai_line_unit(&mut self, i: usize, cells: &mut [Cell], fronts: &[(usize, usize)], axes: &[(f32, f32)]) {
        let (x, y, side, k, order) = {
            let u = &self.units[i];
            (u.x, u.y, u.side, u.kind, u.order)
        };
        if matches!(order, Order::Attack(_)) {
            return;
        }
        // 機動部隊の一部は主攻に振り向ける
        let strike = matches!(k, UK::Tank | UK::Ifv) || self.rng.chance(0.25);
        let st = combat_strength(k);
        let mut best = None;
        let mut bs = f32::MIN;
        for &(fx, fy) in fronts {
            let c = &cells[fy * CW + fx];
            let (cx, cy) = ((fx * CELL) as f32 + 4.0, (fy * CELL) as f32 + 4.0);
            let dist = ((cx - x).powi(2) + (cy - y).powi(2)).sqrt();
            let need = 3.0 + c.foe_str * 1.3 - c.own_str - c.claimed;
            let mut score = need * 2.0 - dist * 0.09;
            if strike {
                for a in axes {
                    let da = ((cx - a.0).powi(2) + (cy - a.1).powi(2)).sqrt();
                    score += (60.0 - da).max(0.0) * 0.11;
                }
            }
            if score > bs {
                bs = score;
                best = Some((fx, fy));
            }
        }
        let Some((fx, fy)) = best else { return };
        cells[fy * CW + fx].claimed += st;
        let c = cells[fy * CW + fx];
        let (cx, cy) = ((fx * CELL) as f32 + 4.0, (fy * CELL) as f32 + 4.0);
        // 押し上げる先：敵のタイルの重心方向
        let (ex, ey) = self.enemy_direction(fx, fy, side);
        let advantage = c.own_str + c.claimed > c.foe_str * 1.15;
        let (gx, gy) = if advantage {
            // 主攻に近ければ目標都市へ、そうでなければ敵側に踏み込む
            let mut g = (cx + ex * 11.0, cy + ey * 11.0);
            if strike {
                if let Some(a) = axes.iter().find(|a| (a.0 - cx).powi(2) + (a.1 - cy).powi(2) < 50.0 * 50.0) {
                    g = *a;
                }
            }
            g
        } else {
            // 劣勢：前線の自国側で守る
            (cx - ex * 3.0, cy - ey * 3.0)
        };
        let (ox, oy) = (self.rng.range(-3.0, 3.0), self.rng.range(-3.0, 3.0));
        let (gx, gy) = self.clamp_halt(side, gx + ox, gy + oy);
        // すでに近くにいて戦闘中なら動かない
        let d = ((gx - x).powi(2) + (gy - y).powi(2)).sqrt();
        if d > 3.0 && self.units[i].target.is_none() {
            let id = self.units[i].id;
            self.command_one(id, Cmd::AttackMove, gx, gy);
        }
    }

    fn enemy_direction(&self, fx: usize, fy: usize, side: u8) -> (f32, f32) {
        let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
        let (cx, cy) = (fx * CELL + 4, fy * CELL + 4);
        for dy in (-12i32..=12).step_by(3) {
            for dx in (-12i32..=12).step_by(3) {
                let (px, py) = (cx as i32 + dx, cy as i32 + dy);
                if in_map(px, py) {
                    let o = self.owner[tidx(px, py)];
                    if o >= 0 && o != side as i8 {
                        sx += dx as f32;
                        sy += dy as f32;
                        n += 1.0;
                    }
                }
            }
        }
        if n == 0.0 {
            return (0.0, 0.0);
        }
        let l = (sx * sx + sy * sy).sqrt().max(1.0);
        (sx / l, sy / l)
    }

    fn ai_engineer(&mut self, i: usize, fronts: &[(usize, usize)]) {
        let (x, y, side) = (self.units[i].x, self.units[i].y, self.units[i].side);
        if !self.units[i].jobs.is_empty() {
            return;
        }
        // 最寄りの前線で、自国側に塹壕と地雷を作る
        let f = fronts.iter().min_by(|a, b| {
            let da = ((a.0 * CELL) as f32 - x).powi(2) + ((a.1 * CELL) as f32 - y).powi(2);
            let db = ((b.0 * CELL) as f32 - x).powi(2) + ((b.1 * CELL) as f32 - y).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        let Some(&(fx, fy)) = f else { return };
        let mut jobs = Vec::new();
        for _ in 0..30 {
            let (px, py) = ((fx * CELL) as i32 + self.rng.below(CELL as u32) as i32, (fy * CELL) as i32 + self.rng.below(CELL as u32) as i32);
            if !in_map(px, py) {
                continue;
            }
            let ti = tidx(px, py);
            if self.owner[ti] != side as i8 || self.feat[ti] & (FEAT_TRENCH | FEAT_TEETH) != 0 || self.map.terrain[ti] != Terrain::Plain {
                continue;
            }
            jobs.push((px as u16, py as u16));
            if jobs.len() >= 5 {
                break;
            }
        }
        if !jobs.is_empty() {
            let id = self.units[i].id;
            let kind = if self.rng.chance(0.3) { JobKind::Mine } else { JobKind::Trench };
            self.add_jobs(&[id], kind, &jobs);
        }
    }

    /// 火砲・防空・電子戦：前線の少し後ろに控える
    fn ai_support(&mut self, i: usize, fronts: &[(usize, usize)], axes: &[(f32, f32)]) {
        let (x, y, side, k) = {
            let u = &self.units[i];
            (u.x, u.y, u.side, u.kind)
        };
        let w = def(k).weapon;
        if self.units[i].target.is_some() || self.units[i].goal.is_some() {
            return;
        }
        let keep = match k {
            UK::Howitzer => 11.0,
            UK::Mlrs => 22.0,
            UK::Coastal => 14.0,
            UK::Sam => 16.0,
            UK::Shorad => 5.0,
            UK::Ew => 5.0,
            _ => 8.0,
        };
        // 対艦は海沿いに
        if k == UK::Coastal {
            if let Some(c) = self.map.cities.iter().filter(|c| c.owner == side && c.port).min_by(|a, b| {
                let da = (a.x - x).powi(2) + (a.y - y).powi(2);
                let db = (b.x - x).powi(2) + (b.y - y).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            }) {
                if (c.x - x).powi(2) + (c.y - y).powi(2) > 100.0 {
                    let id = self.units[i].id;
                    self.command_one(id, Cmd::Move, c.x, c.y);
                }
            }
            return;
        }
        let _ = w;
        // 主攻に近い前線を優先する
        let mut best = None;
        let mut bs = f32::MIN;
        for &(fx, fy) in fronts {
            let (cx, cy) = ((fx * CELL) as f32 + 4.0, (fy * CELL) as f32 + 4.0);
            let dist = ((cx - x).powi(2) + (cy - y).powi(2)).sqrt();
            let mut score = -dist * 0.5 + self.rng.range(0.0, 8.0);
            for a in axes {
                let da = ((cx - a.0).powi(2) + (cy - a.1).powi(2)).sqrt();
                score += (70.0 - da).max(0.0) * 0.1;
            }
            if score > bs {
                bs = score;
                best = Some((fx, fy, cx, cy));
            }
        }
        let Some((fx, fy, cx, cy)) = best else { return };
        let (ex, ey) = self.enemy_direction(fx, fy, side);
        let (ox, oy) = (self.rng.range(-5.0, 5.0), self.rng.range(-5.0, 5.0));
        let (gx, gy) = self.clamp_halt(side, cx - ex * keep + ox, cy - ey * keep + oy);
        if (gx - x).powi(2) + (gy - y).powi(2) > 36.0 {
            let id = self.units[i].id;
            self.command_one(id, Cmd::AttackMove, gx, gy);
        }
    }

    fn ai_flyer(&mut self, i: usize, fronts: &[(usize, usize)]) {
        let (side, k) = (self.units[i].side, self.units[i].kind);
        if self.units[i].target.is_some() && k != UK::ReconDrone {
            return;
        }
        let f = fronts[self.rng.below(fronts.len() as u32) as usize];
        let (ex, ey) = self.enemy_direction(f.0, f.1, side);
        let (cx, cy) = ((f.0 * CELL) as f32 + 4.0, (f.1 * CELL) as f32 + 4.0);
        let depth = match k {
            UK::ReconDrone => self.rng.range(2.0, 14.0),
            UK::Heli => -2.0,
            _ => self.rng.range(1.0, 8.0),
        };
        let (gx, gy) = (cx + ex * depth, cy + ey * depth);
        let id = self.units[i].id;
        let c = if k == UK::ReconDrone { Cmd::Move } else { Cmd::AttackMove };
        self.command_one(id, c, gx, gy);
    }

    fn ai_long_strike(&mut self, i: usize) {
        let side = self.units[i].side;
        if !matches!(self.units[i].order, Order::Idle) {
            return;
        }
        let s = side as usize;
        let (x, y) = (self.units[i].x, self.units[i].y);
        let t = self
            .blds
            .iter()
            .filter(|b| b.side != side && b.seen[s])
            .map(|b| (b.x, b.y))
            .min_by(|a, b| ((a.0 - x).powi(2) + (a.1 - y).powi(2)).partial_cmp(&((b.0 - x).powi(2) + (b.1 - y).powi(2))).unwrap_or(std::cmp::Ordering::Equal));
        let t = t.or_else(|| self.map.cities.iter().find(|c| c.owner != side).map(|c| (c.x, c.y)));
        if let Some(p) = t {
            let id = self.units[i].id;
            self.attack_ground(&[id], p.0, p.1);
        }
    }

    fn ai_jet(&mut self, i: usize) {
        if self.units[i].sortie != 0 || self.units[i].cd > 0.0 {
            return;
        }
        let side = self.units[i].side;
        // 見えている価値の高い地上目標を選ぶ（防空に近いものは避ける）
        let mut best: Option<(f32, u32)> = None;
        for o in &self.units {
            if o.side == side || o.dead || def(o.kind).pass == Pass::Air || !self.can_see(side, o) {
                continue;
            }
            let score = def(o.kind).cost + self.rng.range(0.0, 200.0);
            if best.map_or(true, |b| score > b.0) {
                best = Some((score, o.id));
            }
        }
        if let Some((_, tid)) = best {
            let id = self.units[i].id;
            self.attack_target(&[id], Target::Unit(tid));
        }
    }

    fn ai_frigate(&mut self, i: usize) {
        let side = self.units[i].side;
        let s = side as usize;
        let (x, y) = (self.units[i].x, self.units[i].y);
        // 射程内の見つけた建物を巡航ミサイルで狙う
        let r = def(UK::Frigate).weapon.range;
        let t = self.blds.iter().find(|b| b.side != side && b.seen[s] && (b.x - x).powi(2) + (b.y - y).powi(2) < r * r).map(|b| b.id);
        if let Some(t) = t {
            let id = self.units[i].id;
            self.attack_target(&[id], Target::Bld(t));
            return;
        }
        if matches!(self.units[i].order, Order::Attack(_)) {
            return;
        }
        // 敵の沿岸へゆっくり巡回する
        if self.units[i].goal.is_none() {
            let (gx, gy) = (self.rng.range(120.0, 190.0), self.rng.range(155.0, 195.0));
            let id = self.units[i].id;
            self.command_one(id, Cmd::Move, gx, gy);
        }
    }

    fn ai_sea_drone(&mut self, i: usize) {
        if !matches!(self.units[i].order, Order::Idle) {
            return;
        }
        let side = self.units[i].side;
        let t = self.units.iter().find(|o| o.side != side && o.kind == UK::Frigate && !o.dead && self.can_see(side, o)).map(|o| (o.x, o.y));
        if let Some(p) = t {
            let id = self.units[i].id;
            self.attack_ground(&[id], p.0, p.1);
        }
    }
}
