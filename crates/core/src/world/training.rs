//! 通常戦から独立した、再現可能なチュートリアル用訓練場。
use super::*;
use crate::command::{Doctrine, Region};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TrainingLesson {
    Welcome,
    Camera,
    Select,
    Move,
    Capture,
    Combat,
    Production,
    Construction,
    ReconProduction,
    ReconMove,
    Fpv,
    Delegate,
    Staff,
    Report,
    Complete,
}
pub const TRAINING_LESSONS: [TrainingLesson; 15] = [
    TrainingLesson::Welcome,
    TrainingLesson::Camera,
    TrainingLesson::Select,
    TrainingLesson::Move,
    TrainingLesson::Capture,
    TrainingLesson::Combat,
    TrainingLesson::Production,
    TrainingLesson::Construction,
    TrainingLesson::ReconProduction,
    TrainingLesson::ReconMove,
    TrainingLesson::Fpv,
    TrainingLesson::Delegate,
    TrainingLesson::Staff,
    TrainingLesson::Report,
    TrainingLesson::Complete,
];
pub const TRAINING_TASKS: usize = 13;
#[derive(Clone, Copy, Debug)]
pub struct TrainingActors {
    pub infantry: u32,
    pub tank: Option<u32>,
    pub recon: Option<u32>,
    pub target: Option<u32>,
}

impl World {
    pub fn training(side: u8, lesson: TrainingLesson) -> (Self, TrainingActors) {
        assert!(side < 2);
        let mut w = Self::new(Settings {
            seed: 0x747261696e,
            duration: WAR_SECONDS,
            ai_mult: 1.0,
            player: Some(side),
        });
        w.training = true;
        w.training_fpv_launches = 0;
        w.units.clear();
        w.id_map.clear();
        w.blds.clear();
        w.bld_seq = 0;
        w.projs.clear();
        w.pq.clear();
        w.recent.clear();
        for cell in &mut w.grid {
            cell.clear();
        }
        w.owner.fill(-1);
        w.cap.fill(0);
        w.feat.fill(0);
        w.mines.fill(0);
        w.map.terrain.fill(Terrain::Neutral);
        w.map.start_owner.fill(-1);
        w.map.road.fill(false);
        w.map.rail.fill(false);
        w.map.bridge_at.fill(u16::MAX);
        w.map.bridges.clear();
        w.map.labels.clear();
        w.map.rail_lines.clear();
        w.map.road_lines.clear();
        w.map.ground = Ground::Normal;
        w.last_ground = Ground::Normal;
        for y in 64..120 {
            for x in 80..144 {
                let i = tidx(x, y);
                w.map.terrain[i] = Terrain::Plain;
                w.owner[i] = if x < 106 {
                    side as i8
                } else {
                    (1 - side) as i8
                };
                w.map.start_owner[i] = w.owner[i];
            }
        }
        for x in 82..142 {
            w.map.road[tidx(x, 100)] = true;
        }
        for y in 68..118 {
            w.map.road[tidx(96, y)] = true;
        }
        w.map.road_lines = vec![
            vec![(82.0, 100.0), (142.0, 100.0)],
            vec![(96.0, 68.0), (96.0, 118.0)],
        ];
        for y in 99..=101 {
            for x in 109..=111 {
                w.map.terrain[tidx(x, y)] = Terrain::Urban;
            }
        }
        let captured = lesson as u8 >= TrainingLesson::Combat as u8;
        if captured {
            for y in 96..=104 {
                for x in 106..=114 {
                    w.owner[tidx(x, y)] = side as i8;
                }
            }
        }
        w.map.cities = vec![City {
            name: "訓練拠点",
            x: 110.0,
            y: 100.0,
            value: 1,
            port: false,
            owner: if captured { side } else { 1 - side },
        }];
        w.city_home = w.map.cities.iter().map(|c| c.owner).collect();
        w.total_tiles = w.owner.iter().filter(|o| **o >= 0).count() as u32;
        w.next_event = usize::MAX;
        w.mods.clear();
        w.halts.clear();
        w.flooded.clear();
        w.flood_until = 0;
        w.mud_until = 0;
        w.rail_cuts.clear();
        w.chronicle.clear();
        w.notices.clear();
        w.fx.clear();
        w.scars.clear();
        w.scar_log.clear();
        w.repaint.clear();
        w.dirty.clear();
        w.history.clear();
        w.sats.clear();
        w.weather = Weather::Clear;
        w.weather_t = 1e9;
        for s in &mut w.sides {
            s.funds = 5000.0;
            s.shells = 1000.0;
            s.kills = 0;
            s.losses = 0;
            s.ai = false;
            s.auto_new = false;
            s.auto_buy = false;
            s.plan = Default::default();
            s.command = Default::default();
            s.income = 0.0;
            s.shell_income = 0.0;
            s.power_cd = [0.0; 3];
        }
        w.add_building(BK::Base, side, 96.0, 94.0);
        w.add_building(BK::Workshop, side, 88.0, 106.0);
        if lesson as u8 >= TrainingLesson::ReconProduction as u8 {
            w.add_building(BK::Depot, side, 94.0, 106.0);
        }
        for b in &mut w.blds {
            b.building = 0.0;
        }
        let (ix, iy) = if captured {
            (110.0, 100.0)
        } else if lesson == TrainingLesson::Capture {
            (98.0, 100.0)
        } else {
            (90.0, 100.0)
        };
        let infantry = w.spawn(UK::Infantry, side, ix, iy);
        let tank = if captured {
            Some(w.spawn(UK::Tank, side, 112.0, 96.0))
        } else {
            None
        };
        if lesson as u8 >= TrainingLesson::Construction as u8 {
            w.spawn(UK::Infantry, side, 98.0, 94.0);
        }
        let recon = if lesson as u8 >= TrainingLesson::ReconMove as u8 {
            let (x, y) = if lesson == TrainingLesson::ReconMove {
                (88.0, 106.0)
            } else {
                (126.0, 86.0)
            };
            Some(w.spawn(UK::ReconDrone, side, x, y))
        } else {
            None
        };
        let target = if matches!(lesson, TrainingLesson::Combat | TrainingLesson::Fpv) {
            let (kind, x, hp) = if lesson == TrainingLesson::Combat {
                (UK::Ifv, 120.0, 45.0)
            } else {
                (UK::Tank, 126.0, 80.0)
            };
            let id = w.spawn(kind, 1 - side, x, 100.0);
            let i = w.unit_index(id).unwrap();
            w.units[i].hp = hp;
            w.units[i].cd = 1e9;
            Some(id)
        } else {
            None
        };
        for u in &mut w.units {
            u.auto = false;
            u.order = Order::Hold;
            u.ai_t = 0.0;
        }
        if matches!(
            lesson,
            TrainingLesson::Staff | TrainingLesson::Report | TrainingLesson::Complete
        ) {
            w.sides[side as usize].command.generals[0].region = Region {
                x: 108.0,
                y: 94.0,
                radius: 28.0,
            };
            w.sides[side as usize].command.generals[0].doctrine = Doctrine::Defend;
            let _ = w.assign_general(side, 0, &[infantry]);
            let _ = w.fund_general(side, 0, 100.0);
        }
        if lesson == TrainingLesson::Report {
            w.sides[side as usize].command.report_t = 8.0;
        }
        w.count_tiles();
        w.update_supply();
        w.update_vision();
        w.history.push((0.0, w.share(0)));
        (
            w,
            TrainingActors {
                infantry,
                tank,
                recon,
                target,
            },
        )
    }
    pub fn is_training(&self) -> bool {
        self.training
    }
}
