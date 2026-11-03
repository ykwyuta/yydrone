//! 地図データ：海岸線・河川・国境・都市・道路（実在の地形を下敷きにした架空の戦域）

use crate::rng::{fbm, hash2, value_noise};

pub const W: usize = 320;
pub const H: usize = 200;
const LON0: f32 = 22.0;
const LON1: f32 = 44.0;
const LAT0: f32 = 53.0;
const LAT1: f32 = 44.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Terrain {
    Sea,
    River,
    Plain,
    Forest,
    Urban,
    Bridge,
    /// 中立国（通行できず、支配領域にも数えない）
    Neutral,
    /// 湿地（ポレシア）。凍れば通れるが、ふだんは車両が道路から離れられない
    Marsh,
    /// 畑を仕切る樹林帯。葉のある間は歩兵を隠す
    Treeline,
    /// 高地・ボタ山。見晴らしがよく守りやすい
    Hill,
}

impl Terrain {
    /// 建物や塹壕を作れる地面か
    #[inline]
    pub fn land(self) -> bool {
        matches!(self, Terrain::Plain | Terrain::Forest | Terrain::Urban | Terrain::Treeline | Terrain::Hill)
    }
    /// 市街以外の開けた地面か（竜の歯や水没の対象）
    #[inline]
    pub fn open(self) -> bool {
        matches!(self, Terrain::Plain | Terrain::Forest | Terrain::Treeline | Terrain::Hill)
    }
}

/// 地面の状態
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ground {
    Normal,
    /// 泥濘期：道路の外はほとんど動けない
    Mud,
    /// 凍結：どこでも走れるが塹壕は掘りにくい
    Frozen,
}

/// 通行の種類
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pass {
    Inf,
    Veh,
    Sea,
    Air,
}

pub struct City {
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
    /// 収入の重み（1〜5）
    pub value: u8,
    pub port: bool,
    pub owner: u8,
}

pub struct Bridge {
    pub tiles: Vec<u32>,
    pub hp: f32,
    pub max_hp: f32,
    /// 落ちたら戻す地形（川または海）
    pub under: Terrain,
    pub x: f32,
    pub y: f32,
    pub name: Option<&'static str>,
}

pub struct Label {
    pub text: &'static str,
    pub x: f32,
    pub y: f32,
    pub kind: u8,
}

pub struct Map {
    pub terrain: Vec<Terrain>,
    pub road: Vec<bool>,
    /// 線路（切られている間は false）
    pub rail: Vec<bool>,
    /// 描画用の線路の中心線
    pub rail_lines: Vec<Vec<(f32, f32)>>,
    /// 描画用の道路の中心線
    pub road_lines: Vec<Vec<(f32, f32)>>,
    pub bridge_at: Vec<u16>,
    pub bridges: Vec<Bridge>,
    pub cities: Vec<City>,
    pub start_owner: Vec<i8>,
    pub labels: Vec<Label>,
    /// いまの地面の状態（通行コストに効く）
    pub ground: Ground,
    /// 地図を作ったときの橋の数（これより後ろは浮橋）
    base_bridges: usize,
}

#[inline]
pub fn tidx(x: i32, y: i32) -> usize {
    y as usize * W + x as usize
}
#[inline]
pub fn in_map(x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H
}

pub fn ll(lon: f32, lat: f32) -> (f32, f32) {
    ((lon - LON0) / (LON1 - LON0) * W as f32, (LAT0 - lat) / (LAT0 - LAT1) * H as f32)
}

fn in_poly(p: (f32, f32), poly: &[(f32, f32)]) -> bool {
    let mut c = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > p.1) != (yj > p.1) && p.0 < (xj - xi) * (p.1 - yi) / (yj - yi) + xi {
            c = !c;
        }
        j = i;
    }
    c
}

