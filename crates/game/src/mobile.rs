//! タッチ端末向けの画面と記録画面（yyempire のスマートフォン・タブレット対応を引き継いだもの）
//!
//! 指で操作する端末では、UI を約 1000×480 に詰めて文字を大きくする。パネルは地図に重ねて
//! 必要なときだけ出す。上端は OS やブラウザがタッチを横取りしやすいので、上部バーを高くする。

use crate::gfx::*;
use crate::ui::*;
use crate::view::{draw_unit_icon, SIDE_COL};
use crate::{Game, Mode, Screen};
use yydrone_core::defs::*;
use yydrone_core::world::*;

/// 上部バーの高さ（タッチ向け）
pub const TOP_H_COMPACT: f32 = 50.0;
/// 指先がずれてもボタンを拾う距離
const SNAP: f32 = 10.0;
/// 説明画面を描く仮の高さ（画面に収まらなければ指でスクロールする）
const HELP_H: f32 = 600.0;
const MINI_C: (f32, f32) = (180.0, 112.0);

impl Game {
    /// タッチした場所に何があるか（UI 座標）。ブラウザ側が指の動きの解釈を決めるのに使う
    /// 0=地図 1=画面部品 2=押してすぐ反応する場所（ミニマップ・工事の線引き） 3=なぞってスクロールできる一覧
    pub fn hit_kind(&self, g: &dyn Gfx, x: f32, y: f32) -> u32 {
        if self.show_chron || (self.show_help && self.help_scrolls(g)) {
            return 3;
        }
        if self.screen != Screen::Play || self.modal_open() {
            return 1;
        }
        let (mx0, my0, mx1, my1) = self.minimap_rect(g);
        if x >= mx0 && x <= mx1 && y >= my0 && y <= my1 {
            return 2;
        }
        if self.over_ui(g, x, y) {
            return 1;
        }
        if matches!(self.mode, Mode::Job(_)) {
            return 2;
        }
        0
    }

    pub(crate) fn help_scrolls(&self, g: &dyn Gfx) -> bool {
        self.compact && g.size().1 < HELP_H
    }

    /// タッチは指先がずれやすいので、すぐ近くのボタンも拾う
    pub(crate) fn snap_touch(&self, x: f32, y: f32) -> (f32, f32) {
        if !self.touch || self.hit_prev.iter().any(|r| x >= r.0 && x < r.2 && y >= r.1 && y < r.3) {
            return (x, y);
        }
        let mut best = None;
        let mut bd = SNAP;
        for r in &self.hit_prev {
            let (cx, cy) = (x.clamp(r.0, r.2 - 0.5), y.clamp(r.1, r.3 - 0.5));
            let d = (cx - x).hypot(cy - y);
            if d <= bd {
                bd = d;
                best = Some((cx, cy));
            }
        }
        best.unwrap_or((x, y))
    }

    pub(crate) fn minimap_rect_compact(&self, g: &dyn Gfx) -> (f32, f32, f32, f32) {
        let (_, sh) = g.size();
        (4.0, sh - MINI_C.1 - 4.0, 4.0 + MINI_C.0, sh - 4.0)
    }

    /// 一覧をいつでも閉じられる ✕（右上に固定。スクロールしても動かない）。
    /// 一覧より先に押されたかを調べて、絵は一覧の上に close_button_draw で描く
    pub(crate) fn close_button_press(&mut self, g: &dyn Gfx) {
        if !self.compact {
            return;
        }
        let (sw, _) = g.size();
        if self.take_click(sw - 58.0, 3.0, sw - 10.0, 51.0).is_some() {
            self.show_help = false;
            self.show_chron = false;
            self.show_save = false;
            self.show_command = false;
            self.show_brief = false;
        }
    }

    pub(crate) fn close_button_draw(&self, g: &mut dyn Gfx, tq: &mut TextQ) {
        if !self.compact {
            return;
        }
        let (sw, _) = g.size();
        g.rect(sw - 58.0, 3.0, sw - 10.0, 51.0, PANEL2);
        g.rect_outline(sw - 58.0, 3.0, sw - 10.0, 51.0, 2.0, ACCENT);
        tq.add("✕", sw - 58.0, 3.0, 48.0, 48.0, Font::NormalCenter, TEXT);
    }

