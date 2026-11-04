//! 将軍への実際の指揮・予算委任と、可視情報に基づく参謀本部。
use crate::defs::*;
use crate::geo::*;
use crate::world::*;
use std::collections::VecDeque;
mod proposals;
mod deployment;
pub use proposals::{GeneralProposal, ProposalPart};
pub const GENERAL_COUNT: usize = 9;
pub const DIRECT_COMMAND: u8 = GENERAL_COUNT as u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Doctrine {
    Assault,
    Defend,
    Flexible,
}
impl Doctrine {
    pub fn name(self) -> &'static str {
        match self {
            Self::Assault => "積極攻勢",
            Self::Defend => "全力防御",
            Self::Flexible => "臨機応変",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}
impl Region {
    pub fn contains(self, x: f32, y: f32) -> bool {
        (x - self.x).powi(2) + (y - self.y).powi(2) <= self.radius * self.radius
    }
    pub fn clamp(self, x: f32, y: f32) -> (f32, f32) {
        let (dx, dy) = (x - self.x, y - self.y);
        let d = (dx * dx + dy * dy).sqrt().max(0.01);
        let scale = (self.radius / d).min(1.0);
        (
            (self.x + dx * scale).clamp(0.5, W as f32 - 1.0),
            (self.y + dy * scale).clamp(0.5, H as f32 - 1.0),
        )
    }
}
#[derive(Clone, Debug)]
pub struct General {
    pub doctrine: Doctrine,
    pub region: Region,
    pub region_assigned: bool,
    pub units: Vec<u32>,
    pub budget: f32,
    pub spent: f32,
    pub losses: u32,
    pub buy_t: f32,
}
impl General {
    pub fn has_assignment(&self) -> bool {self.region_assigned || !self.units.is_empty() || self.budget>0.0}
    pub fn threshold(&self, index: usize) -> f32 {
        [0.8, 1.65, 1.15, 1.05, 1.35, 1.3, 1.5, 0.95, 1.4][index]
            * if self.doctrine == Doctrine::Assault {
                0.72
            } else {
                1.0
            }
    }
    pub fn retreat_hp(&self, index: usize) -> f32 {
        [0.22, 0.65, 0.4, 0.35, 0.5, 0.45, 0.6, 0.3, 0.55][index]
    }
}
pub const GENERAL_NAMES: [[&str; GENERAL_COUNT]; 2] = [
    [
        "ヴェレス",
        "ミラ",
        "オリク",
        "ルネ",
        "エダ",
        "セイラ",
        "マルク",
        "ノヴァ",
        "イリス",
    ],
    [
        "ラーダ",
        "セヴェル",
        "ティモル",
        "アルダ",
        "ルク",
        "ソーナ",
        "ダリオ",
        "レナ",
        "イサク",
    ],
];
pub const TRAIT_NAMES: [&str; GENERAL_COUNT] = [
    "突破重視",
    "兵力温存",
    "均衡調整",
    "機甲集中",
    "火力優先",
    "偵察重視",
    "守備重視",
    "機動襲撃",
    "補給重視",
];
pub const TRAIT_TEXT: [&str; GENERAL_COUNT] = [
    "不利でも攻勢を続ける。戦果を狙うが損失も増えやすい。",
    "優勢を確認して前進し、損耗時は早く下がる。損失を抑えるが進撃は遅い。",
    "戦力差と補給を見て攻守を切り替える。突出を避ける。",
    "戦車とIFVを揃えて前進する。突破力を求めるが増援の費用が高い。",
    "砲兵と偵察を優先し、支援部隊を後方に置く。火力が整うまで慎重。",
    "偵察機の配置を確認してから攻勢へ移る。奇襲を避けるが準備に時間がかかる。",
    "臨機応変の指示では守備を優先する。攻勢を任せるときは積極攻勢を指定。",
    "IFVとドローンを好み、価値の高い都市を狙う。進出は速いが損耗には注意。",
    "地域の補給切れが多い間は進撃を控える。継戦を重視するが好機を逃すことも。",
];
#[derive(Clone, Debug)]
pub struct PendingRecruit {
    pub building: u32,
    pub slot: usize,
    pub general: u8,
}
#[derive(Clone, Debug)]
pub struct FrontReport {
    pub at: f32,
    pub sector: usize,
    pub text: String,
}
#[derive(Clone, Debug)]
pub struct CommandState {
    pub generals: [General; GENERAL_COUNT],
    pub pending: Vec<PendingRecruit>,
    pub reports: VecDeque<FrontReport>,
    pub report_t: f32,
    pub front_losses: [u32; 3],
    pub front_kills: [u32; 3],
    pub last_losses: [u32; 3],
    pub last_kills: [u32; 3],
    pub proposals: [Option<GeneralProposal>; GENERAL_COUNT],
    pub proposal_t: f32,
    pub proposal_after: [f32; GENERAL_COUNT],
}
impl Default for CommandState {
    fn default() -> Self {
        Self {
            generals: std::array::from_fn(|i| General {
                doctrine: Doctrine::Flexible,
                region: Region {
                    x: 205.0,
                    y: 35.0 + (i % 3) as f32 * 65.0,
                    radius: 42.0,
                },
                region_assigned: false,
                units: Vec::new(),
                budget: 0.0,
                spent: 0.0,
                losses: 0,
                buy_t: 0.0,
            }),
            pending: Vec::new(),
            reports: VecDeque::new(),
            report_t: 60.0,
            front_losses: [0; 3],
            front_kills: [0; 3],
            last_losses: [0; 3],
            last_kills: [0; 3],
            proposals: std::array::from_fn(|_| None),
            proposal_t: 5.0,
            proposal_after: [0.0; GENERAL_COUNT],
        }
    }
}
pub fn sector(y: f32) -> usize {
    ((y.max(0.0) / (H as f32 / 3.0)) as usize).min(2)
}
pub const FRONT_NAMES: [&str; 3] = ["北部戦線", "中央戦線", "南部戦線"];
pub fn strength(k: UK) -> f32 {
    match k {
        UK::Tank => 3.5,
        UK::Ifv => 2.0,
        UK::Infantry | UK::Atgm | UK::FpvTeam => 1.0,
        UK::Howitzer | UK::Mlrs => 1.5,
        _ => 0.5,
    }
}
pub struct StaffPlan {
    pub units: Vec<(UK, u32)>,
    pub cost: f32,
    pub advice: Vec<String>,
}

impl World {
    pub fn general_for(&self, side: u8, id: u32) -> Option<usize> {
        self.sides[side as usize]
            .command
            .generals
            .iter()
            .position(|g| g.units.contains(&id))
    }
    pub fn assign_general(
        &mut self,
        side: u8,
        index: usize,
        ids: &[u32],
    ) -> Result<usize, &'static str> {
        if side > 1 || index >= GENERAL_COUNT {
            return Err("将軍が見つからない");
        }
        let mut valid: Vec<u32> = ids
            .iter()
            .copied()
            .filter(|id| {
                self.unit(*id).is_some_and(|u| {
                    u.side == side && !u.dead && !matches!(u.kind, UK::Fpv | UK::Cruise)
                })
            })
            .collect();
        valid.sort_unstable();
        valid.dedup();
        for g in &mut self.sides[side as usize].command.generals {
            g.units.retain(|id| !valid.contains(id));
        }
        for id in &valid {
            self.command_one(*id, Cmd::Stop, 0.0, 0.0);
            if let Some(i) = self.unit_index(*id) {
                self.units[i].auto = true;
                self.units[i].ai_t = 0.0;
            }
        }
        let g = &mut self.sides[side as usize].command.generals[index];
        for id in &valid {
            if !g.units.contains(id) {
                g.units.push(*id);
            }
        }
        Ok(valid.len())
    }
    pub fn reclaim_general(&mut self, side: u8, index: usize) {
        let ids = std::mem::take(&mut self.sides[side as usize].command.generals[index].units);
        for id in ids {
            self.command_one(id, Cmd::Auto(false), 0.0, 0.0);
            self.command_one(id, Cmd::Stop, 0.0, 0.0);
        }
    }
    pub fn fund_general(
        &mut self,
        side: u8,
        index: usize,
        amount: f32,
    ) -> Result<(), &'static str> {
        if side > 1 || index >= GENERAL_COUNT || !amount.is_finite() {
            return Err("無効な予算指定");
        }
        let s = &mut self.sides[side as usize];
        if amount >= 0.0 {
            if s.funds < amount {
                return Err("手元資金が足りない");
            }
            s.funds -= amount;
            s.command.generals[index].budget += amount;
        } else {
            let v = (-amount).min(s.command.generals[index].budget);
            s.command.generals[index].budget -= v;
            s.funds += v;
        }
        Ok(())
    }
    /// キューの位置と発注者を記録する。DIRECT_COMMAND は直接指揮部隊。
    pub fn queue_command_unit(&mut self, side: u8, index: u8, k: UK) -> Result<(), &'static str> {
        if side > 1 || index > DIRECT_COMMAND {
            return Err("無効な指揮官");
        }
        let cost = unit_cost(side, k);
        let funds = self.sides[side as usize].funds;
        let need = prod_building(def(k).prod).ok_or("生産できない")?;
        let b = self
            .blds
            .iter()
            .filter(|b| b.side == side && b.kind == need && b.building <= 0.0 && !b.dead)
            .min_by_key(|b| b.queue.len())
            .ok_or("生産施設が必要")?;
        let (building, slot) = (b.id, b.queue.len());
        if index < DIRECT_COMMAND {
            if self.sides[side as usize].command.generals[index as usize].budget < cost {
                return Err("委任予算が足りない");
            }
            self.sides[side as usize].funds += cost;
        }
        let result = self.queue_unit(side, k);
        if index < DIRECT_COMMAND {
            self.sides[side as usize].funds = funds;
        }
        result?;
        if index < DIRECT_COMMAND {
            let g = &mut self.sides[side as usize].command.generals[index as usize];
            g.budget -= cost;
            g.spent += cost;
        }
        self.sides[side as usize]
            .command
            .pending
            .push(PendingRecruit {
                building,
                slot,
                general: index,
            });
        Ok(())
    }
    pub(crate) fn finish_command_recruit(&mut self, side: u8, building: u32) -> Option<u8> {
        let p = &mut self.sides[side as usize].command.pending;
        let owner = p
            .iter()
            .find(|p| p.building == building && p.slot == 0)
            .map(|p| p.general);
        p.retain(|p| !(p.building == building && p.slot == 0));
        for p in p.iter_mut().filter(|p| p.building == building) {
            p.slot = p.slot.saturating_sub(1);
        }
        owner
    }
    pub(crate) fn cancel_command_recruit(
        &mut self,
        side: u8,
        building: u32,
        slot: usize,
        cost: f32,
    ) -> bool {
        let state = &mut self.sides[side as usize].command;
        let owner = state
            .pending
            .iter()
            .find(|p| p.building == building && p.slot == slot)
            .map(|p| p.general);
        state
            .pending
            .retain(|p| !(p.building == building && p.slot == slot));
        for p in state
            .pending
            .iter_mut()
            .filter(|p| p.building == building && p.slot > slot)
        {
            p.slot -= 1;
        }
        if let Some(i) = owner.filter(|i| *i < DIRECT_COMMAND) {
            let g = &mut state.generals[i as usize];
            g.budget += cost;
            g.spent = (g.spent - cost).max(0.0);
            true
        } else {
            false
        }
    }
    pub fn staff_plan(&self, side: u8) -> StaffPlan {
        let own: Vec<_> = self
            .units
            .iter()
            .filter(|u| u.side == side && !u.dead)
            .collect();
        let unsupplied = own
            .iter()
            .filter(|u| !u.supplied && def(u.kind).pass != Pass::Air)
            .count();
        let mut advice = Vec::new();
        if unsupplied > 0 {
            advice.push(format!(
                "補給切れ {unsupplied} 部隊。補給所と線路を確保してから前進してください。"
            ));
        }
        if self.sides[side as usize].shells < 30.0 {
            advice.push("砲弾が不足。砲弾工場を増やし、砲撃を絞ってください。".into());
        }
        let visible_air = self.units.iter().any(|u| {
            u.side != side && !u.dead && def(u.kind).pass == Pass::Air && self.can_see(side, u)
        });
        if visible_air {
            advice.push("敵航空戦力を確認。対空砲・SAMを主力に随伴させてください。".into());
        }
        for front in 0..3 {
            let a: f32 = own
                .iter()
                .filter(|u| sector(u.y) == front)
                .map(|u| strength(u.kind))
                .sum();
            let b: f32 = self
                .units
                .iter()
                .filter(|u| {
                    u.side != side && !u.dead && sector(u.y) == front && self.can_see(side, u)
                })
                .map(|u| strength(u.kind))
                .sum();
            if b > a * 1.2 && b > 0.0 {
                advice.insert(
                    unsupplied.min(1),
                    format!(
                        "{}の確認戦力が不足。予備隊を回し、将軍には全力防御を指示してください。",
                        FRONT_NAMES[front]
                    ),
                );
                break;
            }
        }
        let known_armor = self
            .units
            .iter()
            .filter(|u| {
                u.side != side
                    && !u.dead
                    && matches!(u.kind, UK::Tank | UK::Ifv)
                    && self.can_see(side, u)
            })
            .count();
        let mut units = Vec::new();
        for (k, want) in [
            (UK::Infantry, 6u32),
            (UK::Tank, 2),
            (UK::Ifv, 2),
            (UK::Atgm, 2 + (known_armor / 3).min(3) as u32),
            (UK::Howitzer, 2),
            (UK::ReconDrone, 2),
            (UK::Shorad, if visible_air { 3 } else { 1 }),
        ] {
            let reserve_queue = self
                .blds
                .iter()
                .filter(|b| b.side == side)
                .map(|b| {
                    b.queue
                        .iter()
                        .enumerate()
                        .filter(|(slot, q)| {
                            **q == k
                                && !self.sides[side as usize].command.pending.iter().any(|p| {
                                    p.general < DIRECT_COMMAND
                                        && p.building == b.id
                                        && p.slot == *slot
                                })
                        })
                        .count()
                })
                .sum::<usize>();
            let have = own
                .iter()
                .filter(|u| u.kind == k && self.general_for(side, u.id).is_none())
                .count() as u32
                + reserve_queue as u32;
            if have < want {
                units.push((k, want - have));
            }
        }
        let cost = units
            .iter()
            .map(|(k, n)| unit_cost(side, *k) * *n as f32)
            .fold(0.0, |total, value| total + value);
        advice.push(
            "偵察で敵配置を確認し、砲兵と対空を揃えてから局地的な優勢を作ってください。".into(),
        );
        if !self.sides[side as usize]
            .command
            .generals
            .iter()
            .any(|g| !g.units.is_empty())
        {
            advice.push("守備戦線を将軍に任せると、主攻の直接指揮に集中できます。".into());
        }
        StaffPlan {
            units,
            cost,
            advice,
        }
    }
    pub fn front_summary(&self, side: u8, front: usize) -> String {
        let own: Vec<_> = self
            .units
            .iter()
            .filter(|u| u.side == side && !u.dead && sector(u.y) == front)
            .collect();
        let enemy: Vec<_> = self
            .units
            .iter()
            .filter(|u| u.side != side && !u.dead && sector(u.y) == front && self.can_see(side, u))
            .collect();
        let a: f32 = own.iter().map(|u| strength(u.kind)).sum();
        let b: f32 = enemy.iter().map(|u| strength(u.kind)).sum();
        let status = if b == 0.0 {
            "敵情不明"
        } else if a > b * 1.4 {
            "確認戦力では優勢"
        } else if a < b * 0.8 {
            "確認戦力では劣勢"
        } else {
            "確認戦力では拮抗"
        };
        let c = &self.sides[side as usize].command;
        format!(
            "{}：{}／自軍 {}・確認敵 {}・補給切れ {}／直近戦果 +{}・損失 {}",
            FRONT_NAMES[front],
            status,
            own.len(),
            enemy.len(),
            own.iter()
                .filter(|u| !u.supplied && def(u.kind).pass != Pass::Air)
                .count(),
            c.front_kills[front].saturating_sub(c.last_kills[front]),
            c.front_losses[front].saturating_sub(c.last_losses[front])
        )
    }
    pub(crate) fn command_tick(&mut self, side: u8) {
        let s = side as usize;
        if !self.is_training() { self.general_proposals_tick(side); }
        for gi in 0..GENERAL_COUNT {
            let state = &mut self.sides[s].command.generals[gi];
            state.buy_t -= 1.0;
            if state.buy_t <= 0.0 && state.budget > 0.0 {
                state.buy_t = 6.0;
                let doctrine = state.doctrine;
                let choices = if doctrine == Doctrine::Defend || matches!(gi, 1 | 6) {
                    [
                        UK::Infantry,
                        UK::Atgm,
                        UK::Shorad,
                        UK::Howitzer,
                        UK::ReconDrone,
                    ]
                } else if gi == 3 {
                    [UK::Tank, UK::Ifv, UK::Infantry, UK::Shorad, UK::ReconDrone]
                } else if gi == 4 {
                    [
                        UK::Howitzer,
                        UK::Mlrs,
                        UK::Infantry,
                        UK::ReconDrone,
                        UK::Shorad,
                    ]
                } else if gi == 5 {
                    [
                        UK::ReconDrone,
                        UK::FpvTeam,
                        UK::Infantry,
                        UK::Shorad,
                        UK::Atgm,
                    ]
                } else if gi == 7 {
                    [UK::Ifv, UK::FpvTeam, UK::ReconDrone, UK::Infantry, UK::Tank]
                } else if gi == 8 {
                    [
                        UK::Engineer,
                        UK::Infantry,
                        UK::Shorad,
                        UK::Howitzer,
                        UK::ReconDrone,
                    ]
                } else {
                    [UK::Infantry, UK::Tank, UK::Ifv, UK::FpvTeam, UK::ReconDrone]
                };
                // 編成不足を優先し、発注済みも数える。
                let best = choices
                    .into_iter()
                    .filter(|k| {
                        def(*k).sides[s]
                            && unit_cost(side, *k) <= self.sides[s].command.generals[gi].budget
                            && self.blds.iter().any(|b| {
                                b.side == side
                                    && !b.dead
                                    && b.building <= 0.0
                                    && Some(b.kind) == prod_building(def(*k).prod)
                                    && b.queue.len() < 12
                            })
                    })
                    .min_by_key(|k| {
                        self.sides[s].command.generals[gi]
                            .units
                            .iter()
                            .filter(|id| self.unit(**id).is_some_and(|u| u.kind == *k))
                            .count()
                            + self.sides[s]
                                .command
                                .pending
                                .iter()
                                .filter(|p| {
                                    p.general == gi as u8
                                        && self.blds.iter().any(|b| {
                                            b.id == p.building && b.queue.get(p.slot) == Some(k)
                                        })
                                })
                                .count()
                    });
                if let Some(k) = best {
                    let _ = self.queue_command_unit(side, gi as u8, k);
                }
            }
        }
        let ids: Vec<(u32, usize)> = self.sides[s]
            .command
            .generals
            .iter()
            .enumerate()
            .flat_map(|(i, g)| g.units.iter().map(move |id| (*id, i)))
            .collect();
        for (id, gi) in ids {
            let Some(i) = self.unit_index(id) else {
                continue;
            };
            if self.units[i].dead || !self.units[i].auto {
                continue;
            }
            self.units[i].ai_t -= 1.0;
            if self.units[i].ai_t > 0.0 {
                continue;
            }
            self.units[i].ai_t = 8.0;
            self.general_order(i, gi);
        }
        self.sides[s].command.report_t -= 1.0;
        if self.sides[s].command.report_t <= 0.0 {
            self.sides[s].command.report_t = 60.0;
            for front in (0..3).rev() {
                let text = self.front_summary(side, front);
                self.sides[s].command.reports.push_front(FrontReport {
                    at: self.t,
                    sector: front,
                    text: text.clone(),
                });
                self.notices.push(Notice {
                    text,
                    side: Some(side),
                    x: -1.0,
                    y: -1.0,
                    tone: 0,
                });
            }
            let c = &mut self.sides[s].command;
            while c.reports.len() > 18 {
                c.reports.pop_back();
            }
            c.last_kills = c.front_kills;
            c.last_losses = c.front_losses;
        }
    }
    fn general_order(&mut self, i: usize, gi: usize) {
        let u = &self.units[i];
        let (side, id, x, y, k, hp, supplied) = (
            u.side,
            u.id,
            u.x,
            u.y,
            u.kind,
            u.hp / def(u.kind).hp,
            u.supplied,
        );
        let g = &self.sides[side as usize].command.generals[gi];
        let (region, mut doctrine, threshold, retreat) =
            (g.region, g.doctrine, g.threshold(gi), g.retreat_hp(gi));
        if gi == 6 && doctrine == Doctrine::Flexible {
            doctrine = Doctrine::Defend;
        }
        self.command_one(id, Cmd::Stop, 0.0, 0.0);
        let enemies: Vec<_> = self
            .units
            .iter()
            .filter(|o| {
                o.side != side && !o.dead && region.contains(o.x, o.y) && self.can_see(side, o)
            })
            .collect();
        let own: f32 = self
            .units
            .iter()
            .filter(|o| o.side == side && !o.dead && region.contains(o.x, o.y))
            .map(|o| strength(o.kind))
            .sum();
        let foe: f32 = enemies.iter().map(|o| strength(o.kind)).sum();
        let scouts_ready = gi != 5
            || self.units.iter().any(|u| {
                u.side == side && !u.dead && u.kind == UK::ReconDrone && region.contains(u.x, u.y)
            });
        let regional: Vec<_> = self
            .units
            .iter()
            .filter(|u| {
                u.side == side
                    && !u.dead
                    && region.contains(u.x, u.y)
                    && def(u.kind).pass != Pass::Air
            })
            .collect();
        let supply_ready = gi != 8
            || regional.is_empty()
            || regional.iter().filter(|u| !u.supplied).count() * 3 <= regional.len();
        let advance = doctrine != Doctrine::Defend
            && own > foe * threshold
            && supplied
            && hp >= retreat
            && scouts_ready
            && supply_ready;
        let weak = hp < retreat || (!supplied && def(k).pass != Pass::Air);
        let target = enemies
            .iter()
            .filter(|o| {
                doctrine != Doctrine::Defend
                    || self.owner[tidx(o.x as i32, o.y as i32)] == side as i8
            })
            .min_by(|a, b| {
                ((a.x - x).powi(2) + (a.y - y).powi(2))
                    .total_cmp(&((b.x - x).powi(2) + (b.y - y).powi(2)))
            })
            .map(|o| (o.id, o.x, o.y));
        let support = matches!(
            k,
            UK::Howitzer | UK::Mlrs | UK::Sam | UK::Ew | UK::Shorad | UK::Coastal
        );
        let destination = if weak || !advance {
            self.map
                .cities
                .iter()
                .filter(|c| c.owner == side && region.contains(c.x, c.y))
                .min_by(|a, b| {
                    ((a.x - region.x).powi(2) + (a.y - region.y).powi(2))
                        .total_cmp(&((b.x - region.x).powi(2) + (b.y - region.y).powi(2)))
                })
                .map(|c| (c.x, c.y))
                .unwrap_or((region.x, region.y))
        } else {
            self.map
                .cities
                .iter()
                .filter(|c| c.owner != side && region.contains(c.x, c.y))
                .min_by(|a, b| {
                    let score = |c: &City| {
                        ((c.x - x).powi(2) + (c.y - y).powi(2))
                            / (1.0 + if gi == 7 { c.value as f32 * 0.3 } else { 0.0 })
                    };
                    score(a).total_cmp(&score(b))
                })
                .map(|c| (c.x, c.y))
                .or(target.map(|(_, a, b)| (a, b)))
                .unwrap_or((region.x, region.y))
        };
        if !weak && !support && def(k).weapon.dmg > 0.0 {
            if let Some((tid, tx, ty)) = target {
                if advance
                    || doctrine == Doctrine::Defend
                    || (tx - x).powi(2) + (ty - y).powi(2) < def(k).weapon.range.powi(2)
                {
                    self.attack_target(&[id], Target::Unit(tid));
                    return;
                }
            }
        }
        let (mut gx, mut gy) = destination;
        if support && advance {
            let dx = x - gx;
            let dy = y - gy;
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let back = def(k).weapon.range.min(18.0).max(6.0) * if gi == 4 { 1.3 } else { 1.0 };
            gx += dx / d * back;
            gy += dy / d * back;
        }
        let (gx, gy) = region.clamp(gx + (id % 5) as f32 - 2.0, gy + (id % 7) as f32 - 3.0);
        if (gx - x).powi(2) + (gy - y).powi(2) > 9.0 {
            self.command_one(id, if weak { Cmd::Move } else { Cmd::AttackMove }, gx, gy);
        } else if !advance && !support {
            self.command_one(id, Cmd::Hold, gx, gy);
        }
    }
}
