//! 将軍からの予算・戦地の希望。採用されるまでは既存の指揮系統を変えない。
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct GeneralProposal {
    pub at: f32,
    pub city: usize,
    pub region: Option<Region>,
    /// 追加で希望する金額。承認時に、既存予算と発注済み増援を再確認する。
    pub budget: f32,
    pub reason: String,
}

#[derive(Clone, Copy)]
pub enum ProposalPart {
    Budget,
    Theater,
    Both,
}

const INTERVAL: f32 = 90.0;
const INTENT: [&str; GENERAL_COUNT] = [
    "敵拠点への突破を任せてください",
    "味方拠点で損耗を抑えて守ります",
    "戦力差を見て攻守を調整します",
    "戦車とIFVで突破口を作ります",
    "砲兵と偵察を揃えて火力を集中します",
    "偵察とFPVで敵の配置を崩します",
    "圧力を受ける味方拠点を守ります",
    "価値の高い敵拠点へ機動攻撃します",
    "補給の安定した拠点から戦線を支えます",
];

impl World {
    fn proposal_roster(&self, side: u8, gi: usize) -> [(UK, u32); 4] {
        if self.sides[side as usize].command.generals[gi].doctrine == Doctrine::Defend {
            return [
                (UK::Infantry, 3),
                (UK::Atgm, 2),
                (UK::Shorad, 1),
                (UK::ReconDrone, 1),
            ];
        }
        match gi {
            1 | 6 => [
                (UK::Infantry, 3),
                (UK::Atgm, 2),
                (UK::Shorad, 1),
                (UK::ReconDrone, 1),
            ],
            3 => [
                (UK::Infantry, 2),
                (UK::Tank, 2),
                (UK::Ifv, 2),
                (UK::ReconDrone, 1),
            ],
            4 => [
                (UK::Infantry, 2),
                (UK::Howitzer, 2),
                (UK::Mlrs, 1),
                (UK::ReconDrone, 1),
            ],
            5 => [
                (UK::Infantry, 2),
                (UK::ReconDrone, 2),
                (UK::FpvTeam, 2),
                (UK::Shorad, 1),
            ],
            7 => [
                (UK::Infantry, 2),
                (UK::Ifv, 2),
                (UK::FpvTeam, 2),
                (UK::ReconDrone, 1),
            ],
            8 => [
                (UK::Infantry, 3),
                (UK::Engineer, 1),
                (UK::Shorad, 1),
                (UK::ReconDrone, 1),
            ],
            _ => [
                (UK::Infantry, 3),
                (UK::Tank, 1),
                (UK::Ifv, 1),
                (UK::ReconDrone, 1),
            ],
        }
    }
    fn proposal_shortfall(&self, side: u8, gi: usize) -> (f32, u32) {
        let c = &self.sides[side as usize].command;
        let general = &c.generals[gi];
        let mut cost = 0.0;
        let mut missing = 0;
        for (kind, want) in self.proposal_roster(side, gi) {
            let live = general
                .units
                .iter()
                .filter(|id| {
                    self.unit(**id)
                        .is_some_and(|u| !u.dead && u.side == side && u.kind == kind)
                })
                .count() as u32;
            let queued = c
                .pending
                .iter()
                .filter(|p| {
                    p.general == gi as u8
                        && self.blds.iter().any(|b| {
                            !b.dead
                                && b.side == side
                                && b.id == p.building
                                && b.queue.get(p.slot) == Some(&kind)
                        })
                })
                .count() as u32;
            let count = want.saturating_sub(live + queued);
            cost += unit_cost(side, kind) * count as f32;
            missing += count;
        }
        ((cost - general.budget).max(0.0).ceil(), missing)
    }
    /// 自軍と公開されている都市の支配、視認済みの敵だけで希望を作る。
    pub fn general_proposal(&self, side: u8, gi: usize) -> Option<GeneralProposal> {
        if side > 1 || gi >= GENERAL_COUNT {
            return None;
        }
        let c = &self.sides[side as usize].command;
        let general = &c.generals[gi];
        let own: Vec<_> = self
            .units
            .iter()
            .filter(|u| u.side == side && !u.dead)
            .collect();
        let known: Vec<_> = self
            .units
            .iter()
            .filter(|u| u.side != side && !u.dead && self.can_see(side, u))
            .collect();
        let assigned: Vec<_> = own
            .iter()
            .filter(|u| general.units.contains(&u.id))
            .collect();
        let origin = if assigned.is_empty() {
            (general.region.x, general.region.y)
        } else {
            (
                assigned.iter().map(|u| u.x).sum::<f32>() / assigned.len() as f32,
                assigned.iter().map(|u| u.y).sum::<f32>() / assigned.len() as f32,
            )
        };
        let defensive = general.doctrine == Doctrine::Defend || matches!(gi, 1 | 6 | 8);
        let dist = |a: f32, b: f32, x: f32, y: f32| (a - x).hypot(b - y);
        let city = self
            .map
            .cities
            .iter()
            .enumerate()
            .filter(|(_, city)| {
                if defensive && city.owner != side {
                    return false;
                }
                // 遠い敵後方の都市は希望しない。味方の地上拠点から進出できる近傍を候補にする。
                city.owner == side
                    || self.map.cities.iter().any(|base| {
                        base.owner == side && dist(base.x, base.y, city.x, city.y) < 75.0
                    })
            })
            .max_by(|(ia, a), (ib, b)| {
                let score = |i: usize, city: &City| {
                    let radius = 42.0;
                    let foe: f32 = known
                        .iter()
                        .filter(|u| dist(u.x, u.y, city.x, city.y) < radius)
                        .map(|u| strength(u.kind))
                        .sum();
                    let friendly: f32 = own
                        .iter()
                        .filter(|u| dist(u.x, u.y, city.x, city.y) < radius)
                        .map(|u| strength(u.kind))
                        .sum();
                    let border = self
                        .map
                        .cities
                        .iter()
                        .filter(|other| other.owner != city.owner)
                        .map(|other| dist(city.x, city.y, other.x, other.y))
                        .fold(150.0, f32::min);
                    let supply = self.supply[side as usize][tidx(city.x as i32, city.y as i32)] > 0;
                    let occupied = c
                        .generals
                        .iter()
                        .enumerate()
                        .filter(|(j, g)| {
                            *j != gi && !g.units.is_empty() && g.region.contains(city.x, city.y)
                        })
                        .count() as f32;
                    let home = if city.owner == side { 0.0 } else { 18.0 };
                    let front = (75.0 - border).max(0.0) * 0.5;
                    let urgency = if defensive {
                        (foe - friendly * 0.6).max(0.0) * 3.0
                    } else {
                        (friendly - foe * general.threshold(gi)).clamp(-15.0, 15.0)
                    };
                    let speciality = match gi {
                        3 => {
                            if self.map.terrain[tidx(city.x as i32, city.y as i32)]
                                == Terrain::Urban
                            {
                                0.0
                            } else {
                                8.0
                            }
                        }
                        4 => foe.min(10.0),
                        5 => {
                            if foe == 0.0 {
                                8.0
                            } else {
                                2.0
                            }
                        }
                        7 => city.value as f32 * 7.0,
                        8 => {
                            if supply {
                                24.0
                            } else {
                                -24.0
                            }
                        }
                        _ => city.value as f32 * 2.0,
                    };
                    let preferred = if sector(city.y) == gi % 3 { 7.0 } else { 0.0 };
                    // 都市番号は同点のときだけ用い、乱数や未確認の敵に依存しない。
                    home + front + urgency + speciality + preferred
                        - dist(origin.0, origin.1, city.x, city.y) * 0.65
                        - occupied * 12.0
                        - i as f32 * 0.001
                };
                score(*ia, a).total_cmp(&score(*ib, b))
            })
            .map(|(i, _)| i)?;
        let target = &self.map.cities[city];
        let radius = if gi == 7 { 28.0 } else { 42.0 };
        let region = if (!general.region_assigned && general.units.is_empty())
            || dist(general.region.x, general.region.y, target.x, target.y) > 12.0
        {
            Some(Region {
                x: target.x,
                y: target.y,
                radius,
            })
        } else {
            None
        };
        let (budget, missing) = self.proposal_shortfall(side, gi);
        if budget <= 0.0 && region.is_none() {
            return None;
        }
        let nearby = |u: &&&Unit| dist(u.x, u.y, target.x, target.y) < 42.0;
        let allies = own.iter().filter(nearby).count();
        let foes = known.iter().filter(nearby).count();
        let reason = format!(
            "{}。周辺の自軍{}・確認敵{}。不足する増援{}部隊を揃えたい。",
            INTENT[gi], allies, foes, missing
        );
        Some(GeneralProposal {
            at: self.t,
            city,
            region,
            budget,
            reason,
        })
    }
    pub(crate) fn general_proposals_tick(&mut self, side: u8) {
        let s = side as usize;
        self.sides[s].command.proposal_t -= 1.0;
        if self.sides[s].command.proposal_t > 0.0 {
            return;
        }
        self.sides[s].command.proposal_t = INTERVAL;
        let mut changed = 0;
        let mut first = None;
        for gi in 0..GENERAL_COUNT {
            if self.t < self.sides[s].command.proposal_after[gi] {
                continue;
            }
            let mut proposal = self.general_proposal(side, gi);
            if let Some(next) = &mut proposal {
                let same = self.sides[s].command.proposals[gi]
                    .as_ref()
                    .is_some_and(|old| {
                        old.city == next.city
                            && old.region == next.region
                            && (old.budget - next.budget).abs() < 0.5
                    });
                if same {
                    next.at = self.sides[s].command.proposals[gi].as_ref().unwrap().at;
                } else {
                    changed += 1;
                    if first.is_none() {
                        first = Some(gi);
                    }
                }
            }
            self.sides[s].command.proposals[gi] = proposal;
        }
        if let Some(gi) = first {
            let p = self.sides[s].command.proposals[gi].as_ref().unwrap();
            let city = &self.map.cities[p.city];
            self.notices.push(Notice {
                text: format!(
                    "将軍{}人から提案：{}が{}方面・追加予算{:.0}を希望（司令部で確認）",
                    changed, GENERAL_NAMES[s][gi], city.name, p.budget
                ),
                side: Some(side),
                x: city.x,
                y: city.y,
                tone: 0,
            });
        }
    }
    pub fn accept_general_proposal(
        &mut self,
        side: u8,
        gi: usize,
        part: ProposalPart,
    ) -> Result<f32, &'static str> {
        if side > 1 || gi >= GENERAL_COUNT {
            return Err("将軍が見つからない");
        }
        let proposal = self.sides[side as usize].command.proposals[gi]
            .clone()
            .ok_or("提案がありません")?;
        let pay = matches!(part, ProposalPart::Budget | ProposalPart::Both);
        let deploy = matches!(part, ProposalPart::Theater | ProposalPart::Both);
        // 表示後の手動配分・生産予約も数え、二重に予算を渡さない。
        let budget = if pay {
            proposal.budget.min(self.proposal_shortfall(side, gi).0)
        } else {
            0.0
        };
        if self.sides[side as usize].funds < budget {
            return Err("希望予算に対して手元資金が足りません。戦地だけの承認もできます");
        }
        if budget > 0.0 {
            self.fund_general(side, gi, budget)?;
        }
        if deploy {
            if let Some(region) = proposal.region {
                let g = &mut self.sides[side as usize].command.generals[gi];
                g.region = region;
                g.region_assigned = true;
                for id in &g.units {
                    if let Some(u) = self.units.iter_mut().find(|u| u.id == *id) {
                        u.ai_t = 0.0;
                    }
                }
            }
        }
        let p = self.sides[side as usize].command.proposals[gi]
            .as_mut()
            .unwrap();
        if pay {
            p.budget = 0.0;
        }
        if deploy {
            p.region = None;
        }
        if p.budget <= 0.0 && p.region.is_none() {
            self.dismiss_general_proposal(side, gi);
        }
        Ok(budget)
    }
    pub fn proposal_budget_needed(&self, side: u8, gi: usize) -> f32 {
        if side > 1 || gi >= GENERAL_COUNT {
            return 0.0;
        }
        self.sides[side as usize].command.proposals[gi]
            .as_ref()
            .map_or(0.0, |p| p.budget.min(self.proposal_shortfall(side, gi).0))
    }
    pub fn dismiss_general_proposal(&mut self, side: u8, gi: usize) {
        if side > 1 || gi >= GENERAL_COUNT {
            return;
        }
        let c = &mut self.sides[side as usize].command;
        c.proposals[gi] = None;
        c.proposal_after[gi] = self.t + INTERVAL;
    }
}
