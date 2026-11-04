//! yyempire スタイルのスプライト・タイトル画像・都市・マップ画像の管理と描画

use crate::gfx::{Col, Gfx};
use yydrone_core::defs::{BK, UK, UK_COUNT};
use yydrone_core::command::GENERAL_COUNT;

const PORTRAIT_COUNT:usize=(GENERAL_COUNT+1)*2;

fn portrait_pixels(index:usize)-> &'static [u8] {
    let images:[&[u8];PORTRAIT_COUNT]=[
        include_bytes!("../../../assets/command/general_ua_0.rgba"),
        include_bytes!("../../../assets/command/general_ua_1.rgba"),
        include_bytes!("../../../assets/command/general_ua_2.rgba"),
        include_bytes!("../../../assets/command/general_ua_3.rgba"),
        include_bytes!("../../../assets/command/general_ua_4.rgba"),
        include_bytes!("../../../assets/command/general_ua_5.rgba"),
        include_bytes!("../../../assets/command/general_ua_6.rgba"),
        include_bytes!("../../../assets/command/general_ua_7.rgba"),
        include_bytes!("../../../assets/command/general_ua_8.rgba"),
        include_bytes!("../../../assets/command/staff_ua.rgba"),
        include_bytes!("../../../assets/command/general_ru_0.rgba"),
        include_bytes!("../../../assets/command/general_ru_1.rgba"),
        include_bytes!("../../../assets/command/general_ru_2.rgba"),
        include_bytes!("../../../assets/command/general_ru_3.rgba"),
        include_bytes!("../../../assets/command/general_ru_4.rgba"),
        include_bytes!("../../../assets/command/general_ru_5.rgba"),
        include_bytes!("../../../assets/command/general_ru_6.rgba"),
        include_bytes!("../../../assets/command/general_ru_7.rgba"),
        include_bytes!("../../../assets/command/general_ru_8.rgba"),
        include_bytes!("../../../assets/command/staff_ru.rgba"),
    ];images[index]
}

pub const TITLE_W: u32 = 800;
pub const TITLE_H: u32 = 450;
const RAW_TITLE: &[u8] = include_bytes!("../../../assets/title.rgba");

pub const UNIT_SZ: u32 = 160;

macro_rules! unit_asset {
    ($name:literal) => { include_bytes!(concat!("../../../assets/units/", $name, ".rgba")) };
}

#[cfg(test)]
mod asset_tests {
    use super::*;
    use yydrone_core::defs::{BK_ALL, DEFS};

    #[test]
    fn twenty_command_portraits_are_distinct_and_correctly_sized() {
        assert_eq!(PORTRAIT_COUNT,20);
        for index in 0..PORTRAIT_COUNT {
            let pixels=portrait_pixels(index);assert_eq!(pixels.len(),160*160*4);
            for previous in 0..index {assert!(pixels!=portrait_pixels(previous),"portrait {index} duplicates {previous}");}
        }
    }

    #[test]
    fn weapon_categories_have_distinct_pixels_and_valid_dimensions() {
        for side in 0..2 {
            for (index, def) in DEFS.iter().enumerate() {
                let pixels = unit_pixels(def.kind, side);
                assert_eq!(pixels.len(), (UNIT_SZ * UNIT_SZ * 4) as usize);
                assert!(pixels.chunks_exact(4).any(|p| p[3] == 0), "{:?}: no transparency", def.kind);
                assert!(pixels.chunks_exact(4).any(|p| p[3] == 255), "{:?}: empty subject", def.kind);
                for other in &DEFS[..index] {
                    assert!(pixels != unit_pixels(other.kind, side), "side {side}: {:?} and {:?} share an image", def.kind, other.kind);
                }
            }
        }
    }

