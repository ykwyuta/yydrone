//! HUD とタイトル・結果画面（図形は三角形バッチ、文字は上の層）

use crate::gfx::*;
use crate::view::{draw_unit_icon, SIDE_COL};
use crate::{Game, Mode, Screen};
use yydrone_core::defs::*;
use yydrone_core::geo::*;
use yydrone_core::world::*;

pub const TABS: [&str; 7] = ["地上", "火力", "ドローン", "航空", "海上", "建設", "支援"];
const TAB_KINDS: [Tab; 5] = [Tab::Ground, Tab::Fires, Tab::Drone, Tab::Air, Tab::Sea];

pub(crate) const PANEL: Col = [0.09, 0.10, 0.12, 0.96];
pub(crate) const PANEL2: Col = [0.14, 0.16, 0.19, 1.0];
pub(crate) const LINE: Col = [0.30, 0.34, 0.38, 1.0];
pub(crate) const TEXT: Col = [0.90, 0.92, 0.90, 1.0];
pub(crate) const DIM: Col = [0.62, 0.66, 0.66, 1.0];
pub(crate) const ACCENT: Col = [1.0, 0.82, 0.36, 1.0];
pub(crate) const BAD: Col = [1.0, 0.5, 0.42, 1.0];
const GOOD: Col = [0.55, 1.0, 0.6, 1.0];

const TOP_H: f32 = 44.0;
const BOT_H: f32 = 196.0;
const MINI_W: f32 = 288.0;
const MINI_H: f32 = 180.0;
const PROD_W: f32 = 500.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    Stop,
    Hold,
    MoveOnly,
    AttackGround,
    Delegate,
    Job(JobKind),
}

pub fn action_for_key(k: u32) -> Option<Action> {
    Some(match k {
        0x53 => Action::Stop,         // S
        0x48 => Action::Hold,         // H
        0x51 => Action::MoveOnly,     // Q
        0x47 => Action::AttackGround, // G
        0x5A => Action::Delegate,     // Z
        0x54 => Action::Job(JobKind::Trench),
        0x4D => Action::Job(JobKind::Mine),
        0x42 => Action::Job(JobKind::Teeth),
        0x50 => Action::Job(JobKind::Pontoon),
        _ => return None,
    })
}

/// 文字は図形の上の層に描くので、いったん溜めておく
#[derive(Default)]
pub(crate) struct TextQ(pub(crate) Vec<(String, f32, f32, f32, f32, Font, Col, bool)>);
impl TextQ {
    pub(crate) fn add(&mut self, s: impl Into<String>, x: f32, y: f32, w: f32, h: f32, f: Font, c: Col) {
        self.0.push((s.into(), x, y, w, h, f, c, false));
    }
    pub(crate) fn shadow(&mut self, s: impl Into<String>, x: f32, y: f32, w: f32, h: f32, f: Font, c: Col) {
        self.0.push((s.into(), x, y, w, h, f, c, true));
    }
    pub(crate) fn flush(self, g: &mut dyn Gfx) {
        g.text_begin();
        for (s, x, y, w, h, f, c, sh) in self.0 {
            if sh {
                g.shadow_text(&s, x, y, w, h, f, c);
            } else {
                g.text(&s, x, y, w, h, f, c);
            }
        }
        g.text_end();
    }
}

pub(crate) fn inside(m: (f32, f32), x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    m.0 >= x0 && m.0 < x1 && m.1 >= y0 && m.1 < y1
}

impl Game {
    pub fn over_ui(&self, g: &dyn Gfx, x: f32, y: f32) -> bool {
        let (_, sh) = g.size();
        let inside = |r: &(f32, f32, f32, f32)| x >= r.0 && x < r.2 && y >= r.1 && y < r.3;
        if self.hit_prev.iter().any(inside) || self.bg_prev.iter().any(inside) {
            return true;
        }
        if self.compact {
            return y < self.top_h();
        }
        y < TOP_H || y > sh - BOT_H
    }

    pub(crate) fn top_h(&self) -> f32 {
        if self.compact {
            crate::mobile::TOP_H_COMPACT
        } else {
            TOP_H
        }
    }

    fn bot_h(&self) -> f32 {
        if self.compact {
            120.0
        } else {
            BOT_H
        }
    }

    pub(crate) fn take_click(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) -> Option<u8> {
        self.hit_rects.push((x0, y0, x1, y1));
        if let Some((b, px, py)) = self.pressed {
            if inside((px, py), x0, y0, x1, y1) {
                self.pressed = None;
                return Some(b);
            }
        }
        None
    }

    /// 文字つきボタン。押されたら Some(ボタン番号)
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn button(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, s: &str, x: f32, y: f32, w: f32, h: f32, on: bool, enabled: bool) -> Option<u8> {
        let hover = inside(self.mouse, x, y, x + w, y + h);
        let fill = if on {
            [0.28, 0.34, 0.2, 1.0]
        } else if hover && enabled {
            [0.22, 0.25, 0.29, 1.0]
        } else {
            PANEL2
        };
        g.rect(x, y, x + w, y + h, fill);
        g.rect_outline(x, y, x + w, y + h, 1.0, if on { ACCENT } else { LINE });
        tq.add(s, x, y, w, h, Font::NormalCenter, if enabled { TEXT } else { DIM });
        self.take_click(x, y, x + w, y + h)
    }

    // ------------------------------------------------------------ タイトル

