//! 担当領域の直近の支配増減と、確認できる戦力から戦況を集計する。
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    Advancing,
    Retreating,
    Advantage,
    Disadvantage,
    Contested,
    Unknown,
}
impl Situation {
    pub fn name(self) -> &'static str {
        match self {
            Self::Advancing => "前進",
            Self::Retreating => "後退",
            Self::Advantage => "優勢",
            Self::Disadvantage => "劣勢",
            Self::Contested => "拮抗",
            Self::Unknown => "不明",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneralSituation {
    /// 0=押されている側、0.5=中央、1=押している側。
    pub balance: f32,
    pub situation: Situation,
    pub territory_delta: i32,
    pub own_strength: f32,
    pub enemy_strength: f32,
    pub known_enemies: u32,
    pub checked_at: f32,
}
impl Default for GeneralSituation {
    fn default() -> Self {
        Self {
            balance: 0.5,
            situation: Situation::Unknown,
            territory_delta: 0,
            own_strength: 0.0,
            enemy_strength: 0.0,
            known_enemies: 0,
            checked_at: 0.0,
        }
    }
}

impl World {
    pub fn general_situation(&self, side: u8, gi: usize) -> GeneralSituation {
        if side > 1 || gi >= GENERAL_COUNT {
            return GeneralSituation::default();
        }
        let c = &self.sides[side as usize].command;
        if c.situation_regions[gi] != Some(c.generals[gi].region) {
            return GeneralSituation::default();
        }
        c.situations[gi]
    }
    pub(crate) fn refresh_command_situations(&mut self, side: u8) {
        let s = side as usize;
        for gi in 0..GENERAL_COUNT {
            let region = self.sides[s].command.generals[gi].region;
            if !self.sides[s].command.generals[gi].has_assignment() {
                let c = &mut self.sides[s].command;
                c.situations[gi] = GeneralSituation::default();
                c.situation_regions[gi] = None;
                c.territory_samples[gi].clear();
                continue;
            }
            let (mut owned, mut land) = (0u32, 0u32);
            let x0 = (region.x - region.radius).floor().max(0.0) as i32;
            let x1 = (region.x + region.radius).ceil().min(W as f32 - 1.0) as i32;
            let y0 = (region.y - region.radius).floor().max(0.0) as i32;
            let y1 = (region.y + region.radius).ceil().min(H as f32 - 1.0) as i32;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if region.contains(x as f32 + 0.5, y as f32 + 0.5) {
                        let owner = self.owner[tidx(x, y)];
                        if owner >= 0 {
                            land += 1;
                            if owner == side as i8 {
                                owned += 1;
                            }
                        }
                    }
                }
            }
            let (mut own_strength, mut enemy_strength, mut known_enemies) = (0.0, 0.0, 0u32);
            for u in &self.units {
                if u.dead || !region.contains(u.x, u.y) || matches!(u.kind, UK::Fpv | UK::Cruise) {
                    continue;
                }
                let power = strength(u.kind) * (u.hp / def(u.kind).hp).clamp(0.0, 1.0);
                if u.side == side {
                    own_strength += power
                        * if !u.supplied && def(u.kind).pass != Pass::Air {
                            0.65
                        } else {
                            1.0
                        };
                } else if self.can_see(side, u) {
                    enemy_strength += power;
                    known_enemies += 1;
                }
            }
            let c = &mut self.sides[s].command;
            if c.situation_regions[gi] != Some(region) {
                c.territory_samples[gi].clear();
                c.situation_regions[gi] = Some(region);
            }
            let samples = &mut c.territory_samples[gi];
            // 同じ時刻での再集計は置き換える。担当変更や読込で偽の増減を出さない。
            if samples.back().is_some_and(|(at, _)| *at == self.t) {
                samples.pop_back();
            }
            samples.push_back((self.t, owned));
            while samples.len() > 1 && samples.front().is_some_and(|(at, _)| *at < self.t - 30.1) {
                samples.pop_front();
            }
            while samples.len() > 8 {
                samples.pop_front();
            }
            let delta = owned as i32 - samples.front().map_or(owned, |(_, value)| *value) as i32;
            let (situation, balance) = if delta.abs() >= 3 {
                let change =
                    (delta.unsigned_abs() as f32 / (land.max(1) as f32 * 0.03)).clamp(0.1, 0.45);
                if delta > 0 {
                    (Situation::Advancing, 0.5 + change)
                } else {
                    (Situation::Retreating, 0.5 - change)
                }
            } else if known_enemies > 0 {
                let balance = own_strength / (own_strength + enemy_strength).max(0.001);
                (
                    if balance >= 0.6 {
                        Situation::Advantage
                    } else if balance <= 0.4 {
                        Situation::Disadvantage
                    } else {
                        Situation::Contested
                    },
                    balance,
                )
            } else {
                (Situation::Unknown, 0.5)
            };
            c.situations[gi] = GeneralSituation {
                balance,
                situation,
                territory_delta: delta,
                own_strength,
                enemy_strength,
                known_enemies,
                checked_at: self.t,
            };
        }
    }
}
