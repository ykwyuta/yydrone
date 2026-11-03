//! 史実の年表：2022年の全面侵攻から2026年10月のスターリンカ制限まで
//!
//! 実際の出来事を日付どおりに起こす。国名・地名・人名・兵器名は他と同じく少し変えた架空の名前で、
//! 出典は docs/timeline.md にまとめてある。盤面が史実と離れたとき（条件の都市を持っていないなど）は
//! 効果を出さず、「この戦争では起きなかった」と記録する。

use crate::defs::*;
use crate::geo::*;
use crate::world::*;

/// 暦の開始（2022年2月24日）を 0 日目とした日数
pub const fn date_to_day(y: u32, m: u32, d: u32) -> u32 {
    // うるう年込みの通し日数（グレゴリオ暦）
    let (yy, mm) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let n = 365 * yy + yy / 4 - yy / 100 + yy / 400 + (153 * (mm - 3) + 2) / 5 + d;
    let (y0, m0, d0) = (2021u32, 14u32, 24u32); // 2022年2月24日
    let n0 = 365 * y0 + y0 / 4 - y0 / 100 + y0 / 400 + (153 * (m0 - 3) + 2) / 5 + d0;
    n - n0
}

/// 日数から年月日に戻す
pub fn day_to_date(day: u32) -> (u32, u32, u32) {
    const M: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let (mut y, mut m, mut d) = (2022u32, 2u32, 24 + day);
    loop {
        let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let len = if m == 2 && leap { 29 } else { M[m as usize - 1] };
        if d <= len {
            return (y, m, d);
        }
        d -= len;
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
}

/// 制限時間の割合 → 暦の日数。動きの多い時期ほど長く遊べるように一定にしない
const KNOTS: [(f32, u32); 8] = [
    (0.00, 0),                         // 2022-02-24 開戦
    (0.12, date_to_day(2022, 4, 2)),   // キエヴァ州の解放
    (0.38, date_to_day(2022, 12, 31)),
    (0.55, date_to_day(2023, 12, 31)),
    (0.70, date_to_day(2024, 12, 31)),
    (0.84, date_to_day(2025, 12, 31)),
    (0.97, date_to_day(2026, 10, 14)), // スターシルダの速度制限
    (1.00, date_to_day(2026, 10, 31)),
];

pub fn frac_to_day(f: f32) -> u32 {
    let f = f.clamp(0.0, 1.0);
    for w in KNOTS.windows(2) {
        if f <= w[1].0 {
            let t = (f - w[0].0) / (w[1].0 - w[0].0);
            return w[0].1 + ((w[1].1 - w[0].1) as f32 * t) as u32;
        }
    }
    KNOTS[KNOTS.len() - 1].1
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ModKind {
    /// 地上部隊の被害倍率
    Protect(f32),
    /// 移動速度の倍率
    Slow(f32),
    /// 補給が届かない
    NoSupply,
    /// 通信途絶：ドローンが墜落し、FPV の発射が遅くなる
    Outage,
    /// 衛星通信：ドローンが妨害に強くなり、発射も速くなる
    Uplink,
    IncomeMul(f32),
    ShellMul(f32),
    /// AI の将軍が動かない（反乱など）
    Frozen,
}

#[derive(Clone, Copy, Debug)]
pub struct Mod {
    pub side: u8,
    pub kind: ModKind,
    pub zone: Option<(f32, f32, f32)>,
    pub until: u32,
}

#[derive(Clone, Copy)]
pub enum Place {
    City(&'static str),
    /// 経度・緯度
    LL(f32, f32),
}

pub enum Eff {
    Funds(u8, f32),
    Shells(u8, f32),
    Spawn(u8, UK, u32, Place),
    Mod(u8, ModKind, Option<(Place, f32)>, u32),
    /// 通信の恩恵を打ち切る
    EndUplink(u8),
    /// AI の主攻方向を固定する（都市名、日数）
    Axes(u8, &'static [&'static str], u32),
    /// AI がこの円の内側へ進まない（陣営、中心、半径、日数）
    Halt(u8, Place, f32, u32),
    /// 委任された部隊を円の中から撤退させる
    Retreat(u8, Place, f32, Place),
    /// 円の中の、もともと to の領土だったタイルを to に戻す
    Revert(u8, Place, f32),
    /// 名前で橋を傷つける（耐久の割合）
    BridgeHit(&'static str, f32),
    /// 陣営の最大の艦に耐久の割合のダメージ
    ShipHit(u8, f32),
    /// 陣営の戦闘機の割合を失わせ、飛行場を傷つける
    AirfieldRaid(u8, f32, f32),
    /// ドニプル川下流の氾濫（日数）
    Flood(u32),
    /// この日数の間、季節にかかわらず地面を泥濘にする
    EarlyMud(u32),
    /// 天候をこの秒数だけ霧にする
    FogBank(f32),
    /// 円の中の線路を切る（半径、復旧までの秒数）
    RailCut(Place, f32, f32),
}

pub struct HistEvent {
    pub date: (u32, u32, u32),
    /// 日が特定できず、月までしか分からない
    pub month_only: bool,
    pub title: &'static str,
    pub text: &'static str,
    /// 得をする側（2 はどちらでもない）
    pub side: u8,
    pub place: Option<Place>,
    /// この都市をこの陣営が持っているときだけ起こる
    pub requires: Option<(&'static str, u8)>,
    pub effects: &'static [Eff],
}

const fn ev(date: (u32, u32, u32), title: &'static str, text: &'static str, side: u8, place: Option<Place>, requires: Option<(&'static str, u8)>, effects: &'static [Eff]) -> HistEvent {
    HistEvent { date, month_only: false, title, text, side, place, requires, effects }
}

const KYIV: Place = Place::City("キエヴァ");
const HOSTOMEL: Place = Place::LL(30.2, 50.6);
const LOWER_DNIPRO: Place = Place::LL(33.5, 46.9);

pub const HISTORY: &[HistEvent] = &[
    // ---------------------------------------------------------------- 2022 一斉侵攻とキエヴァ攻防戦
    ev(
        (2022, 2, 24),
        "全面侵攻が始まる",
        "ロスコヴィア軍が北・東・南の国境から一斉に侵攻した。主攻はキエヴァ、ハルキア、ヘルソナ、マリウポラ。",
        1,
        Some(KYIV),
        None,
        &[
            // ベラルシア国境から南下する北方軍
            Eff::Spawn(1, UK::Tank, 5, Place::LL(30.1, 51.2)),
            Eff::Spawn(1, UK::Ifv, 4, Place::LL(30.1, 51.2)),
            Eff::Spawn(1, UK::Infantry, 5, Place::LL(30.1, 51.2)),
            // チェルニガ・スーミャ方面
            Eff::Spawn(1, UK::Tank, 3, Place::LL(32.2, 52.0)),
            Eff::Spawn(1, UK::Infantry, 3, Place::LL(32.2, 52.0)),
            Eff::Spawn(1, UK::Ifv, 3, Place::LL(34.5, 51.3)),
            // ハルキア方面
            Eff::Spawn(1, UK::Tank, 3, Place::City("ベルゴラ")),
            Eff::Spawn(1, UK::Infantry, 4, Place::City("ベルゴラ")),
            // クリメアからの南方軍
            Eff::Spawn(1, UK::Tank, 3, Place::LL(33.8, 46.0)),
            Eff::Spawn(1, UK::Ifv, 3, Place::LL(33.8, 46.0)),
            Eff::Axes(1, &["キエヴァ", "ハルキア", "ヘルソナ", "マリウポラ"], 37),
            Eff::Funds(0, 500.0),
        ],
    ),
    ev(
        (2022, 2, 24),
        "暖冬で泥濘が早い",
        "例年より暖かく、地面が十分に凍らないまま開戦を迎えた。車列は道路に縛られる。",
        0,
        None,
        None,
        &[Eff::EarlyMud(66)],
    ),
    ev(
        (2022, 2, 24),
        "ホストメラ空港への強襲",
        "ヘリで降下した空挺部隊がキエヴァ近郊の空港を襲撃した。ウクラヴィア軍は同日のうちに反撃に出た。",
        1,
        Some(HOSTOMEL),
        None,
        &[
            Eff::Spawn(1, UK::Infantry, 5, HOSTOMEL),
            Eff::Spawn(1, UK::Heli, 2, HOSTOMEL),
            Eff::Spawn(0, UK::Infantry, 8, KYIV),
            Eff::Spawn(0, UK::Atgm, 4, KYIV),
            Eff::Spawn(0, UK::Tank, 2, KYIV),
        ],
    ),
    ev(
        (2022, 2, 25),
        "キエヴァ攻防戦",
        "ロスコヴィア軍が首都へ迫る。ウクラヴィア軍と領土防衛隊は市の手前で進撃を食い止めている。",
        0,
        Some(KYIV),
        None,
        &[
            Eff::Halt(1, KYIV, 13.0, 36),
            Eff::Mod(0, ModKind::Protect(0.55), Some((KYIV, 16.0)), 36),
            Eff::Spawn(0, UK::Infantry, 6, KYIV),
            Eff::Spawn(0, UK::FpvTeam, 2, KYIV),
        ],
    ),
    ev(
        (2022, 2, 25),
        "イルピア川の氾濫",
        "キエヴァ北西のダムが破られ、川と湿地が広く水に沈んだ。ロスコヴィア軍は渡河できず、首都への道を阻まれる。",
        0,
        Some(Place::LL(30.3, 50.75)),
        None,
        &[Eff::Mod(1, ModKind::Slow(0.35), Some((Place::LL(30.25, 50.7), 15.0)), 36)],
    ),
    ev(
        (2022, 2, 26),
        "スターリンカの提供開始",
        "宇宙企業スペースエクサのエロン・マスカ氏がウクラヴィアでのサービス開始を表明し、まもなく端末が到着した。前線の通信とドローンを支える基盤になっていく。",
        0,
        Some(KYIV),
        None,
        &[Eff::Mod(0, ModKind::Uplink, None, 3000)],
    ),
    HistEvent {
        date: (2022, 2, 27),
        month_only: true,
        title: "国境の鉄道を破壊",
        text: "ウクラヴィアは両国をつなぐ線路を自ら断った。ロスコヴィア軍の補給は鉄道に大きく頼っている。",
        side: 0,
        place: Some(Place::City("ハルキア")),
        requires: None,
        effects: &[
            Eff::RailCut(Place::LL(36.45, 50.35), 8.0, 240.0),
            Eff::RailCut(Place::LL(32.6, 52.05), 8.0, 240.0),
            Eff::RailCut(Place::LL(34.8, 51.2), 8.0, 240.0),
        ],
    },
    ev(
        (2022, 2, 28),
        "キエヴァ北方に長大な車列",
        "衛星写真に約64kmのロスコヴィア軍車列が写った。道路に詰まった車列は補給に苦しむ。",
        0,
        Some(Place::LL(30.2, 51.0)),
        None,
        &[
            Eff::Mod(1, ModKind::Slow(0.5), Some((Place::LL(30.3, 50.95), 30.0)), 33),
            Eff::Mod(1, ModKind::NoSupply, Some((Place::LL(30.3, 50.95), 30.0)), 33),
        ],
    ),
    ev(
        (2022, 4, 2),
        "キエヴァ州が解放される",
        "ロスコヴィア軍は首都攻略を断念して北部から撤退し、主攻をドンバへ移した。",
        0,
        Some(KYIV),
        Some(("キエヴァ", 0)),
        &[
            Eff::Retreat(1, KYIV, 60.0, Place::City("ブリャンスカ")),
            Eff::Revert(0, KYIV, 58.0),
            Eff::Axes(1, &["クラマト", "マリウポラ", "イジューマ"], 90),
            Eff::Funds(0, 300.0),
        ],
    ),
    ev(
        (2022, 4, 14),
        "旗艦モスコヴァが沈む",
        "ポント海艦隊の旗艦が対艦ミサイル「ネプトゥ」の命中後に沈没した（ロスコヴィアは火災と説明）。艦隊は沖へ下がる。",
        0,
        Some(Place::City("オデスカ")),
        None,
        &[Eff::ShipHit(1, 1.0), Eff::Spawn(0, UK::Coastal, 1, Place::City("オデスカ"))],
    ),
    ev(
        (2022, 5, 20),
        "マリウポラ陥落",
        "製鉄所に立てこもった守備隊が降伏し、3か月近い包囲が終わった。",
        1,
        Some(Place::City("マリウポラ")),
        Some(("マリウポラ", 1)),
        &[Eff::Funds(1, 300.0), Eff::Shells(1, 200.0)],
    ),
    ev(
        (2022, 6, 23),
        "ハイマルが到着",
        "西側の高機動ロケット砲が届いた。後方の弾薬庫や橋が射程に入る。",
        0,
        Some(Place::City("ドニプロヴァ")),
        None,
        &[Eff::Spawn(0, UK::Mlrs, 3, Place::City("ドニプロヴァ")), Eff::Shells(0, 250.0)],
    ),
    ev(
        (2022, 6, 25),
        "セヴェロドネ陥落",
        "ルガンスカ方面の激戦地が落ちた。ロスコヴィアの砲兵火力が東部を押している。",
        1,
        Some(Place::City("バフムーラ")),
        None,
        &[Eff::Shells(1, 300.0), Eff::Axes(1, &["バフムーラ", "クラマト"], 180)],
    ),
    ev(
        (2022, 6, 30),
        "蛇島からの撤退",
        "ロスコヴィア軍がポント海西部の島から撤退した。オデスカの沖が開く。",
        0,
        Some(Place::City("オデスカ")),
        None,
        &[Eff::Funds(0, 200.0)],
    ),
    ev(
        (2022, 7, 6),
        "メリトーポ近くの橋を爆破",
        "占領地の鉄道の要衝メリトーポの近くで橋が破壊された。陸の回廊を走る補給が細る。",
        0,
        Some(Place::City("メリトーポ")),
        None,
        &[Eff::RailCut(Place::City("メリトーポ"), 7.0, 120.0)],
    ),
    ev(
        (2022, 9, 6),
        "ハルキア反攻",
        "ウクラヴィア軍が東部で電撃的に反攻し、イジューマ周辺を一気に奪い返した。",
        0,
        Some(Place::City("イジューマ")),
        None,
        &[
            Eff::Spawn(0, UK::Tank, 4, Place::City("ハルキア")),
            Eff::Spawn(0, UK::Ifv, 5, Place::City("ハルキア")),
            Eff::Spawn(0, UK::Infantry, 5, Place::City("ハルキア")),
            Eff::Retreat(1, Place::City("イジューマ"), 22.0, Place::City("ルガンスカ")),
            Eff::Revert(0, Place::City("イジューマ"), 22.0),
            Eff::Axes(0, &["イジューマ", "ヘルソナ"], 70),
        ],
    ),
    HistEvent {
        date: (2022, 9, 15),
        month_only: true,
        title: "ヘルソナ方面でスターリンカが止まる",
        text: "反攻中のヘルソナ周辺で接続が遮断されたと報じられた。マスカ氏はクリメア沖への接続拡大も断っている。",
        side: 1,
        place: Some(Place::City("ヘルソナ")),
        requires: None,
        effects: &[Eff::Mod(0, ModKind::Outage, Some((Place::City("ヘルソナ"), 28.0)), 25)],
    },
    ev(
        (2022, 9, 21),
        "部分動員令",
        "ロスコヴィアが予備役30万人の動員を発表した。粗くても兵力が前線へ流れ込む。",
        1,
        Some(Place::City("ロストヴァ")),
        None,
        &[Eff::Spawn(1, UK::Infantry, 12, Place::City("ドネスカ")), Eff::Funds(1, 400.0)],
    ),
    ev(
        (2022, 10, 8),
        "クリメア橋の爆発",
        "ケルチャ海峡の橋で爆発が起き、一部が崩落した。半島への補給が細る。",
        0,
        Some(Place::City("ケルチャ")),
        None,
        &[Eff::BridgeHit("クリメア橋", 0.7), Eff::Mod(1, ModKind::NoSupply, Some((Place::City("シンフェラ"), 30.0)), 30)],
    ),
    ev(
        (2022, 10, 29),
        "セヴァストラ軍港に水上ドローン",
        "無人艇と無人機が軍港を襲った。水上ドローンが海戦の主役になり始める。",
        0,
        Some(Place::City("セヴァストラ")),
        None,
        &[Eff::ShipHit(1, 0.4), Eff::Spawn(0, UK::SeaDrone, 2, Place::City("オデスカ"))],
    ),
    ev(
        (2022, 11, 11),
        "ヘルソナ解放",
        "ロスコヴィア軍がドニプル川西岸から撤退し、ウクラヴィア軍が入城した。2022年の侵攻で占領された唯一の州都の奪還である。",
        0,
        Some(Place::City("ヘルソナ")),
        Some(("ヘルソナ", 1)),
        &[
            Eff::Retreat(1, Place::City("ヘルソナ"), 26.0, Place::City("メリトーポ")),
            Eff::Revert(0, Place::City("ヘルソナ"), 18.0),
            Eff::Funds(0, 300.0),
        ],
    ),
    // ---------------------------------------------------------------- 2023
    ev(
        (2023, 1, 25),
        "主力戦車の供与が決まる",
        "西側諸国がレオパルトなどの主力戦車の供与を表明した。",
        0,
        Some(Place::City("リヴィア")),
        None,
        &[Eff::Spawn(0, UK::Tank, 5, Place::City("ドニプロヴァ")), Eff::Funds(0, 300.0)],
    ),
    ev(
        (2023, 5, 20),
        "バフムーラ陥落",
        "ロスコヴィア側が廃墟となった市の制圧を宣言した。戦争で最も長い消耗戦のひとつだった。",
        1,
        Some(Place::City("バフムーラ")),
        Some(("バフムーラ", 1)),
        &[Eff::Funds(1, 300.0)],
    ),
    ev(
        (2023, 6, 4),
        "ウクラヴィアの2023年反攻",
        "南部で大規模な反攻が始まった。何重もの塹壕と地雷原が機甲部隊を阻む。",
        0,
        Some(Place::City("メリトーポ")),
        None,
        &[
            Eff::Spawn(0, UK::Tank, 5, Place::City("ザポリア")),
            Eff::Spawn(0, UK::Ifv, 5, Place::City("ザポリア")),
            Eff::Spawn(0, UK::Engineer, 2, Place::City("ザポリア")),
            Eff::Axes(0, &["メリトーポ", "ベルジャナ"], 180),
            Eff::Mod(1, ModKind::Protect(0.75), Some((Place::City("メリトーポ"), 40.0)), 180),
        ],
    ),
    ev(
        (2023, 6, 6),
        "カホヴァ・ダムの決壊",
        "ドニプル川下流のダムが崩壊し、広い範囲が水没した。両国は互いに相手の仕業だと非難している。",
        2,
        Some(LOWER_DNIPRO),
        None,
        &[Eff::Flood(90)],
    ),
    ev(
        (2023, 6, 23),
        "ヴァーグナ傭兵団の反乱",
        "民間軍事会社が反旗を翻して首都へ進軍し、翌日に引き返した。ロスコヴィア軍の指揮は一時混乱する。",
        0,
        Some(Place::City("ロストヴァ")),
        None,
        &[Eff::Mod(1, ModKind::Frozen, None, 20), Eff::Mod(1, ModKind::IncomeMul(0.6), None, 20)],
    ),
    ev(
        (2023, 10, 17),
        "長距離ミサイルで飛行場を攻撃",
        "ウクラヴィアが供与された長距離ミサイルを初めて実戦で使い、占領地の飛行場を叩いた。",
        0,
        Some(Place::City("ベルジャナ")),
        None,
        &[Eff::AirfieldRaid(1, 0.3, 0.4), Eff::Spawn(0, UK::Mlrs, 2, Place::City("ドニプロヴァ"))],
    ),
    // ---------------------------------------------------------------- 2024
    ev(
        (2024, 2, 17),
        "アヴデーヤからの撤退",
        "ドネスカ近郊の要塞都市からウクラヴィア軍が撤退した。西側の支援が滞り、砲弾が足りない。",
        1,
        Some(Place::City("ドネスカ")),
        None,
        &[Eff::Mod(0, ModKind::ShellMul(0.4), None, 66), Eff::Shells(1, 300.0), Eff::Axes(1, &["ポクロヴァ", "クラマト"], 500)],
    ),
    ev(
        (2024, 4, 24),
        "610億ドルの支援法が成立",
        "長く止まっていたアメリカナの追加支援が成立した。砲弾と長距離ミサイルがふたたび届き始める。",
        0,
        Some(Place::City("リヴィア")),
        None,
        &[Eff::Funds(0, 1200.0), Eff::Shells(0, 600.0), Eff::Spawn(0, UK::Sam, 2, KYIV)],
    ),
    ev(
        (2024, 8, 6),
        "クルスカ越境攻撃",
        "ウクラヴィア軍が国境を越えてロスコヴィア本土へ奇襲した。第二次大戦後で初めて本土が占領される。",
        0,
        Some(Place::LL(35.2, 51.25)),
        None,
        &[
            Eff::Spawn(0, UK::Tank, 3, Place::LL(35.2, 51.0)),
            Eff::Spawn(0, UK::Ifv, 5, Place::LL(35.2, 51.0)),
            Eff::Spawn(0, UK::Infantry, 6, Place::LL(35.2, 51.0)),
            Eff::Axes(0, &["クルスカ"], 150),
        ],
    ),
    HistEvent {
        date: (2024, 10, 25),
        month_only: true,
        title: "北方の同盟国が派兵",
        text: "朝鮮半島の同盟国の兵1万人余りがクルスカ方面に送られたと各国が報告した。",
        side: 1,
        place: Some(Place::City("クルスカ")),
        requires: None,
        effects: &[Eff::Spawn(1, UK::Infantry, 12, Place::City("クルスカ")), Eff::Shells(1, 400.0)],
    },
    // ---------------------------------------------------------------- 2025
    HistEvent {
        date: (2025, 3, 13),
        month_only: true,
        title: "クルスカから押し戻される",
        text: "同盟国の兵の助けを得たロスコヴィア軍が反撃し、越境部隊は本土からほぼ追い出された。",
        side: 1,
        place: Some(Place::City("クルスカ")),
        requires: None,
        effects: &[Eff::Retreat(0, Place::City("クルスカ"), 38.0, Place::City("スーミャ")), Eff::Revert(1, Place::City("クルスカ"), 38.0)],
    },
    ev(
        (2025, 6, 1),
        "蜘蛛の巣作戦",
        "トラックで密かに運び込んだドローンがロスコヴィア奥地の飛行場を同時に襲い、戦略爆撃機に大きな損害を与えた。",
        0,
        Some(Place::City("クルスカ")),
        None,
        &[Eff::AirfieldRaid(1, 0.5, 0.5)],
    ),
    ev(
        (2025, 8, 15),
        "アラスカ首脳会談",
        "アメリカナとロスコヴィアの首脳が会談したが、停戦には至らなかった。戦闘は続く。",
        2,
        None,
        None,
        &[],
    ),
    HistEvent {
        date: (2025, 10, 9),
        month_only: true,
        title: "鉄道への攻撃が激化",
        text: "ロスコヴィアがウクラヴィアの線路と列車への攻撃を強めた。2か月で220回を超えたと報じられている。「第二の軍」と呼ばれる鉄道が狙われる。",
        side: 1,
        place: Some(Place::City("ドニプロヴァ")),
        requires: None,
        effects: &[
            Eff::RailCut(Place::City("ドニプロヴァ"), 6.0, 150.0),
            Eff::RailCut(Place::City("ポルタ"), 6.0, 150.0),
            Eff::RailCut(Place::City("クラマト"), 6.0, 150.0),
        ],
    },
    HistEvent {
        date: (2025, 11, 15),
        month_only: true,
        title: "霧の中のポクロヴァ",
        text: "濃い霧でドローンが飛べない間に、ロスコヴィアの小部隊が防衛線の隙間へ浸透した。",
        side: 1,
        place: Some(Place::City("ポクロヴァ")),
        requires: None,
        effects: &[Eff::FogBank(90.0), Eff::Spawn(1, UK::Infantry, 6, Place::City("ドネスカ")), Eff::Axes(1, &["ポクロヴァ", "クラマト"], 90)],
    },
    HistEvent {
        date: (2025, 12, 10),
        month_only: true,
        title: "ロスコヴィア軍もスターリンカでドローンを誘導",
        text: "不正に入手した端末で、ロスコヴィア軍がドローンを大規模に誘導し始めた。",
        side: 1,
        place: None,
        requires: None,
        effects: &[Eff::Mod(1, ModKind::Uplink, None, 3000)],
    },
    // ---------------------------------------------------------------- 2026
    HistEvent {
        date: (2026, 1, 15),
        month_only: true,
        title: "ポクロヴァ陥落",
        text: "1年半におよぶ攻勢の末、東部の交通の要衝が落ちた。",
        side: 1,
        place: Some(Place::City("ポクロヴァ")),
        requires: Some(("ポクロヴァ", 1)),
        effects: &[Eff::Funds(1, 300.0)],
    },
    ev(
        (2026, 2, 2),
        "スターリンカのホワイトリスト",
        "登録された端末だけが接続できるようになり、ロスコヴィア軍の端末が一斉に切れた。ウクラヴィア側の部隊にも一時混乱が及ぶ。",
        0,
        None,
        None,
        &[
            Eff::EndUplink(1),
            Eff::Mod(1, ModKind::Outage, None, 45),
            Eff::Mod(0, ModKind::Outage, None, 6),
        ],
    ),
    HistEvent {
        date: (2026, 6, 19),
        month_only: true,
        title: "クリメアへの鉄道橋を攻撃",
        text: "半島へ通じる鉄道橋が相次いで攻撃された。残る線路はメリトーポ経由とケルチャ経由の二本で、半島への補給が細る。",
        side: 0,
        place: Some(Place::City("シンフェラ")),
        requires: None,
        effects: &[Eff::RailCut(Place::LL(34.4, 46.15), 8.0, 150.0), Eff::Mod(1, ModKind::NoSupply, Some((Place::City("シンフェラ"), 30.0)), 40)],
    },
    ev(
        (2026, 7, 25),
        "ホワイトリストの穴",
        "情報機関が、工作員による端末1000台超の不正登録を摘発した。ロスコヴィア軍の接続が戻りつつある。",
        1,
        None,
        None,
        &[Eff::Mod(1, ModKind::Uplink, None, 3000)],
    ),
    ev(
        (2026, 10, 14),
        "マスカ氏の接続制限",
        "スターシルダに予告なく速度制限がかかり、時速160km超では接続が切れるようになった。ウクラヴィアの迎撃ドローンが飛ばせなくなる。マスカ氏は攻撃用途の規約違反を理由に挙げ、責任の所在は政府との間で争われた。",
        1,
        None,
        None,
        &[
            Eff::EndUplink(0),
            Eff::Mod(0, ModKind::Outage, None, 10),
            Eff::Spawn(1, UK::LongDrone, 8, Place::City("ベルゴラ")),
        ],
    ),
    ev(
        (2026, 10, 24),
        "接続が復旧",
        "ウクラヴィア当局がスターシルダの通常運用への復帰を報告した。",
        0,
        None,
        None,
        &[Eff::Mod(0, ModKind::Uplink, None, 3000)],
    ),
];

impl World {
    fn place_pos(&self, p: Place) -> (f32, f32) {
        match p {
            Place::City(n) => self.map.cities.iter().find(|c| c.name == n).map_or((W as f32 / 2.0, H as f32 / 2.0), |c| (c.x, c.y)),
            Place::LL(lon, lat) => ll(lon, lat),
        }
    }

    pub(crate) fn run_history(&mut self) {
        let today = self.day();
        self.mods.retain(|m| m.until > today);
        self.halts.retain(|h| h.4 > today);
        self.end_flood(today);
        while self.next_event < HISTORY.len() {
            let e = &HISTORY[self.next_event];
            if date_to_day(e.date.0, e.date.1, e.date.2) > today {
                break;
            }
            self.next_event += 1;
            self.fire(e);
        }
    }

    pub(crate) fn chronicle_entry(&self, e: &'static HistEvent, happened: bool) -> Chronicle {
        let date = if e.month_only { format!("{}年{}月", e.date.0, e.date.1) } else { format!("{}年{}月{}日", e.date.0, e.date.1, e.date.2) };
        Chronicle { date, title: e.title, text: e.text, happened, side: e.side, pos: e.place.map(|p| self.place_pos(p)) }
    }

    fn fire(&mut self, e: &'static HistEvent) {
        let happens = e.requires.is_none_or(|(c, s)| self.map.cities.iter().any(|k| k.name == c && k.owner == s));
        let c = self.chronicle_entry(e, happens);
        self.chronicle.push(c);
        if !happens {
            return;
        }
        for eff in e.effects {
            self.apply(eff);
        }
    }

    fn apply(&mut self, eff: &Eff) {
        let today = self.day();
        match *eff {
            Eff::Funds(s, a) => self.sides[s as usize].funds += a,
            Eff::Shells(s, a) => self.sides[s as usize].shells += a,
            Eff::Spawn(s, k, n, at) => {
                let p = self.place_pos(at);
                for i in 0..n {
                    let a = i as f32 * 1.9;
                    let r = 1.5 + i as f32 * 0.35;
                    self.spawn(k, s, p.0 + a.cos() * r, p.1 + a.sin() * r);
                }
            }
            Eff::Mod(s, kind, zone, days) => {
                let zone = zone.map(|(p, r)| {
                    let q = self.place_pos(p);
                    (q.0, q.1, r)
                });
                self.mods.push(Mod { side: s, kind, zone, until: today + days });
            }
            Eff::EndUplink(s) => self.mods.retain(|m| !(m.side == s && m.kind == ModKind::Uplink)),
            Eff::Axes(s, names, days) => {
                let forced: Vec<usize> = names.iter().filter_map(|n| self.map.cities.iter().position(|c| c.name == *n)).collect();
                let plan = &mut self.sides[s as usize].plan;
                plan.forced = forced;
                plan.forced_until = today + days;
                plan.axis_t = 0.0;
            }
            Eff::Halt(s, at, r, days) => {
                let p = self.place_pos(at);
                self.halts.push((s, p.0, p.1, r, today + days));
            }
            Eff::Retreat(s, at, r, to) => {
                let (p, q) = (self.place_pos(at), self.place_pos(to));
                let ids: Vec<u32> = self.units.iter().filter(|u| u.side == s && u.auto && !u.dead && def(u.kind).pass != Pass::Air && (u.x - p.0).powi(2) + (u.y - p.1).powi(2) < r * r).map(|u| u.id).collect();
                for id in ids {
                    let (ox, oy) = (self.rng.range(-6.0, 6.0), self.rng.range(-6.0, 6.0));
                    self.command_one(id, Cmd::Move, q.0 + ox, q.1 + oy);
                    if let Some(i) = self.unit_index(id) {
                        self.units[i].ai_t = 25.0;
                    }
                }
            }
            Eff::Revert(to, at, r) => self.revert_area(to, at, r),
            Eff::BridgeHit(name, frac) => {
                let Some(b) = self.map.bridges.iter().find(|b| b.name == Some(name) && b.hp > 0.0) else { return };
                let (x, y, a) = (b.x, b.y, b.max_hp * frac);
                self.damage_bridges(x, y, 2.0, a);
            }
            Eff::ShipHit(s, frac) => {
                let ship = self.units.iter().filter(|u| u.side == s && u.kind == UK::Frigate && !u.dead).max_by(|a, b| a.hp.partial_cmp(&b.hp).unwrap_or(std::cmp::Ordering::Equal)).map(|u| u.id);
                if let Some(i) = ship.and_then(|id| self.unit_index(id)) {
                    let hp = def(UK::Frigate).hp * frac * 3.0;
                    self.damage_unit(i, hp, 1 - s);
                }
            }
            Eff::AirfieldRaid(s, jets, bld) => {
                let ids: Vec<u32> = self.units.iter().filter(|u| u.side == s && matches!(u.kind, UK::Jet | UK::ArmedUav | UK::Heli) && !u.dead).map(|u| u.id).collect();
                for id in ids {
                    if self.rng.chance(jets) {
                        if let Some(i) = self.unit_index(id) {
                            self.damage_unit(i, 1e6, 1 - s);
                        }
                    }
                }
                let fields: Vec<(u32, f32)> = self.blds.iter().filter(|b| b.side == s && b.kind == BK::Airfield).map(|b| (b.id, b.max_hp * bld)).collect();
                for (id, a) in fields {
                    self.damage_bld(id, a, 1 - s);
                }
            }
            Eff::Flood(days) => self.start_flood(today + days),
            Eff::EarlyMud(days) => {
                self.mud_until = today + days;
                self.update_season();
            }
            Eff::RailCut(at, r, secs) => {
                let p = self.place_pos(at);
                self.cut_rail(p.0, p.1, r, secs);
            }
            Eff::FogBank(secs) => {
                self.weather = Weather::Fog;
                self.weather_t = secs;
            }
        }
    }

    /// もともと to の土地を取り戻す。プレイヤーが直接指揮する部隊のいる場所は残る
    fn revert_area(&mut self, to: u8, at: Place, r: f32) {
        let c = self.place_pos(at);
        let holders: Vec<(f32, f32)> = self.units.iter().filter(|u| u.side != to && !u.auto && !u.dead && def(u.kind).cap_r > 0.0).map(|u| (u.x, u.y)).collect();
        let mut changed = false;
        let ri = r as i32 + 1;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let (x, y) = (c.0 as i32 + dx, c.1 as i32 + dy);
                if !in_map(x, y) || ((dx * dx + dy * dy) as f32) > r * r {
                    continue;
                }
                let i = tidx(x, y);
                if self.owner[i] >= 0 && self.owner[i] != to as i8 && self.map.start_owner[i] == to as i8 && !holders.iter().any(|h| (h.0 - x as f32).abs() < 8.0 && (h.1 - y as f32).abs() < 8.0) {
                    self.owner[i] = to as i8;
                    self.cap[i] = 0;
                    changed = true;
                }
            }
        }
        if changed {
            self.territory_changed();
        }
    }

    /// カホヴァ・ダムの決壊：下流の低地が水に沈み、車両が渡れなくなる
    fn start_flood(&mut self, until: u32) {
        let (x0, y0) = ll(32.2, 47.75);
        let (x1, y1) = ll(35.2, 46.3);
        let rivers: Vec<(i32, i32)> = (y0 as i32..=y1 as i32).flat_map(|y| (x0 as i32..=x1 as i32).map(move |x| (x, y))).filter(|&(x, y)| self.map.t(x, y) == Terrain::River).collect();
        let mut tiles = Vec::new();
        for (x, y) in rivers {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if in_map(x + dx, y + dy) {
                        let i = tidx(x + dx, y + dy);
                        if self.map.terrain[i].open() && !self.map.rail[i] {
                            tiles.push((i as u32, self.map.terrain[i]));
                            self.map.terrain[i] = Terrain::River;
                        }
                    }
                }
            }
        }
        for t in &tiles {
            self.repaint.push(t.0);
        }
        self.flooded = tiles;
        self.flood_until = until;
        self.clear_path_cache();
    }

    fn end_flood(&mut self, today: u32) {
        if self.flooded.is_empty() || today < self.flood_until {
            return;
        }
        for (i, t) in std::mem::take(&mut self.flooded) {
            self.map.terrain[i as usize] = t;
            self.repaint.push(i);
        }
        self.clear_path_cache();
    }
}

#[cfg(test)]
impl World {
    pub(crate) fn run_history_for_test(&mut self) {
        self.run_history();
    }
    pub(crate) fn repair_rails_for_test(&mut self) {
        self.repair_rails();
    }
}
