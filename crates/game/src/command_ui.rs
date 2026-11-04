//! 将軍・参謀本部の共通操作画面（マウス／タッチ）。
use crate::gfx::*;
use crate::ui::{TextQ, ACCENT, DIM, PANEL, TEXT};
use crate::{Game, Mode};
use yydrone_core::command::*;
use yydrone_core::defs::*;

impl Game {
    pub(crate) fn toggle_command(&mut self) {
        self.show_command = !self.show_command;
        if self.show_command {
            self.show_help = false;
            self.show_brief = false;
            self.show_chron = false;
            self.show_save = false;
        }
    }
    pub(crate) fn command_launcher(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32) {
        let count = self.world.as_ref().map_or(0, |w| {
            w.sides[self.viewer.unwrap_or(0) as usize]
                .command
                .proposals
                .iter()
                .flatten()
                .count()
        });
        let label = if count > 0 {
            format!("司令部 提案{count}")
        } else if self.compact {
            "司令部".into()
        } else {
            "司令部 F3".into()
        };
        if self
            .button(
                g,
                tq,
                &label,
                sw - 128.0,
                self.top_h() + 8.0,
                116.0,
                if self.compact { 44.0 } else { 34.0 },
                false,
                true,
            )
            .is_some()
        {
            self.toggle_command();
        }
    }
    pub(crate) fn draw_command(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        let side = self.viewer.unwrap_or(0);
        let enabled = self.viewer.is_some();
        let width = (sw - if self.compact { 16.0 } else { 32.0 }).min(1050.0);
        let height = if self.compact {
            sh - 12.0
        } else {
            (sh - 78.0).min(624.0)
        };
        let (x, y) = ((sw - width) / 2.0, (sh - height) / 2.0);
        g.rect(0.0, 0.0, sw, sh, [0.0, 0.0, 0.0, 0.8]);
        g.rect(x, y, x + width, y + height, PANEL);
        self.bg_rects.push((0.0, 0.0, sw, sh));
        tq.add(
            "司令部 — 将軍への委任と参謀本部",
            x + 16.0,
            y + 10.0,
            width - 100.0,
            30.0,
            Font::Big,
            TEXT,
        );
        if self
            .button(
                g,
                tq,
                "閉じる",
                x + width - if self.compact { 140.0 } else { 82.0 },
                y + 8.0,
                68.0,
                30.0,
                false,
                true,
            )
            .is_some()
        {
            self.show_command = false;
        }
        for (i, label) in ["将軍", "参謀本部"].iter().enumerate() {
            if self
                .button(
                    g,
                    tq,
                    label,
                    x + 16.0 + i as f32 * 146.0,
                    y + 48.0,
                    138.0,
                    if self.compact { 40.0 } else { 32.0 },
                    self.command_tab == i,
                    true,
                )
                .is_some()
            {
                self.command_tab = i;
            }
        }
        if self.command_tab == 0 {
            let page = self.command_general / 3;
            if self
                .button(
                    g,
                    tq,
                    "前の3人",
                    x + width - 306.0,
                    y + 48.0,
                    108.0,
                    if self.compact { 40.0 } else { 32.0 },
                    false,
                    page > 0,
                )
                .is_some()
            {
                self.command_general = (page - 1) * 3;
            }
            if self
                .button(
                    g,
                    tq,
                    "次の3人",
                    x + width - 188.0,
                    y + 48.0,
                    108.0,
                    if self.compact { 40.0 } else { 32.0 },
                    false,
                    page + 1 < GENERAL_COUNT / 3,
                )
                .is_some()
            {
                self.command_general = (page + 1) * 3;
            }
            tq.add(
                format!("{}/{}", self.command_general / 3 + 1, GENERAL_COUNT / 3),
                x + width - 70.0,
                y + 50.0,
                56.0,
                28.0,
                Font::NormalCenter,
                DIM,
            );
            self.draw_generals(
                g,
                tq,
                side,
                enabled,
                x + 16.0,
                y + if self.compact { 98.0 } else { 92.0 },
                width - 32.0,
                height - if self.compact { 110.0 } else { 104.0 },
            );
        } else {
            self.draw_staff(
                g,
                tq,
                side,
                enabled,
                x + 16.0,
                y + if self.compact { 98.0 } else { 92.0 },
                width - 32.0,
                height - if self.compact { 110.0 } else { 104.0 },
            );
        }
    }
    fn draw_generals(
        &mut self,
        g: &mut dyn Gfx,
        tq: &mut TextQ,
        side: u8,
        enabled: bool,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        let card = (width - 16.0) / 3.0;
        for local in 0..3 {
            let i = self.command_general / 3 * 3 + local;
            let name = format!("{} / {}", GENERAL_NAMES[side as usize][i], TRAIT_NAMES[i]);
            if self
                .button(
                    g,
                    tq,
                    &name,
                    x + local as f32 * (card + 8.0),
                    y,
                    card,
                    if self.compact { 40.0 } else { 34.0 },
                    self.command_general == i,
                    true,
                )
                .is_some()
            {
                self.command_general = i;
            }
        }
        let gi = self.command_general;
        let Some(w) = self.world.as_ref() else { return };
        let general = w.sides[side as usize].command.generals[gi].clone();
        let funds = w.sides[side as usize].funds;
        let cities = w
            .map
            .cities
            .iter()
            .filter(|c| c.owner == side && general.region.contains(c.x, c.y))
            .count();
        let selected = self.selected.len();
        let py = y + if self.compact { 50.0 } else { 44.0 };
        let tex = self
            .art
            .portrait_texture(g, side as usize * (GENERAL_COUNT + 1) + gi);
        let portrait = if self.compact && height < 340.0 {
            52.0
        } else if height >= 420.0 {
            128.0
        } else {
            64.0
        };
        g.draw_textured(tex, x, py, x + portrait, py + portrait, [1.0; 4], true);
        tq.add(
            format!(
                "{}将軍 — {}（指示：{}）",
                GENERAL_NAMES[side as usize][gi],
                TRAIT_NAMES[gi],
                general.doctrine.name()
            ),
            x + portrait + 16.0,
            py,
            width - portrait - 16.0,
            28.0,
            Font::Big,
            TEXT,
        );
        tq.add(
            TRAIT_TEXT[gi],
            x + portrait + 16.0,
            py + 30.0,
            width - portrait - 24.0,
            if portrait < 64.0 { 28.0 } else { 44.0 },
            if portrait > 64.0 {
                Font::Normal
            } else {
                Font::Small
            },
            DIM,
        );
        let mut cy = py + portrait + 12.0;
        tq.add(
            format!(
                "委任 {} 部隊　予算 {:.0}　使用 {:.0}　損失 {}　担当内の自軍都市 {}　手元 {:.0}",
                general.units.len(),
                general.budget,
                general.spent,
                general.losses,
                cities,
                funds
            ),
            x,
            cy,
            width,
            26.0,
            Font::Normal,
            ACCENT,
        );
        cy += 30.0;
        for (i, label) in ["指示と予算", "地域と部隊", "将軍からの提案"]
            .iter()
            .enumerate()
        {
            if self
                .button(
                    g,
                    tq,
                    label,
                    x + i as f32 * 158.0,
                    cy,
                    150.0,
                    if self.compact { 36.0 } else { 30.0 },
                    self.command_detail_tab == i,
                    true,
                )
                .is_some()
            {
                self.command_detail_tab = i;
            }
        }
        cy += if self.compact { 42.0 } else { 38.0 };
        if self.command_detail_tab == 0 {
            for (i, d) in [Doctrine::Assault, Doctrine::Defend, Doctrine::Flexible]
                .into_iter()
                .enumerate()
            {
                if self
                    .button(
                        g,
                        tq,
                        d.name(),
                        x + i as f32 * (card + 8.0),
                        cy,
                        card,
                        if self.compact { 44.0 } else { 32.0 },
                        general.doctrine == d,
                        enabled,
                    )
                    .is_some()
                {
                    if let Some(w) = self.world.as_mut() {
                        w.sides[side as usize].command.generals[gi].doctrine = d;
                        for id in general.units.iter() {
                            if let Some(i) = w.unit_index(*id) {
                                w.units[i].ai_t = 0.0;
                            }
                        }
                    }
                }
            }
            cy += if self.compact { 52.0 } else { 42.0 };
            let labels = ["予算 +100", "予算 +500", "未使用予算を返還"];
            for (i, label) in labels.iter().enumerate() {
                let afford = if i < 2 {
                    funds >= [100.0, 500.0][i]
                } else {
                    general.budget > 0.0
                };
                if self
                    .button(
                        g,
                        tq,
                        label,
                        x + i as f32 * (card + 8.0),
                        cy,
                        card,
                        if self.compact { 44.0 } else { 32.0 },
                        false,
                        enabled && afford,
                    )
                    .is_some()
                {
                    if let Some(w) = self.world.as_mut() {
                        let amount = if i < 2 {
                            [100.0, 500.0][i]
                        } else {
                            -general.budget
                        };
                        if let Err(e) = w.fund_general(side, gi, amount) {
                            self.message(e);
                        }
                    }
                }
            }
            cy += if self.compact { 52.0 } else { 42.0 };
        } else if self.command_detail_tab == 1 {
            tq.add(
                format!(
                    "担当地域：中心 ({:.0}, {:.0})・半径 {:.0} タイル",
                    general.region.x, general.region.y, general.region.radius
                ),
                x,
                cy,
                width,
                25.0,
                Font::Normal,
                TEXT,
            );
            cy += 28.0;
            let bw = (width - 24.0) / 4.0;
            if self
                .button(
                    g,
                    tq,
                    "地図で中心を指定",
                    x,
                    cy,
                    bw,
                    if self.compact { 44.0 } else { 32.0 },
                    false,
                    enabled,
                )
                .is_some()
            {
                self.show_command = false;
                self.mode = Mode::CommandArea(gi as u8);
            }
            for (j, radius) in [28.0, 42.0, 64.0].into_iter().enumerate() {
                if self
                    .button(
                        g,
                        tq,
                        &format!("半径 {radius:.0}"),
                        x + (j + 1) as f32 * (bw + 8.0),
                        cy,
                        bw,
                        if self.compact { 44.0 } else { 32.0 },
                        general.region.radius == radius,
                        enabled,
                    )
                    .is_some()
                {
                    if let Some(w) = self.world.as_mut() {
                        w.sides[side as usize].command.generals[gi].region.radius = radius;
                        w.sides[side as usize].command.generals[gi].region_assigned = true;
                    }
                }
            }
            cy += if self.compact { 48.0 } else { 36.0 };
            let labels = [
                format!("選択 {selected} 部隊を委任"),
                "地域内の部隊を委任".into(),
                "部隊の指揮を返還".into(),
            ];
            for (j, label) in labels.iter().enumerate() {
                if self
                    .button(
                        g,
                        tq,
                        label,
                        x + j as f32 * (card + 8.0),
                        cy,
                        card,
                        if self.compact { 44.0 } else { 34.0 },
                        false,
                        enabled && (j != 0 || selected > 0),
                    )
                    .is_some()
                {
                    if let Some(w) = self.world.as_mut() {
                        if j == 2 {
                            w.reclaim_general(side, gi);
                            self.message("将軍から部隊の直接指揮を引き継ぎました");
                        } else {
                            let ids = if j == 0 {
                                self.selected.clone()
                            } else {
                                w.units
                                    .iter()
                                    .filter(|u| {
                                        u.side == side
                                            && !u.dead
                                            && general.region.contains(u.x, u.y)
                                    })
                                    .map(|u| u.id)
                                    .collect()
                            };
                            let result = w.assign_general(side, gi, &ids);
                            match result {
                                Ok(n) => {
                                    self.selected.clear();
                                    self.cmd_pause = false;
                                    self.message(format!("{n} 部隊を将軍に委任しました"));
                                }
                                Err(e) => self.message(e),
                            }
                        }
                    }
                }
            }
            cy += if self.compact { 48.0 } else { 36.0 };
        } else {
            self.draw_general_proposal(g, tq, side, gi, enabled, x, cy, width);
            return;
        }
        if cy + 40.0 < y + height {
            tq.add("予算内で増援を発注し、担当内で指揮します。直接命令するとその部隊は委任を離れます。",x,cy,width,42.0,Font::Small,DIM);
        }
    }
    fn draw_general_proposal(
        &mut self,
        g: &mut dyn Gfx,
        tq: &mut TextQ,
        side: u8,
        gi: usize,
        enabled: bool,
        x: f32,
        y: f32,
        width: f32,
    ) {
        let Some(w) = self.world.as_ref() else { return };
        let Some(p) = w.sides[side as usize].command.proposals[gi].clone() else {
            tq.add(
                "現在、未回答の提案はありません。",
                x,
                y,
                width,
                26.0,
                Font::Normal,
                TEXT,
            );
            tq.add(
                "将軍は戦況・特性・不足戦力を定期的に見直し、希望戦地と予算を自ら提案します。",
                x,
                y + 30.0,
                width,
                44.0,
                Font::Small,
                DIM,
            );
            return;
        };
        let city = &w.map.cities[p.city];
        let budget = w.proposal_budget_needed(side, gi);
        let afford = w.sides[side as usize].funds >= budget;
        tq.add(
            format!(
                "希望：{}方面（{}）　追加予算 {:.0}　{}",
                city.name,
                FRONT_NAMES[sector(city.y)],
                budget,
                if p.region.is_none() {
                    "戦地は承認済み"
                } else {
                    "戦地の変更を希望"
                }
            ),
            x,
            y,
            width,
            26.0,
            Font::Normal,
            ACCENT,
        );
        tq.add(&p.reason, x, y + 28.0, width, 44.0, Font::Small, TEXT);
        let bw = (width - 24.0) / 5.0;
        for (i, label) in [
            "予算を承認",
            "戦地を承認",
            "両方を承認",
            "地図で見る",
            "見送る",
        ]
        .iter()
        .enumerate()
        {
            let can = match i {
                0 => p.budget > 0.0 && afford,
                1 => p.region.is_some(),
                2 => afford,
                _ => true,
            };
            if self
                .button(
                    g,
                    tq,
                    label,
                    x + i as f32 * (bw + 6.0),
                    y + 76.0,
                    bw,
                    if self.compact { 44.0 } else { 32.0 },
                    false,
                    enabled && can,
                )
                .is_some()
            {
                if i == 3 {
                    self.proposal_preview = Some(gi);
                    self.show_regions = true;
                    self.show_command = false;
                    let c = &self.world.as_ref().unwrap().map.cities[p.city];
                    self.cam = (c.x, c.y);
                    let (sw, sh) = g.size();
                    self.zoom = (sw / 120.0)
                        .min((sh - self.top_h() - if self.compact { 120.0 } else { 190.0 }) / 95.0)
                        .max(2.0);
                    self.clamp_camera(g);
                } else if let Some(w) = self.world.as_mut() {
                    if i == 4 {
                        w.dismiss_general_proposal(side, gi);
                        self.proposal_preview = None;
                        self.message("提案を見送りました。次の見直しを待ちます。");
                    } else {
                        let part = [
                            ProposalPart::Budget,
                            ProposalPart::Theater,
                            ProposalPart::Both,
                        ][i];
                        match w.accept_general_proposal(side, gi, part) {
                            Ok(paid) => {
                                self.proposal_preview = None;
                                self.message(format!("提案を承認しました（追加予算 {paid:.0}）"))
                            }
                            Err(e) => self.message(e),
                        };
                    }
                }
                return;
            }
        }
    }
    fn draw_staff(
        &mut self,
        g: &mut dyn Gfx,
        tq: &mut TextQ,
        side: u8,
        enabled: bool,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        let Some(w) = self.world.as_ref() else { return };
        let plan = w.staff_plan(side);
        let report_t = w.sides[side as usize].command.report_t;
        let reports = w.sides[side as usize].command.reports.clone();
        let summaries: Vec<String> = (0..3).map(|f| w.front_summary(side, f)).collect();
        let tex = self
            .art
            .portrait_texture(g, side as usize * (GENERAL_COUNT + 1) + GENERAL_COUNT);
        let portrait = if height >= 420.0 { 112.0 } else { 76.0 };
        g.draw_textured(tex, x, y, x + portrait, y + portrait, [1.0; 4], true);
        tq.add(
            "参謀本部 — 直接指揮を支援",
            x + portrait + 16.0,
            y,
            width - portrait - 16.0,
            28.0,
            Font::Big,
            TEXT,
        );
        tq.add(
            "自軍と確認済みの敵情報から判断。戦力案は未委任部隊の不足を補う編成です。",
            x + portrait + 16.0,
            y + 34.0,
            width - portrait - 16.0,
            44.0,
            Font::Small,
            DIM,
        );
        let mut cy = y + portrait + 6.0;
        for (i, label) in ["助言と戦力案", "戦線レポート"].iter().enumerate() {
            if self
                .button(
                    g,
                    tq,
                    label,
                    x + i as f32 * 166.0,
                    cy,
                    158.0,
                    30.0,
                    self.command_staff_tab == i,
                    true,
                )
                .is_some()
            {
                self.command_staff_tab = i;
            }
        }
        cy += 36.0;
        if self.command_staff_tab == 0 {
            let proposed = if plan.units.is_empty() {
                "必要な基本編成は揃っています".into()
            } else {
                plan.units
                    .iter()
                    .map(|(k, n)| format!("{} ×{}", def(*k).short, n))
                    .collect::<Vec<_>>()
                    .join("　")
            };
            tq.add(
                format!("戦力案（必要資金 {:.0}）：{proposed}", plan.cost),
                x,
                cy,
                width,
                42.0,
                Font::Normal,
                ACCENT,
            );
            cy += 46.0;
            if self
                .button(
                    g,
                    tq,
                    "戦力案を発注（可能な分）",
                    x,
                    cy,
                    248.0,
                    32.0,
                    false,
                    enabled && !plan.units.is_empty(),
                )
                .is_some()
            {
                let mut bought = 0;
                if let Some(w) = self.world.as_mut() {
                    for (k, n) in &plan.units {
                        for _ in 0..*n {
                            if w.queue_command_unit(side, DIRECT_COMMAND, *k).is_ok() {
                                bought += 1;
                            } else {
                                break;
                            }
                        }
                    }
                }
                self.message(format!("{bought} 部隊を発注。完成部隊は直接指揮になります。未発注分は施設と資金を確認してください。"));
            }
            cy += 40.0;
            for advice in plan.advice.iter().take(3) {
                tq.add(format!("・{advice}"), x, cy, width, 30.0, Font::Small, TEXT);
                cy += 30.0;
            }
        } else {
            tq.add(
                format!(
                    "戦線レポート（60秒間隔・次回まで {:.0} 秒）",
                    report_t.max(0.0)
                ),
                x,
                cy,
                width,
                25.0,
                Font::Normal,
                ACCENT,
            );
            cy += 30.0;
            let page_count = ((reports.len() + 2) / 3).max(1);
            self.command_report_page = self.command_report_page.min(page_count - 1);
            let lines: Vec<String> = if reports.is_empty() {
                summaries
            } else {
                reports
                    .iter()
                    .skip(self.command_report_page * 3)
                    .take(3)
                    .map(|r| {
                        format!(
                            "[{:02}:{:02}] {}",
                            r.at.round() as u32 / 60,
                            r.at.round() as u32 % 60,
                            r.text
                        )
                    })
                    .collect()
            };
            for line in lines {
                if cy + 34.0 > y + height - 36.0 {
                    break;
                }
                tq.add(line, x, cy, width, 34.0, Font::Small, TEXT);
                cy += 36.0;
            }
            let by = y + height - 32.0;
            if self
                .button(
                    g,
                    tq,
                    "新しい報告",
                    x,
                    by,
                    138.0,
                    30.0,
                    false,
                    self.command_report_page > 0,
                )
                .is_some()
            {
                self.command_report_page -= 1;
            }
            if self
                .button(
                    g,
                    tq,
                    "前の報告",
                    x + 146.0,
                    by,
                    138.0,
                    30.0,
                    false,
                    self.command_report_page + 1 < page_count,
                )
                .is_some()
            {
                self.command_report_page += 1;
            }
            tq.add(
                format!(
                    "{}/{}　確認できない敵戦力は計上していません",
                    self.command_report_page + 1,
                    page_count
                ),
                x + 300.0,
                by,
                width - 300.0,
                28.0,
                Font::Small,
                DIM,
            );
        }
    }
}