/// ポント海（黒海に当たる）
const SEA_MAIN: &[(f32, f32)] = &[
    (28.6, 43.0), (28.7, 44.2), (29.1, 44.9), (29.75, 45.2), (30.2, 45.9), (30.8, 46.5), (31.5, 46.6), (31.8, 46.35), (32.5, 46.12), (33.55, 46.08), (33.2, 45.8), (32.5, 45.35), (33.4, 45.15),
    (33.5, 44.6), (33.9, 44.38), (34.4, 44.55), (35.4, 45.0), (36.5, 45.05), (36.62, 45.2), (36.72, 45.12), (37.8, 44.7), (38.8, 44.3), (39.8, 43.7), (40.2, 43.0),
];
/// アゾヴァ海
const SEA_AZOV: &[(f32, f32)] = &[
    (35.3, 45.35), (36.4, 45.48), (36.56, 45.32), (36.62, 45.2), (36.7, 45.32), (36.9, 45.42), (37.6, 45.7), (38.0, 46.3), (38.4, 46.7), (39.3, 47.05), (38.9, 47.22), (37.6, 47.05), (36.8, 46.72), (35.4, 46.32), (34.9, 46.1), (35.2, 45.6),
];
/// シヴァ湾（クリメア半島の付け根の浅瀬）
const SEA_SIVASH: &[(f32, f32)] = &[(34.0, 46.1), (34.7, 46.15), (35.15, 45.65), (34.6, 45.55), (34.1, 45.9)];
/// ウクラヴィアの国土
const UKRAVIA: &[(f32, f32)] = &[
    (22.2, 48.4), (22.7, 49.0), (22.65, 49.6), (23.6, 50.4), (24.1, 50.7), (23.7, 51.6), (25.5, 51.95), (27.5, 51.6), (29.3, 51.4), (30.6, 51.3), (31.0, 52.1), (32.1, 52.1), (33.8, 52.35), (34.4, 51.75), (35.4, 51.05), (36.3, 50.3), (38.0, 49.95), (40.1, 49.6), (40.2, 49.2), (39.8, 48.6), (39.8, 47.85), (38.3, 47.6), (38.25, 47.1),
    (37.0, 46.4), (35.0, 45.3), (34.5, 44.2), (33.0, 44.2), (32.2, 45.3), (30.0, 45.0), (29.0, 45.1), (28.2, 45.45), (28.2, 46.7), (29.1, 47.6), (29.2, 47.95), (27.8, 48.4), (26.6, 48.3), (24.9, 47.7), (23.1, 48.0),
];
/// 開戦時にロスコヴィアが押さえているドンバ地方
const OCCUPIED_DONBA: &[(f32, f32)] = &[(37.55, 47.05), (37.75, 47.75), (37.95, 48.25), (38.15, 48.75), (38.75, 49.1), (39.3, 49.7), (40.4, 49.8), (40.5, 47.5), (38.3, 46.9)];
/// クリメア半島
const CRIMEA: &[(f32, f32)] = &[(32.0, 44.0), (32.0, 46.0), (33.55, 46.08), (34.0, 46.12), (34.75, 46.15), (35.4, 45.7), (37.2, 45.3), (37.0, 44.0)];