    pub fn draw_title(&mut self, g: &mut dyn Gfx) {
        if self.compact {
            self.draw_title_compact(g);
            return;
        }
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
        // 暗めのグラデーションと陣営アクセントオーバーレイ
        g.rect(0.0, 0.0, sw, sh, [0.03, 0.04, 0.06, 0.58]);
        g.tri((0.0, 0.0), (sw * 0.42, 0.0), (0.0, sh), [0.12, 0.25, 0.5, 0.4]);
        g.tri((sw, sh), (sw * 0.58, sh), (sw, 0.0), [0.5, 0.12, 0.1, 0.4]);

        let mut gx = 0.0;
        while gx < sw {
            g.rect(gx, 0.0, gx + 1.0, sh, [1.0, 1.0, 1.0, 0.02]);
            gx += 48.0;
        }
        let mut gy = 0.0;
        while gy < sh {
            g.rect(0.0, gy, sw, gy + 1.0, [1.0, 1.0, 1.0, 0.02]);
            gy += 48.0;
        }
        // 飛び交うドローン
        let t = self.clock;
        for i in 0..14 {
            let f = i as f32;
            let x = (t * (18.0 + f * 3.0) + f * 173.0) % (sw + 80.0) - 40.0;
            let y = 70.0 + (f * 61.0) % (sh - 140.0) + (t * 0.8 + f).sin() * 10.0;
            let c = SIDE_COL[(i % 2) as usize];
            let k = [UK::ReconDrone, UK::Fpv, UK::BomberDrone, UK::ArmedUav][i % 4];
            draw_unit_icon(g, k, x, y, 0.0, 10.0, [c[0], c[1], c[2], 0.5], t);
        }
        let (cx, mut y) = (sw / 2.0, (sh / 2.0 - 320.0).max(20.0));
        tq.shadow(crate::TITLE, cx - 300.0, y, 600.0, 60.0, Font::Title, [1.0, 0.96, 0.85, 1.0]);
        y += 62.0;
        tq.add("ウクラヴィア＝ロスコヴィア戦争 ── 現代戦リアルタイムストラテジー", cx - 400.0, y, 800.0, 26.0, Font::NormalCenter, DIM);
        y += 44.0;

        // 陣営
        tq.add("指揮する陣営", cx - 400.0, y, 800.0, 22.0, Font::NormalCenter, ACCENT);
        y += 28.0;
        let (cw, ch) = (390.0, 158.0);
        let infos: [(&str, &str, i32); 2] = [
            ("ウクラヴィア共和国", "西側の精密兵器と安価なドローンで戦う防衛側。\n対戦車ミサイル・ハイマル・FPV が強く、\n水上ドローンで敵艦隊を脅かす。支援は時間とともに届く。", 0),
            ("ロスコヴィア連邦", "砲弾と兵力の物量で押す攻撃側。\n自走砲・戦車・戦闘爆撃機が安く、砲弾も豊富。\n黒海艦隊の巡航ミサイルで後方を叩く。", 1),
        ];
        for (i, (name, desc, id)) in infos.iter().enumerate() {
            let x = cx - cw - 10.0 + i as f32 * (cw + 20.0);
            let on = self.cfg_side == *id;
            let col = SIDE_COL[i];
            g.rect(x, y, x + cw, y + ch, [col[0] * 0.2, col[1] * 0.2, col[2] * 0.2, 0.95]);
            g.rect_outline(x, y, x + cw, y + ch, if on { 3.0 } else { 1.0 }, if on { col } else { LINE });
            g.rect(x + 16.0, y + 18.0, x + 38.0, y + 40.0, col);
            tq.add(*name, x + 48.0, y + 14.0, cw - 60.0, 30.0, Font::Big, TEXT);
            tq.add(*desc, x + 16.0, y + 54.0, cw - 32.0, 96.0, Font::Normal, [0.82, 0.85, 0.85, 1.0]);
            if self.take_click(x, y, x + cw, y + ch).is_some() {
                self.cfg_side = *id;
            }
        }
        y += ch + 16.0;

        // 長さ・難易度
        let row = |lab: &str, y: f32, tq: &mut TextQ| tq.add(lab, cx - 390.0, y, 110.0, 34.0, Font::Normal, ACCENT);
        row("プレイ時間", y, &mut tq);
        tq.add("2時間（2022年2月の開戦から2026年10月まで）", cx - 270.0, y, 620.0, 34.0, Font::Normal, TEXT);
        y += 42.0;
        row("敵の強さ", y, &mut tq);
        for (i, s) in ["新兵（敵の収入 75%）", "標準", "精鋭（敵の収入 135%）"].iter().enumerate() {
            if self.button(g, &mut tq, s, cx - 270.0 + i as f32 * 220.0, y, 212.0, 34.0, self.cfg_diff == i, true).is_some() {
                self.cfg_diff = i;
            }
        }
        y += 52.0;
        if self.button(g, &mut tq, "作戦開始", cx - 170.0, y, 340.0, 50.0, true, true).is_some() {
            self.start_game(g);
            return;
        }
        y += 60.0;
        if self.button(g, &mut tq, "続きから", cx - 170.0, y, 166.0, 34.0, false, true).is_some() {
            self.show_save = true;
        }
        if self.button(g, &mut tq, "AI 同士を観戦", cx + 4.0, y, 166.0, 34.0, false, true).is_some() {
            self.cfg_side = -1;
            self.start_game(g);
            self.cfg_side = 0;
            return;
        }
        y += 44.0;
        tq.add("時間切れの時点で支配領域が広い陣営の勝ち。すべての国名・地名・兵器名は架空です。", cx - 450.0, y, 900.0, 24.0, Font::NormalCenter, DIM);
        if self.show_save {
            tq.0.clear();
            self.pressed = press;
            self.draw_save(g, &mut tq, sw, sh);
        }
        tq.flush(g);
    }

    // ------------------------------------------------------------ 結果

    pub fn draw_result(&mut self, g: &mut dyn Gfx) {
        let (sw, sh) = g.size();
        let mut tq = TextQ::default();
        g.rect(0.0, 0.0, sw, sh, [0.06, 0.07, 0.08, 1.0]);
        let Some(w) = self.world.as_ref() else {
            tq.flush(g);
            return;
        };
        let viewer = self.viewer;
        let c = self.compact;
        let (cx, mut y) = (sw / 2.0, if c { 2.0 } else { (sh / 2.0 - 300.0).max(20.0) });
        let (s0, s1) = (w.share(0), w.share(1));
        let (head, col) = match (w.winner, viewer) {
            (None, _) => ("停戦 ── 引き分け".to_string(), TEXT),
            (Some(a), Some(v)) if a == v => ("勝利".to_string(), GOOD),
            (Some(_), Some(_)) => ("敗北".to_string(), BAD),
            (Some(a), None) => (format!("{}の勝利", SIDE_FULL[a as usize]), SIDE_COL[a as usize]),
        };
        tq.shadow(head, cx - 400.0, y, 800.0, if c { 44.0 } else { 60.0 }, if c { Font::TitleMid } else { Font::Title }, col);
        y += if c { 44.0 } else { 66.0 };
        if let Some(a) = w.winner {
            tq.add(format!("{} が国土の {:.1}% を支配して終戦を迎えた", SIDE_FULL[a as usize], w.share(a) * 100.0), cx - 400.0, y, 800.0, 26.0, Font::NormalCenter, TEXT);
        }
        y += if c { 28.0 } else { 40.0 };
        // 領土バー
        let (bw, bh) = (700.0, 26.0);
        let bx = cx - bw / 2.0;
        g.rect(bx, y, bx + bw * s0, y + bh, SIDE_COL[0]);
        g.rect(bx + bw * s0, y, bx + bw, y + bh, SIDE_COL[1]);
        tq.shadow(format!("{} {:.1}%", SIDE_NAMES[0], s0 * 100.0), bx + 8.0, y + 2.0, 300.0, 24.0, Font::Normal, TEXT);
        tq.shadow(format!("{} {:.1}%", SIDE_NAMES[1], s1 * 100.0), bx + bw - 208.0, y + 2.0, 200.0, 24.0, Font::Normal, TEXT);
        y += if c { 32.0 } else { 44.0 };
        // 領土の推移
        let (cw, ch) = (700.0, if c { 110.0 } else { 200.0 });
        g.rect(bx, y, bx + cw, y + ch, PANEL);
        g.rect(bx, y + ch / 2.0, bx + cw, y + ch / 2.0 + 1.0, LINE);
        let tmax = w.duration;
        let pts: Vec<(f32, f32)> = w.history.iter().map(|(t, s)| (bx + cw * (t / tmax), y + ch * (1.0 - s))).collect();
        for p in pts.windows(2) {
            g.tri(p[0], p[1], (p[1].0, y + ch), [SIDE_COL[0][0], SIDE_COL[0][1], SIDE_COL[0][2], 0.35]);
            g.tri(p[0], (p[1].0, y + ch), (p[0].0, y + ch), [SIDE_COL[0][0], SIDE_COL[0][1], SIDE_COL[0][2], 0.35]);
            g.tri(p[0], p[1], (p[1].0, y), [SIDE_COL[1][0], SIDE_COL[1][1], SIDE_COL[1][2], 0.22]);
            g.tri(p[0], (p[1].0, y), (p[0].0, y), [SIDE_COL[1][0], SIDE_COL[1][1], SIDE_COL[1][2], 0.22]);
            g.line(p[0], p[1], 2.0, TEXT);
        }
        tq.add("領土の推移（50% の線が互角）", bx + 8.0, y + 4.0, 400.0, 22.0, Font::Small, DIM);
        y += ch + if c { 6.0 } else { 14.0 };
        let st = |s: usize| &w.sides[s];
        let lines = [
            ("撃破", st(0).kills.to_string(), st(1).kills.to_string()),
            ("損失", st(0).losses.to_string(), st(1).losses.to_string()),
            ("制圧都市", w.map.cities.iter().filter(|c| c.owner == 0).count().to_string(), w.map.cities.iter().filter(|c| c.owner == 1).count().to_string()),
        ];
        for (name, a, b) in lines {
            tq.add(name, cx - 100.0, y, 200.0, 24.0, Font::NormalCenter, DIM);
            tq.add(a, cx - 260.0, y, 160.0, 24.0, Font::NormalCenter, SIDE_COL[0]);
            tq.add(b, cx + 100.0, y, 160.0, 24.0, Font::NormalCenter, SIDE_COL[1]);
            y += if c { 24.0 } else { 26.0 };
        }
        y += if c { 6.0 } else { 12.0 };
        if self.button(g, &mut tq, "タイトルへ戻る", cx - 150.0, y, 300.0, if c { 48.0 } else { 42.0 }, false, true).is_some() {
            self.screen = Screen::Title;
            self.world = None;
        }
        tq.flush(g);
    }