    /// 説明画面を仮の高さに描いて、指でスクロールさせる
    pub(crate) fn draw_help_scrolled(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        let max = HELP_H - sh;
        self.ov_scroll = self.ov_scroll.clamp(0.0, max);
        let sc = self.ov_scroll;
        g.set_xform(2.0 / sw, -2.0 / sh, -1.0, 1.0 + 2.0 * sc / sh);
        let (t0, h0) = (tq.0.len(), self.hit_rects.len());
        self.mouse.1 += sc;
        if let Some(p) = &mut self.pressed {
            p.2 += sc;
        }
        self.draw_help(g, tq, sw, HELP_H);
        self.mouse.1 -= sc;
        g.set_screen_xform();
        for t in tq.0[t0..].iter_mut() {
            t.2 -= sc;
        }
        for r in self.hit_rects[h0..].iter_mut() {
            r.1 -= sc;
            r.3 -= sc;
        }
        // スクロールの目印
        let bar = (sh * sh / HELP_H).max(30.0);
        let y = (sh - bar) * (sc / max.max(1.0));
        g.rect(sw - 7.0, 2.0, sw - 2.0, sh - 2.0, [0.0, 0.0, 0.0, 0.35]);
        g.rect(sw - 7.0, y + 2.0, sw - 2.0, y + bar - 2.0, [1.0, 0.86, 0.5, 0.85]);
    }

    // ------------------------------------------------------------ タイトル