struct CityDef(&'static str, f32, f32, u8, bool);
/// 架空の都市名（実在の位置を少しずらして借りている）
const CITIES: &[CityDef] = &[
    // ウクラヴィア
    CityDef("キエヴァ", 30.5, 50.45, 5, false),
    CityDef("ハルキア", 36.25, 49.95, 4, false),
    CityDef("オデスカ", 30.7, 46.55, 4, true),
    CityDef("ドニプロヴァ", 35.0, 48.5, 4, false),
    CityDef("リヴィア", 24.0, 49.85, 3, false),
    CityDef("ザポリア", 35.15, 47.85, 3, false),
    CityDef("クリヴォイ", 33.4, 47.9, 3, false),
    CityDef("ミコラ", 32.0, 47.0, 2, false),
    CityDef("ヘルソナ", 32.6, 46.7, 2, false),
    CityDef("マリウポラ", 37.5, 47.15, 3, false),
    CityDef("ポルタ", 34.55, 49.6, 2, false),
    CityDef("スーミャ", 34.8, 50.9, 2, false),
    CityDef("チェルニガ", 31.3, 51.5, 2, false),
    CityDef("ヴィニツァ", 28.5, 49.25, 2, false),
    CityDef("ジトーマ", 28.65, 50.25, 2, false),
    CityDef("テルノパ", 25.6, 49.55, 1, false),
    CityDef("クレメンチャ", 33.4, 49.1, 1, false),
    CityDef("イジューマ", 37.25, 49.2, 1, false),
    CityDef("クラマト", 37.55, 48.7, 2, false),
    CityDef("バフムーラ", 38.0, 48.6, 1, false),
    CityDef("ポクロヴァ", 37.15, 48.3, 1, false),
    CityDef("ベルジャナ", 36.8, 46.8, 1, false),
    CityDef("メリトーポ", 35.35, 46.85, 2, false),
    // ロスコヴィア
    CityDef("ロストヴァ", 39.7, 47.25, 4, false),
    CityDef("ヴォロニア", 39.2, 51.65, 4, false),
    CityDef("クルスカ", 36.2, 51.75, 3, false),
    CityDef("ベルゴラ", 36.6, 50.6, 2, false),
    CityDef("ブリャンスカ", 34.4, 52.75, 3, false),
    CityDef("クラスノーダ", 39.0, 45.05, 3, false),
    CityDef("ノヴォロス", 37.75, 44.8, 2, true),
    CityDef("タガンラ", 38.9, 47.3, 2, false),
    CityDef("シャフタ", 40.2, 47.7, 1, false),
    CityDef("ミレロヴァ", 40.4, 48.9, 1, false),
    CityDef("タンボヴァ", 41.45, 52.7, 2, false),
    CityDef("スタヴロ", 41.95, 45.05, 2, false),
    CityDef("ヴォルガラ", 43.4, 48.7, 3, false),
    CityDef("ボリソグラ", 42.1, 51.35, 1, false),
    CityDef("サリスカ", 41.5, 46.5, 1, false),
    // 開戦時にロスコヴィアの占領下
    CityDef("ドネスカ", 37.8, 48.0, 3, false),
    CityDef("ルガンスカ", 39.3, 48.55, 2, false),
    CityDef("シンフェラ", 34.1, 44.95, 2, false),
    CityDef("セヴァストラ", 33.5, 44.65, 2, true),
    CityDef("ケルチャ", 36.45, 45.35, 1, false),
];
/// 主要道路（都市名の組）
const ROADS: &[(&str, &str)] = &[
    ("リヴィア", "テルノパ"), ("テルノパ", "ヴィニツァ"), ("リヴィア", "ジトーマ"), ("ジトーマ", "キエヴァ"), ("ヴィニツァ", "キエヴァ"), ("ヴィニツァ", "オデスカ"),
    ("キエヴァ", "チェルニガ"), ("キエヴァ", "ポルタ"), ("キエヴァ", "クレメンチャ"), ("チェルニガ", "ブリャンスカ"), ("チェルニガ", "スーミャ"), ("スーミャ", "ハルキア"), ("スーミャ", "クルスカ"),
    ("ポルタ", "ハルキア"), ("ポルタ", "クレメンチャ"), ("クレメンチャ", "クリヴォイ"), ("クレメンチャ", "ドニプロヴァ"), ("ドニプロヴァ", "ハルキア"), ("ドニプロヴァ", "ザポリア"), ("ドニプロヴァ", "クリヴォイ"),
    ("クリヴォイ", "ミコラ"), ("ミコラ", "オデスカ"), ("ミコラ", "ヘルソナ"), ("ヘルソナ", "メリトーポ"), ("ザポリア", "メリトーポ"), ("メリトーポ", "ベルジャナ"), ("ベルジャナ", "マリウポラ"),
    ("ヘルソナ", "シンフェラ"), ("メリトーポ", "シンフェラ"), ("シンフェラ", "セヴァストラ"), ("シンフェラ", "ケルチャ"),
    ("ハルキア", "イジューマ"), ("ハルキア", "ベルゴラ"), ("イジューマ", "クラマト"), ("クラマト", "バフムーラ"), ("クラマト", "ポクロヴァ"), ("ポクロヴァ", "ドネスカ"), ("バフムーラ", "ドネスカ"), ("クラマト", "ドニプロヴァ"), ("ドネスカ", "ザポリア"), ("ドネスカ", "マリウポラ"), ("ドネスカ", "ルガンスカ"),
    ("マリウポラ", "タガンラ"), ("タガンラ", "ロストヴァ"), ("ドネスカ", "ロストヴァ"), ("ルガンスカ", "シャフタ"), ("シャフタ", "ロストヴァ"), ("ルガンスカ", "ミレロヴァ"), ("ミレロヴァ", "ヴォロニア"), ("ミレロヴァ", "シャフタ"),
    ("ベルゴラ", "クルスカ"), ("クルスカ", "ブリャンスカ"), ("クルスカ", "ヴォロニア"), ("ベルゴラ", "ヴォロニア"), ("ヴォロニア", "タンボヴァ"), ("ヴォロニア", "ボリソグラ"), ("ボリソグラ", "ヴォルガラ"), ("シャフタ", "ヴォルガラ"),
    ("ロストヴァ", "クラスノーダ"), ("ロストヴァ", "サリスカ"), ("サリスカ", "ヴォルガラ"), ("サリスカ", "スタヴロ"), ("クラスノーダ", "スタヴロ"), ("クラスノーダ", "ノヴォロス"),
];

/// 主要な鉄道（都市名の組）。両国は同じ軌間で、線路はそのまま国境を越えてつながっている
const RAILS: &[(&str, &str)] = &[
    ("リヴィア", "テルノパ"), ("テルノパ", "ヴィニツァ"), ("ヴィニツァ", "キエヴァ"), ("ヴィニツァ", "オデスカ"), ("オデスカ", "ミコラ"), ("ミコラ", "ヘルソナ"), ("ミコラ", "クリヴォイ"),
    ("クリヴォイ", "ドニプロヴァ"), ("キエヴァ", "ポルタ"), ("ポルタ", "ハルキア"), ("キエヴァ", "チェルニガ"), ("クレメンチャ", "ポルタ"), ("クレメンチャ", "クリヴォイ"),
    ("ドニプロヴァ", "ザポリア"), ("ザポリア", "メリトーポ"), ("メリトーポ", "シンフェラ"), ("ヘルソナ", "シンフェラ"), ("シンフェラ", "セヴァストラ"), ("シンフェラ", "ケルチャ"),
    ("ハルキア", "イジューマ"), ("イジューマ", "クラマト"), ("クラマト", "ポクロヴァ"), ("ポクロヴァ", "ドニプロヴァ"), ("クラマト", "ドネスカ"), ("ドネスカ", "マリウポラ"), ("ドネスカ", "ルガンスカ"),
    ("ドネスカ", "タガンラ"), ("タガンラ", "ロストヴァ"), ("ルガンスカ", "ミレロヴァ"), ("ミレロヴァ", "ヴォロニア"), ("ミレロヴァ", "ロストヴァ"), ("ヴォロニア", "クルスカ"), ("クルスカ", "ベルゴラ"),
    ("ベルゴラ", "ハルキア"), ("クルスカ", "ブリャンスカ"), ("ブリャンスカ", "チェルニガ"), ("クルスカ", "スーミャ"), ("スーミャ", "ポルタ"), ("ヴォロニア", "タンボヴァ"), ("ロストヴァ", "クラスノーダ"),
    ("クラスノーダ", "ノヴォロス"), ("クラスノーダ", "スタヴロ"), ("ロストヴァ", "サリスカ"), ("サリスカ", "ヴォルガラ"), ("ミレロヴァ", "ヴォルガラ"),
];

/// ドンバの主なボタ山の場所（経度・緯度）
const SPOIL_HEAPS: &[(f32, f32)] = &[(37.68, 48.13), (38.02, 48.31), (37.2, 48.27), (37.87, 48.59), (38.05, 48.72), (37.85, 48.05), (39.1, 48.4)];

/// 川：(点列, 太さ)
const RIVERS: &[(&[(f32, f32)], f32)] = &[
    // ドニプル川
    (&[(30.6, 53.2), (30.55, 51.6), (30.6, 50.5), (31.5, 49.75), (32.5, 49.25), (33.4, 49.0), (34.5, 48.55), (35.05, 48.3), (35.15, 47.75), (34.4, 47.45), (33.4, 46.95), (32.75, 46.6), (32.1, 46.4)], 1.5),
    // セヴェル・ドネ川
    (&[(36.7, 50.3), (37.3, 49.3), (38.4, 48.95), (39.5, 48.65), (40.3, 47.85), (40.9, 47.35)], 0.8),
    // ドーナ川
    (&[(39.1, 52.9), (39.2, 51.2), (39.6, 50.1), (41.0, 49.55), (43.0, 48.6), (42.3, 47.65), (40.9, 47.35), (39.4, 47.1)], 1.1),
    // ブーガ川
    (&[(26.5, 49.5), (28.5, 48.9), (30.5, 47.9), (31.5, 47.0), (31.6, 46.6)], 0.8),
    // ドニエスタ川
    (&[(23.2, 49.4), (25.6, 48.9), (27.6, 48.45)], 0.8),
    // プリピア川
    (&[(24.0, 51.8), (27.0, 51.6), (29.5, 51.45), (30.55, 51.3)], 0.8),
    // デスナ川
    (&[(33.9, 52.8), (32.8, 51.8), (31.4, 51.45), (30.6, 50.75)], 0.7),
    // クバーナ川
    (&[(41.4, 45.0), (39.1, 45.0), (37.6, 45.3)], 0.7),
];

fn jitter(seg: &[(f32, f32)], seed: u32, amp: f32) -> Vec<(f32, f32)> {
    // 折れ線を細かく分けてノイズでうねらせる（タイル座標）
    let mut out = Vec::new();
    for w in seg.windows(2) {
        let (a, b) = (w[0], w[1]);
        let d = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let n = (d / 0.5).ceil().max(1.0) as usize;
        for i in 0..n {
            let t = i as f32 / n as f32;
            let (x, y) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            let e = (t * (1.0 - t) * 4.0).min(1.0);
            let (nx, ny) = (fbm(x * 0.09, y * 0.09, seed, 3) - 0.5, fbm(x * 0.09 + 31.0, y * 0.09, seed + 5, 3) - 0.5);
            out.push((x + nx * amp * e, y + ny * amp * e));
        }
    }
    if let Some(l) = seg.last() {
        out.push(*l);
    }
    out
}

impl Map {
    pub fn new() -> Map {
        let n = W * H;
        let mut terrain = vec![Terrain::Plain; n];
        let mut start_owner = vec![-1i8; n];
        for y in 0..H {
            for x in 0..W {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let lon = LON0 + fx / W as f32 * (LON1 - LON0);
                let lat = LAT0 - fy / H as f32 * (LAT0 - LAT1);
                // 海岸線をノイズで少し揺らす
                let wob = (fbm(fx * 0.14, fy * 0.14, 11, 3) - 0.5) * 0.28;
                let wob2 = (fbm(fx * 0.14, fy * 0.14, 23, 3) - 0.5) * 0.2;
                let p = (lon + wob, lat + wob2);
                let sea = in_poly(p, SEA_MAIN) || in_poly(p, SEA_AZOV) || in_poly((lon, lat), SEA_SIVASH) || (lat < 44.5 && lon > 28.0 && lon < 37.0) || (lat < 44.2 && lon >= 37.0);
                let t = &mut terrain[y * W + x];
                if sea {
                    *t = Terrain::Sea;
                    continue;
                }
                let bp = (lon + wob * 0.5, lat + wob2 * 0.5);
                let in_ua = in_poly(bp, UKRAVIA);
                if !in_ua && lon < 31.5 {
                    *t = Terrain::Neutral;
                    continue;
                }
                let occupied = in_poly(bp, OCCUPIED_DONBA) || in_poly((lon, lat), CRIMEA);
                start_owner[y * W + x] = if in_ua && !occupied { 0 } else { 1 };
                // 森：北部と西の山地に多い
                let north = ((lat - 49.8) / 2.0).clamp(0.0, 1.0) * 0.17;
                let carp = if lon < 25.5 && lat < 49.0 { 0.2 } else { 0.0 };
                let cau = if lat < 45.0 && lon > 37.5 { 0.2 } else { 0.0 };
                let crim = if lat < 44.85 && lon < 35.5 { 0.2 } else { 0.0 };
                let f = fbm(fx * 0.11, fy * 0.11, 77, 4) + value_noise(fx * 0.4, fy * 0.4, 3) * 0.1;
                if f + north + carp + cau + crim > 0.72 {
                    *t = Terrain::Forest;
                    continue;
                }
                // ポレシアの湿地
                let wet = fbm(fx * 0.16, fy * 0.16, 131, 3);
                if lat > 50.9 && lat < 52.2 && lon > 23.8 && lon < 32.6 && wet > 0.53 {
                    *t = Terrain::Marsh;
                    continue;
                }
                // 山地と丘陵（カルパチア・クリメア南部・カフカス）とドンバの尾根
                let hilly = fbm(fx * 0.2, fy * 0.2, 151, 3);
                let ridge = lat > 47.75 && lat < 48.75 && lon > 37.2 && lon < 40.2;
                if ((carp + cau + crim) > 0.0 && hilly > 0.38) || (ridge && hilly > 0.56) {
                    *t = Terrain::Hill;
                    continue;
                }
                // 南部のステップは、畑の間を細い樹林帯が仕切っている
                if lat < 50.8 {
                    let (nx, ny) = (fx + (value_noise(fx * 0.07, fy * 0.07, 171) - 0.5) * 8.0, fy + (value_noise(fx * 0.07, fy * 0.07, 173) - 0.5) * 8.0);
                    let on_line = nx.rem_euclid(11.0) < 1.0 || ny.rem_euclid(8.0) < 1.0;
                    if on_line && hash2((nx / 11.0).floor() as i32, (ny / 8.0).floor() as i32, 177) < 0.55 {
                        *t = Terrain::Treeline;
                    }
                }
            }
        }
        let mut cities = Vec::new();
        for c in CITIES {
            let (x, y) = ll(c.1, c.2);
            let ti = tidx(x as i32, y as i32);
            let owner = start_owner[ti].max(0) as u8;
            cities.push(City { name: c.0, x, y, value: c.3, port: c.4, owner });
        }
        // 都市の市街地
        for c in &cities {
            let r = 1.0 + c.value as f32 * 0.8;
            stamp(&mut terrain, c.x, c.y, r, Terrain::Urban, Some(&[Terrain::Plain, Terrain::Forest, Terrain::Treeline, Terrain::Marsh, Terrain::Hill]));
        }
        // ボタ山（炭鉱の残土の山。周りを見渡せる）
        for (lon, lat) in SPOIL_HEAPS {
            let (x, y) = ll(*lon, *lat);
            stamp(&mut terrain, x, y, 1.3, Terrain::Hill, Some(&[Terrain::Plain, Terrain::Forest, Terrain::Treeline]));
        }
        // 川
        for (i, (pts, w)) in RIVERS.iter().enumerate() {
            let tp: Vec<_> = pts.iter().map(|p| ll(p.0, p.1)).collect();
            for p in jitter(&tp, 100 + i as u32, 8.0) {
                stamp(&mut terrain, p.0, p.1, *w, Terrain::River, Some(&[Terrain::Plain, Terrain::Forest, Terrain::Urban, Terrain::Marsh, Terrain::Treeline, Terrain::Hill]));
            }
        }
        // 道路と橋
        let mut road = vec![false; n];
        let mut pending_bridge = vec![false; n];
        let mut road_lines = Vec::new();
        let named = |s: &str| cities.iter().find(|c| c.name == s).map(|c| (c.x, c.y));
        for (i, (a, b)) in ROADS.iter().enumerate() {
            let (Some(pa), Some(pb)) = (named(a), named(b)) else { continue };
            let line = jitter(&[pa, pb], 300 + i as u32, 6.0);
            for p in &line {
                let (x, y) = (p.0 as i32, p.1 as i32);
                for (dx, dy) in [(0, 0), (1, 0), (0, 1)] {
                    let (px, py) = (x + dx, y + dy);
                    if !in_map(px, py) {
                        continue;
                    }
                    let ti = tidx(px, py);
                    match terrain[ti] {
                        Terrain::River => pending_bridge[ti] = true,
                        t if t.land() || t == Terrain::Marsh => road[ti] = true,
                        _ => {}
                    }
                }
            }
            road_lines.push(line);
        }
        // 鉄道（川を渡るところは鉄橋になる）
        let mut rail = vec![false; n];
        let mut rail_lines = Vec::new();
        for (i, (a, b)) in RAILS.iter().enumerate() {
            let (Some(pa), Some(pb)) = (named(a), named(b)) else { continue };
            rail_lines.push(jitter(&[pa, pb], 500 + i as u32, 3.0));
        }
        // ケルチャからクリメア橋を渡ってクラスノーダへ
        if let (Some(pa), Some(pb)) = (named("ケルチャ"), named("クラスノーダ")) {
            rail_lines.push(jitter(&[pa, ll(36.45, 45.38), ll(36.85, 45.25), ll(37.3, 45.25), pb], 590, 0.0));
        }
        for line in &rail_lines {
            let mut prev: Option<(i32, i32)> = None;
            for p in line {
                let (x, y) = (p.0 as i32, p.1 as i32);
                // 上下左右でつながるように、斜めに進むときは角も埋める
                let mut cells = vec![(x, y)];
                if let Some((px, py)) = prev {
                    if px != x && py != y {
                        cells.push((x, py));
                    }
                }
                for (cx, cy) in cells {
                    if !in_map(cx, cy) {
                        continue;
                    }
                    let ti = tidx(cx, cy);
                    match terrain[ti] {
                        Terrain::River => {
                            pending_bridge[ti] = true;
                            rail[ti] = true;
                        }
                        t if t.land() || t == Terrain::Marsh => rail[ti] = true,
                        _ => {}
                    }
                }
                prev = Some((x, y));
            }
        }
        // クリメア橋（ケルチャ海峡）
        let (kx0, ky0) = ll(36.45, 45.38);
        let (kx1, ky1) = ll(36.85, 45.25);
        let steps = 40;
        let mut kerch = Vec::new();
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let (x, y) = ((kx0 + (kx1 - kx0) * t) as i32, (ky0 + (ky1 - ky0) * t) as i32);
            for (dx, dy) in [(0, 0), (0, 1)] {
                let ti = tidx(x + dx, y + dy);
                if terrain[ti] == Terrain::Sea && !kerch.contains(&ti) {
                    kerch.push(ti);
                }
            }
        }
        // 橋を塊ごとにまとめる
        let mut bridge_at = vec![u16::MAX; n];
        let mut bridges = Vec::new();
        for i in 0..n {
            if !pending_bridge[i] || bridge_at[i] != u16::MAX {
                continue;
            }
            let id = bridges.len() as u16;
            let mut st = vec![i];
            let mut tiles = Vec::new();
            bridge_at[i] = id;
            while let Some(c) = st.pop() {
                tiles.push(c as u32);
                let (cx, cy) = ((c % W) as i32, (c / W) as i32);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if !in_map(cx + dx, cy + dy) {
                        continue;
                    }
                    let t2 = tidx(cx + dx, cy + dy);
                    if pending_bridge[t2] && bridge_at[t2] == u16::MAX {
                        bridge_at[t2] = id;
                        st.push(t2);
                    }
                }
            }
            bridges.push(make_bridge(tiles, Terrain::River, 400.0, None));
        }
        if !kerch.is_empty() {
            let id = bridges.len() as u16;
            for &t in &kerch {
                bridge_at[t] = id;
                // 海峡の橋は両岸をつなぐ陸として扱う
                start_owner[t] = 1;
                rail[t] = true;
            }
            bridges.push(make_bridge(kerch.iter().map(|t| *t as u32).collect(), Terrain::Sea, 1200.0, Some("クリメア橋")));
        }
        for b in &bridges {
            for &t in &b.tiles {
                terrain[t as usize] = Terrain::Bridge;
            }
        }
        for (i, r) in road.iter_mut().enumerate() {
            if terrain[i] == Terrain::Bridge {
                *r = true;
            }
        }
        let mut labels = Vec::new();
        for (text, lon, lat, kind) in [
            ("ウクラヴィア共和国", 29.5, 49.6, 0u8),
            ("ロスコヴィア連邦", 40.8, 50.6, 0),
            ("クリメア半島", 34.3, 45.3, 1),
            ("ドンバ地方", 38.9, 48.0, 1),
            ("ポント海", 32.5, 44.0, 2),
            ("アゾヴァ海", 37.2, 46.3, 2),
            ("ベラルシア（中立）", 27.5, 52.4, 3),
            ("ポロニア（中立）", 22.6, 51.8, 3),
            ("ルマニア（中立）", 26.2, 46.2, 3),
            ("ドニプル川", 33.2, 48.6, 2),
        ] {
            let (x, y) = ll(lon, lat);
            labels.push(Label { text, x, y, kind });
        }
        let base_bridges = bridges.len();
        Map { terrain, road, rail, rail_lines, road_lines, bridge_at, bridges, cities, start_owner, labels, ground: Ground::Normal, base_bridges }
    }

    pub fn base_bridge_count(m: &Map) -> usize {
        m.base_bridges
    }

    #[inline]
    pub fn t(&self, x: i32, y: i32) -> Terrain {
        if in_map(x, y) {
            self.terrain[tidx(x, y)]
        } else {
            Terrain::Neutral
        }
    }

    /// 地形の通行コスト（None は通れない）
    #[inline]
    pub fn cost(&self, i: usize, p: Pass) -> Option<f32> {
        let t = self.terrain[i];
        match p {
            Pass::Air => Some(1.0),
            Pass::Sea => {
                if t == Terrain::Sea {
                    Some(1.0)
                } else {
                    None
                }
            }
            Pass::Inf => {
                if self.rail[i] && t != Terrain::River {
                    return Some(0.7);
                }
                if self.road[i] && t != Terrain::River {
                    return Some(if t == Terrain::Bridge { 0.7 } else { 0.8 });
                }
                let mud = if self.ground == Ground::Mud { 1.25 } else { 1.0 };
                match t {
                    Terrain::Sea | Terrain::Neutral => None,
                    Terrain::River => Some(5.0),
                    Terrain::Forest => Some(1.3 * mud),
                    Terrain::Hill => Some(1.3 * mud),
                    Terrain::Marsh => Some(if self.ground == Ground::Frozen { 1.1 } else { 2.5 * mud }),
                    _ => Some(mud),
                }
            }
            Pass::Veh => {
                if matches!(t, Terrain::Sea | Terrain::Neutral | Terrain::River) {
                    return None;
                }
                // 線路は泥濘でも使える
                if self.rail[i] {
                    return Some(0.45);
                }
                if self.road[i] {
                    return Some(if t == Terrain::Urban { 0.7 } else { 0.6 });
                }
                // 泥濘期は道路の外を避け、凍れば湿地も走れる
                let (g, frozen) = match self.ground {
                    Ground::Mud => (2.6, false),
                    Ground::Frozen => (0.9, true),
                    Ground::Normal => (1.0, false),
                };
                Some(match t {
                    Terrain::Bridge => 0.6,
                    Terrain::Urban => 1.3,
                    Terrain::Forest => 1.9 * g,
                    Terrain::Hill => 1.5 * g,
                    Terrain::Treeline => 1.2 * g,
                    Terrain::Marsh if frozen => 1.2,
                    Terrain::Marsh => 5.0 * g,
                    _ => g,
                })
            }
        }
    }
}

fn make_bridge(tiles: Vec<u32>, under: Terrain, hp: f32, name: Option<&'static str>) -> Bridge {
    let (mut sx, mut sy) = (0.0, 0.0);
    for &t in &tiles {
        sx += (t as usize % W) as f32 + 0.5;
        sy += (t as usize / W) as f32 + 0.5;
    }
    let c = tiles.len().max(1) as f32;
    Bridge { tiles, hp, max_hp: hp, under, x: sx / c, y: sy / c, name }
}

fn stamp(terrain: &mut [Terrain], x: f32, y: f32, r: f32, v: Terrain, only: Option<&[Terrain]>) {
    let ri = r.ceil() as i32;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let (px, py) = (x as i32 + dx, y as i32 + dy);
            if !in_map(px, py) {
                continue;
            }
            let (ax, ay) = (px as f32 + 0.5 - x, py as f32 + 0.5 - y);
            if ax * ax + ay * ay > r * r {
                continue;
            }
            let t = &mut terrain[tidx(px, py)];
            if only.map_or(true, |o| o.contains(t)) {
                *t = v;
            }
        }
    }
}
