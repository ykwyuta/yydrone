//! シミュレーション本体：部隊・建物・戦闘・領土・補給・経済

use crate::defs::*;
use crate::geo::*;
use crate::history::*;
use crate::path::*;
use crate::rng::Rng;
use std::collections::VecDeque;

mod save;
pub use save::SAVE_VERSION;

/// 部隊 1 つあたりの維持費（資金／秒）
const UPKEEP: f32 = 0.3;

/// 1試合の長さ（秒）。2時間で固定
pub const WAR_SECONDS: f32 = 7200.0;

/// 1ステップの秒数（固定。描画とは独立）
pub const DT: f32 = 0.05;

/// 部隊への被害の全体倍率（小さいほど前線が長持ちする）
const LETHALITY: f32 = 0.55;

pub const VIS_GROUND: u8 = 1;
pub const VIS_AIR: u8 = 2;
pub const VIS_CLOSE: u8 = 4;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    Unit(u32),
    Bld(u32),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Order {
    Idle,
    Move(f32, f32),
    AttackMove(f32, f32),
    Attack(Target),
    AttackGround(f32, f32),
    Hold,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobKind {
    Trench,
    Mine,
    Teeth,
    Pontoon,
}

#[derive(Clone, Copy, Debug)]
pub struct Job {
    pub kind: JobKind,
    pub x: u16,
    pub y: u16,
}

pub struct Unit {
    pub id: u32,
    pub kind: UK,
    pub side: u8,
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub face: f32,
    pub order: Order,
    pub path: Vec<(u16, u16)>,
    pub pi: usize,
    pub goal: Option<(f32, f32)>,
    pub path_pending: bool,
    pub target: Option<Target>,
    pub cd: f32,
    pub still: f32,
    pub dig: f32,
    pub auto: bool,
    pub life: f32,
    pub jobs: VecDeque<Job>,
    pub job_t: f32,
    pub dead: bool,
    pub stuck: u16,
    /// 戦闘機の状態：0=待機 1=出撃 2=帰還
    pub sortie: u8,
    pub home: (f32, f32),
    /// 発砲で位置が露見している残り時間
    pub flash: f32,
    pub supplied: bool,
    pub ai_t: f32,
    pub jam_t: f32,
    pub moving: bool,
}

pub struct Bld {
    pub id: u32,
    pub kind: BK,
    pub side: u8,
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
    /// 建設の残り秒（0 で完成）
    pub building: f32,
    pub queue: VecDeque<UK>,
    pub prog: f32,
    pub rally: Option<(f32, f32)>,
    pub seen: [bool; 2],
    pub dead: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ProjKind {
    Shell,
    Missile,
    Bomb,
}

pub struct Projectile {
    pub kind: ProjKind,
    pub side: u8,
    pub x: f32,
    pub y: f32,
    pub sx: f32,
    pub sy: f32,
    pub tx: f32,
    pub ty: f32,
    pub speed: f32,
    pub target: Option<Target>,
    pub dmg: f32,
    pub splash: f32,
    pub vs: [f32; 7],
    pub done: bool,
    pub air: bool,
    /// 地点を狙った攻撃（橋に全力で当たる）
    pub aimed: bool,
}

/// 描画用の出来事（シミュレーションには影響しない）
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FxKind {
    Tracer,
    Muzzle,
    Launch,
    Blast,
    BigBlast,
    AirBurst,
    Jam,
    MineBlast,
}

#[derive(Clone, Copy, Debug)]
pub struct Fx {
    pub kind: FxKind,
    pub x: f32,
    pub y: f32,
    pub x2: f32,
    pub y2: f32,
    pub side: u8,
    /// 爆発の大きさ
    pub r: f32,
}

/// 年表に記録された出来事
#[derive(Clone, Debug)]
pub struct Chronicle {
    pub date: String,
    pub title: &'static str,
    pub text: &'static str,
    /// false は史実と盤面が離れて起きなかった出来事
    pub happened: bool,
    pub side: u8,
    pub pos: Option<(f32, f32)>,
}

#[derive(Clone, Debug)]
pub struct Notice {
    pub text: String,
    /// None は両陣営に出す
    pub side: Option<u8>,
    pub x: f32,
    pub y: f32,
    pub tone: u8,
}

pub struct SideState {
    pub funds: f32,
    pub shells: f32,
    pub tiles: u32,
    pub kills: u32,
    pub losses: u32,
    pub income: f32,
    pub shell_income: f32,
    pub power_cd: [f32; 3],
    pub ai: bool,
    pub ai_mult: f32,
    pub auto_new: bool,
    /// 生産と建設も AI に任せる
    pub auto_buy: bool,
    pub plan: crate::ai::Plan,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Move,
    AttackMove,
    Stop,
    Hold,
    Auto(bool),
}

pub struct Settings {
    pub duration: f32,
    pub seed: u64,
    /// 敵 AI の収入倍率
    pub ai_mult: f32,
    /// プレイヤーの陣営（None は AI 同士の観戦）
    pub player: Option<u8>,
}

pub struct World {
    pub map: Map,
    pub owner: Vec<i8>,
    pub cap: Vec<i16>,
    pub feat: Vec<u8>,
    pub mines: Vec<u8>,
    pub vis: [Vec<u8>; 2],
    pub supply: [Vec<u8>; 2],
    infl: [Vec<u16>; 2],
    pub units: Vec<Unit>,
    pub blds: Vec<Bld>,
    pub projs: Vec<Projectile>,
    id_map: Vec<u32>,
    bld_seq: u32,
    pub sides: [SideState; 2],
    pub t: f32,
    pub ticks: u64,
    pub duration: f32,
    pub over: bool,
    pub winner: Option<u8>,
    pub total_tiles: u32,
    pub player: Option<u8>,
    pub fx: Vec<Fx>,
    pub notices: Vec<Notice>,
    /// 地図の描き直しが要るタイル（塹壕など）
    pub dirty: Vec<u32>,
    /// 砲撃跡（x, y, 半径）
    pub scars: Vec<(f32, f32, f32)>,
    /// 記録に残す砲撃跡（読み込んだときに描き直す）
    pub scar_log: Vec<(f32, f32, f32)>,
    pub history: Vec<(f32, f32)>,
    pub sats: Vec<(u8, f32, f32, f32)>,
    pub rng: Rng,
    grid: Vec<Vec<u32>>,
    pf: PathFinder,
    pq: VecDeque<u32>,
    recent: Vec<(u32, u32, Pass, i32, i32, Vec<(u16, u16)>)>,
    pub(crate) next_event: usize,
    pub owner_rev: u32,
    /// 史実の年表
    pub chronicle: Vec<Chronicle>,
    /// 時限つきの効果
    pub mods: Vec<Mod>,
    /// AI が進まない円（陣営, x, y, 半径, 期限の日）
    pub(crate) halts: Vec<(u8, f32, f32, f32, u32)>,
    pub(crate) flooded: Vec<(u32, Terrain)>,
    pub(crate) flood_until: u32,
    /// 地形の絵を丸ごと描き直すタイル
    pub repaint: Vec<u32>,
    /// 泥濘期か（毎秒更新）
    mud_now: bool,
    /// 都市のもともとの持ち主
    city_home: Vec<u8>,
}

const GC: usize = 6;
const GW: usize = W / GC + 1;
const GH: usize = H / GC + 1;

impl World {
    pub fn new(s: Settings) -> World {
        let map = Map::new();
        let n = W * H;
        let owner = map.start_owner.clone();
        let total_tiles = owner.iter().filter(|o| **o >= 0).count() as u32;
        let mk = |funds: f32, shells: f32, ai: bool, m: f32| SideState {
            funds,
            shells,
            tiles: 0,
            kills: 0,
            losses: 0,
            income: 0.0,
            shell_income: 0.0,
            power_cd: [0.0; 3],
            ai,
            ai_mult: m,
            auto_new: true,
            auto_buy: false,
            plan: Default::default(),
        };
        let ai0 = s.player != Some(0);
        let ai1 = s.player != Some(1);
        let m0 = if ai0 && s.player.is_some() { s.ai_mult } else { 1.0 };
        let m1 = if ai1 && s.player.is_some() { s.ai_mult } else { 1.0 };
        let mut w = World {
            map,
            owner,
            cap: vec![0; n],
            feat: vec![0; n],
            mines: vec![0; n],
            vis: [vec![0; n], vec![0; n]],
            supply: [vec![0; n], vec![0; n]],
            infl: [vec![0; n], vec![0; n]],
            units: Vec::new(),
            blds: Vec::new(),
            projs: Vec::new(),
            id_map: Vec::new(),
            bld_seq: 0,
            sides: [mk(2000.0, 350.0, ai0, m0), mk(2000.0, 500.0, ai1, m1)],
            t: 0.0,
            ticks: 0,
            duration: s.duration,
            over: false,
            winner: None,
            total_tiles,
            player: s.player,
            fx: Vec::new(),
            notices: Vec::new(),
            dirty: Vec::new(),
            scars: Vec::new(),
            scar_log: Vec::new(),
            history: Vec::new(),
            sats: Vec::new(),
            rng: Rng::new(s.seed ^ 0x5eed),
            grid: vec![Vec::new(); GW * GH],
            pf: PathFinder::new(),
            pq: VecDeque::new(),
            recent: Vec::new(),
            next_event: 0,
            owner_rev: 1,
            chronicle: Vec::new(),
            mods: Vec::new(),
            halts: Vec::new(),
            flooded: Vec::new(),
            flood_until: 0,
            repaint: Vec::new(),
            mud_now: false,
            city_home: Vec::new(),
        };
        w.city_home = w.map.cities.iter().map(|c| c.owner).collect();
        w.count_tiles();
        w.setup_buildings();
        w.setup_front();
        w.update_supply();
        w.update_vision();
        w.history.push((0.0, w.share(0)));
        w.update_season();
        w.run_history();
        w
    }

    pub fn share(&self, side: u8) -> f32 {
        self.sides[side as usize].tiles as f32 / self.total_tiles.max(1) as f32
    }

    /// 開戦からの日数（2022年2月24日〜2026年10月31日を制限時間に割り当てる）
    pub fn day(&self) -> u32 {
        frac_to_day(self.t / self.duration)
    }

    pub fn date(&self) -> (u32, u32, u32) {
        day_to_date(self.day())
    }

    /// 3/15〜4/30 と 10/10〜11/20 は泥濘期
    pub fn mud(&self) -> bool {
        self.mud_now
    }

    pub fn winter(&self) -> bool {
        let (_, m, _) = self.date();
        m == 12 || m <= 2
    }

    pub fn date_text(&self) -> String {
        let (y, m, d) = self.date();
        format!("{y}年{m}月{d}日")
    }

    fn update_season(&mut self) {
        let (_, m, d) = self.date();
        self.mud_now = (m == 3 && d >= 15) || m == 4 || (m == 10 && d >= 10) || (m == 11 && d <= 20);
    }

    /// その陣営・場所にかかっている効果を順に見る
    fn mods_at(&self, side: u8, x: f32, y: f32) -> impl Iterator<Item = ModKind> + '_ {
        self.mods.iter().filter(move |m| m.side == side && m.zone.is_none_or(|z| (z.0 - x).powi(2) + (z.1 - y).powi(2) < z.2 * z.2)).map(|m| m.kind)
    }

    pub fn has_outage(&self, side: u8, x: f32, y: f32) -> bool {
        self.mods_at(side, x, y).any(|k| k == ModKind::Outage)
    }

    pub fn has_uplink(&self, side: u8) -> bool {
        self.mods.iter().any(|m| m.side == side && m.kind == ModKind::Uplink)
    }

    pub fn is_frozen(&self, side: u8) -> bool {
        self.mods.iter().any(|m| m.side == side && m.kind == ModKind::Frozen)
    }

    /// AI の目標が進入禁止の円の中なら、円の縁まで押し戻す
    pub(crate) fn clamp_halt(&self, side: u8, x: f32, y: f32) -> (f32, f32) {
        for h in &self.halts {
            if h.0 == side {
                let (dx, dy) = (x - h.1, y - h.2);
                let d = (dx * dx + dy * dy).sqrt();
                if d < h.3 {
                    let k = h.3 / d.max(0.01);
                    return (h.1 + dx * k, h.2 + dy * k);
                }
            }
        }
        (x, y)
    }

    pub(crate) fn clear_path_cache(&mut self) {
        self.recent.clear();
    }

    pub(crate) fn territory_changed(&mut self) {
        self.owner_rev = self.owner_rev.wrapping_add(1);
        self.count_tiles();
        self.check_cities();
        self.check_buildings_captured();
    }

    fn city_pos(&self, name: &str) -> (f32, f32) {
        let c = self.map.cities.iter().find(|c| c.name == name).expect("city");
        (c.x, c.y)
    }

    fn setup_buildings(&mut self) {
        let list: [(&str, BK, u8, f32, f32); 26] = [
            ("キエヴァ", BK::Base, 0, 3.0, 5.0),
            ("ドニプロヴァ", BK::Base, 0, -4.0, -4.0),
            ("ハルキア", BK::Base, 0, -6.0, 3.0),
            ("ポルタ", BK::Airfield, 0, 3.0, 3.0),
            ("クリヴォイ", BK::Factory, 0, 3.0, -3.0),
            ("ドニプロヴァ", BK::Workshop, 0, -6.0, 4.0),
            ("ザポリア", BK::Depot, 0, -4.0, 2.0),
            ("クラマト", BK::Depot, 0, -5.0, -1.0),
            ("ミコラ", BK::Base, 0, 3.0, -3.0),
            ("オデスカ", BK::Port, 0, 0.0, -1.0),
            ("ミコラ", BK::Depot, 0, 4.0, 2.0),
            ("ポルタ", BK::Depot, 0, 5.0, -3.0),
            ("スーミャ", BK::Depot, 0, -3.0, 3.0),
            ("ロストヴァ", BK::Base, 1, 3.0, 2.0),
            ("ベルゴラ", BK::Base, 1, 3.0, -2.0),
            ("ドネスカ", BK::Base, 1, 4.0, 3.0),
            ("クルスカ", BK::Airfield, 1, 4.0, 2.0),
            ("クラスノーダ", BK::Airfield, 1, 3.0, -3.0),
            ("ロストヴァ", BK::Factory, 1, 6.0, -4.0),
            ("ヴォロニア", BK::Factory, 1, 3.0, 3.0),
            ("ベルゴラ", BK::Workshop, 1, 6.0, 4.0),
            ("ドネスカ", BK::Depot, 1, 2.0, -4.0),
            ("シンフェラ", BK::Depot, 1, 2.0, 2.0),
            ("セヴァストラ", BK::Port, 1, 0.0, 0.0),
            ("ノヴォロス", BK::Port, 1, 0.0, 0.0),
            ("クルスカ", BK::Depot, 1, -4.0, 3.0),
        ];
        for (c, k, side, dx, dy) in list {
            let p = self.city_pos(c);
            let id = self.add_building(k, side, p.0 + dx, p.1 + dy);
            if let Some(b) = self.blds.iter_mut().find(|b| b.id == id) {
                b.building = 0.0;
            }
        }
        // クリメアのシンフェラに小さな基地
        let p = self.city_pos("シンフェラ");
        let id = self.add_building(BK::Base, 1, p.0 - 4.0, p.1 - 2.0);
        if let Some(b) = self.blds.iter_mut().find(|b| b.id == id) {
            b.building = 0.0;
        }
    }

    pub fn add_building(&mut self, k: BK, side: u8, x: f32, y: f32) -> u32 {
        let d = bdef(k);
        self.bld_seq += 1;
        let id = self.bld_seq;
        self.blds.push(Bld { id, kind: k, side, x, y, hp: d.hp, max_hp: d.hp, building: d.time, queue: VecDeque::new(), prog: 0.0, rally: None, seen: [side == 0, side == 1], dead: false });
        id
    }

    /// 開戦時の前線に部隊を並べる
    fn setup_front(&mut self) {
        let mut front: [Vec<(i32, i32)>; 2] = [Vec::new(), Vec::new()];
        for y in 2..H as i32 - 2 {
            for x in 2..W as i32 - 2 {
                let o = self.owner[tidx(x, y)];
                if o < 0 {
                    continue;
                }
                for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                    let o2 = self.owner[tidx(x + dx, y + dy)];
                    if o2 >= 0 && o2 != o {
                        front[o as usize].push((x, y));
                        break;
                    }
                }
            }
        }
        let mut anchors: [Vec<(i32, i32)>; 2] = [Vec::new(), Vec::new()];
        for side in 0..2usize {
            for &p in &front[side] {
                if anchors[side].iter().all(|c| (c.0 - p.0).abs() + (c.1 - p.1).abs() >= 18) {
                    anchors[side].push(p);
                }
            }
        }
        // 両軍の前線部隊の数を揃える
        let squads = anchors[0].len().min(anchors[1].len());
        for side in 0..2u8 {
            let mut count = 0;
            for &(cx, cy) in anchors[side as usize].iter().take(squads) {
                // 自国側へ少し下げて置く
                let (bx, by) = self.back_from_front(side, cx, cy, 8);
                let squad: &[UK] = match count % 4 {
                    0 => &[UK::Infantry, UK::Infantry, UK::Tank, UK::Atgm],
                    1 => &[UK::Infantry, UK::Ifv, UK::FpvTeam],
                    2 => &[UK::Infantry, UK::Infantry, UK::Ifv, UK::Engineer],
                    _ => &[UK::Infantry, UK::Tank, UK::Ifv, UK::Shorad],
                };
                for (i, k) in squad.iter().enumerate() {
                    let a = i as f32 * 2.1;
                    let id = self.spawn(*k, side, bx as f32 + a.cos() * 1.6, by as f32 + a.sin() * 1.6);
                    // 開戦時の前線部隊はすでに陣地についている
                    if let Some(u) = self.unit_index(id).map(|j| &mut self.units[j]) {
                        u.still = 5.0;
                        u.dig = 1.0;
                    }
                }
                // 砲兵は後ろに
                if count % 3 == 0 {
                    let (ax, ay) = self.back_from_front(side, cx, cy, 14);
                    self.spawn(UK::Howitzer, side, ax as f32, ay as f32);
                }
                count += 1;
            }
        }
        // 航空戦力・艦艇・偵察機
        for (k, side, c) in [
            (UK::Heli, 0u8, "ポルタ"),
            (UK::ArmedUav, 0, "ポルタ"),
            (UK::ReconDrone, 0, "ドニプロヴァ"),
            (UK::ReconDrone, 0, "ハルキア"),
            (UK::Mlrs, 0, "ドニプロヴァ"),
            (UK::Sam, 0, "ドニプロヴァ"),
            (UK::Sam, 0, "キエヴァ"),
            (UK::Ew, 0, "クラマト"),
            (UK::Coastal, 0, "オデスカ"),
            (UK::Jet, 0, "ポルタ"),
            (UK::Heli, 1, "クルスカ"),
            (UK::Jet, 1, "クルスカ"),
            (UK::Jet, 1, "クラスノーダ"),
            (UK::ReconDrone, 1, "ベルゴラ"),
            (UK::ReconDrone, 1, "ドネスカ"),
            (UK::Mlrs, 1, "ドネスカ"),
            (UK::Sam, 1, "ベルゴラ"),
            (UK::Sam, 1, "シンフェラ"),
            (UK::Ew, 1, "ドネスカ"),
        ] {
            let p = self.city_pos(c);
            let (dx, dy) = (self.rng.range(-3.0, 3.0), self.rng.range(-3.0, 3.0));
            self.spawn(k, side, p.0 + dx, p.1 + dy);
        }
        for c in ["セヴァストラ", "ノヴォロス"] {
            let p = self.city_pos(c);
            if let Some((sx, sy)) = nearest_passable(&self.map, &self.feat, Pass::Sea, p.0 as i32, p.1 as i32) {
                self.spawn(UK::Frigate, 1, sx as f32 + 0.5, sy as f32 + 0.5);
            }
        }
        // 開戦前からドンバの前線には塹壕がある
        let tiles: Vec<usize> = (0..W * H)
            .filter(|&i| {
                let (x, y) = ((i % W) as i32, (i / W) as i32);
                let lat_ok = (150..W as i32 - 60).contains(&x) && y > 60 && y < 130;
                let scatter = crate::rng::hash2(x, y, 91) < 0.08;
                lat_ok && self.owner[i] >= 0 && (self.near_enemy_tile(x, y, 1) || (scatter && self.near_enemy_tile(x, y, 6)))
            })
            .collect();
        for i in tiles {
            if matches!(self.map.terrain[i], Terrain::Plain | Terrain::Forest) {
                self.feat[i] |= FEAT_TRENCH;
            }
        }
    }

    fn near_enemy_tile(&self, x: i32, y: i32, r: i32) -> bool {
        let o = self.owner[tidx(x, y)];
        for dy in -r..=r {
            for dx in -r..=r {
                if in_map(x + dx, y + dy) {
                    let o2 = self.owner[tidx(x + dx, y + dy)];
                    if o2 >= 0 && o2 != o {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 前線の点から自国側へ下がった通行可能な位置
    fn back_from_front(&self, side: u8, x: i32, y: i32, dist: i32) -> (i32, i32) {
        let mut best = (x, y);
        let mut bs = -1.0f32;
        for a in 0..16 {
            let ang = a as f32 / 16.0 * std::f32::consts::TAU;
            let (px, py) = (x + (ang.cos() * dist as f32) as i32, y + (ang.sin() * dist as f32) as i32);
            if !in_map(px, py) {
                continue;
            }
            let i = tidx(px, py);
            if self.owner[i] != side as i8 || self.map.cost(i, Pass::Veh).is_none() {
                continue;
            }
            // 敵から離れた方向ほど良い
            let mut s = 0.0;
            for b in 0..8 {
                let an2 = b as f32 / 8.0 * std::f32::consts::TAU;
                let (qx, qy) = (px + (an2.cos() * 8.0) as i32, py + (an2.sin() * 8.0) as i32);
                if in_map(qx, qy) && self.owner[tidx(qx, qy)] == side as i8 {
                    s += 1.0;
                }
            }
            if s > bs {
                bs = s;
                best = (px, py);
            }
        }
        best
    }

    // ------------------------------------------------------------ 部隊の追加と検索

    pub fn spawn(&mut self, k: UK, side: u8, x: f32, y: f32) -> u32 {
        let d = def(k);
        let id = self.id_map.len() as u32;
        self.id_map.push(self.units.len() as u32);
        let (mut x, mut y) = (x, y);
        if d.pass != Pass::Air {
            if let Some((px, py)) = nearest_passable(&self.map, &self.feat, d.pass, x as i32, y as i32) {
                if (px, py) != (x as i32, y as i32) {
                    x = px as f32 + 0.5;
                    y = py as f32 + 0.5;
                }
            }
        }
        x = x.clamp(0.5, W as f32 - 1.0);
        y = y.clamp(0.5, H as f32 - 1.0);
        self.units.push(Unit {
            id,
            kind: k,
            side,
            x,
            y,
            hp: d.hp,
            face: if side == 0 { 0.0 } else { std::f32::consts::PI },
            order: Order::Idle,
            path: Vec::new(),
            pi: 0,
            goal: None,
            path_pending: false,
            target: None,
            cd: self.rng.range(0.0, 1.0),
            still: 0.0,
            dig: 0.0,
            auto: self.sides[side as usize].ai || self.sides[side as usize].auto_new,
            life: d.endurance,
            jobs: VecDeque::new(),
            job_t: 0.0,
            dead: false,
            stuck: 0,
            sortie: 0,
            home: (x, y),
            flash: 0.0,
            supplied: true,
            ai_t: self.rng.range(0.0, 4.0),
            jam_t: 0.0,
            moving: false,
        });
        id
    }

    #[inline]
    pub fn unit_index(&self, id: u32) -> Option<usize> {
        let i = *self.id_map.get(id as usize)? as usize;
        if i < self.units.len() && self.units[i].id == id && !self.units[i].dead {
            Some(i)
        } else {
            None
        }
    }

    pub fn unit(&self, id: u32) -> Option<&Unit> {
        self.unit_index(id).map(|i| &self.units[i])
    }

    pub fn bld(&self, id: u32) -> Option<&Bld> {
        self.blds.iter().find(|b| b.id == id && !b.dead)
    }

    fn target_pos(&self, t: Target) -> Option<(f32, f32)> {
        match t {
            Target::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
            Target::Bld(id) => self.bld(id).map(|b| (b.x, b.y)),
        }
    }

    /// その陣営からこの部隊が見えているか
    pub fn can_see(&self, side: u8, u: &Unit) -> bool {
        if u.side == side {
            return true;
        }
        let d = def(u.kind);
        let i = tidx(u.x as i32, u.y as i32);
        let v = self.vis[side as usize][i];
        if d.pass == Pass::Air || d.pass == Pass::Sea {
            return v != 0;
        }
        if u.flash > 0.0 {
            return true;
        }
        if self.concealed(u) {
            v & (VIS_AIR | VIS_CLOSE) != 0
        } else {
            v != 0
        }
    }

    /// 森・市街・塹壕で動かずにいると、地上からは見つけにくい
    pub fn concealed(&self, u: &Unit) -> bool {
        if u.still < 2.0 {
            return false;
        }
        let i = tidx(u.x as i32, u.y as i32);
        matches!(self.map.terrain[i], Terrain::Forest | Terrain::Urban) || self.feat[i] & FEAT_TRENCH != 0
    }

    // ------------------------------------------------------------ 命令（プレイヤーと AI の共通窓口）

    /// 部隊を一か所に向かわせる（複数なら少しずつ位置をずらす）
    pub fn command(&mut self, ids: &[u32], c: Cmd, x: f32, y: f32) {
        let n = ids.len();
        let cols = (n as f32).sqrt().ceil() as i32;
        for (i, &id) in ids.iter().enumerate() {
            let (ox, oy) = if n > 1 && matches!(c, Cmd::Move | Cmd::AttackMove) {
                let (cx, cy) = ((i as i32 % cols) as f32 - cols as f32 / 2.0, (i as i32 / cols) as f32 - cols as f32 / 2.0);
                (cx * 1.6, cy * 1.6)
            } else {
                (0.0, 0.0)
            };
            self.command_one(id, c, x + ox, y + oy);
        }
    }

    pub fn command_one(&mut self, id: u32, c: Cmd, x: f32, y: f32) {
        let Some(i) = self.unit_index(id) else { return };
        let d = def(self.units[i].kind);
        match c {
            Cmd::Move | Cmd::AttackMove => {
                if matches!(self.units[i].kind, UK::Fpv | UK::Cruise) {
                    return;
                }
                let u = &mut self.units[i];
                u.order = if c == Cmd::Move { Order::Move(x, y) } else { Order::AttackMove(x, y) };
                u.target = None;
                if c == Cmd::Move {
                    u.jobs.clear();
                }
                if u.kind == UK::Jet {
                    u.sortie = 1;
                }
                self.set_goal(i, x, y);
            }
            Cmd::Stop => {
                let u = &mut self.units[i];
                if !d.suicide {
                    u.order = Order::Idle;
                    u.path.clear();
                    u.goal = None;
                    u.target = None;
                    u.jobs.clear();
                }
            }
            Cmd::Hold => {
                let u = &mut self.units[i];
                if !d.suicide {
                    u.order = Order::Hold;
                    u.path.clear();
                    u.goal = None;
                    u.jobs.clear();
                }
            }
            Cmd::Auto(a) => self.units[i].auto = a,
        }
    }

    pub fn attack_target(&mut self, ids: &[u32], t: Target) {
        let Some(p) = self.target_pos(t) else { return };
        for &id in ids {
            let Some(i) = self.unit_index(id) else { continue };
            if def(self.units[i].kind).weapon.dmg <= 0.0 {
                continue;
            }
            self.units[i].order = Order::Attack(t);
            self.units[i].target = Some(t);
            if self.units[i].kind == UK::Jet {
                self.units[i].sortie = 1;
            }
            self.set_goal(i, p.0, p.1);
        }
    }

    pub fn attack_ground(&mut self, ids: &[u32], x: f32, y: f32) {
        for &id in ids {
            let Some(i) = self.unit_index(id) else { continue };
            let d = def(self.units[i].kind);
            if d.weapon.dmg <= 0.0 {
                continue;
            }
            self.units[i].order = Order::AttackGround(x, y);
            self.units[i].target = None;
            if self.units[i].kind == UK::Jet {
                self.units[i].sortie = 1;
            }
            if d.suicide {
                self.set_goal(i, x, y);
            }
        }
    }

    pub fn add_jobs(&mut self, ids: &[u32], kind: JobKind, tiles: &[(u16, u16)]) {
        let engineers: Vec<u32> = ids.iter().copied().filter(|id| self.unit(*id).is_some_and(|u| u.kind == UK::Engineer)).collect();
        if engineers.is_empty() || tiles.is_empty() {
            return;
        }
        let per = tiles.len().div_ceil(engineers.len());
        for (ei, chunk) in tiles.chunks(per).enumerate() {
            let Some(i) = self.unit_index(engineers[ei]) else { continue };
            let u = &mut self.units[i];
            for &(x, y) in chunk {
                u.jobs.push_back(Job { kind, x, y });
            }
            if !matches!(u.order, Order::Attack(_)) {
                u.order = Order::Idle;
            }
        }
    }

    fn set_goal(&mut self, i: usize, x: f32, y: f32) {
        let u = &mut self.units[i];
        u.goal = Some((x.clamp(0.5, W as f32 - 1.0), y.clamp(0.5, H as f32 - 1.0)));
        u.path.clear();
        u.pi = 0;
        u.stuck = 0;
        if def(u.kind).pass != Pass::Air && !u.path_pending {
            u.path_pending = true;
            let id = u.id;
            self.pq.push_back(id);
        }
    }

    /// 生産の予約。理由つきで失敗を返す
    pub fn queue_unit(&mut self, side: u8, k: UK) -> Result<(), &'static str> {
        let d = def(k);
        if !d.sides[side as usize] {
            return Err("この陣営では使えない");
        }
        let need = prod_building(d.prod).ok_or("生産できない")?;
        let cost = unit_cost(side, k);
        if self.sides[side as usize].funds < cost {
            return Err("資金が足りない");
        }
        let b = self.blds.iter_mut().filter(|b| b.side == side && b.kind == need && b.building <= 0.0 && !b.dead).min_by_key(|b| b.queue.len());
        let Some(b) = b else {
            return Err(match need {
                BK::Base => "軍事基地が必要",
                BK::Airfield => "飛行場が必要",
                BK::Workshop => "ドローン工房が必要",
                _ => "軍港が必要",
            });
        };
        if b.queue.len() >= 12 {
            return Err("生産の予約がいっぱい");
        }
        b.queue.push_back(k);
        self.sides[side as usize].funds -= cost;
        Ok(())
    }

    pub fn queued_count(&self, side: u8, k: UK) -> usize {
        self.blds.iter().filter(|b| b.side == side).map(|b| b.queue.iter().filter(|q| **q == k).count()).sum()
    }

    /// 予約を1つ取り消して資金を返す
    pub fn cancel_unit(&mut self, side: u8, k: UK) -> bool {
        for b in self.blds.iter_mut().filter(|b| b.side == side) {
            if let Some(p) = b.queue.iter().rposition(|q| *q == k) {
                b.queue.remove(p);
                if p == 0 {
                    b.prog = 0.0;
                }
                self.sides[side as usize].funds += unit_cost(side, k);
                return true;
            }
        }
        false
    }

    /// 建物をこの場所に置けるか（理由つき）
    pub fn can_place(&self, side: u8, k: BK, x: f32, y: f32) -> Result<(), &'static str> {
        let (xi, yi) = (x as i32, y as i32);
        if !in_map(xi, yi) {
            return Err("地図の外");
        }
        let i = tidx(xi, yi);
        if self.owner[i] != side as i8 {
            return Err("自国の領土に建ててください");
        }
        if !matches!(self.map.terrain[i], Terrain::Plain | Terrain::Forest | Terrain::Urban) {
            return Err("この地形には建てられない");
        }
        if self.blds.iter().any(|b| !b.dead && (b.x - x).abs() < 3.0 && (b.y - y).abs() < 3.0) {
            return Err("他の建物に近すぎる");
        }
        if k == BK::Port {
            let near_sea = (-5..=5).any(|dy| (-5..=5).any(|dx| in_map(xi + dx, yi + dy) && self.map.terrain[tidx(xi + dx, yi + dy)] == Terrain::Sea));
            if !near_sea {
                return Err("軍港は海沿いにしか建てられない");
            }
        }
        Ok(())
    }

    pub fn place_building(&mut self, side: u8, k: BK, x: f32, y: f32) -> Result<u32, &'static str> {
        self.can_place(side, k, x, y)?;
        let cost = bdef(k).cost;
        if self.sides[side as usize].funds < cost {
            return Err("資金が足りない");
        }
        self.sides[side as usize].funds -= cost;
        Ok(self.add_building(k, side, x, y))
    }

    pub fn use_power(&mut self, side: u8, p: Power, x: f32, y: f32) -> Result<(), &'static str> {
        let d = pdef(p);
        let s = &self.sides[side as usize];
        if s.power_cd[p as usize] > 0.0 {
            return Err("準備中");
        }
        if s.funds < d.cost {
            return Err("資金が足りない");
        }
        match p {
            Power::Cruise => {
                // 自国の後方から発射する
                let from = self.map.cities.iter().filter(|c| c.owner == side).map(|c| (c.x, c.y)).max_by(|a, b| {
                    let da = (a.0 - x).powi(2) + (a.1 - y).powi(2);
                    let db = (b.0 - x).powi(2) + (b.1 - y).powi(2);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                });
                let Some(from) = from else { return Err("発射できる都市がない") };
                self.launch_cruise(side, from.0, from.1, x, y);
            }
            Power::Satellite => self.sats.push((side, x, y, 15.0)),
            Power::Mobilize => {
                let c = self.map.cities.iter().filter(|c| c.owner == side).map(|c| (c.x, c.y)).min_by(|a, b| {
                    let da = (a.0 - x).powi(2) + (a.1 - y).powi(2);
                    let db = (b.0 - x).powi(2) + (b.1 - y).powi(2);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                });
                let Some(c) = c else { return Err("動員できる都市がない") };
                for k in 0..5 {
                    let a = k as f32 * 1.26;
                    self.spawn(UK::Infantry, side, c.0 + a.cos() * 2.0, c.1 + a.sin() * 2.0);
                }
            }
        }
        let s = &mut self.sides[side as usize];
        s.funds -= d.cost;
        s.power_cd[p as usize] = d.cooldown;
        Ok(())
    }

    fn launch_cruise(&mut self, side: u8, x: f32, y: f32, tx: f32, ty: f32) {
        let id = self.spawn(UK::Cruise, side, x, y);
        if let Some(i) = self.unit_index(id) {
            self.units[i].order = Order::AttackGround(tx, ty);
            self.units[i].goal = Some((tx, ty));
        }
        self.fx.push(Fx { kind: FxKind::Launch, x, y, x2: tx, y2: ty, side, r: 1.0 });
    }

    // ------------------------------------------------------------ 1ステップ

    pub fn tick(&mut self) {
        if self.over {
            return;
        }
        self.t += DT;
        self.ticks += 1;
        let tk = self.ticks;
        if tk % 5 == 0 {
            self.update_vision();
        }
        self.rebuild_grid();
        self.run_paths();
        let n = self.units.len();
        for i in 0..n {
            if !self.units[i].dead {
                self.update_unit(i);
            }
        }
        self.update_projectiles();
        if tk % 2 == 0 {
            self.separate();
        }
        if tk % 10 == 0 {
            self.update_control();
            self.update_buildings(0.5);
        }
        if tk % 20 == 0 {
            self.update_season();
            self.economy(1.0);
            self.run_history();
            for s in 0..2u8 {
                self.ai_tick(s);
            }
        }
        if tk % 60 == 0 {
            self.update_supply();
            self.pockets();
        }
        if tk % 400 == 0 {
            self.partisans();
        }
        if tk % 100 == 0 {
            self.history.push((self.t, self.share(0)));
        }
        for s in &mut self.sides {
            for c in &mut s.power_cd {
                *c = (*c - DT).max(0.0);
            }
        }
        for sat in &mut self.sats {
            sat.3 -= DT;
        }
        self.sats.retain(|s| s.3 > 0.0);
        self.cleanup();
        if self.t >= self.duration {
            self.over = true;
            self.count_tiles();
            let (a, b) = (self.sides[0].tiles, self.sides[1].tiles);
            self.winner = if a == b { None } else if a > b { Some(0) } else { Some(1) };
            self.history.push((self.t, self.share(0)));
        }
    }

    fn cleanup(&mut self) {
        let side_dead: Vec<(u8, UK)> = self.units.iter().filter(|u| u.dead).map(|u| (u.side, u.kind)).collect();
        for (s, k) in side_dead {
            if !matches!(k, UK::Fpv | UK::Cruise | UK::LongDrone | UK::SeaDrone) {
                self.sides[s as usize].losses += 1;
            }
        }
        if self.units.iter().any(|u| u.dead) {
            self.units.retain(|u| !u.dead);
            for (i, u) in self.units.iter().enumerate() {
                self.id_map[u.id as usize] = i as u32;
            }
        }
        self.projs.retain(|p| !p.done);
        self.blds.retain(|b| !b.dead);
    }

    fn rebuild_grid(&mut self) {
        for c in &mut self.grid {
            c.clear();
        }
        for (i, u) in self.units.iter().enumerate() {
            if u.dead {
                continue;
            }
            let gi = (u.y as usize / GC).min(GH - 1) * GW + (u.x as usize / GC).min(GW - 1);
            self.grid[gi].push(i as u32);
        }
    }

    fn run_paths(&mut self) {
        let mut budget = 14000i64;
        while budget > 0 {
            let Some(id) = self.pq.pop_front() else { break };
            let Some(i) = self.unit_index(id) else { continue };
            let (x, y, goal, pass) = {
                let u = &mut self.units[i];
                u.path_pending = false;
                (u.x as i32, u.y as i32, u.goal, def(u.kind).pass)
            };
            let Some(g) = goal else { continue };
            let (gx, gy) = (g.0 as i32, g.1 as i32);
            // 同じ目的地へ近くから向かう部隊の道順は使い回す
            let gkey = (gx / 3) as u32 * 1000 + (gy / 3) as u32;
            let hit = self.recent.iter().find(|r| r.0 == gkey && r.2 == pass && (r.3 - x).abs() + (r.4 - y).abs() <= 7 && self.units[i].stuck == 0);
            if let Some(r) = hit {
                let p = r.5.clone();
                let u = &mut self.units[i];
                u.path = p;
                u.pi = 0;
                budget -= 20;
                continue;
            }
            let r = self.pf.find(&self.map, &self.feat, pass, x, y, gx, gy, 1.25, 60000);
            budget -= self.pf.last_exp as i64 + 50;
            if let Some(p) = r {
                if self.recent.len() >= 24 {
                    self.recent.remove(0);
                }
                self.recent.push((gkey, 0, pass, x, y, p.clone()));
                let u = &mut self.units[i];
                u.path = p;
                u.pi = 0;
                u.stuck = 0;
            } else {
                // 行けない：その場で止まる
                let u = &mut self.units[i];
                u.goal = None;
                u.path.clear();
                if matches!(u.order, Order::Move(..) | Order::AttackMove(..)) {
                    u.order = Order::Idle;
                }
            }
        }
        // 古い経路の使い回しは地形が変わるので時々捨てる
        if self.ticks % 200 == 0 {
            self.recent.clear();
        }
    }

    fn speed_of(&self, u: &Unit) -> f32 {
        let d = def(u.kind);
        let mut s = d.speed;
        if d.pass == Pass::Air || d.pass == Pass::Sea {
            return s;
        }
        let i = tidx(u.x as i32, u.y as i32);
        let on_road = self.map.road[i];
        match self.map.terrain[i] {
            Terrain::River => s *= 0.3,
            Terrain::Forest if !on_road => s *= if d.pass == Pass::Veh { 0.6 } else { 0.8 },
            Terrain::Urban if d.pass == Pass::Veh && !on_road => s *= 0.8,
            _ => {}
        }
        if on_road {
            s *= 1.4;
        } else if self.mud() && d.pass == Pass::Veh {
            s *= 0.55;
        }
        if !u.supplied {
            s *= 0.75;
        }
        for k in self.mods_at(u.side, u.x, u.y) {
            if let ModKind::Slow(m) = k {
                s *= m;
            }
        }
        s
    }

    fn update_unit(&mut self, i: usize) {
        let k = self.units[i].kind;
        let d = def(k);
        {
            let u = &mut self.units[i];
            u.cd -= DT;
            u.flash -= DT;
            if d.endurance > 0.0 {
                u.life -= DT;
                if u.life <= 0.0 {
                    u.dead = true;
                    if d.suicide {
                        // 電池切れの自爆機はその場に落ちる
                    }
                    return;
                }
            }
        }
        // 電波妨害
        if d.armor == Armor::Drone && k != UK::Cruise && self.ticks % 10 == (i as u64 % 10) {
            self.check_jam(i);
            if self.units[i].dead {
                return;
            }
        }
        if d.suicide {
            self.update_suicide(i);
            return;
        }
        if k == UK::Jet {
            self.update_jet(i);
            return;
        }
        // 補給の確認
        if self.ticks % 20 == (i as u64 % 20) {
            self.update_unit_supply(i);
        }
        // 標的の更新
        let retarget = self.ticks % 10 == (i as u64 % 10);
        if d.weapon.dmg > 0.0 && retarget {
            self.pick_target(i);
        }
        self.fire_logic(i);
        let moved = self.move_logic(i);
        {
            let u = &mut self.units[i];
            u.moving = moved;
            if moved {
                u.still = 0.0;
                u.dig = 0.0;
            } else {
                u.still += DT;
            }
        }
        if self.ticks % 10 == (i as u64 % 10) && matches!(d.pass, Pass::Inf | Pass::Veh) {
            self.ground_chores(i);
        }
        if !moved && d.pass == Pass::Air {
            // 滞空：その場でゆっくり回る
            let u = &mut self.units[i];
            u.face += DT * 0.8;
            let r = 0.25;
            u.x += u.face.cos() * r * DT * d.speed;
            u.y += u.face.sin() * r * DT * d.speed;
        }
        if !moved && matches!(d.pass, Pass::Air) && matches!(self.units[i].order, Order::Idle) && self.units[i].target.is_none() {
            // 目的地に着いた飛行部隊は高度を保つだけ
        }
        let u = &mut self.units[i];
        u.x = u.x.clamp(0.5, W as f32 - 1.0);
        u.y = u.y.clamp(0.5, H as f32 - 1.0);
    }

    fn update_unit_supply(&mut self, i: usize) {
        let (x, y, side) = {
            let u = &self.units[i];
            (u.x as i32, u.y as i32, u.side as usize)
        };
        let sup = &self.supply[side];
        let mut ok = false;
        for (dx, dy) in [(0, 0), (5, 0), (-5, 0), (0, 5), (0, -5)] {
            if in_map(x + dx, y + dy) && sup[tidx(x + dx, y + dy)] != 0 {
                ok = true;
                break;
            }
        }
        let cut = self.mods_at(side as u8, x as f32, y as f32).any(|k| k == ModKind::NoSupply);
        let ok = ok && !cut;
        let u = &mut self.units[i];
        let d = def(u.kind);
        u.supplied = ok || d.pass == Pass::Air || d.pass == Pass::Sea;
        // 補給下で止まっていれば少しずつ回復する。補給切れの地上部隊は消耗する
        if u.supplied && u.still > 4.0 && d.pass != Pass::Air {
            u.hp = (u.hp + d.hp * 0.012).min(d.hp);
        } else if !u.supplied {
            u.hp -= d.hp * 0.004;
            if u.hp <= 0.0 {
                u.dead = true;
            }
        }
    }

    fn check_jam(&mut self, i: usize) {
        let (x, y, side, k) = {
            let u = &self.units[i];
            (u.x, u.y, u.side, u.kind)
        };
        let mut jammed = false;
        self.for_near(x, y, 12.0, |w, j| {
            let o = &w.units[j];
            let dj = def(o.kind).jam;
            if !jammed && o.side != side && dj > 0.0 && (o.x - x).powi(2) + (o.y - y).powi(2) < dj * dj {
                jammed = true;
            }
        });
        // 通信途絶は妨害と同じように効き、衛星通信があれば妨害に強い
        let outage = self.has_outage(side, x, y);
        if jammed || outage {
            let link = if self.has_uplink(side) { 0.5 } else { 1.0 };
            let p = if k == UK::Fpv { 0.28 } else if k == UK::LongDrone { 0.05 } else { 0.1 } * link;
            if self.rng.chance(p) {
                self.units[i].dead = true;
                self.fx.push(Fx { kind: FxKind::Jam, x, y, x2: x, y2: y, side, r: 0.4 });
            } else {
                let u = &mut self.units[i];
                u.jam_t = 0.6;
                u.x += self.rng.range(-0.5, 0.5);
                u.y += self.rng.range(-0.5, 0.5);
            }
        }
    }

    fn for_near(&mut self, x: f32, y: f32, r: f32, mut f: impl FnMut(&mut World, usize)) {
        let (x0, x1) = ((((x - r).max(0.0)) as usize / GC).min(GW - 1), (((x + r).max(0.0)) as usize / GC).min(GW - 1));
        let (y0, y1) = ((((y - r).max(0.0)) as usize / GC).min(GH - 1), (((y + r).max(0.0)) as usize / GC).min(GH - 1));
        for gy in y0..=y1 {
            for gx in x0..=x1 {
                let gi = gy * GW + gx;
                for idx in 0..self.grid[gi].len() {
                    let j = self.grid[gi][idx] as usize;
                    if j < self.units.len() && !self.units[j].dead {
                        f(self, j);
                    }
                }
            }
        }
    }

    /// 自爆機（FPV・長距離ドローン・巡航ミサイル・水上ドローン）
    fn update_suicide(&mut self, i: usize) {
        let k = self.units[i].kind;
        let d = def(k);
        // 目的地の更新
        let dest = match self.units[i].order {
            Order::Attack(t) => match self.target_pos(t) {
                Some(p) => Some(p),
                None => {
                    // 標的を失った FPV は近くの別の敵を探す
                    let (ux, uy, s) = (self.units[i].x, self.units[i].y, self.units[i].side);
                    let mut best: Option<(f32, u32)> = None;
                    self.for_near(ux, uy, 9.0, |w, j| {
                        let o = &w.units[j];
                        let dd = (o.x - ux).powi(2) + (o.y - uy).powi(2);
                        if o.side != s && def(o.kind).pass != Pass::Air && dd < 81.0 && w.can_see(s, o) && best.map_or(true, |b| dd < b.0) {
                            best = Some((dd, o.id));
                        }
                    });
                    if let Some((_, id)) = best {
                        self.units[i].order = Order::Attack(Target::Unit(id));
                        self.target_pos(Target::Unit(id))
                    } else {
                        self.units[i].dead = true;
                        return;
                    }
                }
            },
            Order::AttackGround(x, y) | Order::Move(x, y) | Order::AttackMove(x, y) => Some((x, y)),
            _ => None,
        };
        let Some((tx, ty)) = dest else {
            // 命令待ち（水上ドローン・長距離ドローン）
            return;
        };
        if d.pass == Pass::Sea {
            // 水上ドローン：経路をたどる
            if self.units[i].goal.is_none() {
                self.set_goal(i, tx, ty);
            }
            if self.ticks % 40 == 0 {
                self.set_goal(i, tx, ty);
            }
            self.move_logic(i);
            // 近くの敵艦や橋で爆発
            let (ux, uy, s) = (self.units[i].x, self.units[i].y, self.units[i].side);
            let mut hit = false;
            self.for_near(ux, uy, 2.0, |w, j| {
                let o = &w.units[j];
                if o.side != s && o.kind == UK::Frigate && (o.x - ux).powi(2) + (o.y - uy).powi(2) < 4.0 {
                    hit = true;
                }
            });
            let near_goal = (tx - ux).powi(2) + (ty - uy).powi(2) < 4.0;
            let bridge_near = (-1..=1).any(|dy| (-1..=1).any(|dx| in_map(ux as i32 + dx, uy as i32 + dy) && self.map.bridge_at[tidx(ux as i32 + dx, uy as i32 + dy)] != u16::MAX && self.map.terrain[tidx(ux as i32 + dx, uy as i32 + dy)] == Terrain::Bridge));
            if hit || near_goal && bridge_near {
                self.explode(ux, uy, d.weapon.splash, d.weapon.dmg, d.weapon.vs, s);
                if bridge_near {
                    self.damage_bridges(ux, uy, 2.0, d.weapon.dmg * 0.7);
                }
                self.units[i].dead = true;
            } else if near_goal {
                self.units[i].order = Order::Idle;
                self.units[i].goal = None;
            }
            return;
        }
        let u = &mut self.units[i];
        let (dx, dy) = (tx - u.x, ty - u.y);
        let dist = (dx * dx + dy * dy).sqrt();
        u.face = dy.atan2(dx);
        let step = d.speed * DT;
        if dist <= d.weapon.range.max(step) {
            u.dead = true;
            let (ux, uy, s, aimed) = (u.x, u.y, u.side, matches!(u.order, Order::AttackGround(..)));
            self.explode(ux, uy, d.weapon.splash, d.weapon.dmg, d.weapon.vs, s);
            if aimed {
                self.damage_bridges(ux, uy, d.weapon.splash.max(0.6), d.weapon.dmg * d.weapon.vs[Armor::Bld as usize] * 0.7);
            }
            return;
        }
        u.x += dx / dist * step;
        u.y += dy / dist * step;
    }

    /// 戦闘機：飛行場から出て爆弾を落として帰る
    fn update_jet(&mut self, i: usize) {
        let d = def(UK::Jet);
        // 帰る先は最寄りの飛行場
        let side = self.units[i].side;
        if self.ticks % 40 == (i as u64 % 40) {
            let (ux, uy) = (self.units[i].x, self.units[i].y);
            let home = self
                .blds
                .iter()
                .filter(|b| b.side == side && b.kind == BK::Airfield && b.building <= 0.0)
                .map(|b| (b.x, b.y))
                .min_by(|a, b| ((a.0 - ux).powi(2) + (a.1 - uy).powi(2)).partial_cmp(&((b.0 - ux).powi(2) + (b.1 - uy).powi(2))).unwrap_or(std::cmp::Ordering::Equal));
            if let Some(h) = home {
                self.units[i].home = h;
            }
        }
        let sortie = self.units[i].sortie;
        if sortie == 0 {
            // 待機中：整備
            let u = &mut self.units[i];
            u.hp = (u.hp + d.hp * 0.02 * DT).min(d.hp);
            u.moving = false;
            return;
        }
        let dest = match self.units[i].order {
            Order::Attack(t) => self.target_pos(t),
            Order::AttackGround(x, y) | Order::AttackMove(x, y) | Order::Move(x, y) => Some((x, y)),
            _ => None,
        };
        if sortie == 1 {
            let Some((tx, ty)) = dest else {
                self.units[i].sortie = 2;
                return;
            };
            let (ux, uy) = (self.units[i].x, self.units[i].y);
            let dist = ((tx - ux).powi(2) + (ty - uy).powi(2)).sqrt();
            if dist <= d.weapon.range {
                // 投下して帰る
                let (dmg, splash, vs) = (d.weapon.dmg, d.weapon.splash, d.weapon.vs);
                let target = if let Order::Attack(t) = self.units[i].order { Some(t) } else { None };
                for k in 0..2 {
                    let (ox, oy) = (self.rng.range(-1.2, 1.2) * k as f32, self.rng.range(-1.2, 1.2) * k as f32);
                    self.projs.push(Projectile { kind: ProjKind::Bomb, side, x: ux, y: uy, sx: ux, sy: uy, tx: tx + ox, ty: ty + oy, speed: 7.0, target: None, dmg, splash, vs, done: false, air: false, aimed: true });
                }
                let _ = target;
                self.units[i].sortie = 2;
                self.units[i].order = Order::Idle;
                self.units[i].flash = 3.0;
                return;
            }
            self.fly_to(i, tx, ty, d.speed);
        } else {
            let h = self.units[i].home;
            self.fly_to(i, h.0, h.1, d.speed);
            let u = &mut self.units[i];
            if (u.x - h.0).powi(2) + (u.y - h.1).powi(2) < 2.0 {
                u.sortie = 0;
                u.order = Order::Idle;
                u.cd = 14.0;
            }
        }
    }

    fn fly_to(&mut self, i: usize, tx: f32, ty: f32, speed: f32) {
        let u = &mut self.units[i];
        let (dx, dy) = (tx - u.x, ty - u.y);
        let dist = (dx * dx + dy * dy).sqrt().max(0.001);
        u.face = dy.atan2(dx);
        u.moving = true;
        let step = (speed * DT).min(dist);
        u.x += dx / dist * step;
        u.y += dy / dist * step;
    }

    fn pick_target(&mut self, i: usize) {
        let (x, y, side, order, cur) = {
            let u = &self.units[i];
            (u.x, u.y, u.side, u.order, u.target)
        };
        let d = def(self.units[i].kind);
        let w = d.weapon;
        // 明示の攻撃命令はそのまま
        if let Order::Attack(t) = order {
            if self.target_pos(t).is_some() {
                self.units[i].target = Some(t);
                return;
            }
            self.units[i].order = Order::Idle;
            self.units[i].target = None;
            self.units[i].goal = None;
        }
        if matches!(order, Order::AttackGround(..)) {
            return;
        }
        // 長射程の砲は、移動命令中は撃たない
        let indirect = matches!(w.proj, Proj::Shell(_) | Proj::Missile(_) | Proj::SpawnCruise) && w.range > 15.0;
        if indirect && matches!(order, Order::Move(..)) {
            self.units[i].target = None;
            return;
        }
        if let Some(Target::Unit(id)) = cur {
            if let Some(o) = self.unit(id) {
                let dd = (o.x - x).powi(2) + (o.y - y).powi(2);
                if dd <= w.range * w.range && self.can_see(side, o) {
                    return;
                }
            }
        }
        let r = w.range;
        let mut best: Option<(f32, Target)> = None;
        self.for_near(x, y, r, |wd, j| {
            let o = &wd.units[j];
            if o.side == side {
                return;
            }
            let od = def(o.kind);
            let m = w.vs[od.armor as usize];
            if m <= 0.0 {
                return;
            }
            let dd = (o.x - x).powi(2) + (o.y - y).powi(2);
            if dd > r * r || dd < w.min_range * w.min_range {
                return;
            }
            if matches!(o.kind, UK::Fpv) && w.proj == Proj::Missile(20.0) {
                return; // SAM を FPV に使わない
            }
            if !wd.can_see(side, o) {
                return;
            }
            let score = m * (od.cost + 50.0) / (dd.sqrt() + 3.0);
            if best.map_or(true, |b| score > b.0) {
                best = Some((score, Target::Unit(o.id)));
            }
        });
        // 部隊がいなければ建物
        if best.is_none() && w.vs[Armor::Bld as usize] > 0.0 && !matches!(order, Order::Move(..)) {
            for b in &self.blds {
                if b.side != side && b.seen[side as usize] {
                    let dd = (b.x - x).powi(2) + (b.y - y).powi(2);
                    if dd <= r * r && dd >= w.min_range * w.min_range {
                        best = Some((1.0, Target::Bld(b.id)));
                        break;
                    }
                }
            }
        }
        self.units[i].target = best.map(|b| b.1);
    }

    fn fire_logic(&mut self, i: usize) {
        let d = def(self.units[i].kind);
        let w = d.weapon;
        if w.dmg <= 0.0 || self.units[i].cd > 0.0 {
            return;
        }
        let (x, y, side, order, target) = {
            let u = &self.units[i];
            (u.x, u.y, u.side, u.order, u.target)
        };
        // 地点攻撃
        let (tx, ty, tgt) = if let Order::AttackGround(gx, gy) = order {
            let dd = (gx - x).powi(2) + (gy - y).powi(2);
            if dd > w.range * w.range || dd < w.min_range * w.min_range {
                return;
            }
            (gx, gy, None)
        } else if let Some(t) = target {
            let Some(p) = self.target_pos(t) else {
                self.units[i].target = None;
                return;
            };
            let dd = (p.0 - x).powi(2) + (p.1 - y).powi(2);
            if dd > w.range * w.range {
                return;
            }
            (p.0, p.1, Some(t))
        } else {
            return;
        };
        if w.shells > 0.0 {
            let s = &mut self.sides[side as usize];
            if s.shells < w.shells {
                self.units[i].cd = 1.0;
                return;
            }
            s.shells -= w.shells;
        }
        let mut reload_mul = if self.units[i].supplied { 1.0 } else { 2.0 };
        if matches!(self.units[i].kind, UK::FpvTeam | UK::BomberDrone) {
            if self.has_outage(side, x, y) {
                reload_mul *= 2.5;
            } else if self.has_uplink(side) {
                reload_mul *= 0.8;
            }
        }
        {
            let u = &mut self.units[i];
            u.cd = w.reload * reload_mul;
            u.face = (ty - y).atan2(tx - x);
            u.flash = if w.range > 15.0 { 5.0 } else { u.flash.max(1.5) };
        }
        match w.proj {
            Proj::None => {}
            Proj::Direct => {
                if let Some(t) = tgt {
                    self.hit_target(t, w.dmg, w.vs, side);
                }
                self.fx.push(Fx { kind: FxKind::Tracer, x, y, x2: tx, y2: ty, side, r: if w.dmg > 40.0 { 1.0 } else { 0.4 } });
            }
            Proj::Shell(sp) | Proj::Bomb(sp) => {
                let dist = ((tx - x).powi(2) + (ty - y).powi(2)).sqrt();
                let sc = dist * 0.03 + 0.25;
                let (ox, oy) = (self.rng.range(-sc, sc), self.rng.range(-sc, sc));
                let kind = if matches!(w.proj, Proj::Shell(_)) { ProjKind::Shell } else { ProjKind::Bomb };
                self.projs.push(Projectile { kind, side, x, y, sx: x, sy: y, tx: tx + ox, ty: ty + oy, speed: sp, target: None, dmg: w.dmg, splash: w.splash, vs: w.vs, done: false, air: false, aimed: tgt.is_none() });
                self.fx.push(Fx { kind: FxKind::Muzzle, x, y, x2: tx, y2: ty, side, r: 1.0 });
            }
            Proj::Missile(sp) => {
                let air = matches!(tgt, Some(Target::Unit(id)) if self.unit(id).is_some_and(|o| def(o.kind).pass == Pass::Air));
                self.projs.push(Projectile { kind: ProjKind::Missile, side, x, y, sx: x, sy: y, tx, ty, speed: sp, target: tgt, dmg: w.dmg, splash: w.splash, vs: w.vs, done: false, air, aimed: tgt.is_none() });
                self.fx.push(Fx { kind: FxKind::Launch, x, y, x2: tx, y2: ty, side, r: 0.6 });
            }
            Proj::SpawnFpv => {
                if let Some(t) = tgt {
                    let id = self.spawn(UK::Fpv, side, x, y);
                    if let Some(j) = self.unit_index(id) {
                        self.units[j].order = Order::Attack(t);
                    }
                }
            }
            Proj::SpawnCruise => {
                self.launch_cruise(side, x, y, tx, ty);
            }
        }
    }

    fn hit_target(&mut self, t: Target, dmg: f32, vs: [f32; 7], from: u8) {
        match t {
            Target::Unit(id) => {
                if let Some(j) = self.unit_index(id) {
                    let m = vs[def(self.units[j].kind).armor as usize];
                    self.damage_unit(j, dmg * m, from);
                }
            }
            Target::Bld(id) => self.damage_bld(id, dmg * vs[Armor::Bld as usize], from),
        }
    }

    pub fn damage_unit(&mut self, j: usize, raw: f32, from: u8) {
        let (prot, was_dead) = {
            let u = &self.units[j];
            (self.protection(u), u.dead)
        };
        if was_dead {
            return;
        }
        // ウクラヴィアは西側の精密兵器で一発の重みがある（ロスコヴィアは数で補う）
        let quality = if from == 0 { 1.18 } else { 1.0 };
        // 国土の大半を失いかけた側は本土防衛で粘る（一方的な雪崩を防ぐ）
        let share = self.share(self.units[j].side);
        let homeland = if share < 0.42 { (0.55 + (share - 0.15).max(0.0) * 1.67).min(1.0) } else { 1.0 };
        let u = &mut self.units[j];
        u.hp -= raw * prot * quality * homeland * LETHALITY;
        if u.hp <= 0.0 {
            u.dead = true;
            let (x, y, k) = (u.x, u.y, u.kind);
            let d = def(k);
            let big = matches!(d.armor, Armor::Heavy | Armor::Ship);
            let air = d.pass == Pass::Air;
            self.fx.push(Fx { kind: if air { FxKind::AirBurst } else if big { FxKind::BigBlast } else { FxKind::Blast }, x, y, x2: x, y2: y, side: from, r: if big { 1.6 } else { 0.7 } });
            if !matches!(k, UK::Fpv | UK::Cruise | UK::LongDrone | UK::SeaDrone) {
                self.sides[from as usize].kills += 1;
            }
        }
    }

    /// 地形と塹壕による被害の軽減
    fn protection(&self, u: &Unit) -> f32 {
        let d = def(u.kind);
        if d.pass == Pass::Air || d.pass == Pass::Sea {
            return 1.0;
        }
        let i = tidx(u.x as i32, u.y as i32);
        let trench = self.feat[i] & FEAT_TRENCH != 0;
        let t = self.map.terrain[i];
        let mut p = 1.0;
        if d.pass == Pass::Inf {
            if trench {
                p *= 0.4;
            } else if u.dig > 0.0 {
                p *= 1.0 - 0.4 * u.dig;
            }
            match t {
                Terrain::Urban => p *= 0.65,
                Terrain::Forest => p *= 0.8,
                _ => {}
            }
        } else if trench {
            p *= 0.75;
        }
        for k in self.mods_at(u.side, u.x, u.y) {
            if let ModKind::Protect(m) = k {
                p *= m;
            }
        }
        p
    }

    pub fn damage_bld(&mut self, id: u32, amount: f32, from: u8) {
        let Some(b) = self.blds.iter_mut().find(|b| b.id == id && !b.dead) else { return };
        b.hp -= amount;
        if b.hp <= 0.0 {
            b.dead = true;
            let (x, y, k, s) = (b.x, b.y, b.kind, b.side);
            self.fx.push(Fx { kind: FxKind::BigBlast, x, y, x2: x, y2: y, side: from, r: 2.2 });
            self.add_scar(x, y, 2.5);
            self.notices.push(Notice { text: format!("{}の{}が破壊された", SIDE_NAMES[s as usize], bdef(k).name), side: None, x, y, tone: if Some(s) == self.player { 2 } else { 1 } });
        }
    }

    fn add_scar(&mut self, x: f32, y: f32, r: f32) {
        self.scars.push((x, y, r));
        self.scar_log.push((x, y, r));
        // 記録が膨らまないように古いものから捨てる
        if self.scar_log.len() > 6000 {
            self.scar_log.drain(..1000);
        }
    }

    fn explode(&mut self, x: f32, y: f32, r: f32, dmg: f32, vs: [f32; 7], side: u8) {
        let big = r >= 1.5;
        self.fx.push(Fx { kind: if big { FxKind::BigBlast } else { FxKind::Blast }, x, y, x2: x, y2: y, side, r: r.max(0.5) });
        if r > 0.4 {
            self.add_scar(x, y, r.min(3.0));
        }
        let mut hits: Vec<(usize, f32)> = Vec::new();
        let reach = r.max(0.6);
        self.for_near(x, y, reach, |w, j| {
            let o = &w.units[j];
            if o.side == side || def(o.kind).pass == Pass::Air {
                return;
            }
            let m = vs[def(o.kind).armor as usize];
            let dd = ((o.x - x).powi(2) + (o.y - y).powi(2)).sqrt();
            if m > 0.0 && dd <= reach {
                hits.push((j, m * dmg * (1.0 - 0.5 * dd / reach)));
            }
        });
        for (j, a) in hits {
            self.damage_unit(j, a, side);
        }
        let bm = vs[Armor::Bld as usize];
        if bm > 0.0 {
            let ids: Vec<(u32, f32)> = self
                .blds
                .iter()
                .filter(|b| b.side != side && (b.x - x).abs() <= reach + 1.5 && (b.y - y).abs() <= reach + 1.5)
                .map(|b| (b.id, bm * dmg))
                .collect();
            for (id, a) in ids {
                self.damage_bld(id, a, side);
            }
            // 橋（巻き添えでは落ちにくい。狙って撃てば落とせる）
            self.damage_bridges(x, y, reach, bm * dmg * 0.3);
        }
    }

    pub(crate) fn damage_bridges(&mut self, x: f32, y: f32, r: f32, a: f32) {
        let mut hit = Vec::new();
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let (px, py) = (x as i32 + dx, y as i32 + dy);
                if in_map(px, py) {
                    let b = self.map.bridge_at[tidx(px, py)];
                    if b != u16::MAX && self.map.terrain[tidx(px, py)] == Terrain::Bridge && !hit.contains(&b) {
                        hit.push(b);
                    }
                }
            }
        }
        for b in hit {
            let br = &mut self.map.bridges[b as usize];
            if br.hp <= 0.0 {
                continue;
            }
            br.hp -= a;
            if br.hp <= 0.0 {
                let under = br.under;
                let tiles = br.tiles.clone();
                let (bx, by) = (br.x, br.y);
                let name = br.name.unwrap_or("橋");
                for t in tiles {
                    self.map.terrain[t as usize] = under;
                    self.map.road[t as usize] = false;
                    self.dirty.push(t);
                    if under == Terrain::Sea {
                        self.owner[t as usize] = -1;
                        self.total_tiles -= 1;
                    }
                }
                self.fx.push(Fx { kind: FxKind::BigBlast, x: bx, y: by, x2: bx, y2: by, side: 0, r: 2.0 });
                self.notices.push(Notice { text: format!("{name}が破壊された"), side: None, x: bx, y: by, tone: 0 });
                self.recent.clear();
            }
        }
    }

    fn move_logic(&mut self, i: usize) -> bool {
        let d = def(self.units[i].kind);
        let (order, target, x, y) = {
            let u = &self.units[i];
            (u.order, u.target, u.x, u.y)
        };
        let w = d.weapon;
        // 戦闘中の停止判定
        match order {
            Order::Hold => return false,
            Order::AttackMove(..) | Order::Idle => {
                if target.is_some() && (w.proj != Proj::Direct || d.pass != Pass::Air) {
                    if let Some(p) = target.and_then(|t| self.target_pos(t)) {
                        let dd = (p.0 - x).powi(2) + (p.1 - y).powi(2);
                        if dd <= w.range * w.range {
                            return false;
                        }
                    }
                }
            }
            Order::Attack(t) => {
                if let Some(p) = self.target_pos(t) {
                    let dd = (p.0 - x).powi(2) + (p.1 - y).powi(2);
                    if dd <= (w.range * 0.92).powi(2) {
                        return false;
                    }
                    // 動く標的を追う
                    if self.ticks % 40 == (i as u64 % 40) {
                        self.set_goal(i, p.0, p.1);
                    }
                }
            }
            Order::AttackGround(gx, gy) => {
                let dd = (gx - x).powi(2) + (gy - y).powi(2);
                if dd <= (w.range * 0.92).powi(2) {
                    return false;
                }
                if self.units[i].goal.is_none() {
                    self.set_goal(i, gx, gy);
                }
            }
            _ => {}
        }
        // 工兵の仕事
        if !self.units[i].jobs.is_empty() && matches!(self.units[i].order, Order::Idle | Order::AttackMove(..) | Order::Hold) {
            if self.do_job(i) {
                return true;
            }
            return false;
        }
        let Some(goal) = self.units[i].goal else { return false };
        let speed = self.speed_of(&self.units[i]) * DT;
        if d.pass == Pass::Air {
            let u = &mut self.units[i];
            let (dx, dy) = (goal.0 - u.x, goal.1 - u.y);
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 0.5 {
                u.goal = None;
                if matches!(u.order, Order::Move(..) | Order::AttackMove(..)) {
                    u.order = Order::Idle;
                }
                return false;
            }
            u.face = dy.atan2(dx);
            u.x += dx / dist * speed;
            u.y += dy / dist * speed;
            return true;
        }
        // 経路待ち
        if self.units[i].path_pending {
            return false;
        }
        let (wx, wy) = {
            let u = &mut self.units[i];
            if u.pi >= u.path.len() {
                // 経路の終わり：目的地へ
                let (dx, dy) = (goal.0 - u.x, goal.1 - u.y);
                if dx * dx + dy * dy < 0.5 || u.path.is_empty() && u.stuck > 30 {
                    u.goal = None;
                    u.path.clear();
                    if matches!(u.order, Order::Move(..) | Order::AttackMove(..)) {
                        u.order = Order::Idle;
                    }
                    return false;
                }
                goal
            } else {
                let p = u.path[u.pi];
                (p.0 as f32 + 0.5, p.1 as f32 + 0.5)
            }
        };
        let (dx, dy, dist) = {
            let u = &self.units[i];
            let (dx, dy) = (wx - u.x, wy - u.y);
            (dx, dy, (dx * dx + dy * dy).sqrt().max(0.0001))
        };
        if dist < 0.7 && self.units[i].pi < self.units[i].path.len() {
            self.units[i].pi += 1;
            return true;
        }
        let (nx, ny) = (self.units[i].x + dx / dist * speed, self.units[i].y + dy / dist * speed);
        let pass = d.pass;
        let ok = |w: &World, x: f32, y: f32| in_map(x as i32, y as i32) && tile_cost(&w.map, &w.feat, tidx(x as i32, y as i32), pass).is_some();
        let (cx, cy) = (self.units[i].x, self.units[i].y);
        let was_blocked = !ok(self, cx, cy);
        if ok(self, nx, ny) || was_blocked {
            let u = &mut self.units[i];
            u.x = nx;
            u.y = ny;
            u.face = dy.atan2(dx);
            u.stuck = 0;
            self.check_mines(i);
            true
        } else {
            let u = &mut self.units[i];
            u.stuck += 1;
            if u.stuck % 25 == 24 && !u.path_pending {
                // 道が塞がった：引き直す（使い回さない）
                u.path_pending = true;
                let id = u.id;
                self.pq.push_back(id);
            }
            if u.stuck > 200 {
                u.goal = None;
                u.path.clear();
                if matches!(u.order, Order::Move(..) | Order::AttackMove(..)) {
                    u.order = Order::Idle;
                }
            }
            false
        }
    }

    fn check_mines(&mut self, i: usize) {
        let (x, y, side, pass) = {
            let u = &self.units[i];
            (u.x as i32, u.y as i32, u.side, def(u.kind).pass)
        };
        let ti = tidx(x, y);
        let m = self.mines[ti];
        if m != 0 && m - 1 != side {
            self.mines[ti] = 0;
            self.dirty.push(ti as u32);
            let a = if pass == Pass::Veh { 130.0 } else { 40.0 };
            self.fx.push(Fx { kind: FxKind::MineBlast, x: x as f32 + 0.5, y: y as f32 + 0.5, x2: 0.0, y2: 0.0, side, r: 0.8 });
            self.damage_unit(i, a, 1 - side);
        }
    }

    /// 工兵の作業。歩いていれば true
    fn do_job(&mut self, i: usize) -> bool {
        let job = *self.units[i].jobs.front().expect("job");
        let (x, y) = (self.units[i].x, self.units[i].y);
        let (tx, ty) = (job.x as f32 + 0.5, job.y as f32 + 0.5);
        let dist = ((tx - x).powi(2) + (ty - y).powi(2)).sqrt();
        if dist > 1.6 {
            if self.units[i].goal.is_none() && !self.units[i].path_pending {
                self.set_goal(i, tx, ty);
                self.units[i].stuck = 1; // 道順を使い回さない
            }
            if self.units[i].path_pending {
                return false;
            }
            let goal = self.units[i].goal;
            if goal.is_none() {
                self.units[i].jobs.pop_front();
                return false;
            }
            // 通常の移動の流れで動かす
            let order = std::mem::replace(&mut self.units[i].order, Order::Move(tx, ty));
            let jobs = std::mem::take(&mut self.units[i].jobs);
            let moved = self.move_logic(i);
            self.units[i].jobs = jobs;
            self.units[i].order = order;
            if !moved && self.units[i].goal.is_none() {
                // 着かない：諦める
                self.units[i].jobs.pop_front();
            }
            return moved;
        }
        self.units[i].goal = None;
        self.units[i].path.clear();
        let time = match job.kind {
            JobKind::Trench => 1.2,
            JobKind::Mine => 1.0,
            JobKind::Teeth => 1.8,
            JobKind::Pontoon => 3.0,
        };
        self.units[i].job_t += DT;
        if self.units[i].job_t < time {
            return false;
        }
        self.units[i].job_t = 0.0;
        self.units[i].jobs.pop_front();
        let ti = tidx(job.x as i32, job.y as i32);
        let side = self.units[i].side;
        let t = self.map.terrain[ti];
        match job.kind {
            JobKind::Trench => {
                if matches!(t, Terrain::Plain | Terrain::Forest | Terrain::Urban) {
                    self.feat[ti] |= FEAT_TRENCH;
                    self.dirty.push(ti as u32);
                }
            }
            JobKind::Teeth => {
                if matches!(t, Terrain::Plain | Terrain::Forest) {
                    self.feat[ti] |= FEAT_TEETH;
                    self.dirty.push(ti as u32);
                    self.recent.clear();
                }
            }
            JobKind::Mine => {
                if matches!(t, Terrain::Plain | Terrain::Forest | Terrain::Urban) && self.sides[side as usize].funds >= 8.0 {
                    self.sides[side as usize].funds -= 8.0;
                    self.mines[ti] = side + 1;
                    self.dirty.push(ti as u32);
                }
            }
            JobKind::Pontoon => {
                if t == Terrain::River {
                    let id = self.map.bridges.len() as u16;
                    self.map.bridges.push(Bridge { tiles: vec![ti as u32], hp: 200.0, max_hp: 200.0, under: Terrain::River, x: job.x as f32 + 0.5, y: job.y as f32 + 0.5, name: Some("浮橋") });
                    self.map.bridge_at[ti] = id;
                    self.map.terrain[ti] = Terrain::Bridge;
                    self.map.road[ti] = true;
                    self.dirty.push(ti as u32);
                    self.recent.clear();
                }
            }
        }
        false
    }

    /// 地上部隊の雑務：塹壕掘り、地雷除去
    fn ground_chores(&mut self, i: usize) {
        let (x, y, kind, side, still) = {
            let u = &self.units[i];
            (u.x as i32, u.y as i32, u.kind, u.side, u.still)
        };
        let ti = tidx(x, y);
        if matches!(kind, UK::Infantry | UK::Atgm | UK::FpvTeam | UK::Engineer) && still > 2.0 && self.units[i].supplied {
            let rate = if kind == UK::Engineer { 0.2 } else { 0.05 };
            let u = &mut self.units[i];
            u.dig = (u.dig + rate).min(1.0);
            if u.dig >= 1.0 && self.feat[ti] & FEAT_TRENCH == 0 && matches!(self.map.terrain[ti], Terrain::Plain | Terrain::Forest | Terrain::Urban) {
                self.feat[ti] |= FEAT_TRENCH;
                self.dirty.push(ti as u32);
            }
        }
        if kind == UK::Engineer {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if in_map(x + dx, y + dy) {
                        let t2 = tidx(x + dx, y + dy);
                        let m = self.mines[t2];
                        if m != 0 && m - 1 != side {
                            self.mines[t2] = 0;
                            self.dirty.push(t2 as u32);
                            return;
                        }
                    }
                }
            }
        }
    }

    fn update_projectiles(&mut self) {
        let mut hits: Vec<usize> = Vec::new();
        for (pi, p) in self.projs.iter_mut().enumerate() {
            if let Some(t) = p.target {
                // 誘導：標的を追いかける
                let pos = match t {
                    Target::Unit(id) => id_lookup(&self.id_map, &self.units, id),
                    Target::Bld(id) => self.blds.iter().find(|b| b.id == id && !b.dead).map(|b| (b.x, b.y)),
                };
                if let Some(q) = pos {
                    p.tx = q.0;
                    p.ty = q.1;
                } else if p.air {
                    p.done = true;
                    continue;
                }
            }
            let (dx, dy) = (p.tx - p.x, p.ty - p.y);
            let d = (dx * dx + dy * dy).sqrt();
            let step = p.speed * DT;
            if d <= step {
                p.x = p.tx;
                p.y = p.ty;
                p.done = true;
                hits.push(pi);
            } else {
                p.x += dx / d * step;
                p.y += dy / d * step;
            }
        }
        for pi in hits {
            let (x, y, dmg, splash, vs, side, target, air, aimed) = {
                let p = &self.projs[pi];
                (p.x, p.y, p.dmg, p.splash, p.vs, p.side, p.target, p.air, p.aimed)
            };
            if splash > 0.0 {
                self.explode(x, y, splash, dmg, vs, side);
                if aimed {
                    self.damage_bridges(x, y, splash.max(0.6), dmg * vs[Armor::Bld as usize] * 0.7);
                }
            } else if let Some(t) = target {
                self.hit_target(t, dmg, vs, side);
                self.fx.push(Fx { kind: if air { FxKind::AirBurst } else { FxKind::Blast }, x, y, x2: x, y2: y, side, r: 0.45 });
            } else {
                self.fx.push(Fx { kind: FxKind::Blast, x, y, x2: x, y2: y, side, r: 0.4 });
            }
        }
    }

    /// 地上部隊が重ならないように押し合う
    fn separate(&mut self) {
        let mut moves: Vec<(usize, f32, f32)> = Vec::new();
        for gy in 0..GH {
            for gx in 0..GW {
                let cell = &self.grid[gy * GW + gx];
                if cell.len() < 2 {
                    continue;
                }
                for a in 0..cell.len() {
                    let ia = cell[a] as usize;
                    if ia >= self.units.len() {
                        continue;
                    }
                    let ua = &self.units[ia];
                    if !matches!(def(ua.kind).pass, Pass::Inf | Pass::Veh) {
                        continue;
                    }
                    for b in (a + 1)..cell.len() {
                        let ib = cell[b] as usize;
                        if ib >= self.units.len() {
                            continue;
                        }
                        let ub = &self.units[ib];
                        if !matches!(def(ub.kind).pass, Pass::Inf | Pass::Veh) {
                            continue;
                        }
                        let (dx, dy) = (ub.x - ua.x, ub.y - ua.y);
                        let dd = dx * dx + dy * dy;
                        if dd < 1.0 && dd > 1e-6 {
                            let f = (1.0 - dd.sqrt()) * 0.06;
                            let l = dd.sqrt();
                            moves.push((ia, -dx / l * f, -dy / l * f));
                            moves.push((ib, dx / l * f, dy / l * f));
                        } else if dd <= 1e-6 {
                            moves.push((ib, 0.05, 0.03));
                        }
                    }
                }
            }
        }
        for (i, dx, dy) in moves {
            let u = &self.units[i];
            let (nx, ny) = (u.x + dx, u.y + dy);
            let pass = def(u.kind).pass;
            if in_map(nx as i32, ny as i32) && tile_cost(&self.map, &self.feat, tidx(nx as i32, ny as i32), pass).is_some() {
                let u = &mut self.units[i];
                u.x = nx;
                u.y = ny;
            }
        }
    }

    // ------------------------------------------------------------ 視界

    pub fn update_vision(&mut self) {
        let n = W * H;
        for s in 0..2 {
            let v = &mut self.vis[s];
            let own = s as i8;
            for i in 0..n {
                v[i] = if self.owner[i] == own { VIS_GROUND } else { 0 };
            }
        }
        for ui in 0..self.units.len() {
            let u = &self.units[ui];
            if u.dead {
                continue;
            }
            let d = def(u.kind);
            let bit = if d.pass == Pass::Air { VIS_AIR | VIS_GROUND } else { VIS_GROUND };
            let (x, y, s) = (u.x, u.y, u.side as usize);
            stamp_vis(&mut self.vis[s], x, y, d.vision, bit);
            if d.pass != Pass::Air {
                stamp_vis(&mut self.vis[s], x, y, 6.0, VIS_CLOSE);
            }
        }
        for b in &self.blds {
            let s = b.side as usize;
            stamp_vis(&mut self.vis[s], b.x, b.y, bdef(b.kind).vision, VIS_GROUND);
        }
        for c in &self.map.cities {
            stamp_vis(&mut self.vis[c.owner as usize], c.x, c.y, 10.0, VIS_GROUND);
        }
        for s in &self.sats {
            stamp_vis(&mut self.vis[s.0 as usize], s.1, s.2, 24.0, VIS_AIR | VIS_GROUND | VIS_CLOSE);
        }
        for b in &mut self.blds {
            let i = tidx(b.x as i32, b.y as i32);
            for s in 0..2 {
                if self.vis[s][i] != 0 && self.owner[i] != s as i8 {
                    b.seen[s] = true;
                }
            }
        }
    }

    // ------------------------------------------------------------ 領土

    pub fn count_tiles(&mut self) {
        let (mut a, mut b) = (0, 0);
        for o in &self.owner {
            match *o {
                0 => a += 1,
                1 => b += 1,
                _ => {}
            }
        }
        self.sides[0].tiles = a;
        self.sides[1].tiles = b;
    }

    fn update_control(&mut self) {
        for s in 0..2 {
            self.infl[s].fill(0);
        }
        for ui in 0..self.units.len() {
            let u = &self.units[ui];
            if u.dead {
                continue;
            }
            let d = def(u.kind);
            if d.cap_r <= 0.0 {
                continue;
            }
            let (x, y, s) = (u.x, u.y, u.side as usize);
            let mut st = (d.cap_s * 4.0) as u16;
            if !u.supplied {
                st = st / 2 + 1;
            }
            stamp_infl(&mut self.infl[s], x, y, d.cap_r, st);
        }
        let mut changed = false;
        // 領土を広げた側ほど占領が遅くなる（戦線が伸びて占領地の維持に手を取られる）
        let reach = [overreach(self.share(0)), overreach(self.share(1))];
        for i in 0..W * H {
            let o = self.owner[i];
            if o < 0 {
                continue;
            }
            let (own, foe) = (self.infl[o as usize][i] as i32, self.infl[1 - o as usize][i] as i32);
            let c = &mut self.cap[i];
            if foe > own {
                // 占領が進む（市街地は時間がかかる）
                let urban = self.map.terrain[i] == Terrain::Urban;
                let base = ((foe - own) * if urban { 1 } else { 3 }).min(if urban { 4 } else { 25 }) as f32;
                let rate = (base * reach[1 - o as usize]).max(1.0) as i16;
                *c = c.saturating_add(rate);
                if *c >= 100 {
                    *c = 0;
                    self.owner[i] = 1 - o;
                    changed = true;
                }
            } else if *c != 0 {
                *c = (*c - 10).max(0);
            }
        }
        // 突き出した前線をならす：周りをほぼ囲まれた無人のタイルは降伏する
        if self.ticks % 40 == 0 {
            let snap: Vec<(usize, i8)> = (0..W * H)
                .filter_map(|i| {
                    let o = self.owner[i];
                    if o < 0 || self.infl[o as usize][i] > 0 {
                        return None;
                    }
                    let (x, y) = ((i % W) as i32, (i / W) as i32);
                    let mut foe = 0;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if (dx != 0 || dy != 0) && in_map(x + dx, y + dy) {
                                let o2 = self.owner[tidx(x + dx, y + dy)];
                                if o2 >= 0 && o2 != o {
                                    foe += 1;
                                }
                            }
                        }
                    }
                    if foe >= 6 {
                        Some((i, 1 - o))
                    } else {
                        None
                    }
                })
                .collect();
            for (i, o) in snap {
                self.owner[i] = o;
                changed = true;
            }
        }
        if changed {
            self.owner_rev = self.owner_rev.wrapping_add(1);
            self.count_tiles();
            self.check_cities();
            self.check_buildings_captured();
        }
    }

    fn check_cities(&mut self) {
        for ci in 0..self.map.cities.len() {
            let c = &self.map.cities[ci];
            let o = self.owner[tidx(c.x as i32, c.y as i32)];
            if o >= 0 && o as u8 != c.owner {
                let (name, cx, cy, ov) = (c.name, c.x, c.y, c.owner);
                self.map.cities[ci].owner = o as u8;
                let tone = if Some(o as u8) == self.player { 1 } else if Some(ov) == self.player { 2 } else { 0 };
                self.notices.push(Notice { text: format!("{}が{}を制圧した", SIDE_NAMES[o as usize], name), side: None, x: cx, y: cy, tone });
            }
        }
    }

    fn check_buildings_captured(&mut self) {
        let mut lost = Vec::new();
        for b in &self.blds {
            let o = self.owner[tidx(b.x as i32, b.y as i32)];
            if o >= 0 && o as u8 != b.side {
                lost.push((b.id, o as u8));
            }
        }
        for (id, by) in lost {
            self.damage_bld(id, 1e6, by);
        }
    }

    /// 無人で孤立した小さな飛び地は敵に降る（包囲の効果）
    fn pockets(&mut self) {
        let n = W * H;
        let mut comp = vec![0u32; n];
        let mut stack = Vec::new();
        let mut unit_tile = vec![false; n];
        for u in &self.units {
            let d = def(u.kind);
            if !u.dead && d.cap_r > 0.0 {
                unit_tile[tidx(u.x as i32, u.y as i32)] = true;
            }
        }
        let mut comps: Vec<(i8, u32, bool, u32)> = Vec::new();
        for i in 0..n {
            let o = self.owner[i];
            if o < 0 || comp[i] != 0 {
                continue;
            }
            let id = comps.len() as u32 + 1;
            let mut size = 0;
            let mut has_units = false;
            comp[i] = id;
            stack.push(i);
            while let Some(c) = stack.pop() {
                size += 1;
                has_units |= unit_tile[c];
                let (x, y) = ((c % W) as i32, (c / W) as i32);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if in_map(x + dx, y + dy) {
                        let t = tidx(x + dx, y + dy);
                        if comp[t] == 0 && self.owner[t] == o {
                            comp[t] = id;
                            stack.push(t);
                        }
                    }
                }
            }
            comps.push((o, size, has_units, id));
        }
        let largest = |s: i8| comps.iter().filter(|c| c.0 == s).map(|c| c.1).max().unwrap_or(0);
        let (l0, l1) = (largest(0), largest(1));
        let mut flip = vec![false; comps.len() + 1];
        let mut any = false;
        for c in &comps {
            let l = if c.0 == 0 { l0 } else { l1 };
            // 海で隔てられた土地（クリメアなど）は本土と同じ程度の大きさなら残る
            if c.1 < l && c.1 < l / 4 && !c.2 {
                flip[c.3 as usize] = true;
                any = true;
            }
        }
        if !any {
            return;
        }
        for i in 0..n {
            if comp[i] != 0 && flip[comp[i] as usize] {
                let o = self.owner[i];
                self.owner[i] = 1 - o;
            }
        }
        self.owner_rev = self.owner_rev.wrapping_add(1);
        self.count_tiles();
        self.check_cities();
        self.check_buildings_captured();
    }

    // ------------------------------------------------------------ 補給

    pub fn update_supply(&mut self) {
        for side in 0..2u8 {
            let mut near: Vec<(i32, i32, u8)> = Vec::new();
            for c in &self.map.cities {
                if c.owner == side {
                    near.push((c.x as i32, c.y as i32, 26));
                }
            }
            for b in &self.blds {
                if b.side == side && b.building <= 0.0 {
                    let r = match b.kind {
                        BK::Depot => 62,
                        BK::Base => 34,
                        _ => 14,
                    };
                    near.push((b.x as i32, b.y as i32, r));
                }
            }
            near.sort_by_key(|n| std::cmp::Reverse(n.2));
            let sup = &mut self.supply[side as usize];
            sup.fill(0);
            // 値の大きい順に広げていく（残り距離つきの幅優先探索）
            let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); 64];
            for (x, y, r) in near {
                if in_map(x, y) {
                    let i = tidx(x, y);
                    if sup[i] < r {
                        sup[i] = r;
                        buckets[r as usize].push(i);
                    }
                }
            }
            for r in (2..64usize).rev() {
                while let Some(c) = buckets[r].pop() {
                    if sup[c] as usize != r {
                        continue;
                    }
                    let (x, y) = ((c % W) as i32, (c / W) as i32);
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        if in_map(x + dx, y + dy) {
                            let t = tidx(x + dx, y + dy);
                            if self.owner[t] == side as i8 && (sup[t] as usize) < r - 1 {
                                sup[t] = (r - 1) as u8;
                                buckets[r - 1].push(t);
                            }
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ 建物と生産

    fn update_buildings(&mut self, dt: f32) {
        let mut spawns: Vec<(UK, u8, f32, f32, Option<(f32, f32)>)> = Vec::new();
        for b in &mut self.blds {
            if b.dead {
                continue;
            }
            if b.building > 0.0 {
                b.building -= dt;
                continue;
            }
            // 建物は少しずつ修理される
            b.hp = (b.hp + b.max_hp * 0.004 * dt).min(b.max_hp);
            if let Some(&k) = b.queue.front() {
                b.prog += dt;
                if b.prog >= def(k).time {
                    b.prog = 0.0;
                    b.queue.pop_front();
                    spawns.push((k, b.side, b.x, b.y, b.rally));
                }
            }
        }
        for (k, side, x, y, rally) in spawns {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let id = self.spawn(k, side, x + a.cos() * 2.5, y + a.sin() * 2.5);
            if let (Some(r), Some(i)) = (rally, self.unit_index(id)) {
                // 集結地点を決めた建物の部隊はプレイヤーが直接指揮する
                self.units[i].auto = self.sides[side as usize].ai;
                if k == UK::Jet {
                    self.units[i].home = (x, y);
                    continue;
                }
                self.units[i].order = Order::AttackMove(r.0, r.1);
                self.set_goal(i, r.0, r.1);
            }
        }
    }

    /// 追い詰められた側は総動員で本土を守る：残った都市に増援が出て、基地を失っていれば再建される
    fn last_stand(&mut self, side: u8) {
        let cities: Vec<(f32, f32)> = self.map.cities.iter().filter(|c| c.owner == side).map(|c| (c.x, c.y)).collect();
        if cities.is_empty() {
            return;
        }
        let (x, y) = cities[self.rng.below(cities.len() as u32) as usize];
        for (k, kind) in [UK::Infantry, UK::Infantry, UK::Ifv, UK::Tank].into_iter().enumerate() {
            let a = k as f32 * 1.6;
            self.spawn(kind, side, x + a.cos() * 3.0, y + a.sin() * 3.0);
        }
        for kind in [BK::Base, BK::Depot, BK::Factory] {
            if self.blds.iter().any(|b| b.side == side && b.kind == kind) {
                continue;
            }
            for _ in 0..12 {
                let (dx, dy) = (self.rng.range(-6.0, 6.0), self.rng.range(-6.0, 6.0));
                if self.can_place(side, kind, x + dx, y + dy).is_ok() {
                    let id = self.add_building(kind, side, x + dx, y + dy);
                    if let Some(b) = self.blds.iter_mut().find(|b| b.id == id) {
                        b.building = 5.0;
                    }
                    break;
                }
            }
        }
    }

    /// 国土の大半を失った側では、占領された都市でパルチザンが蜂起する
    fn partisans(&mut self) {
        for side in 0..2u8 {
            if self.share(side) >= 0.3 {
                continue;
            }
            self.last_stand(side);
            let list: Vec<(f32, f32, &'static str)> = self.map.cities.iter().enumerate().filter(|(i, c)| c.owner != side && self.city_home[*i] == side).map(|(_, c)| (c.x, c.y, c.name)).collect();
            if list.is_empty() {
                continue;
            }
            let (x, y, name) = list[self.rng.below(list.len() as u32) as usize];
            for k in 0..5 {
                let a = k as f32 * 1.3;
                let kind = if k == 0 { UK::Atgm } else { UK::Infantry };
                self.spawn(kind, side, x + a.cos() * 3.0, y + a.sin() * 3.0);
            }
            let tone = if self.player == Some(side) { 1 } else { 2 };
            self.notices.push(Notice { text: format!("{name}でパルチザンが蜂起した"), side: None, x, y, tone });
        }
    }

    fn economy(&mut self, dt: f32) {
        for side in 0..2usize {
            let mut city = 0.0;
            for c in &self.map.cities {
                if c.owner as usize == side {
                    city += c.value as f32;
                }
            }
            let share = self.sides[side].tiles as f32 / self.total_tiles.max(1) as f32;
            let factories = self.blds.iter().filter(|b| b.side as usize == side && b.kind == BK::Factory && b.building <= 0.0).count() as f32;
            // 押されている側には国際支援と総動員で追い風が吹く（押し切りすぎを防ぐ）
            let catch_up = 1.0 + ((0.5 - share) * 2.4).clamp(-0.45, 0.9);
            let (mut im, mut sm) = (1.0, 1.0);
            for m in self.mods.iter().filter(|m| m.side as usize == side) {
                match m.kind {
                    ModKind::IncomeMul(v) => im *= v,
                    ModKind::ShellMul(v) => sm *= v,
                    _ => {}
                }
            }
            // 大軍ほど維持費がかかる
            let army = self.units.iter().filter(|u| u.side as usize == side && !u.dead && !def(u.kind).suicide).count() as f32;
            let gross = (17.0 + city * 0.65 + factories * 1.5) * catch_up * self.sides[side].ai_mult * im;
            let inc = (gross - army * UPKEEP).max(gross * 0.2);
            let sh_base = if side == 0 { 0.7 } else { 1.3 };
            let shi = (sh_base + factories * 1.3) * self.sides[side].ai_mult * sm;
            let s = &mut self.sides[side];
            s.income = inc;
            s.shell_income = shi;
            s.funds += inc * dt;
            s.shells = (s.shells + shi * dt).min(1500.0);
        }
    }

    /// プレイヤーが読む通知を取り出す
    pub fn drain_notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut self.notices)
    }
}

/// 領土の割合に応じた占領の速さ（55% までは等倍、85% で最小）
fn overreach(share: f32) -> f32 {
    ((0.85 - share) / 0.30).clamp(0.08, 1.0)
}

fn id_lookup(id_map: &[u32], units: &[Unit], id: u32) -> Option<(f32, f32)> {
    let i = *id_map.get(id as usize)? as usize;
    let u = units.get(i)?;
    if u.id == id && !u.dead {
        Some((u.x, u.y))
    } else {
        None
    }
}

fn stamp_vis(v: &mut [u8], x: f32, y: f32, r: f32, bit: u8) {
    let ri = r as i32 + 1;
    let (cx, cy) = (x as i32, y as i32);
    let r2 = r * r;
    for dy in -ri..=ri {
        let py = cy + dy;
        if py < 0 || py >= H as i32 {
            continue;
        }
        for dx in -ri..=ri {
            let px = cx + dx;
            if px >= 0 && px < W as i32 && ((dx * dx + dy * dy) as f32) <= r2 {
                v[py as usize * W + px as usize] |= bit;
            }
        }
    }
}

fn stamp_infl(v: &mut [u16], x: f32, y: f32, r: f32, s: u16) {
    let ri = r as i32 + 1;
    let (cx, cy) = (x as i32, y as i32);
    let r2 = r * r;
    for dy in -ri..=ri {
        let py = cy + dy;
        if py < 0 || py >= H as i32 {
            continue;
        }
        for dx in -ri..=ri {
            let px = cx + dx;
            if px >= 0 && px < W as i32 && ((dx * dx + dy * dy) as f32) <= r2 {
                let c = &mut v[py as usize * W + px as usize];
                *c = c.saturating_add(s);
            }
        }
    }
}