    #[test]
    fn faction_specific_models_have_different_pixels() {
        for kind in [UK::Atgm, UK::Ifv, UK::Tank, UK::Howitzer, UK::Mlrs, UK::Shorad, UK::Sam, UK::Ew, UK::ReconDrone, UK::BomberDrone, UK::ArmedUav, UK::LongDrone, UK::Heli, UK::Jet, UK::Cruise] {
            assert!(unit_pixels(kind, 0) != unit_pixels(kind, 1), "{kind:?}: faction image reused");
        }
        for kind in [UK::Infantry, UK::Engineer, UK::FpvTeam, UK::Fpv] {
            assert_eq!(unit_pixels(kind, 0), unit_pixels(kind, 1), "{kind:?}: generic gear should be shared");
        }
    }

    #[test]
    fn every_facility_has_its_own_image() {
        for (index, kind) in BK_ALL.iter().enumerate() {
            let pixels = building_pixels(*kind);
            assert_eq!(pixels.len(), (BLD_SZ * BLD_SZ * 4) as usize);
            for other in &BK_ALL[..index] {
                assert!(pixels != building_pixels(*other), "{kind:?} and {other:?} share an image");
            }
        }
    }
}

/// 型式が異なる兵器は陣営別。一般歩兵・工兵・FPV機材は共通。
fn unit_pixels(k: UK, side: u8) -> &'static [u8] {
    let pair: [&[u8]; 2] = match k {
        UK::Infantry => [unit_asset!("infantry"), unit_asset!("infantry")],
        UK::Engineer => [unit_asset!("engineer"), unit_asset!("engineer")],
        UK::Atgm => [unit_asset!("atgm_ua"), unit_asset!("atgm")],
        UK::FpvTeam => [unit_asset!("fpv_team"), unit_asset!("fpv_team")],
        UK::Ifv => [unit_asset!("ifv"), unit_asset!("ifv_ru")],
        UK::Tank => [unit_asset!("tank_ua"), unit_asset!("tank")],
        UK::Howitzer => [unit_asset!("howitzer"), unit_asset!("howitzer_ru")],
        UK::Mlrs => [unit_asset!("mlrs_ua"), unit_asset!("mlrs_ru")],
        UK::Coastal => [unit_asset!("coastal"), unit_asset!("coastal")],
        UK::Shorad => [unit_asset!("shorad_ua"), unit_asset!("shorad_ru")],
        UK::Sam => [unit_asset!("sam_ua"), unit_asset!("sam_ru")],
        UK::Ew => [unit_asset!("ew_ua"), unit_asset!("ew_ru")],
        UK::ReconDrone => [unit_asset!("recon_drone_ua"), unit_asset!("recon_drone_ru")],
        UK::BomberDrone => [unit_asset!("bomber_drone_ua"), unit_asset!("bomber_drone_ru")],
        UK::ArmedUav => [unit_asset!("armed_uav"), unit_asset!("armed_uav_ru")],
        UK::LongDrone => [unit_asset!("long_drone_ua"), unit_asset!("long_drone_ru")],
        UK::Heli => [unit_asset!("heli_ua"), unit_asset!("heli")],
        UK::Jet => [unit_asset!("jet_ua"), unit_asset!("jet_ru")],
        // 片陣営専用兵器は、利用不可の陣営に別素材を用意しない。
        UK::Frigate => [unit_asset!("frigate"), unit_asset!("frigate")],
        UK::SeaDrone => [unit_asset!("seadrone"), unit_asset!("seadrone")],
        UK::Fpv => [unit_asset!("fpv_drone"), unit_asset!("fpv_drone")],
        UK::Cruise => [unit_asset!("cruise_ua"), unit_asset!("cruise_ru")],
    };
    pair[side as usize]
}

pub const BLD_SZ: u32 = 192;
const RAW_BLD_BASE: &[u8] = include_bytes!("../../../assets/map/military_base.rgba");
const RAW_BLD_AIRFIELD: &[u8] = include_bytes!("../../../assets/map/airfield.rgba");
const RAW_BLD_WORKSHOP: &[u8] = include_bytes!("../../../assets/map/drone_workshop.rgba");
const RAW_BLD_FACTORY: &[u8] = include_bytes!("../../../assets/map/shell_factory.rgba");
const RAW_BLD_DEPOT: &[u8] = include_bytes!("../../../assets/map/supply_depot.rgba");
const RAW_BLD_PORT: &[u8] = include_bytes!("../../../assets/map/military_port.rgba");

