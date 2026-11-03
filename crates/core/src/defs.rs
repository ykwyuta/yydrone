//! 兵器・建物・支援の定義（数値はすべてここに集める）

use crate::geo::Pass;

pub const SIDE_NAMES: [&str; 2] = ["ウクラヴィア", "ロスコヴィア"];
pub const SIDE_FULL: [&str; 2] = ["ウクラヴィア共和国", "ロスコヴィア連邦"];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum UK {
    Infantry,
    Engineer,
    Atgm,
    FpvTeam,
    Ifv,
    Tank,
    Howitzer,
    Mlrs,
    Coastal,
    Shorad,
    Sam,
    Ew,
    ReconDrone,
    BomberDrone,
    ArmedUav,
    LongDrone,
    Heli,
    Jet,
    Frigate,
    SeaDrone,
    /// FPV 班が飛ばす自爆ドローン（直接は作れない）
    Fpv,
    /// 巡航ミサイル（直接は作れない）
    Cruise,
}
pub const UK_COUNT: usize = 22;
pub const BUYABLE: [UK; 20] = [
    UK::Infantry, UK::Engineer, UK::Atgm, UK::FpvTeam, UK::Ifv, UK::Tank, UK::Howitzer, UK::Mlrs, UK::Coastal, UK::Shorad, UK::Sam, UK::Ew, UK::ReconDrone, UK::BomberDrone, UK::ArmedUav, UK::LongDrone, UK::Heli, UK::Jet, UK::Frigate, UK::SeaDrone,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Armor {
    Soft,
    Light,
    Heavy,
    Drone,
    Air,
    Ship,
    Bld,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Proj {
    None,
    Direct,
    Shell(f32),
    Missile(f32),
    Bomb(f32),
    SpawnFpv,
    SpawnCruise,
}

#[derive(Clone, Copy)]
pub struct Weapon {
    pub range: f32,
    pub min_range: f32,
    pub reload: f32,
    pub dmg: f32,
    pub splash: f32,
    pub proj: Proj,
    /// 砲弾の消費（1発ごと）
    pub shells: f32,
    /// 対象ごとの威力倍率 [軟目標, 軽装甲, 重装甲, ドローン, 航空機, 艦船, 建物]（0 は狙えない）
    pub vs: [f32; 7],
}
const NO_WEAPON: Weapon = Weapon { range: 0.0, min_range: 0.0, reload: 1.0, dmg: 0.0, splash: 0.0, proj: Proj::None, shells: 0.0, vs: [0.0; 7] };

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Ground,
    Fires,
    Drone,
    Air,
    Sea,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prod {
    Base,
    Workshop,
    Airfield,
    Port,
    None,
}

#[derive(Clone, Copy)]
pub struct UDef {
    pub kind: UK,
    /// 型式名（ウクラヴィア, ロスコヴィア）
    pub names: [&'static str; 2],
    pub short: &'static str,
    pub desc: &'static str,
    pub pass: Pass,
    pub armor: Armor,
    pub hp: f32,
    pub speed: f32,
    pub vision: f32,
    pub cost: f32,
    pub time: f32,
    pub tab: Tab,
    pub prod: Prod,
    pub weapon: Weapon,
    /// 支配を広げる半径・強さ（0 は広げない）
    pub cap_r: f32,
    pub cap_s: f32,
    /// 電波妨害の半径
    pub jam: f32,
    /// 滞空できる秒数（0 は無制限）
    pub endurance: f32,
    /// 当たると爆発して消える（爆発の威力は weapon.dmg）
    pub suicide: bool,
    /// どちらの陣営が使えるか
    pub sides: [bool; 2],
}

const BASE: UDef = UDef {
    kind: UK::Infantry,
    names: ["", ""],
    short: "",
    desc: "",
    pass: Pass::Veh,
    armor: Armor::Light,
    hp: 100.0,
    speed: 1.0,
    vision: 9.0,
    cost: 100.0,
    time: 10.0,
    tab: Tab::Ground,
    prod: Prod::Base,
    weapon: NO_WEAPON,
    cap_r: 0.0,
    cap_s: 0.0,
    jam: 0.0,
    endurance: 0.0,
    suicide: false,
    sides: [true, true],
};

pub const DEFS: [UDef; UK_COUNT] = [
    UDef {
        kind: UK::Infantry,
        names: ["機械化歩兵（領土防衛）", "自動車化狙撃兵"],
        short: "歩兵",
        desc: "安価で領土を奪う主力。止まると塹壕を掘り、塹壕内では被害が激減する。",
        pass: Pass::Inf,
        armor: Armor::Soft,
        hp: 70.0,
        speed: 0.6,
        vision: 8.0,
        cost: 70.0,
        time: 6.0,
        weapon: Weapon { range: 4.0, reload: 1.0, dmg: 6.0, proj: Proj::Direct, vs: [1.0, 0.4, 0.08, 0.3, 0.15, 0.0, 0.25], ..NO_WEAPON },
        cap_r: 5.0,
        cap_s: 2.0,
        ..BASE
    },
    UDef {
        kind: UK::Engineer,
        names: ["工兵", "工兵"],
        short: "工兵",
        desc: "塹壕・地雷原・竜の歯・浮橋を作る。近くの敵地雷を除去する。",
        pass: Pass::Inf,
        armor: Armor::Soft,
        hp: 55.0,
        speed: 0.75,
        vision: 7.0,
        cost: 110.0,
        time: 8.0,
        weapon: Weapon { range: 3.0, reload: 1.2, dmg: 3.0, proj: Proj::Direct, vs: [1.0, 0.3, 0.05, 0.1, 0.0, 0.0, 0.2], ..NO_WEAPON },
        cap_r: 3.0,
        cap_s: 1.0,
        ..BASE
    },
    UDef {
        kind: UK::Atgm,
        names: ["対戦車班（ジャベリナ）", "対戦車班（コルネ）"],
        short: "対戦車",
        desc: "射程の長い対戦車ミサイルで装甲を撃ち抜く。歩兵には弱い。",
        pass: Pass::Inf,
        armor: Armor::Soft,
        hp: 45.0,
        speed: 0.55,
        vision: 9.0,
        cost: 170.0,
        time: 9.0,
        weapon: Weapon { range: 9.0, reload: 6.0, dmg: 95.0, proj: Proj::Missile(9.0), vs: [0.2, 1.0, 1.2, 0.0, 0.4, 0.0, 0.4], ..NO_WEAPON },
        cap_r: 3.0,
        cap_s: 1.0,
        ..BASE
    },
    UDef {
        kind: UK::FpvTeam,
        names: ["FPV 班", "FPV 班"],
        short: "FPV班",
        desc: "見えている敵に自爆ドローンを自動で飛ばす。電波妨害と対空砲に弱い。",
        pass: Pass::Inf,
        armor: Armor::Soft,
        hp: 40.0,
        speed: 0.6,
        vision: 10.0,
        cost: 190.0,
        time: 10.0,
        weapon: Weapon { range: 22.0, min_range: 2.0, reload: 7.0, dmg: 110.0, proj: Proj::SpawnFpv, vs: [0.5, 1.0, 1.0, 0.0, 0.0, 0.3, 0.3], ..NO_WEAPON },
        cap_r: 2.0,
        cap_s: 0.5,
        ..BASE
    },
    UDef {
        kind: UK::Ifv,
        names: ["歩兵戦闘車 ブラドリ", "歩兵戦闘車 BMX-3"],
        short: "IFV",
        desc: "機動力のある軽装甲。歩兵に強く、前線を素早く押し上げる。",
        armor: Armor::Light,
        hp: 170.0,
        speed: 1.25,
        vision: 9.0,
        cost: 210.0,
        time: 11.0,
        weapon: Weapon { range: 5.5, reload: 0.8, dmg: 13.0, proj: Proj::Direct, vs: [1.0, 0.8, 0.2, 0.5, 0.25, 0.0, 0.5], ..NO_WEAPON },
        cap_r: 4.0,
        cap_s: 2.0,
        ..BASE
    },
    UDef {
        kind: UK::Tank,
        names: ["主力戦車 レオパルト", "主力戦車 T-9M"],
        short: "戦車",
        desc: "重装甲と大火力で突破する。FPV と対戦車ミサイル、地雷に注意。",
        armor: Armor::Heavy,
        hp: 340.0,
        speed: 1.05,
        vision: 9.0,
        cost: 400.0,
        time: 16.0,
        weapon: Weapon { range: 7.0, reload: 3.0, dmg: 70.0, splash: 0.0, proj: Proj::Direct, vs: [0.7, 1.2, 1.0, 0.0, 0.1, 0.0, 1.0], ..NO_WEAPON },
        cap_r: 4.0,
        cap_s: 3.0,
        ..BASE
    },
    UDef {
        kind: UK::Howitzer,
        names: ["155mm 自走砲 パラディア", "152mm 自走砲 ムスタ"],
        short: "榴弾砲",
        desc: "見えている敵を遠距離から砲撃する。砲弾を消費し、撃つと位置が露見する。",
        tab: Tab::Fires,
        armor: Armor::Light,
        hp: 110.0,
        speed: 0.75,
        vision: 7.0,
        cost: 340.0,
        time: 14.0,
        weapon: Weapon { range: 24.0, min_range: 4.0, reload: 5.0, dmg: 60.0, splash: 1.8, proj: Proj::Shell(11.0), shells: 1.0, vs: [1.0, 0.8, 0.5, 0.0, 0.0, 0.5, 1.0] },
        cap_r: 0.0,
        ..BASE
    },
    UDef {
        kind: UK::Mlrs,
        names: ["高機動ロケット砲 ハイマル", "多連装ロケット トルナード"],
        short: "ロケット",
        desc: "超長射程の精密ロケット。橋・補給所・防空網を叩く。砲弾の消費が大きい。",
        tab: Tab::Fires,
        armor: Armor::Light,
        hp: 90.0,
        speed: 1.3,
        vision: 6.0,
        cost: 680.0,
        time: 22.0,
        weapon: Weapon { range: 50.0, min_range: 8.0, reload: 15.0, dmg: 150.0, splash: 1.4, proj: Proj::Missile(16.0), shells: 4.0, vs: [0.9, 1.0, 0.8, 0.0, 0.0, 0.8, 1.6] },
        ..BASE
    },
    UDef {
        kind: UK::Coastal,
        names: ["沿岸対艦ミサイル ネプトゥ", ""],
        short: "対艦",
        desc: "長射程の地対艦ミサイル。艦隊を沖へ追い払う。",
        tab: Tab::Fires,
        armor: Armor::Light,
        hp: 90.0,
        speed: 0.9,
        vision: 7.0,
        cost: 520.0,
        time: 18.0,
        weapon: Weapon { range: 60.0, reload: 16.0, dmg: 420.0, proj: Proj::Missile(14.0), vs: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0], ..NO_WEAPON },
        sides: [true, false],
        ..BASE
    },
    UDef {
        kind: UK::Shorad,
        names: ["自走対空砲 ゲパルド", "対空システム パンツィリ"],
        short: "対空砲",
        desc: "短射程でドローンを次々と撃ち落とす。ヘリにも有効。",
        tab: Tab::Fires,
        armor: Armor::Light,
        hp: 120.0,
        speed: 1.1,
        vision: 11.0,
        cost: 280.0,
        time: 12.0,
        weapon: Weapon { range: 9.0, reload: 0.35, dmg: 12.0, proj: Proj::Direct, vs: [0.2, 0.0, 0.0, 1.0, 0.7, 0.0, 0.0], ..NO_WEAPON },
        ..BASE
    },
    UDef {
        kind: UK::Sam,
        names: ["地対空ミサイル パトリオン", "地対空ミサイル S-4X"],
        short: "SAM",
        desc: "長射程の防空。航空機と巡航ミサイルを落とす。",
        tab: Tab::Fires,
        armor: Armor::Light,
        hp: 110.0,
        speed: 0.8,
        vision: 26.0,
        cost: 620.0,
        time: 20.0,
        weapon: Weapon { range: 30.0, reload: 6.0, dmg: 170.0, proj: Proj::Missile(20.0), vs: [0.0, 0.0, 0.0, 0.8, 1.0, 0.0, 0.0], ..NO_WEAPON },
        ..BASE
    },
    UDef {
        kind: UK::Ew,
        names: ["電子戦車 ブコヴェリ", "電子戦車 クラスーハ"],
        short: "電子戦",
        desc: "周囲の敵ドローンを妨害して墜落・迷走させる。武器は持たない。",
        tab: Tab::Fires,
        armor: Armor::Light,
        hp: 80.0,
        speed: 1.0,
        vision: 10.0,
        cost: 300.0,
        time: 13.0,
        jam: 11.0,
        ..BASE
    },
    UDef {
        kind: UK::ReconDrone,
        names: ["偵察ドローン レレカ", "偵察ドローン オルラン"],
        short: "偵察機",
        desc: "広い範囲の敵を、塹壕や森に隠れていても暴く。砲兵の目になる。",
        tab: Tab::Drone,
        prod: Prod::Workshop,
        pass: Pass::Air,
        armor: Armor::Drone,
        hp: 28.0,
        speed: 2.6,
        vision: 17.0,
        cost: 90.0,
        time: 6.0,
        endurance: 220.0,
        ..BASE
    },
    UDef {
        kind: UK::BomberDrone,
        names: ["爆撃ドローン バーバ", "爆撃ドローン モルニヤ"],
        short: "爆撃機",
        desc: "敵の真上まで飛び、繰り返し爆弾を落とす大型マルチコプター。",
        tab: Tab::Drone,
        prod: Prod::Workshop,
        pass: Pass::Air,
        armor: Armor::Drone,
        hp: 40.0,
        speed: 2.1,
        vision: 9.0,
        cost: 170.0,
        time: 9.0,
        endurance: 200.0,
        weapon: Weapon { range: 1.6, reload: 5.0, dmg: 80.0, splash: 1.0, proj: Proj::Bomb(8.0), vs: [1.0, 1.0, 0.9, 0.0, 0.0, 0.3, 0.8], ..NO_WEAPON },
        ..BASE
    },
    UDef {
        kind: UK::ArmedUav,
        names: ["武装 UAV バイラク", "武装 UAV オリオン"],
        short: "UAV",
        desc: "長時間飛び、精密ミサイルで車両を狙う。視界も広い。防空に弱い。",
        tab: Tab::Drone,
        prod: Prod::Airfield,
        pass: Pass::Air,
        armor: Armor::Air,
        hp: 90.0,
        speed: 2.6,
        vision: 15.0,
        cost: 480.0,
        time: 18.0,
        weapon: Weapon { range: 10.0, reload: 7.0, dmg: 110.0, proj: Proj::Missile(10.0), vs: [0.5, 1.0, 1.0, 0.0, 0.0, 0.5, 0.7], ..NO_WEAPON },
        ..BASE
    },
    UDef {
        kind: UK::LongDrone,
        names: ["長距離自爆機 リューテ", "長距離自爆機 シャヘラ"],
        short: "長距離",
        desc: "指定地点まで飛んで爆発する片道ドローン。後方の工場や補給所を狙う。",
        tab: Tab::Drone,
        prod: Prod::Workshop,
        pass: Pass::Air,
        armor: Armor::Drone,
        hp: 40.0,
        speed: 3.5,
        vision: 3.0,
        cost: 150.0,
        time: 7.0,
        suicide: true,
        weapon: Weapon { range: 0.8, reload: 1.0, dmg: 260.0, splash: 1.6, proj: Proj::None, vs: [0.8, 0.8, 0.5, 0.0, 0.0, 0.6, 1.2], ..NO_WEAPON },
        ..BASE
    },
    UDef {
        kind: UK::Heli,
        names: ["攻撃ヘリ ミーラ24", "攻撃ヘリ アリガートル"],
        short: "ヘリ",
        desc: "前線の機甲部隊を叩く速い航空戦力。対空砲と歩兵の対空火器に注意。",
        tab: Tab::Air,
        prod: Prod::Airfield,
        pass: Pass::Air,
        armor: Armor::Air,
        hp: 200.0,
        speed: 3.2,
        vision: 10.0,
        cost: 560.0,
        time: 18.0,
        weapon: Weapon { range: 8.0, reload: 3.5, dmg: 85.0, proj: Proj::Missile(11.0), vs: [0.6, 1.0, 1.1, 0.0, 0.0, 0.5, 0.6], ..NO_WEAPON },
        ..BASE
    },
    UDef {
        kind: UK::Jet,
        names: ["戦闘攻撃機 ファルク", "戦闘爆撃機 スホーナ34"],
        short: "戦闘機",
        desc: "飛行場から出撃し滑空爆弾を投下して帰還する。指定した敵を強襲する。",
        tab: Tab::Air,
        prod: Prod::Airfield,
        pass: Pass::Air,
        armor: Armor::Air,
        hp: 160.0,
        speed: 9.0,
        vision: 12.0,
        cost: 820.0,
        time: 24.0,
        weapon: Weapon { range: 13.0, reload: 2.0, dmg: 240.0, splash: 2.4, proj: Proj::Bomb(7.0), vs: [1.0, 1.0, 0.9, 0.0, 0.0, 0.6, 1.5], ..NO_WEAPON },
        ..BASE
    },
    UDef {
        kind: UK::Frigate,
        names: ["", "フリゲート アドミラル"],
        short: "艦艇",
        desc: "海上から巡航ミサイルを撃ち、対空射撃もする。水上ドローンに弱い。",
        tab: Tab::Sea,
        prod: Prod::Port,
        pass: Pass::Sea,
        armor: Armor::Ship,
        hp: 700.0,
        speed: 1.1,
        vision: 16.0,
        cost: 900.0,
        time: 30.0,
        weapon: Weapon { range: 80.0, min_range: 6.0, reload: 50.0, dmg: 1.0, proj: Proj::SpawnCruise, shells: 2.0, vs: [0.5, 0.5, 0.5, 0.0, 0.0, 0.5, 1.0], ..NO_WEAPON },
        sides: [false, true],
        ..BASE
    },
    UDef {
        kind: UK::SeaDrone,
        names: ["水上ドローン マゴラ", ""],
        short: "水上D",
        desc: "海を走る自爆艇。艦艇や海峡の橋を狙う。",
        tab: Tab::Sea,
        prod: Prod::Port,
        pass: Pass::Sea,
        armor: Armor::Light,
        hp: 40.0,
        speed: 2.8,
        vision: 7.0,
        cost: 180.0,
        time: 8.0,
        suicide: true,
        weapon: Weapon { range: 1.2, reload: 1.0, dmg: 380.0, splash: 1.2, proj: Proj::None, vs: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0], ..NO_WEAPON },
        sides: [true, false],
        ..BASE
    },
    UDef {
        kind: UK::Fpv,
        names: ["FPV", "FPV"],
        short: "FPV",
        tab: Tab::None,
        prod: Prod::None,
        pass: Pass::Air,
        armor: Armor::Drone,
        hp: 6.0,
        speed: 6.0,
        vision: 2.0,
        cost: 0.0,
        time: 0.0,
        endurance: 12.0,
        suicide: true,
        weapon: Weapon { range: 0.6, reload: 1.0, dmg: 110.0, splash: 0.5, proj: Proj::None, vs: [0.5, 1.0, 1.0, 0.0, 0.0, 0.3, 0.3], ..NO_WEAPON },
        ..BASE
    },
    UDef {
        kind: UK::Cruise,
        names: ["巡航ミサイル ストーマ", "巡航ミサイル カリバ"],
        short: "巡航",
        tab: Tab::None,
        prod: Prod::None,
        pass: Pass::Air,
        armor: Armor::Drone,
        hp: 60.0,
        speed: 9.0,
        vision: 2.0,
        cost: 0.0,
        time: 0.0,
        suicide: true,
        weapon: Weapon { range: 0.8, reload: 1.0, dmg: 300.0, splash: 2.4, proj: Proj::None, vs: [0.8, 0.8, 0.6, 0.0, 0.0, 0.8, 1.0], ..NO_WEAPON },
        ..BASE
    },
];

#[inline]
pub fn def(k: UK) -> &'static UDef {
    &DEFS[k as usize]
}