    pub(crate) fn draw_title_compact(&mut self, g: &mut dyn Gfx) {
        let (sw, sh) = g.size();
        let mut tq = TextQ::default();
        // 記録画面が開いているときは、後ろのボタンを押せないようにする
        let press = if self.show_save { self.pressed.take() } else { None };

        // 背景：シネマティックアート（title.rgba）を全画面アスペクト比カバーで描画
        let title_tex = self.art.ensure_title(g);
        let aspect = 16.0 / 9.0;
        let (tw, th) = if sw / sh > aspect {
            (sw, sw / aspect)
        } else {
            (sh * aspect, sh)
        };
        let (tx0, ty0) = ((sw - tw) / 2.0, (sh - th) / 2.0);
        g.draw_textured(title_tex, tx0, ty0, tx0 + tw, ty0 + th, [0.85, 0.85, 0.85, 1.0], true);
        g.rect(0.0, 0.0, sw, sh, [0.03, 0.04, 0.06, 0.62]);
        g.tri((0.0, 0.0), (sw * 0.42, 0.0), (0.0, sh), [0.12, 0.25, 0.5, 0.4]);
        g.tri((sw, sh), (sw * 0.58, sh), (sw, 0.0), [0.5, 0.12, 0.1, 0.4]);
        let cx = sw / 2.0;
        // タブレットでは縦に余るので真ん中に寄せる
        let mut y = ((sh - 400.0) / 2.0).max(4.0);
        tq.shadow(crate::TITLE, cx - 300.0, y, 600.0, 44.0, Font::TitleMid, [1.0, 0.96, 0.85, 1.0]);
        y += 46.0;
        let (cw, ch) = (((sw - 40.0) / 2.0).min(420.0), 122.0);
        let infos: [(&str, &str, i32); 2] = [
            ("ウクラヴィア共和国", "西側の精密兵器と安価なドローンで戦う防衛側。対戦車ミサイル・ハイマル・FPV が強い。", 0),
            ("ロスコヴィア連邦", "砲弾と兵力の物量で押す攻撃側。自走砲・戦車が安く、艦隊の巡航ミサイルで後方を叩く。", 1),
        ];
        for (i, (name, desc, id)) in infos.iter().enumerate() {
            let x = cx - cw - 6.0 + i as f32 * (cw + 12.0);
            let on = self.cfg_side == *id;
            let col = SIDE_COL[i];
            g.rect(x, y, x + cw, y + ch, [col[0] * 0.2, col[1] * 0.2, col[2] * 0.2, 0.95]);
            g.rect_outline(x, y, x + cw, y + ch, if on { 3.0 } else { 1.0 }, if on { col } else { LINE });
            g.rect(x + 12.0, y + 14.0, x + 32.0, y + 34.0, col);
            tq.add(*name, x + 40.0, y + 10.0, cw - 50.0, 28.0, Font::Big, TEXT);
            tq.add(*desc, x + 12.0, y + 44.0, cw - 24.0, 72.0, Font::Normal, [0.82, 0.85, 0.85, 1.0]);
            if self.take_click(x, y, x + cw, y + ch).is_some() {
                self.cfg_side = *id;
            }
        }
        y += ch + 8.0;
        let bw = ((cw * 2.0 + 12.0 - 8.0) / 3.0).floor();
        let x0 = cx - cw - 6.0;
        for (i, s) in ["新兵（敵の収入 75%）", "標準", "精鋭（敵の収入 135%）"].iter().enumerate() {
            if self.button(g, &mut tq, s, x0 + i as f32 * (bw + 4.0), y, bw, 44.0, self.cfg_diff == i, true).is_some() {
                self.cfg_diff = i;
            }
        }
        y += 50.0;
        if self.button(g, &mut tq, "作戦開始（2時間）", x0, y, cw, 56.0, true, true).is_some() {
            self.start_game(g);
            return;
        }
        if self.button(g, &mut tq, "チュートリアル", x0 + cw + 12.0, y, cw, 56.0, false, true).is_some() { self.start_tutorial(g); return; }
        y += 62.0;
        let sub = ((cw * 2.0 + 12.0 - 12.0) / 4.0).floor();
        if self.button(g, &mut tq, "続きから", x0, y, sub, 48.0, false, true).is_some() {
            self.show_save = true;
        }
        if self.button(g, &mut tq, "観戦", x0 + sub + 4.0, y, sub, 48.0, false, true).is_some() {
            self.cfg_side = -1;
            self.start_game(g);
            self.cfg_side = 0;
            return;
        }
        if self.button(g, &mut tq, "全画面", x0 + (sub + 4.0) * 2.0, y, sub, 48.0, false, true).is_some() {
            g.toggle_fullscreen();
        }
        let half = ((sub - 4.0) / 2.0).floor();
        let xa = x0 + (sub + 4.0) * 3.0;
        if self.button(g, &mut tq, "A−", xa, y, half, 48.0, false, true).is_some() {
            g.adjust_scale(-1);
        }
        if self.button(g, &mut tq, "A＋", xa + half + 4.0, y, half, 48.0, false, true).is_some() {
            g.adjust_scale(1);
        }
        if self.show_save {
            tq.0.clear();
            self.pressed = press;
            self.close_button_press(g);
        }
        if self.show_save {
            self.draw_save(g, &mut tq, sw, sh);
            self.close_button_draw(g, &mut tq);
        }
        tq.flush(g);
    }

    // ------------------------------------------------------------ 上部バー

