//! 将軍の担当円・名前・部隊の対応。希望戦地は破線で区別する。
use crate::gfx::*;
use crate::ui::{TextQ, ACCENT, DIM, LINE, PANEL, TEXT};
use crate::{Game, Mode};
use yydrone_core::command::*;

pub(crate) const GENERAL_COLORS: [Col; GENERAL_COUNT] = [
    [1.0, 0.76, 0.23, 1.0],
    [0.35, 0.85, 1.0, 1.0],
    [0.86, 0.55, 1.0, 1.0],
    [0.42, 1.0, 0.58, 1.0],
    [1.0, 0.52, 0.69, 1.0],
    [0.98, 0.94, 0.55, 1.0],
    [0.43, 0.65, 1.0, 1.0],
    [1.0, 0.62, 0.3, 1.0],
    [0.61, 0.95, 0.87, 1.0],
];

impl Game {
    pub(crate) fn draw_general_regions(&self, g: &mut dyn Gfx) {
        let Some(side) = self.viewer else { return };
        let Some(w) = self.world.as_ref() else { return };
        let pixel = 1.0 / self.zoom;
        for (gi, general) in w.sides[side as usize].command.generals.iter().enumerate() {
            if (!self.show_regions || !general.has_assignment())
                && self.mode != Mode::CommandArea(gi as u8)
            {
                continue;
            }
            let r = general.region;
            let c = GENERAL_COLORS[gi];
            g.circle(r.x, r.y, r.radius, 96, [c[0], c[1], c[2], 0.045]);
            g.ring(
                r.x,
                r.y,
                r.radius,
                if self.command_general == gi {
                    2.5 * pixel
                } else {
                    1.7 * pixel
                },
                96,
                [c[0], c[1], c[2], 0.8],
            );
            g.circle(r.x, r.y, 3.0 * pixel, 12, c);
            for id in &general.units {
                if let Some(u) = w.unit(*id) {
                    g.ring(
                        u.x,
                        u.y,
                        (self.zoom * 0.8).max(6.0) * pixel,
                        1.5 * pixel,
                        12,
                        c,
                    );
                }
            }
        }
        if let Some(gi) = self.proposal_preview.filter(|_| self.show_regions) {
            if let Some(p) = &w.sides[side as usize].command.proposals[gi] {
                if let Some(r) = p.region {
                    for seg in (0..96).step_by(2) {
                        let point = |i| {
                            let a = i as f32 * std::f32::consts::TAU / 96.0;
                            (r.x + a.cos() * r.radius, r.y + a.sin() * r.radius)
                        };
                        g.line(point(seg), point(seg + 1), 2.5 * pixel, ACCENT);
                    }
                }
            }
        }
    }
    pub(crate) fn draw_general_minimap(&self, g: &mut dyn Gfx) {
        if !self.show_regions {
            return;
        }
        let Some(side) = self.viewer else { return };
        let Some(w) = self.world.as_ref() else { return };
        let (x0, y0, x1, y1) = self.minimap_rect(g);
        let kx = (x1 - x0) / yydrone_core::geo::W as f32;
        let ky = (y1 - y0) / yydrone_core::geo::H as f32;
        for (gi, general) in w.sides[side as usize].command.generals.iter().enumerate() {
            if !general.has_assignment() {
                continue;
            }
            let r = general.region;
            let c = GENERAL_COLORS[gi];
            for seg in 0..64 {
                let point = |i| {
                    let a = i as f32 * std::f32::consts::TAU / 64.0;
                    (
                        (x0 + (r.x + a.cos() * r.radius) * kx).clamp(x0, x1),
                        (y0 + (r.y + a.sin() * r.radius) * ky).clamp(y0, y1),
                    )
                };
                g.line(point(seg), point(seg + 1), 1.0, c);
            }
            let (x, y) = (
                (x0 + r.x * kx).clamp(x0 + 2.0, x1 - 2.0),
                (y0 + r.y * ky).clamp(y0 + 2.0, y1 - 2.0),
            );
            g.rect(x - 2.0, y - 2.0, x + 2.0, y + 2.0, c);
        }
    }
    fn general_card(
        &mut self,
        g: &mut dyn Gfx,
        tq: &mut TextQ,
        side: u8,
        gi: usize,
        region: Region,
        count: usize,
        doctrine: Doctrine,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> bool {
        let c = GENERAL_COLORS[gi];
        // 地図上の都市ラベルなどをカードの下へ隠す。
        tq.0.retain(|(_, tx, ty, tw, th, _, _, _)| {
            *tx + *tw <= x || *tx >= x + width || *ty + *th <= y || *ty >= y + height
        });
        let clicked = self
            .button(g, tq, "", x, y, width, height, false, true)
            .is_some();
        g.rect_outline(x, y, x + width, y + height, 1.5, c);
        g.rect(x, y, x + 4.0, y + height, c);
        let portrait = height - 12.0;
        let tex = self
            .art
            .portrait_texture(g, side as usize * (GENERAL_COUNT + 1) + gi);
        g.draw_textured(
            tex,
            x + 7.0,
            y + 6.0,
            x + 7.0 + portrait,
            y + 6.0 + portrait,
            [1.0; 4],
            true,
        );
        tq.add(
            format!("{} {}", gi + 1, GENERAL_NAMES[side as usize][gi]),
            x + portrait + 15.0,
            y + 4.0,
            width - portrait - 20.0,
            22.0,
            Font::Normal,
            c,
        );
        tq.add(
            format!("{} / {}部隊", doctrine.name(), count),
            x + portrait + 15.0,
            y + 27.0,
            width - portrait - 20.0,
            22.0,
            Font::Small,
            TEXT,
        );
        if clicked {
            self.command_general = gi;
            self.command_tab = 0;
            self.command_detail_tab = 1;
            self.cam = (region.x, region.y);
            self.clamp_camera(g);
            self.toggle_command();
        }
        clicked
    }
    pub(crate) fn draw_command_map(&mut self, g: &mut dyn Gfx, tq: &mut TextQ) {
        if self.modal_open() {
            return;
        }
        let Some(side) = self.viewer else { return };
        let Some(w) = self.world.as_ref() else { return };
        let active: Vec<_> = w.sides[side as usize]
            .command
            .generals
            .iter()
            .enumerate()
            .filter(|(_, general)| general.has_assignment())
            .map(|(gi, general)| (gi, general.region, general.units.len(), general.doctrine))
            .collect();
        let preview = self.proposal_preview.and_then(|gi| {
            w.sides[side as usize].command.proposals[gi]
                .as_ref()
                .map(|p| (gi, p.region, p.city))
        });
        let (sw, sh) = g.size();
        let top = self.top_h();
        let bottom = sh - if self.compact { 120.0 } else { 190.0 };
        if self
            .button(
                g,
                tq,
                if self.show_regions {
                    "担当領域 ON"
                } else {
                    "担当領域 OFF"
                },
                sw - 128.0,
                top + 48.0,
                116.0,
                32.0,
                self.show_regions,
                true,
            )
            .is_some()
        {
            self.show_regions = !self.show_regions;
        }
        if !self.show_regions {
            return;
        }
        let (x, y) = (sw - 252.0, top + 88.0);
        let capacity = ((bottom - y - if preview.is_some() { 68.0 } else { 0.0 } - 52.0) / 54.0)
            .floor()
            .max(1.0) as usize;
        let per_page = capacity.min(if self.compact { 3 } else { GENERAL_COUNT });
        let pages = active.len().max(1).div_ceil(per_page);
        self.region_card_page = self.region_card_page.min(pages - 1);
        let start = self.region_card_page * per_page;
        let shown = active.len().saturating_sub(start).min(per_page);
        let height = 26.0 + shown.max(1) as f32 * 54.0 + if pages > 1 { 26.0 } else { 0.0 };
        tq.0.retain(|(_, tx, ty, tw, th, _, _, _)| {
            *tx + *tw <= x || *tx >= sw - 12.0 || *ty + *th <= y || *ty >= y + height
        });
        g.rect(x, y, sw - 12.0, y + height, PANEL);
        self.bg_rects.push((x, y, sw - 12.0, y + height));
        tq.add(
            format!(
                "担当 {}人（重複可） {}/{}",
                active.len(),
                self.region_card_page + 1,
                pages
            ),
            x + 8.0,
            y + 3.0,
            224.0,
            22.0,
            Font::Small,
            TEXT,
        );
        if active.is_empty() {
            tq.add(
                "担当は司令部で設定できます",
                x + 8.0,
                y + 32.0,
                224.0,
                22.0,
                Font::Small,
                DIM,
            );
        }
        for (row, (gi, r, count, doctrine)) in active.iter().skip(start).take(shown).enumerate() {
            if self.general_card(
                g,
                tq,
                side,
                *gi,
                *r,
                *count,
                *doctrine,
                x + 6.0,
                y + 25.0 + row as f32 * 54.0,
                228.0,
                52.0,
            ) {
                return;
            }
        }
        if pages > 1 {
            let cy = y + height - 26.0;
            if self
                .button(
                    g,
                    tq,
                    "前の担当",
                    x + 6.0,
                    cy,
                    110.0,
                    24.0,
                    false,
                    self.region_card_page > 0,
                )
                .is_some()
            {
                self.region_card_page -= 1;
            }
            if self
                .button(
                    g,
                    tq,
                    "次の担当",
                    x + 122.0,
                    cy,
                    110.0,
                    24.0,
                    false,
                    self.region_card_page + 1 < pages,
                )
                .is_some()
            {
                self.region_card_page += 1;
            }
        }
        let mut placed = vec![(x, y, sw - 12.0, y + height)];
        if preview.is_some() {
            placed.push((x, (y + height + 8.0).min(bottom - 64.0), sw - 12.0, bottom));
        }
        for (gi, r, count, doctrine) in active {
            let (sx, sy) = self.world_to_screen(g, r.x, r.y);
            let radius = r.radius * self.zoom;
            if sx + radius < 0.0 || sx - radius > sw || sy + radius < top || sy - radius > bottom {
                continue;
            }
            let (width, h) = (224.0, 56.0);
            let mut candidates = vec![(
                (sx - width / 2.0).clamp(8.0, sw - width - 8.0),
                sy.clamp(top + 8.0, bottom - h - 8.0),
            )];
            let columns = ((sw - 16.0) / (width + 8.0)).floor() as usize;
            let rows = ((bottom - top - 16.0) / (h + 8.0)).floor() as usize;
            for row in 0..rows {
                for col in 0..columns {
                    candidates.push((
                        8.0 + col as f32 * (width + 8.0),
                        top + 8.0 + row as f32 * (h + 8.0),
                    ));
                }
            }
            candidates.sort_by(|a, b| {
                let score = |p: &(f32, f32)| (p.0 + width / 2.0 - sx).hypot(p.1 + h / 2.0 - sy);
                score(a).total_cmp(&score(b))
            });
            let pos = candidates.into_iter().find(|(lx, ly)| {
                let overlaps = |r: &(f32, f32, f32, f32)| {
                    *lx < r.2 + 4.0
                        && *lx + width > r.0 - 4.0
                        && *ly < r.3 + 4.0
                        && *ly + h > r.1 - 4.0
                };
                !placed.iter().any(overlaps) && !self.bg_rects.iter().any(overlaps)
            });
            let Some((lx, ly)) = pos else {
                continue;
            };
            let c = GENERAL_COLORS[gi];
            g.line(
                (sx.clamp(0.0, sw), sy.clamp(top, bottom)),
                (lx + width / 2.0, ly + h / 2.0),
                1.0,
                [c[0], c[1], c[2], 0.65],
            );
            if self.general_card(g, tq, side, gi, r, count, doctrine, lx, ly, width, h) {
                return;
            }
            placed.push((lx, ly, lx + width, ly + h));
        }
        if let Some((gi, region, city)) = preview {
            let cy = (y + height + 8.0).min(bottom - 64.0);
            g.rect(x, cy, sw - 12.0, cy + 60.0, PANEL);
            g.rect_outline(x, cy, sw - 12.0, cy + 60.0, 1.0, LINE);
            self.bg_rects.push((x, cy, sw - 12.0, cy + 60.0));
            let name = self.world.as_ref().unwrap().map.cities[city].name;
            tq.add(
                format!("希望：{} / {}方面", GENERAL_NAMES[side as usize][gi], name),
                x + 6.0,
                cy + 4.0,
                228.0,
                24.0,
                Font::Small,
                ACCENT,
            );
            if self
                .button(
                    g,
                    tq,
                    if region.is_some() {
                        "破線は未承認・提案へ戻る"
                    } else {
                        "提案へ戻る"
                    },
                    x + 6.0,
                    cy + 30.0,
                    190.0,
                    26.0,
                    false,
                    true,
                )
                .is_some()
            {
                self.command_general = gi;
                self.command_tab = 0;
                self.command_detail_tab = 2;
                self.toggle_command();
            }
            if self
                .button(g, tq, "✕", sw - 48.0, cy + 30.0, 30.0, 26.0, false, true)
                .is_some()
            {
                self.proposal_preview = None;
            }
        }
    }
}
