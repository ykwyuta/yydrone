//! 新規作戦の初期配属。既存の戦力を使い、通常AIが運用する予備隊も残す。
use super::*;

const THEATERS: [[&str; GENERAL_COUNT]; 2] = [
    [
        "ハルキア",
        "キエヴァ",
        "ドニプロヴァ",
        "ザポリア",
        "クラマト",
        "スーミャ",
        "チェルニガ",
        "マリウポラ",
        "オデスカ",
    ],
    [
        "ベルゴラ",
        "クルスカ",
        "ドネスカ",
        "ルガンスカ",
        "ロストヴァ",
        "ミレロヴァ",
        "ブリャンスカ",
        "タガンラ",
        "シンフェラ",
    ],
];

fn suited(index: usize, kind: UK) -> bool {
    match index {
        0 | 3 => matches!(kind, UK::Tank | UK::Ifv),
        1 | 6 => matches!(kind, UK::Infantry | UK::Atgm | UK::Shorad),
        4 => matches!(kind, UK::Howitzer | UK::Mlrs),
        5 => matches!(kind, UK::ReconDrone | UK::FpvTeam),
        7 => matches!(kind, UK::Ifv | UK::FpvTeam),
        8 => matches!(kind, UK::Engineer | UK::Shorad),
        _ => matches!(kind, UK::Infantry | UK::Ifv),
    }
}

impl World {
    pub(crate) fn setup_initial_command(&mut self) {
        for side in 0..2u8 {
            for gi in 0..GENERAL_COUNT {
                let city = self
                    .map
                    .cities
                    .iter()
                    .find(|c| c.name == THEATERS[side as usize][gi] && c.owner == side);
                if let Some(city) = city {
                    let g = &mut self.sides[side as usize].command.generals[gi];
                    g.region = Region {
                        x: city.x,
                        y: city.y,
                        radius: 42.0,
                    };
                    g.region_assigned = true;
                }
            }
            let eligible: Vec<_> = self
                .units
                .iter()
                .filter(|u| {
                    u.side == side
                        && !u.dead
                        && (matches!(def(u.kind).pass, Pass::Inf | Pass::Veh)
                            || u.kind == UK::ReconDrone)
                })
                .map(|u| u.id)
                .collect();
            let limit = eligible.len().saturating_sub(eligible.len() / 4);
            let mut assigned = 0;
            // 最初に全将軍へ地上部隊を配属し、その後に近傍の専門兵種を足す。
            for round in 0..4 {
                for gi in 0..GENERAL_COUNT {
                    if assigned >= limit {
                        break;
                    }
                    let region = self.sides[side as usize].command.generals[gi].region;
                    let best = eligible
                        .iter()
                        .filter_map(|id| {
                            let u = self.unit(*id)?;
                            if self.general_for(side, *id).is_some()
                                || (round == 0 && def(u.kind).cap_r <= 0.0)
                            {
                                return None;
                            }
                            let distance = (u.x - region.x).hypot(u.y - region.y);
                            if round > 0 && distance > region.radius {
                                return None;
                            }
                            let score = distance
                                - if round > 0 && suited(gi, u.kind) {
                                    12.0
                                } else {
                                    0.0
                                };
                            Some((*id, score))
                        })
                        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                        .map(|(id, _)| id);
                    if let Some(id) = best {
                        if round == 0 {
                            let u = self.unit(id).unwrap();
                            let (x, y) = (u.x, u.y);
                            let g = &mut self.sides[side as usize].command.generals[gi];
                            if !g.region.contains(x, y) {
                                g.region.x = x;
                                g.region.y = y;
                            }
                            g.region_assigned = true;
                        }
                        let _ = self.assign_general(side, gi, &[id]);
                        assigned += 1;
                    }
                }
            }
        }
    }
}