    pub(crate) fn draw_top_compact(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32) {
        const H: f32 = TOP_H_COMPACT;
        g.rect(0.0, 0.0, sw, H, PANEL);
        g.rect(0.0, H - 1.0, sw, H, LINE);
        let Some(w) = self.world.as_ref() else { return };
        let viewer = self.viewer;
        let (s0, s1) = (w.share(0), w.share(1));
        let info = viewer.map(|v| (w.sides[v as usize].funds, w.sides[v as usize].income, w.sides[v as usize].shells));
        let left = (w.duration - w.t).max(0.0) as u32;
        let date = w.date_text();
        let mud = w.mud();
        let cond = w.conditions_text();
        let cond_help = w.conditions_help();
        if let Some((f, fi, sh)) = info {
            tq.add(format!("資金 {f:.0} (+{fi:.0})"), 6.0, 3.0, 146.0, 24.0, Font::Normal, ACCENT);
            tq.add(format!("砲弾 {sh:.0}"), 6.0, 26.0, 146.0, 22.0, Font::Normal, if sh < 30.0 { BAD } else { TEXT });
        } else {
            tq.add("観戦中", 6.0, 13.0, 146.0, 24.0, Font::Normal, ACCENT);
        }
        // 領土バー
        let (bx, bw) = (154.0, (sw - 154.0 - 332.0 - 150.0).max(120.0));
        g.rect(bx, 8.0, bx + bw * s0, 42.0, SIDE_COL[0]);
        g.rect(bx + bw * s0, 8.0, bx + bw, 42.0, SIDE_COL[1]);
        g.rect(bx + bw / 2.0 - 1.0, 4.0, bx + bw / 2.0 + 1.0, 46.0, [1.0, 1.0, 1.0, 0.9]);
        tq.shadow(format!("{:.1}%", s0 * 100.0), bx + 4.0, 14.0, 60.0, 22.0, Font::Normal, TEXT);
        tq.shadow(format!("{:.1}%", s1 * 100.0), bx + bw - 56.0, 14.0, 54.0, 22.0, Font::Normal, TEXT);
        // 日付と残り時間
        let dx = bx + bw + 6.0;
        tq.add(format!("{date} {cond}"), dx, 3.0, 142.0, 24.0, Font::Normal, if mud { BAD } else { TEXT });
        // タップすると地面と天候の効果を説明する
        if inside(self.mouse, dx, 0.0, dx + 142.0, H) {
            self.tip = Some(cond_help);
            self.panel_top = H + 130.0;
        }
        tq.add(format!("残り {}:{:02}:{:02}", left / 3600, left / 60 % 60, left % 60), dx, 26.0, 142.0, 22.0, Font::Normal, if left < 60 { BAD } else { ACCENT });
        // 右側：一時停止・速度・各画面（上端から離れた所でも押せるよう、ボタンを下まで伸ばす）
        let paused = self.paused;
        let mut x = sw - 332.0;
        if self.button(g, tq, if paused { "▶" } else { "❚❚" }, x, 3.0, 50.0, H - 6.0, paused, true).is_some() {
            self.paused = !self.paused;
        }
        x += 53.0;
        let lbl = format!("×{}", self.speed);
        if self.button(g, tq, &lbl, x, 3.0, 50.0, H - 6.0, self.speed > 1, true).is_some() {
            // 押すたびに ×1 → ×2 → ×4 → ×8
            self.speed = if self.speed >= 8 { 1 } else { self.speed * 2 };
        }
        x += 53.0;
        for (s, a) in [("司令部", 3), ("年表", 0), ("記録", 1), ("？", 2)] {
            let on = match a {0=>self.show_chron,1=>self.show_save,2=>self.show_help,_=>self.show_command};
            if self.button(g, tq, s, x, 3.0, 54.0, H - 6.0, on, true).is_some() {
                match a {
                    0 => self.show_chron = !self.show_chron,
                    1 => self.show_save = !self.show_save,
                    2 => self.show_help = !self.show_help,
                    _ => {
                        self.toggle_command();
                        self.command_tab=0;
                        self.command_detail_tab=0;
                    }
                }
            }
            x += 57.0;
        }
    }

    // ------------------------------------------------------------ 下部

    pub(crate) fn draw_bottom_compact(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        self.draw_minimap(g);
        let (mx0, my0, mx1, my1) = self.minimap_rect(g);
        self.bg_rects.push((mx0, my0, mx1, my1));
        if self.pressed.is_some_and(|(_, x, y)| x >= mx0 && x <= mx1 && y >= my0 && y <= my1) {
            let m = self.pressed.map_or(self.mouse, |p| (p.1, p.2));
            self.pressed = None;
            self.minimap_jump(g, m.0, m.1);
            self.minimap_drag = true;
        }
        if self.viewer.is_none() {
            return;
        }
        // 右下の常設ボタン
        let open = self.show_prod;
        if self.button(g, tq, "生産", sw - 92.0, sh - 54.0, 88.0, 50.0, open, true).is_some() {
            self.show_prod = !open;
        }
        if self.button(g, tq, "画面内", sw - 92.0, sh - 108.0, 88.0, 50.0, false, true).is_some() {
            self.select_on_screen(g);
        }
        if self.show_prod {
            let (pw, ph) = (500.0f32.min(sw - MINI_C.0 - 112.0), 164.0);
            let (px, py) = (sw - 96.0 - pw, sh - ph - 4.0);
            g.rect(px - 4.0, py - 4.0, sw - 96.0, sh, PANEL);
            self.bg_rects.push((px - 4.0, py - 4.0, sw - 96.0, sh));
            self.panel_top = py - 4.0;
            self.draw_production(g, tq, px, py, pw, ph);
        } else {
            self.panel_top = sh - 120.0;
            self.draw_selection_compact(g, tq, sw, sh);
        }
    }

