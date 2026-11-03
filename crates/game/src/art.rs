//! yyempire スタイルのスプライト・タイトル画像・都市・マップ画像の管理と描画

use crate::gfx::{Col, Gfx};
use yydrone_core::defs::{BK, UK};

pub const TITLE_W: u32 = 800;
pub const TITLE_H: u32 = 450;
const RAW_TITLE: &[u8] = include_bytes!("../../../assets/title.rgba");

pub const UNIT_SZ: u32 = 160;
const RAW_UNIT_TANK: &[u8] = include_bytes!("../../../assets/units/tank.rgba");
const RAW_UNIT_IFV: &[u8] = include_bytes!("../../../assets/units/ifv.rgba");
const RAW_UNIT_HOWITZER: &[u8] = include_bytes!("../../../assets/units/howitzer.rgba");
const RAW_UNIT_MLRS: &[u8] = include_bytes!("../../../assets/units/mlrs.rgba");
const RAW_UNIT_SAM: &[u8] = include_bytes!("../../../assets/units/sam.rgba");
const RAW_UNIT_INFANTRY: &[u8] = include_bytes!("../../../assets/units/infantry.rgba");
const RAW_UNIT_ATGM: &[u8] = include_bytes!("../../../assets/units/atgm.rgba");
const RAW_UNIT_FPV_TEAM: &[u8] = include_bytes!("../../../assets/units/fpv_team.rgba");
const RAW_UNIT_ENGINEER: &[u8] = include_bytes!("../../../assets/units/engineer.rgba");
const RAW_UNIT_FPV: &[u8] = include_bytes!("../../../assets/units/fpv_drone.rgba");
const RAW_UNIT_UAV: &[u8] = include_bytes!("../../../assets/units/armed_uav.rgba");
const RAW_UNIT_BOMBER: &[u8] = include_bytes!("../../../assets/units/bomber_drone.rgba");
const RAW_UNIT_HELI: &[u8] = include_bytes!("../../../assets/units/heli.rgba");
const RAW_UNIT_JET: &[u8] = include_bytes!("../../../assets/units/jet.rgba");
const RAW_UNIT_FRIGATE: &[u8] = include_bytes!("../../../assets/units/frigate.rgba");
const RAW_UNIT_SEADRONE: &[u8] = include_bytes!("../../../assets/units/seadrone.rgba");

pub const BLD_SZ: u32 = 192;
const RAW_BLD_BASE: &[u8] = include_bytes!("../../../assets/map/military_base.rgba");
const RAW_BLD_AIRFIELD: &[u8] = include_bytes!("../../../assets/map/airfield.rgba");
const RAW_BLD_WORKSHOP: &[u8] = include_bytes!("../../../assets/map/drone_workshop.rgba");
const RAW_BLD_TRENCH: &[u8] = include_bytes!("../../../assets/map/trench_fort.rgba");

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
    pub unit_tank: Option<u32>,
    pub unit_ifv: Option<u32>,
    pub unit_howitzer: Option<u32>,
    pub unit_mlrs: Option<u32>,
    pub unit_sam: Option<u32>,
    pub unit_infantry: Option<u32>,
    pub unit_atgm: Option<u32>,
    pub unit_fpv_team: Option<u32>,
    pub unit_engineer: Option<u32>,
    pub unit_fpv: Option<u32>,
    pub unit_uav: Option<u32>,
    pub unit_bomber: Option<u32>,
    pub unit_heli: Option<u32>,
    pub unit_jet: Option<u32>,
    pub unit_frigate: Option<u32>,
    pub unit_seadrone: Option<u32>,

    pub bld_base: Option<u32>,
    pub bld_airfield: Option<u32>,
    pub bld_workshop: Option<u32>,
    pub bld_trench: Option<u32>,

    pub city_capital: Option<u32>,
    pub city_town: Option<u32>,
    pub city_port: Option<u32>,
    pub city_industrial: Option<u32>,
}

impl Art {
    pub fn ensure_title(&mut self, g: &mut dyn Gfx) -> u32 {
        if let Some(id) = self.title_tex {
            return id;
        }
        let data = bytes_to_u32(RAW_TITLE);
        let id = g.create_texture(TITLE_W, TITLE_H, &data);
        self.title_tex = Some(id);
        id
    }

    pub fn unit_texture(&mut self, g: &mut dyn Gfx, k: UK) -> Option<u32> {
        let (opt, raw) = match k {
            UK::Tank => (&mut self.unit_tank, RAW_UNIT_TANK),
            UK::Ifv => (&mut self.unit_ifv, RAW_UNIT_IFV),
            UK::Howitzer => (&mut self.unit_howitzer, RAW_UNIT_HOWITZER),
            UK::Mlrs | UK::Coastal => (&mut self.unit_mlrs, RAW_UNIT_MLRS),
            UK::Sam | UK::Shorad | UK::Ew => (&mut self.unit_sam, RAW_UNIT_SAM),
            UK::Infantry => (&mut self.unit_infantry, RAW_UNIT_INFANTRY),
            UK::Atgm => (&mut self.unit_atgm, RAW_UNIT_ATGM),
            UK::FpvTeam => (&mut self.unit_fpv_team, RAW_UNIT_FPV_TEAM),
            UK::Engineer => (&mut self.unit_engineer, RAW_UNIT_ENGINEER),
            UK::Fpv => (&mut self.unit_fpv, RAW_UNIT_FPV),
            UK::ArmedUav | UK::ReconDrone | UK::LongDrone | UK::Cruise => (&mut self.unit_uav, RAW_UNIT_UAV),
            UK::BomberDrone => (&mut self.unit_bomber, RAW_UNIT_BOMBER),
            UK::Heli => (&mut self.unit_heli, RAW_UNIT_HELI),
            UK::Jet => (&mut self.unit_jet, RAW_UNIT_JET),
            UK::Frigate => (&mut self.unit_frigate, RAW_UNIT_FRIGATE),
            UK::SeaDrone => (&mut self.unit_seadrone, RAW_UNIT_SEADRONE),
        };
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
        let (opt, raw) = match k {
            BK::Base => (&mut self.bld_base, RAW_BLD_BASE),
            BK::Airfield => (&mut self.bld_airfield, RAW_BLD_AIRFIELD),
            BK::Workshop | BK::Factory => (&mut self.bld_workshop, RAW_BLD_WORKSHOP),
            BK::Depot | BK::Port => (&mut self.bld_trench, RAW_BLD_TRENCH),
        };
        if let Some(id) = *opt {
            Some(id)
        } else {
            let data = bytes_to_u32(raw);
            let id = g.create_texture(BLD_SZ, BLD_SZ, &data);
            *opt = Some(id);
            Some(id)
        }
    }

    pub fn city_texture(&mut self, g: &mut dyn Gfx, value: u8, port: bool) -> u32 {
        let (opt, raw) = if value >= 5 {
            (&mut self.city_capital, RAW_CITY_CAPITAL)
        } else if port {
            (&mut self.city_port, RAW_CITY_PORT)
        } else if value >= 4 {
            (&mut self.city_industrial, RAW_CITY_INDUSTRIAL)
        } else {
            (&mut self.city_town, RAW_CITY_TOWN)
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
    pub fn draw_unit_sprite(&mut self, g: &mut dyn Gfx, k: UK, x: f32, y: f32, sz: f32, col: Col) -> bool {
        if let Some(tex) = self.unit_texture(g, k) {
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