    // ------------------------------------------------------------ 戦闘中の画面

    pub fn draw_hud(&mut self, g: &mut dyn Gfx) {
        let (sw, sh) = g.size();
        let mut tq = TextQ::default();
        // 説明画面が開いているときは、後ろのボタンを押せないようにする
        let modal_press = if self.modal_open() { self.pressed.take() } else { None };
        self.draw_labels(g, &mut tq);
        self.draw_marquee(g);
        if self.compact {
            self.draw_top_compact(g, &mut tq, sw);
            self.draw_bottom_compact(g, &mut tq, sw, sh);
        } else {
            self.draw_top(g, &mut tq, sw);
            self.draw_bottom(g, &mut tq, sw, sh);
        }
        self.draw_ticker(g, &mut tq);
        self.draw_mode_hint(g, &mut tq, sw, sh);
        self.draw_banner(g, &mut tq, sw);
        if self.modal_open() {
            tq.0.clear();
            self.pressed = modal_press;
            self.tip = None;
            self.close_button_press(g);
        }
        if self.show_brief {
            self.draw_brief(g, &mut tq, sw, sh);
        } else if self.show_save {
            self.draw_save(g, &mut tq, sw, sh);
        } else if self.show_help {
            if self.help_scrolls(g) {
                self.draw_help_scrolled(g, &mut tq, sw, sh);
            } else {
                self.draw_help(g, &mut tq, sw, sh);
            }
        } else if self.show_chron {
            self.draw_chronicle(g, &mut tq, sw, sh);
        }
        if self.modal_open() {
            self.close_button_draw(g, &mut tq);
        }
        if !self.modal_open() && self.tip.is_none() && !self.over_ui(g, self.mouse.0, self.mouse.1) {
            let (mx, my) = self.screen_to_world(g, self.mouse.0, self.mouse.1);
            if let Some(w) = self.world.as_ref() {
                for c in &w.map.cities {
                    let d = ((c.x - mx).powi(2) + (c.y - my).powi(2)).sqrt();
                    let hit_r = (1.2 + c.value as f32 * 0.25).max(1.5);
                    if d <= hit_r {
                        let kind_str = if c.value >= 5 {
                            "首都 / 司令部中枢"
                        } else if c.port {
                            "重要軍港 / 補給港湾"
                        } else if c.value >= 4 {
                            "主要工業拠点 / 重工業都市"
                        } else {
                            "前線拠点 / 交通結節点"
                        };
                        let owner_str = SIDE_NAMES[c.owner as usize];
                        self.tip = Some(format!(
                            "【{}】 {}\n支配: {}\n戦略価値: ★{}\n都市経済貢献: +{:.1}/s",
                            c.name, kind_str, owner_str, c.value, c.value as f32 * 0.65
                        ));
                        break;
                    }
                }
                if self.tip.is_none() {
                    for b in &w.blds {
                        if self.viewer.is_some_and(|v| b.side != v && !b.seen[v as usize]) {
                            continue;
                        }
                        let d = ((b.x - mx).powi(2) + (b.y - my).powi(2)).sqrt();
                        if d <= 1.4 {
                            let bd = bdef(b.kind);
                            let owner_str = SIDE_NAMES[b.side as usize];
                            let st = if b.building > 0.0 { " [建設中]" } else { "" };
                            self.tip = Some(format!(
                                "【{}】{}\n所属: {}\n耐久: {:.0}/{:.0}\n{}",
                                bd.name, st, owner_str, b.hp, b.max_hp, bd.desc
                            ));
                            break;
                        }
                    }
                }
            }
        }
        self.draw_tip(g, &mut tq, sw, sh);
        tq.flush(g);
    }

    fn draw_labels(&mut self, g: &mut dyn Gfx, tq: &mut TextQ) {
        if self.modal_open() {
            return;
        }
        let (sw, sh) = g.size();
        let Some(w) = self.world.as_ref() else { return };
        let z = self.zoom;
        // パネルや見出しの裏には地図の文字を出さない（文字は図形より上の層に描かれるので）
        let (top, bot) = (self.top_h(), if self.compact { 0.0 } else { BOT_H });
        let cover: Vec<(f32, f32, f32, f32)> = self.bg_prev.iter().chain(&self.hit_prev).copied().collect();
        let vis = |x: f32, y: f32| {
            x > -60.0 && x < sw + 60.0 && y > top + 10.0 && y < sh - bot - 30.0 && !cover.iter().any(|r| x > r.0 - 40.0 && x < r.2 + 40.0 && y > r.1 - 24.0 && y < r.3 + 6.0)
        };
        for c in &w.map.cities {
            if (c.value as f32) < 3.2 - z * 0.3 {
                continue;
            }
            let (x, y) = self.world_to_screen(g, c.x, c.y);
            if vis(x, y) {
                let yoff = if z >= 3.0 { (1.8 + c.value as f32 * 0.4).min(3.8) * 0.33 * z + 4.0 } else { 9.0 };
                tq.shadow(c.name, x - 60.0, y + yoff, 120.0, 18.0, Font::NormalCenter, [1.0, 0.98, 0.9, 1.0]);
            }
        }
        for l in &w.map.labels {
            if z >= 3.0 && (l.kind == 0 || l.kind == 3) {
                // 拡大戦術ビューでは国家名などの大文字が部隊や拠点と重ならないよう非表示
                continue;
            }
            if z >= 4.5 {
                // 最大拡大時は戦術戦闘に専念できるよう広域地名ラベルを非表示
                continue;
            }
            let (x, y) = self.world_to_screen(g, l.x, l.y);
            if !vis(x, y) {
                continue;
            }
            let c: Col = match l.kind {
                0 => [1.0, 1.0, 1.0, 0.3],
                1 => [1.0, 1.0, 1.0, 0.42],
                2 => [0.6, 0.82, 1.0, 0.55],
                _ => [0.8, 0.8, 0.8, 0.4],
            };
            let f = if l.kind == 0 { Font::BigCenter } else { Font::NormalCenter };
            tq.add(l.text, x - 120.0, y - 14.0, 240.0, 28.0, f, c);
        }
    }