    fn draw_selection_compact(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        let Some(side) = self.viewer else { return };
        let (x0, x1) = (MINI_C.0 + 10.0, sw - 96.0);
        let (y0, y1) = (sh - 88.0, sh - 4.0);
        let w0 = x1 - x0;
        // 建物
        if let Some(bid) = self.sel_bld {
            let info = self.world.as_ref().and_then(|w| w.bld(bid)).map(|b| (b.kind, b.hp, b.max_hp, b.building, b.queue.len()));
            let Some((k, hp, max, building, q)) = info else { return };
            g.rect(x0, y0, x1, y1, PANEL);
            self.bg_rects.push((x0, y0, x1, y1));
            let st = if building > 0.0 { format!("　建設中 残り {building:.0} 秒") } else if q > 0 { format!("　生産の予約 {q}") } else { String::new() };
            tq.add(format!("{}　耐久 {:.0}/{:.0}{}", bdef(k).name, hp, max, st), x0 + 8.0, y0 + 4.0, w0 - 70.0, 26.0, Font::Normal, TEXT);
            let on = self.mode == Mode::Rally;
            if self.button(g, tq, "集結地点を決める", x0 + 6.0, y1 - 50.0, 220.0, 46.0, on, true).is_some() {
                self.mode = if on { Mode::Normal } else { Mode::Rally };
            }
            if self.button(g, tq, "✕", x1 - 50.0, y0 + 4.0, 46.0, 46.0, false, true).is_some() {
                self.sel_bld = None;
                self.mode = Mode::Normal;
            }
            return;
        }
        let sel: Vec<(UK, f32, bool)> = self.selected.iter().filter_map(|id| self.world.as_ref().and_then(|w| w.unit(*id))).map(|u| (u.kind, u.hp, u.auto)).collect();
        if sel.is_empty() {
            return;
        }
        g.rect(x0, y0, x1, y1, PANEL);
        self.bg_rects.push((x0, y0, x1, y1));
        let d = def(sel[0].0);
        if sel.len() == 1 {
            let auto = if sel[0].2 { "　AI委任中" } else { "" };
            tq.add(format!("{}　耐久 {:.0}/{:.0}{}", d.names[side as usize], sel[0].1, d.hp, auto), x0 + 8.0, y0 + 4.0, w0 - 64.0, 26.0, Font::Normal, TEXT);
        } else {
            let mut counts = [0u32; UK_COUNT];
            for s in &sel {
                counts[s.0 as usize] += 1;
            }
            let mut ix = x0 + 6.0;
            for k in BUYABLE {
                let c = counts[k as usize];
                if c > 0 && ix < x1 - 96.0 {
                    draw_unit_icon(g, k, ix + 10.0, y0 + 16.0, -0.6, 8.0, SIDE_COL[side as usize], self.clock);
                    tq.add(format!("{c}"), ix + 20.0, y0 + 5.0, 26.0, 22.0, Font::Normal, TEXT);
                    ix += 46.0;
                }
            }
        }
        // 選択や建設場所選びを解く ✕
        if self.button(g, tq, "✕", x1 - 50.0, y0 + 2.0, 46.0, 34.0, false, true).is_some() {
            self.selected.clear();
            self.mode = Mode::Normal;
        }
        let has_eng = sel.iter().any(|s| s.0 == UK::Engineer);
        let mut acts: Vec<(&str, Action, bool)> = vec![("移動", Action::MoveOnly, self.mode == Mode::MoveOnly), ("地点攻撃", Action::AttackGround, self.mode == Mode::AttackGround), ("停止", Action::Stop, false), ("固守", Action::Hold, false), ("委任", Action::Delegate, false)];
        if has_eng {
            acts.push(("塹壕", Action::Job(JobKind::Trench), self.mode == Mode::Job(JobKind::Trench)));
            acts.push(("地雷", Action::Job(JobKind::Mine), self.mode == Mode::Job(JobKind::Mine)));
            acts.push(("竜の歯", Action::Job(JobKind::Teeth), self.mode == Mode::Job(JobKind::Teeth)));
            acts.push(("浮橋", Action::Job(JobKind::Pontoon), self.mode == Mode::Job(JobKind::Pontoon)));
        }
        let bw = ((w0 - 8.0) / acts.len() as f32).floor() - 2.0;
        for (i, (s, a, on)) in acts.iter().enumerate() {
            let bx = x0 + 4.0 + i as f32 * (bw + 2.0);
            if self.button(g, tq, s, bx, y1 - 48.0, bw, 46.0, *on, true).is_some() {
                self.run_action(*a);
            }
        }
    }