fn building_pixels(k: BK) -> &'static [u8] {
    match k {
        BK::Base => RAW_BLD_BASE,
        BK::Airfield => RAW_BLD_AIRFIELD,
        BK::Workshop => RAW_BLD_WORKSHOP,
        BK::Factory => RAW_BLD_FACTORY,
        BK::Depot => RAW_BLD_DEPOT,
        BK::Port => RAW_BLD_PORT,
    }
}

const RAW_CITY_KYIV: &[u8] = include_bytes!("../../../assets/map/city_kyiv.rgba");
const RAW_CITY_KHARKIV: &[u8] = include_bytes!("../../../assets/map/city_kharkiv.rgba");
const RAW_CITY_ODESA: &[u8] = include_bytes!("../../../assets/map/city_odesa.rgba");
const RAW_CITY_DNIPRO: &[u8] = include_bytes!("../../../assets/map/city_dnipro.rgba");
const RAW_CITY_LVIV: &[u8] = include_bytes!("../../../assets/map/city_lviv.rgba");
const RAW_CITY_ROSTOV: &[u8] = include_bytes!("../../../assets/map/city_rostov.rgba");
const RAW_CITY_SEVASTOPOL: &[u8] = include_bytes!("../../../assets/map/city_sevastopol.rgba");
const RAW_CITY_MARIUPOL: &[u8] = include_bytes!("../../../assets/map/city_mariupol.rgba");
const RAW_CITY_DONETSK: &[u8] = include_bytes!("../../../assets/map/city_donetsk.rgba");
const RAW_CITY_BAKHMUT: &[u8] = include_bytes!("../../../assets/map/city_bakhmut.rgba");

const RAW_CITY_CAPITAL: &[u8] = include_bytes!("../../../assets/map/city_capital.rgba");
const RAW_CITY_TOWN: &[u8] = include_bytes!("../../../assets/map/city_town.rgba");
const RAW_CITY_PORT: &[u8] = include_bytes!("../../../assets/map/city_port.rgba");
const RAW_CITY_INDUSTRIAL: &[u8] = include_bytes!("../../../assets/map/city_industrial.rgba");

fn bytes_to_u32(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| c[0] as u32 | ((c[1] as u32) << 8) | ((c[2] as u32) << 16) | ((c[3] as u32) << 24))
        .collect()
}

#[derive(Default)]
pub struct Art {
    pub title_tex: Option<u32>,
    unit_tex: [[Option<u32>; UK_COUNT]; 2],
    bld_tex: [Option<u32>; 6],
    portrait_tex: [Option<u32>; PORTRAIT_COUNT],

    pub city_kyiv: Option<u32>,
    pub city_kharkiv: Option<u32>,
    pub city_odesa: Option<u32>,
    pub city_dnipro: Option<u32>,
    pub city_lviv: Option<u32>,
    pub city_rostov: Option<u32>,
    pub city_sevastopol: Option<u32>,
    pub city_mariupol: Option<u32>,
    pub city_donetsk: Option<u32>,
    pub city_bakhmut: Option<u32>,
    pub city_capital: Option<u32>,
    pub city_town: Option<u32>,
    pub city_port: Option<u32>,
    pub city_industrial: Option<u32>,
}

impl Art {
    pub fn portrait_texture(&mut self,g:&mut dyn Gfx,index:usize)->u32 {
        if let Some(id)=self.portrait_tex[index]{return id;}
        let id=g.create_texture(160,160,&bytes_to_u32(portrait_pixels(index)));self.portrait_tex[index]=Some(id);id
    }
    pub fn ensure_title(&mut self, g: &mut dyn Gfx) -> u32 {
        if let Some(id) = self.title_tex {
            return id;
        }
        let data = bytes_to_u32(RAW_TITLE);
        let id = g.create_texture(TITLE_W, TITLE_H, &data);
        self.title_tex = Some(id);
        id
    }