/// 装輪車両か（泥濘に特に弱い）
pub fn is_wheeled(k: UK) -> bool {
    matches!(k, UK::Mlrs | UK::Coastal | UK::Sam | UK::Ew)
}

/// 陣営ごとの値段（ウクラヴィアはドローン・対戦車が安く、ロスコヴィアは火砲・戦車が安い）
pub fn unit_cost(side: u8, k: UK) -> f32 {
    let c = def(k).cost;
    match (side, k) {
        (0, UK::FpvTeam | UK::ReconDrone | UK::BomberDrone | UK::ArmedUav | UK::LongDrone | UK::SeaDrone) => c * 0.8,
        (0, UK::Atgm | UK::Mlrs) => c * 0.85,
        (1, UK::Howitzer | UK::Tank | UK::Infantry | UK::Jet) => c * 0.92,
        _ => c,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum BK {
    Base,
    Airfield,
    Workshop,
    Factory,
    Depot,
    Port,
}
pub const BK_ALL: [BK; 6] = [BK::Base, BK::Airfield, BK::Workshop, BK::Factory, BK::Depot, BK::Port];

#[derive(Clone, Copy)]
pub struct BDef {
    pub name: &'static str,
    pub desc: &'static str,
    pub cost: f32,
    pub time: f32,
    pub hp: f32,
    pub vision: f32,
}

pub fn bdef(k: BK) -> BDef {
    match k {
        BK::Base => BDef { name: "軍事基地", desc: "地上部隊を編成する。周囲に補給が届く。", cost: 500.0, time: 18.0, hp: 1100.0, vision: 8.0 },
        BK::Airfield => BDef { name: "飛行場", desc: "ヘリ・戦闘機・武装 UAV を運用する。", cost: 700.0, time: 24.0, hp: 1000.0, vision: 8.0 },
        BK::Workshop => BDef { name: "ドローン工房", desc: "偵察・爆撃・長距離ドローンを作る。", cost: 300.0, time: 10.0, hp: 450.0, vision: 6.0 },
        BK::Factory => BDef { name: "砲弾工場", desc: "砲弾を増産し、資金も少し増える。", cost: 450.0, time: 18.0, hp: 800.0, vision: 5.0 },
        BK::Depot => BDef { name: "補給所", desc: "広い範囲に補給を届ける。切られると前線が弱る。", cost: 220.0, time: 8.0, hp: 500.0, vision: 6.0 },
        BK::Port => BDef { name: "軍港", desc: "海沿いに建てる。艦艇・水上ドローンを作る。", cost: 450.0, time: 20.0, hp: 700.0, vision: 8.0 },
    }
}

pub fn prod_building(p: Prod) -> Option<BK> {
    match p {
        Prod::Base => Some(BK::Base),
        Prod::Workshop => Some(BK::Workshop),
        Prod::Airfield => Some(BK::Airfield),
        Prod::Port => Some(BK::Port),
        Prod::None => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Power {
    Cruise,
    Satellite,
    Mobilize,
}
pub const POWERS: [Power; 3] = [Power::Cruise, Power::Satellite, Power::Mobilize];
pub struct PDef {
    pub name: &'static str,
    pub desc: &'static str,
    pub cost: f32,
    pub cooldown: f32,
}
pub fn pdef(p: Power) -> PDef {
    match p {
        Power::Cruise => PDef { name: "巡航ミサイル", desc: "指定地点へ巡航ミサイルを撃ち込む。SAM に迎撃されることがある。", cost: 320.0, cooldown: 50.0 },
        Power::Satellite => PDef { name: "衛星偵察", desc: "指定範囲を 15 秒間見通す。隠れた部隊も暴く。", cost: 90.0, cooldown: 40.0 },
        Power::Mobilize => PDef { name: "緊急動員", desc: "指定地点に最も近い自国の都市に歩兵 5 部隊を出す。", cost: 200.0, cooldown: 80.0 },
    }
}