    /// 画面に見えている自軍の部隊を全部選ぶ
    fn select_on_screen(&mut self, g: &dyn Gfx) {
        let Some(side) = self.viewer else { return };
        let (sw, sh) = g.size();
        let (x0, y0) = self.screen_to_world(g, 0.0, self.top_h());
        let (x1, y1) = self.screen_to_world(g, sw, sh);
        let Some(w) = self.world.as_ref() else { return };
        self.selected = w.units.iter().filter(|u| u.side == side && !matches!(u.kind, UK::Fpv | UK::Cruise) && u.x >= x0 && u.x <= x1 && u.y >= y0 && u.y <= y1).map(|u| u.id).collect();
        self.sel_bld = None;
    }

    // ------------------------------------------------------------ 記録

    /// 記録画面（両版共通）
    pub(crate) fn draw_save(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        let (mw, row) = (760.0f32.min(sw - 16.0), if self.compact { 66.0 } else { 74.0 });
        let files = g.has_files();
        let mh: f32 = 58.0 + row * 4.0 + if files { 56.0 } else { 8.0 };
        let (x, y) = self.modal(g, sw, sh, mw, mh.min(sh - 8.0));
        tq.add("記録", x + 20.0, y + 10.0, 200.0, 30.0, Font::Big, ACCENT);
        let in_game = self.screen == Screen::Play && self.world.is_some();
        let mut ry = y + 46.0;
        for slot in 0..4u8 {
            let name = if slot == 0 { "自動記録".to_string() } else { format!("枠 {slot}") };
            let info = g.slot_info(slot);
            g.rect(x + 12.0, ry, x + mw - 12.0, ry + row - 6.0, PANEL2);
            tq.add(name, x + 20.0, ry + 4.0, 110.0, 24.0, Font::Normal, ACCENT);
            let body = info.clone().unwrap_or_else(|| "（空き）".into()).replace('\t', "\n");
            tq.add(body, x + 110.0, ry + 3.0, mw - 110.0 - 300.0, row - 12.0, Font::Small, if info.is_some() { TEXT } else { DIM });
            let (bw, bh) = (88.0, row - 14.0);
            let mut bx = x + mw - 16.0 - bw;
            if files && self.button(g, tq, "書き出す", bx, ry + 4.0, bw, bh, false, info.is_some()).is_some() && info.is_some() {
                g.export_slot(slot);
            }
            if files {
                bx -= bw + 4.0;
            }
            if self.button(g, tq, "読み込む", bx, ry + 4.0, bw, bh, false, info.is_some()).is_some() && info.is_some() {
                if let Some(data) = g.load_slot(slot) {
                    self.apply_loaded(g, &data);
                    return;
                }
            }
            bx -= bw + 4.0;
            if slot > 0 && in_game && self.button(g, tq, "保存", bx, ry + 4.0, bw, bh, false, true).is_some() {
                self.save_to(g, slot);
                self.message(format!("枠 {slot} に記録した"));
            }
            ry += row;
        }
        if files && self.button(g, tq, "ファイルから読み込む", x + 12.0, ry + 2.0, 260.0, 44.0, false, true).is_some() {
            g.import_file();
        }
        if !self.compact && self.button(g, tq, "閉じる", x + mw - 122.0, y + 8.0, 110.0, 32.0, false, true).is_some() {
            self.show_save = false;
        }
        self.pressed = None;
    }
}