    pub fn unit_texture(&mut self, g: &mut dyn Gfx, k: UK, side: u8) -> Option<u32> {
        let raw = unit_pixels(k, side);
        let opt = &mut self.unit_tex[side as usize][k as usize];
        if let Some(id) = *opt {
            Some(id)
        } else {
            let data = bytes_to_u32(raw);
            let id = g.create_texture(UNIT_SZ, UNIT_SZ, &data);
            *opt = Some(id);
            Some(id)
        }
    }

    pub fn bld_texture(&mut self, g: &mut dyn Gfx, k: BK) -> Option<u32> {
        let raw = building_pixels(k);
        let opt = &mut self.bld_tex[k as usize];
        if let Some(id) = *opt {
            Some(id)
        } else {
            let data = bytes_to_u32(raw);
            let id = g.create_texture(BLD_SZ, BLD_SZ, &data);
            *opt = Some(id);
            Some(id)
        }
    }

    pub fn city_texture(&mut self, g: &mut dyn Gfx, name: &str, value: u8, port: bool) -> u32 {
        let (opt, raw) = match name {
            "キエヴァ" => (&mut self.city_kyiv, RAW_CITY_KYIV),
            "ハルキア" => (&mut self.city_kharkiv, RAW_CITY_KHARKIV),
            "オデスカ" => (&mut self.city_odesa, RAW_CITY_ODESA),
            "ドニプロヴァ" | "ザポリア" => (&mut self.city_dnipro, RAW_CITY_DNIPRO),
            "リヴィア" => (&mut self.city_lviv, RAW_CITY_LVIV),
            "ロストヴァ" => (&mut self.city_rostov, RAW_CITY_ROSTOV),
            "セヴァストラ" | "ノヴォロス" => (&mut self.city_sevastopol, RAW_CITY_SEVASTOPOL),
            "マリウポラ" => (&mut self.city_mariupol, RAW_CITY_MARIUPOL),
            "ドネスカ" | "ルガンスカ" => (&mut self.city_donetsk, RAW_CITY_DONETSK),
            "バフムーラ" | "ポクロヴァ" | "イジューマ" | "クラマト" => {
                (&mut self.city_bakhmut, RAW_CITY_BAKHMUT)
            }
            _ => {
                if value >= 5 {
                    (&mut self.city_capital, RAW_CITY_CAPITAL)
                } else if port {
                    (&mut self.city_port, RAW_CITY_PORT)
                } else if value >= 4 {
                    (&mut self.city_industrial, RAW_CITY_INDUSTRIAL)
                } else {
                    (&mut self.city_town, RAW_CITY_TOWN)
                }
            }
        };
        if let Some(id) = *opt {
            id
        } else {
            let data = bytes_to_u32(raw);
            let id = g.create_texture(BLD_SZ, BLD_SZ, &data);
            *opt = Some(id);
            id
        }
    }

    /// UI用のユニットスプライト描画
    pub fn draw_unit_sprite(&mut self, g: &mut dyn Gfx, k: UK, side: u8, x: f32, y: f32, sz: f32, col: Col) -> bool {
        if let Some(tex) = self.unit_texture(g, k, side) {
            let half = sz / 2.0;
            g.draw_textured(tex, x - half, y - half, x + half, y + half, col, true);
            true
        } else {
            false
        }
    }

    /// UI用の建物スプライト描画
    pub fn draw_bld_sprite(&mut self, g: &mut dyn Gfx, k: BK, x: f32, y: f32, sz: f32, col: Col) -> bool {
        if let Some(tex) = self.bld_texture(g, k) {
            let half = sz / 2.0;
            g.draw_textured(tex, x - half, y - half, x + half, y + half, col, true);
            true
        } else {
            false
        }
    }
}