    fn draw_marquee(&mut self, g: &mut dyn Gfx) {
        if let Some((a, b)) = self.drag_box() {
            g.rect(a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1), [0.5, 1.0, 0.5, 0.12]);
            g.rect_outline(a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1), 1.0, [0.5, 1.0, 0.5, 0.9]);
        }
        if let Some((a, b)) = self.job_line() {
            g.line(a, b, 3.0, [1.0, 0.8, 0.3, 0.85]);
        }
        // 設置モードの目印
        if let Mode::Place(k) = self.mode {
            let m = if self.touch { self.place_tap } else { Some(self.mouse) };
            if let Some(m) = m {
                let (wx, wy) = self.screen_to_world(g, m.0, m.1);
                let ok = self.viewer.zip(self.world.as_ref()).map(|(v, w)| w.can_place(v, k, wx, wy).is_ok()).unwrap_or(false);
                let s = 0.95 * self.zoom;
                let c: Col = if ok { [0.4, 1.0, 0.5, 0.55] } else { [1.0, 0.3, 0.25, 0.55] };
                g.rect(m.0 - s, m.1 - s, m.0 + s, m.1 + s, c);
            }
        }
        if let Mode::Power(p) = self.mode {
            let r = match p {
                Power::Cruise => 2.4,
                Power::Satellite => 24.0,
                Power::Mobilize => 2.0,
            } * self.zoom;
            if let Some(m) = if self.touch { self.place_tap } else { Some(self.mouse) } {
                g.ring(m.0, m.1, r, 2.0, 40, [1.0, 0.6, 0.3, 0.8]);
            }
        }
    }

    fn draw_top(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32) {
        g.rect(0.0, 0.0, sw, TOP_H, PANEL);
        g.rect(0.0, TOP_H - 1.0, sw, TOP_H, LINE);
        let Some(w) = self.world.as_ref() else { return };
        let viewer = self.viewer;
        let (s0, s1) = (w.share(0), w.share(1));
        let side_info = viewer.map(|v| {
            let s = &w.sides[v as usize];
            (s.funds, s.income, s.shells, s.shell_income)
        });
        let left = (w.duration - w.t).max(0.0);
        let date = w.date_text();
        let mud = w.mud();
        let cond = w.conditions_text();
        let cond_help = w.conditions_help();
        // 領土バー（中央）
        let bw = (sw - 340.0 - 372.0 - 280.0).clamp(240.0, 420.0);
        let bx = (sw / 2.0 - bw / 2.0).clamp(372.0, sw - 340.0 - 280.0 - bw);
        g.rect(bx, 6.0, bx + bw * s0, 32.0, SIDE_COL[0]);
        g.rect(bx + bw * s0, 6.0, bx + bw, 32.0, SIDE_COL[1]);
        g.rect(bx + bw / 2.0 - 1.0, 3.0, bx + bw / 2.0 + 1.0, 35.0, [1.0, 1.0, 1.0, 0.9]);
        tq.shadow(format!("{:.1}%", s0 * 100.0), bx + 8.0, 8.0, 80.0, 22.0, Font::Normal, TEXT);
        tq.shadow(format!("{:.1}%", s1 * 100.0), bx + bw - 62.0, 8.0, 60.0, 22.0, Font::Normal, TEXT);
        tq.shadow("支配領域", bx + bw / 2.0 - 60.0, 7.0, 120.0, 22.0, Font::NormalCenter, TEXT);
        // 左：資源
        if let Some((f, fi, sh, si)) = side_info {
            tq.add(format!("資金 {:.0}  (+{:.0}/秒)", f, fi), 12.0, 10.0, 180.0, 24.0, Font::Normal, ACCENT);
            tq.add(format!("砲弾 {:.0}  (+{:.1}/秒)", sh, si), 192.0, 10.0, 170.0, 24.0, Font::Normal, if sh < 30.0 { BAD } else { TEXT });
        } else {
            tq.add("観戦中（霧なし）", 12.0, 10.0, 200.0, 24.0, Font::Normal, ACCENT);
        }
        // 右：時間
        let rx = bx + bw + 14.0;
        let total = left as u32;
        let (hh, m, sc) = (total / 3600, total / 60 % 60, total % 60);
        tq.add(format!("{date}  {cond}"), rx, 10.0, 186.0, 24.0, Font::Normal, if mud { BAD } else { TEXT });
        if inside(self.mouse, rx, 0.0, rx + 186.0, TOP_H) {
            self.tip = Some(cond_help);
            self.panel_top = TOP_H + 130.0;
        }
        tq.add(format!("残り {hh}:{m:02}:{sc:02}"), rx + 186.0, 10.0, 90.0, 24.0, Font::Normal, if left < 60.0 { BAD } else { ACCENT });
        // 速度
        if self.button(g, tq, "記録", sw - 292.0, 8.0, 44.0, 28.0, self.show_save, true).is_some() {
            self.show_save = !self.show_save;
        }
        if self.button(g, tq, "年表", sw - 244.0, 8.0, 44.0, 28.0, self.show_chron, true).is_some() {
            self.show_chron = !self.show_chron;
        }
        let mut bxx = sw - 196.0;
        let paused = self.paused;
        if self.button(g, tq, if paused { "再開" } else { "停止" }, bxx, 8.0, 44.0, 28.0, paused, true).is_some() {
            self.paused = !self.paused;
        }
        bxx += 48.0;
        let speed = self.speed;
        for sp in [1u32, 2, 4] {
            if self.button(g, tq, &format!("×{sp}"), bxx, 8.0, 32.0, 28.0, speed == sp, true).is_some() {
                self.speed = sp;
            }
            bxx += 34.0;
        }
        if self.button(g, tq, "?", bxx + 2.0, 8.0, 32.0, 28.0, self.show_help, true).is_some() {
            self.show_help = !self.show_help;
        }
    }

    pub fn minimap_rect(&self, g: &dyn Gfx) -> (f32, f32, f32, f32) {
        if self.compact {
            return self.minimap_rect_compact(g);
        }
        let (_, sh) = g.size();
        let (x, y) = (8.0, sh - BOT_H + 8.0);
        (x, y, x + MINI_W, y + MINI_H)
    }

    pub fn minimap_jump(&mut self, g: &dyn Gfx, x: f32, y: f32) {
        let (x0, y0, x1, y1) = self.minimap_rect(g);
        let (fx, fy) = (((x - x0) / (x1 - x0)).clamp(0.0, 1.0), ((y - y0) / (y1 - y0)).clamp(0.0, 1.0));
        self.cam = (fx * W as f32, fy * H as f32);
    }

    fn draw_bottom(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        let top = sh - BOT_H;
        self.panel_top = top;
        g.rect(0.0, top, sw, sh, PANEL);
        g.rect(0.0, top, sw, top + 1.0, LINE);
        self.draw_minimap(g);
        let (mx0, my0, mx1, my1) = self.minimap_rect(g);
        if self.pressed.is_some_and(|(b, x, y)| b == 0 && inside((x, y), mx0, my0, mx1, my1)) {
            let m = self.mouse;
            self.pressed = None;
            self.minimap_jump(g, m.0, m.1);
            self.minimap_drag = true;
        }
        // 右クリック：ミニマップ上で命令
        if let Some((1, x, y)) = self.pressed {
            if inside((x, y), mx0, my0, mx1, my1) {
                self.pressed = None;
                let tx = (x - mx0) / MINI_W * W as f32;
                let ty = (y - my0) / MINI_H * H as f32;
                let ids = self.selected.clone();
                if let Some(wd) = self.world.as_mut() {
                    wd.command(&ids, Cmd::Auto(false), 0.0, 0.0);
                    wd.command(&ids, Cmd::AttackMove, tx, ty);
                    self.cmd_pause = false;
                }
            }
        }
        let cx0 = MINI_W + 18.0;
        let cx1 = sw - PROD_W - 10.0;
        self.draw_selection(g, tq, cx0, top + 8.0, cx1 - cx0, BOT_H - 16.0);
        self.draw_production(g, tq, sw - PROD_W - 4.0, top + 6.0, PROD_W, BOT_H - 12.0);
    }

    pub(crate) fn draw_minimap(&mut self, g: &mut dyn Gfx) {
        let (x0, y0, x1, y1) = self.minimap_rect(g);
        g.rect(x0 - 1.0, y0 - 1.0, x1 + 1.0, y1 + 1.0, LINE);
        let (tt, ot) = (self.terrain_tex, self.overlay_tex);
        g.draw_textured(tt, x0, y0, x1, y1, [0.85, 0.85, 0.85, 1.0], true);
        g.draw_textured(ot, x0, y0, x1, y1, [1.0; 4], true);
        let (kx, ky) = ((x1 - x0) / W as f32, (y1 - y0) / H as f32);
        let Some(w) = self.world.as_ref() else { return };
        for c in &w.map.cities {
            let (x, y) = (x0 + c.x * kx, y0 + c.y * ky);
            g.rect(x - 1.5, y - 1.5, x + 1.5, y + 1.5, [1.0, 1.0, 1.0, 0.9]);
        }
        for u in &w.units {
            if matches!(u.kind, UK::Fpv) || self.viewer.is_some_and(|v| !w.can_see(v, u)) {
                continue;
            }
            let (x, y) = (x0 + u.x * kx, y0 + u.y * ky);
            let c = SIDE_COL[u.side as usize];
            g.rect(x - 1.0, y - 1.0, x + 1.0, y + 1.0, [c[0] * 0.85, c[1] * 0.85, c[2] * 0.85, 1.0]);
        }
        let (sw, sh) = g.size();
        let (ax, ay) = self.screen_to_world(g, 0.0, 0.0);
        let (bx, by) = self.screen_to_world(g, sw, sh);
        let f = |v: f32, lo: f32, hi: f32| v.clamp(lo, hi);
        g.rect_outline(f(x0 + ax * kx, x0, x1), f(y0 + ay * ky, y0, y1), f(x0 + bx * kx, x0, x1), f(y0 + by * ky, y0, y1), 1.0, [1.0, 1.0, 1.0, 0.85]);
    }

    fn draw_selection(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, x: f32, y: f32, w0: f32, h: f32) {
        g.rect(x, y, x + w0, y + h, [0.11, 0.12, 0.15, 1.0]);
        let Some(side) = self.viewer else {
            tq.add("観戦モード：両軍とも AI が指揮します。\nホイールで拡大、矢印キーで移動。", x + 10.0, y + 10.0, w0 - 20.0, 60.0, Font::Normal, DIM);
            return;
        };
        // 建物の情報
        if let Some(bid) = self.sel_bld {
            let info = self.world.as_ref().and_then(|w| w.bld(bid)).map(|b| (b.kind, b.hp, b.max_hp, b.building, b.queue.iter().copied().collect::<Vec<_>>(), b.prog));
            if let Some((k, hp, max, building, queue, prog)) = info {
                let d = bdef(k);
                self.art.draw_bld_sprite(g, k, x + w0 - 36.0, y + 36.0, 52.0, [1.0; 4]);
                tq.add(d.name, x + 10.0, y + 6.0, w0 - 70.0, 26.0, Font::Big, TEXT);
                tq.add(format!("耐久 {:.0}/{:.0}", hp, max), x + 10.0, y + 34.0, w0 - 70.0, 22.0, Font::Normal, DIM);
                tq.add(d.desc, x + 10.0, y + 56.0, w0 - 70.0, 40.0, Font::Small, DIM);
                if building > 0.0 {
                    tq.add(format!("建設中… 残り {building:.0} 秒"), x + 10.0, y + 100.0, w0 - 20.0, 22.0, Font::Normal, ACCENT);
                } else if !queue.is_empty() {
                    let t = def(queue[0]).time;
                    g.rect(x + 10.0, y + 102.0, x + w0 - 10.0, y + 110.0, PANEL2);
                    g.rect(x + 10.0, y + 102.0, x + 10.0 + (w0 - 20.0) * (prog / t).clamp(0.0, 1.0), y + 110.0, ACCENT);
                    let names: Vec<&str> = queue.iter().map(|k| def(*k).short).collect();
                    tq.add(format!("生産中：{}", names.join(" ")), x + 10.0, y + 114.0, w0 - 20.0, 44.0, Font::Small, TEXT);
                } else {
                    tq.add("右クリックで集結地点を設定", x + 10.0, y + 100.0, w0 - 20.0, 22.0, Font::Small, DIM);
                }
            }
            return;
        }
        let w = self.world.as_ref();
        let sel: Vec<(UK, f32, bool, bool)> = self.selected.iter().filter_map(|id| w.and_then(|w| w.unit(*id))).map(|u| (u.kind, u.hp, u.auto, u.supplied)).collect();
        if sel.is_empty() {
            let (blds, units) = w.map_or((0, 0), |w| (w.blds.iter().filter(|b| b.side == side).count(), w.units.iter().filter(|u| u.side == side && !matches!(u.kind, UK::Fpv | UK::Cruise)).count()));
            tq.add(format!("保有部隊 {units}　建物 {blds}"), x + 10.0, y + 8.0, w0 - 20.0, 24.0, Font::Normal, TEXT);
            tq.add("部隊は AI の将軍に委任中。命令すると直接指揮に\n左ドラッグで部隊を選択\n右クリックで交戦しつつ前進／敵の上で攻撃\nF1 で操作説明", x + 10.0, y + 36.0, w0 - 20.0, 90.0, Font::Normal, DIM);
            let on = self.world.as_ref().is_some_and(|w| w.sides[side as usize].auto_buy);
            let label = if on { "生産と建設を AI に委任中（押すと自分で行う）" } else { "生産と建設も AI に委任する" };
            if self.button(g, tq, label, x + 10.0, y + h - 36.0, w0 - 20.0, 30.0, on, true).is_some() {
                if let Some(w) = self.world.as_mut() {
                    w.sides[side as usize].auto_buy = !on;
                }
            }
            return;
        }
        // 種類ごとの数
        let mut counts = [0u32; UK_COUNT];
        for s in &sel {
            counts[s.0 as usize] += 1;
        }
        let first = sel[0].0;
        let d = def(first);
        let single = sel.len() == 1;
        let name = if single { d.names[side as usize].to_string() } else { format!("{} 部隊を選択中", sel.len()) };
        tq.add(name, x + 10.0, y + 4.0, w0 - 20.0, 26.0, Font::Big, TEXT);
        if single {
            self.art.draw_unit_sprite(g, first, x + w0 - 36.0, y + 40.0, 52.0, [1.0; 4]);
            tq.add(format!("耐久 {:.0}/{:.0}{}{}", sel[0].1, d.hp, if sel[0].2 { "　AI委任中" } else { "" }, if !sel[0].3 { "　補給切れ" } else { "" }), x + 10.0, y + 30.0, w0 - 70.0, 22.0, Font::Small, if sel[0].3 { DIM } else { BAD });
            tq.add(d.desc, x + 10.0, y + 50.0, w0 - 70.0, 40.0, Font::Small, DIM);
        } else {
            let mut ix = x + 10.0;
            let mut iy = y + 34.0;
            for k in BUYABLE {
                let c = counts[k as usize];
                if c == 0 {
                    continue;
                }
                if !self.art.draw_unit_sprite(g, k, ix + 12.0, iy + 12.0, 24.0, [1.0; 4]) {
                    draw_unit_icon(g, k, ix + 12.0, iy + 12.0, -0.6, 8.0, SIDE_COL[side as usize], self.clock);
                }
                tq.add(format!("{c}"), ix + 22.0, iy + 4.0, 24.0, 20.0, Font::Small, TEXT);
                ix += 46.0;
                if ix > x + w0 - 50.0 {
                    ix = x + 10.0;
                    iy += 26.0;
                }
            }
        }
        // 命令ボタン
        let has_eng = sel.iter().any(|s| s.0 == UK::Engineer);
        let mut acts: Vec<(&str, Action, bool)> = vec![("移動 Q", Action::MoveOnly, self.mode == Mode::MoveOnly), ("地点攻撃 G", Action::AttackGround, self.mode == Mode::AttackGround), ("停止 S", Action::Stop, false), ("固守 H", Action::Hold, false), ("委任 Z", Action::Delegate, false)];
        if has_eng {
            acts.push(("塹壕 T", Action::Job(JobKind::Trench), self.mode == Mode::Job(JobKind::Trench)));
            acts.push(("地雷 M", Action::Job(JobKind::Mine), self.mode == Mode::Job(JobKind::Mine)));
            acts.push(("竜の歯 B", Action::Job(JobKind::Teeth), self.mode == Mode::Job(JobKind::Teeth)));
            acts.push(("浮橋 P", Action::Job(JobKind::Pontoon), self.mode == Mode::Job(JobKind::Pontoon)));
        }
        let bw = ((w0 - 20.0) / 5.0).floor() - 2.0;
        for (i, (s, a, on)) in acts.iter().enumerate() {
            let (cx, cy) = (x + 10.0 + (i % 5) as f32 * (bw + 2.0), y + h - 66.0 + (i / 5) as f32 * 32.0);
            if self.button(g, tq, s, cx, cy, bw, 30.0, *on, true).is_some() {
                self.run_action(*a);
            }
        }
    }

    pub(crate) fn draw_production(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, x: f32, y: f32, w0: f32, h: f32) {
        let Some(side) = self.viewer else { return };
        let tw = w0 / TABS.len() as f32;
        for (i, t) in TABS.iter().enumerate() {
            if self.button(g, tq, t, x + i as f32 * tw, y, tw - 2.0, if self.compact { 38.0 } else { 26.0 }, self.tab == i, true).is_some() {
                self.tab = i;
            }
        }
        let (gx, gy) = (x, y + if self.compact { 42.0 } else { 30.0 });
        let (bw, bh) = ((w0 / 4.0).floor() - 2.0, if self.compact { 58.0 } else { 74.0 });
        let at = |i: usize| (gx + (i % 4) as f32 * (bw + 2.0), gy + (i / 4) as f32 * (bh + 2.0));
        let funds = self.world.as_ref().map_or(0.0, |w| w.sides[side as usize].funds);
        if self.tab < 5 {
            let kind = TAB_KINDS[self.tab];
            let kinds: Vec<UK> = BUYABLE.iter().copied().filter(|k| def(*k).tab == kind && def(*k).sides[side as usize]).collect();
            for (i, k) in kinds.iter().enumerate() {
                let d = def(*k);
                let cost = unit_cost(side, *k);
                let (cx, cy) = at(i);
                let afford = funds >= cost;
                let hover = inside(self.mouse, cx, cy, cx + bw, cy + bh);
                g.rect(cx, cy, cx + bw, cy + bh, if hover { [0.2, 0.23, 0.27, 1.0] } else { PANEL2 });
                g.rect_outline(cx, cy, cx + bw, cy + bh, 1.0, LINE);
                let col = if afford { SIDE_COL[side as usize] } else { [0.45, 0.47, 0.5, 1.0] };
                if !self.art.draw_unit_sprite(g, *k, cx + 22.0, cy + 24.0, 32.0, if afford { [1.0, 1.0, 1.0, 1.0] } else { [0.45, 0.47, 0.5, 0.7] }) {
                    draw_unit_icon(g, *k, cx + 22.0, cy + 26.0, -0.5, 14.0, col, self.clock);
                }
                tq.add(d.short, cx + 42.0, cy + 6.0, bw - 44.0, 20.0, Font::Normal, if afford { TEXT } else { DIM });
                tq.add(format!("{cost:.0}"), cx + 42.0, cy + 28.0, bw - 44.0, 20.0, Font::Small, if afford { ACCENT } else { BAD });
                let q = self.world.as_ref().map_or(0, |w| w.queued_count(side, *k));
                if q > 0 {
                    tq.add(format!("予約 {q}"), cx + 6.0, cy + bh - 22.0, 60.0, 20.0, Font::Small, GOOD);
                    // タッチには右クリックがないので、取り消しのボタンを出す
                    if self.compact {
                        let (rx, ry) = (cx + bw - 34.0, cy + 2.0);
                        g.rect(rx, ry, rx + 32.0, ry + 32.0, [0.3, 0.1, 0.1, 1.0]);
                        tq.add("✕", rx, ry, 32.0, 32.0, Font::NormalCenter, TEXT);
                        if self.take_click(rx, ry, rx + 32.0, ry + 32.0).is_some() {
                            if let Some(w) = self.world.as_mut() {
                                w.cancel_unit(side, *k);
                            }
                        }
                    }
                }
                if hover {
                    let need = prod_building(d.prod).map(|b| bdef(b).name).unwrap_or("");
                    self.tip = Some(format!("{}\n{}\n価格 {:.0}　生産 {:.0} 秒　生産場所：{}\n左クリックで生産、右クリックで取り消し", d.names[side as usize], d.desc, cost, d.time, need));
                }
                match self.take_click(cx, cy, cx + bw, cy + bh) {
                    Some(0) => {
                        if let Some(w) = self.world.as_mut() {
                            if let Err(e) = w.queue_unit(side, *k) {
                                self.message(e);
                            }
                        }
                    }
                    Some(1) => {
                        if let Some(w) = self.world.as_mut() {
                            w.cancel_unit(side, *k);
                        }
                    }
                    _ => {}
                }
            }
        } else if self.tab == 5 {
            for (i, k) in BK_ALL.iter().enumerate() {
                let d = bdef(*k);
                let (cx, cy) = at(i);
                let afford = funds >= d.cost;
                let on = self.mode == Mode::Place(*k);
                let hover = inside(self.mouse, cx, cy, cx + bw, cy + bh);
                g.rect(cx, cy, cx + bw, cy + bh, if on { [0.28, 0.34, 0.2, 1.0] } else if hover { [0.2, 0.23, 0.27, 1.0] } else { PANEL2 });
                g.rect_outline(cx, cy, cx + bw, cy + bh, 1.0, if on { ACCENT } else { LINE });
                let bcol = if afford { [1.0, 1.0, 1.0, 1.0] } else { [0.45, 0.47, 0.5, 0.7] };
                self.art.draw_bld_sprite(g, *k, cx + bw - 20.0, cy + 24.0, 32.0, bcol);
                tq.add(d.name, cx + 6.0, cy + 8.0, bw - 38.0, 22.0, Font::Normal, if afford { TEXT } else { DIM });
                tq.add(format!("{:.0}", d.cost), cx + 6.0, cy + 32.0, bw - 38.0, 20.0, Font::Small, if afford { ACCENT } else { BAD });
                if hover {
                    self.tip = Some(format!("{}\n{}\n価格 {:.0}　建設 {:.0} 秒\n自国の領土に置く", d.name, d.desc, d.cost, d.time));
                }
                if self.take_click(cx, cy, cx + bw, cy + bh) == Some(0) {
                    self.mode = if on { Mode::Normal } else { Mode::Place(*k) };
                    self.place_tap = None;
                    // 詰めた画面では地図が見えるようにパネルを閉じる
                    self.show_prod = false;
                }
            }
        } else {
            for (i, p) in POWERS.iter().enumerate() {
                let d = pdef(*p);
                let (cx, cy) = at(i);
                let cd = self.world.as_ref().map_or(0.0, |w| w.sides[side as usize].power_cd[*p as usize]);
                let afford = funds >= d.cost && cd <= 0.0;
                let on = self.mode == Mode::Power(*p);
                let hover = inside(self.mouse, cx, cy, cx + bw, cy + bh);
                g.rect(cx, cy, cx + bw, cy + bh, if on { [0.28, 0.34, 0.2, 1.0] } else if hover { [0.2, 0.23, 0.27, 1.0] } else { PANEL2 });
                g.rect_outline(cx, cy, cx + bw, cy + bh, 1.0, if on { ACCENT } else { LINE });
                tq.add(d.name, cx + 6.0, cy + 8.0, bw - 10.0, 22.0, Font::Normal, if afford { TEXT } else { DIM });
                let sub = if cd > 0.0 { format!("準備中 {cd:.0}秒") } else { format!("{:.0}", d.cost) };
                tq.add(sub, cx + 6.0, cy + 32.0, bw - 10.0, 20.0, Font::Small, if afford { ACCENT } else { BAD });
                if hover {
                    self.tip = Some(format!("{}\n{}\n価格 {:.0}　再使用まで {:.0} 秒", d.name, d.desc, d.cost, d.cooldown));
                }
                if self.take_click(cx, cy, cx + bw, cy + bh) == Some(0) {
                    self.mode = if on { Mode::Normal } else { Mode::Power(*p) };
                    self.place_tap = None;
                    self.show_prod = false;
                }
            }
        }
        // ボタン以外のパネルのクリックは吸収する
        let _ = h;
    }

    fn draw_ticker(&mut self, g: &mut dyn Gfx, tq: &mut TextQ) {
        let mut y = self.top_h() + 8.0;
        // 詰めた画面では見出しと重ならないよう短く
        let (count, w) = if self.compact { (3, 300.0) } else { (7, 470.0) };
        let items: Vec<(String, f32, u8, Option<(f32, f32)>)> = self.ticker.iter().take(count).map(|t| (t.text.clone(), t.t, t.tone, t.pos)).collect();
        for (text, t, tone, pos) in items {
            let a = (t / 2.0).min(1.0);
            let col = match tone {
                1 => [GOOD[0], GOOD[1], GOOD[2], a],
                2 => [BAD[0], BAD[1], BAD[2], a],
                _ => [TEXT[0], TEXT[1], TEXT[2], a],
            };
            g.rect(8.0, y, 8.0 + w, y + 24.0, [0.0, 0.0, 0.0, 0.5 * a]);
            tq.add(text, 14.0, y + 2.0, w - 12.0, 22.0, Font::Small, col);
            if let Some(p) = pos {
                if self.take_click(8.0, y, 8.0 + w, y + 24.0).is_some() {
                    self.cam = p;
                }
            }
            y += 26.0;
        }
        if let Some((s, t)) = self.toast.clone() {
            let (sw, sh) = g.size();
            let a = t.min(1.0);
            let by = sh - self.bot_h() - 44.0;
            g.rect(sw / 2.0 - 220.0, by, sw / 2.0 + 220.0, by + 32.0, [0.0, 0.0, 0.0, 0.7 * a]);
            tq.add(s, sw / 2.0 - 220.0, by, 440.0, 32.0, Font::NormalCenter, [ACCENT[0], ACCENT[1], ACCENT[2], a]);
        }
    }

    fn draw_mode_hint(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        let s = match self.mode {
            Mode::Normal => {
                if self.cmd_hold() {
                    "指揮中（時は止まっている。命令を出すか選択を解くと進む）".to_string()
                } else if self.paused {
                    if self.touch { "一時停止中（右上の ▶ で再開）".to_string() } else { "一時停止中（Space で再開）".to_string() }
                } else {
                    return;
                }
            }
            Mode::MoveOnly => "移動先をクリック（交戦しない）".into(),
            Mode::AttackGround => "攻撃する地点をクリック（橋の破壊にも）".into(),
            Mode::Place(k) => format!("{}を置く場所をクリック（右クリックで中止）", bdef(k).name),
            Mode::Power(p) => format!("{}の目標をクリック（右クリックで中止）", pdef(p).name),
            Mode::Job(JobKind::Trench) => "塹壕線をドラッグで引く".into(),
            Mode::Job(JobKind::Mine) => "地雷原をドラッグで引く".into(),
            Mode::Job(JobKind::Teeth) => "竜の歯（車両止め）をドラッグで引く".into(),
            Mode::Job(JobKind::Pontoon) => "川の上をドラッグして浮橋を架ける".into(),
            Mode::Rally => "集結地点をタップ".into(),
        };
        // タッチ向けの言い方に直す
        let s = if self.touch { s.replace("をクリック", "をタップ").replace("（右クリックで中止）", "").replace("ドラッグ", "なぞって") } else { s };
        let top = self.top_h();
        g.rect(sw / 2.0 - 240.0, top + 6.0, sw / 2.0 + 240.0, top + 34.0, [0.0, 0.0, 0.0, 0.65]);
        tq.add(s, sw / 2.0 - 240.0, top + 6.0, 480.0, 28.0, Font::NormalCenter, ACCENT);
        // タッチには右クリックがないので、やめるボタンを出す
        if self.compact && self.mode != Mode::Normal && self.button(g, tq, "✕", sw / 2.0 + 244.0, top + 2.0, 46.0, 46.0, false, true).is_some() {
            self.mode = Mode::Normal;
            self.place_tap = None;
        }
        let _ = sh;
    }

    fn draw_tip(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, _sh: f32) {
        let Some(s) = self.tip.clone() else { return };
        if self.touch && self.clock > self.tip_until {
            return;
        }
        let w = 320.0;
        let h = g.measure_h(&s, Font::Small, w - 16.0) + 12.0;
        let x = (self.mouse.0 - w - 10.0).clamp(4.0, sw - w - 4.0);
        // パネルの文字と重ならないように、パネルの上に出す
        let y = (self.panel_top - h - 6.0).max(4.0);
        g.rect(x, y, x + w, y + h, [0.02, 0.03, 0.04, 0.96]);
        g.rect_outline(x, y, x + w, y + h, 1.0, ACCENT);
        tq.add(s, x + 8.0, y + 6.0, w - 16.0, h - 8.0, Font::Small, TEXT);
    }

    pub(crate) fn modal(&mut self, g: &mut dyn Gfx, sw: f32, sh: f32, w: f32, h: f32) -> (f32, f32) {
        g.rect(0.0, 0.0, sw, sh, [0.0, 0.0, 0.0, 0.5]);
        let (x, y) = ((sw - w) / 2.0, (sh - h) / 2.0);
        g.rect(x, y, x + w, y + h, [0.08, 0.09, 0.11, 0.99]);
        g.rect_outline(x, y, x + w, y + h, 2.0, ACCENT);
        (x, y)
    }

    fn draw_brief(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        let (x, y) = self.modal(g, sw, sh, 720.0, 400.0);
        let side = self.viewer.unwrap_or(0) as usize;
        tq.add(format!("作戦命令書 ── {}", SIDE_FULL[side]), x + 24.0, y + 16.0, 670.0, 34.0, Font::Big, ACCENT);
        let ops = if self.touch {
            "操作の基本：タップ＝選択／部隊を選んでいれば命令　長押ししてなぞる＝範囲選択\nなぞる＝地図の移動　2本指＝拡大縮小　右上の「？」＝説明"
        } else {
            "操作の基本：左ドラッグ＝選択　右クリック＝交戦しつつ前進　ホイール＝拡大縮小\n矢印キー・画面端・中ボタンドラッグ＝地図の移動　F1＝説明　Space＝一時停止"
        };
        let body = format!(
            "目標：2時間後（2026年10月31日）に、地図全体の支配領域で敵を上回ること。\n戦争は 2022年2月24日の全面侵攻から始まり、実際の出来事が日付どおりに起こります（F2 で年表）。\n\n\
             ・歩兵や車両が進むと周囲の土地が自国の色に変わります。守る部隊のいない土地は奪われます。\n\
             ・敵は見えていないと撃てません。偵察ドローンで前線の先を見ると砲兵が働きます。\n\
             ・止まった歩兵は塹壕を掘ります。塹壕は砲撃に強く、工兵は塹壕線や地雷原を素早く作れます。\n\
             ・ドンバ地方が激戦地です。泥濘・凍結・葉の有無・霧が戦場を変えます（上部の日付に触れると説明）。\n\
             ・補給所から遠い部隊は弱ります。橋を落とせば敵の戦車は川を渡れません。\n\
             ・全部隊は最初 AI の将軍に委任されています（黄色の点）。命令した部隊だけ直接指揮に変わります。\n・部隊を選んでいる間は時間が止まります。命令を出すか選択を解くと進みます。\n\n\
             {ops}"
        );
        tq.add(body, x + 24.0, y + 58.0, 672.0, 280.0, Font::Normal, TEXT);
        if self.button(g, tq, "作戦開始", x + 240.0, y + 344.0, 240.0, 42.0, true, true).is_some() {
            self.show_brief = false;
        }
        self.pressed = None;
    }

    /// 史実の出来事の見出し
    fn draw_banner(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32) {
        let Some(&(i, t)) = self.banner.front() else { return };
        let Some(w) = self.world.as_ref() else { return };
        let Some(c) = w.chronicle.get(i).cloned() else { return };
        let a = (t * 2.0).min(1.0);
        let col = if c.side > 1 { TEXT } else { SIDE_COL[c.side as usize] };
        // 通知の右に出す
        let left = if self.compact { 316.0 } else { 486.0 };
        let bw = 640.0f32.min(sw - left - 8.0);
        let (x, y) = ((sw / 2.0 - bw / 2.0).max(left), self.top_h() + 40.0);
        self.bg_rects.push((x, y, x + bw, y + 80.0));
        let h = 34.0 + g.measure_h(c.text, Font::Normal, bw - 28.0);
        g.rect(x, y, x + bw, y + h + 8.0, [0.03, 0.04, 0.05, 0.9 * a]);
        g.rect(x, y, x + 5.0, y + h + 8.0, [col[0], col[1], col[2], a]);
        tq.add(format!("{}　{}", c.date, c.title), x + 16.0, y + 5.0, bw - 28.0, 28.0, Font::Big, [ACCENT[0], ACCENT[1], ACCENT[2], a]);
        tq.add(c.text, x + 16.0, y + 34.0, bw - 28.0, h - 26.0, Font::Normal, [TEXT[0], TEXT[1], TEXT[2], a]);
        if let Some(p) = c.pos {
            // 押すと現地を映して見出しを閉じる
            if self.take_click(x, y, x + bw, y + h + 8.0).is_some() {
                self.cam = p;
                self.banner.pop_front();
            }
        }
    }

    /// 年表画面：起きた出来事の一覧
    fn draw_chronicle(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        let (mw, mh) = (860.0f32.min(sw - 16.0), if self.compact { sh - 8.0 } else { (sh - 100.0).min(640.0) });
        let (x, y) = self.modal(g, sw, sh, mw, mh);
        tq.add("戦争の年表", x + 24.0, y + 14.0, 400.0, 30.0, Font::Big, ACCENT);
        let list: Vec<Chronicle> = self.world.as_ref().map_or(Vec::new(), |w| w.chronicle.clone());
        let total = yydrone_core::history::HISTORY.len();
        tq.add(format!("{} / {} 件（ホイールで移動。実際の出来事を基にしています）", list.len(), total), x + 24.0, y + 44.0, 640.0, 22.0, Font::Small, DIM);
        let (top, bottom) = (y + 72.0, y + mh - 12.0);
        let (mut cy, tw) = (top - self.chron_scroll, mw - 48.0);
        let mut content = 0.0;
        for c in list.iter().rev() {
            let body_h = g.measure_h(c.text, Font::Small, tw);
            let h = 24.0 + body_h + 10.0;
            content += h;
            if cy >= top - 1.0 && cy + h <= bottom {
                let col = if !c.happened { DIM } else if c.side > 1 { TEXT } else { SIDE_COL[c.side as usize] };
                let note = if c.happened { "" } else { "（この戦争では起きなかった）" };
                tq.add(format!("{}　{}{}", c.date, c.title, note), x + 24.0, cy, tw, 24.0, Font::Normal, col);
                tq.add(c.text, x + 24.0, cy + 24.0, tw, body_h + 6.0, Font::Small, if c.happened { TEXT } else { DIM });
            }
            cy += h;
        }
        let max_scroll = (content - (bottom - top)).max(0.0);
        self.chron_scroll = self.chron_scroll.min(max_scroll);
        if !self.compact && self.button(g, tq, "閉じる", x + mw - 130.0, y + 12.0, 110.0, 32.0, false, true).is_some() {
            self.show_chron = false;
        }
        self.pressed = None;
    }

    pub(crate) fn draw_help(&mut self, g: &mut dyn Gfx, tq: &mut TextQ, sw: f32, sh: f32) {
        let (x, y) = self.modal(g, sw, sh, 860.0, 560.0);
        tq.add("操作説明", x + 24.0, y + 14.0, 400.0, 30.0, Font::Big, ACCENT);
        tq.add(
            "【部隊】\n左クリック／左ドラッグ：選択（Shift で追加）\nダブルクリック：画面内の同じ種類を全部\n右クリック：交戦しつつ前進／敵の上で攻撃\nQ：移動のみ　G：地点攻撃　S：停止　H：固守\nZ：AI 将軍に委任（黄色の点）。命令すると解除\n部隊を選ぶと時間が止まり、命令か選択の解除で進む\nCtrl+数字：群に登録　数字：群を呼び出し\n\n【工兵】\nT：塹壕線　M：地雷原　B：竜の歯　P：浮橋\n（ドラッグで線を引く）\n\n【地図と時間】\n矢印キー・画面端・中ボタン：移動　ホイール：拡大縮小\nSpace：一時停止　＋／−：速度　F2：年表　F5：記録　F11：全画面\nCtrl＋＋／−：文字の大きさ\n\n【タッチ操作】\nタップ：選択／部隊を選んでいれば命令\n長押ししてなぞる：範囲選択　なぞる：地図の移動\n2本指：移動と拡大縮小",
            x + 24.0, y + 52.0, 400.0, 480.0, Font::Normal, TEXT,
        );
        tq.add(
            "【現代戦のコツ】\n・見えない敵は撃てない。偵察機は森や塹壕の敵も暴く\n・榴弾砲とロケットは撃つと位置が敵に露見する\n・FPV 班は見えた敵に自動で飛ばす。対空砲と電子戦で防ぐ\n・電子戦車はドローンを落とすが自分は戦えない\n・戦闘機は飛行場から出撃して爆弾を落とし帰還する\n・SAM は航空機と巡航ミサイルを落とす\n・ハイマルで補給所・橋・防空を叩く\n・補給切れの部隊は消耗し、攻撃も遅くなる\n・包囲された無人の飛び地は降伏する\n・市街地は占領に時間がかかる\n・泥濘は道路の外を塞ぎ、凍結は道を開くが塹壕を拒む\n・葉の茂る樹林帯は歩兵を隠し、落葉すると丸見え\n・霧と雨はドローンを奪う。その間に戦車と歩兵を動かす\n・高地とボタ山は遠くまで見渡せて守りやすい\n・鉄道は補給を遠くへ運び、自国の線路上は部隊も速い\n・線路を地点攻撃で切れば、敵の前線への補給が細る\n・領土を広げるほど占領は遅くなり、大軍ほど維持費がかかる\n・押されている側は収入が増え、本土防衛で粘る\n・国土の大半を失うと総動員とパルチザンで巻き返す\n\n【勝利条件】\n時間切れで支配領域が広い陣営の勝ち",
            x + 440.0, y + 52.0, 400.0, 480.0, Font::Normal, TEXT,
        );
        if !self.compact && self.button(g, tq, "閉じる", x + 730.0, y + 12.0, 110.0, 32.0, false, true).is_some() {
            self.show_help = false;
        }
        self.pressed = None;
    }
}
