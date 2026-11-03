//! ドローン戦線 — 画面と操作（Windows 版とブラウザ版で共有）

pub mod gfx;
pub mod art;
mod mobile;
mod paint;
pub mod ui;
mod view;

#[cfg(test)]
mod tests;

use gfx::*;
use std::collections::VecDeque;
use view::{Particle, Streak};
use yydrone_core::defs::*;
use yydrone_core::geo::*;
use yydrone_core::rng::Rng;
use yydrone_core::world::*;

pub const TITLE: &str = "ドローン戦線";

#[derive(Clone, Copy, Debug)]
pub enum Ev {
    MouseMove(f32, f32),
    Down(u8, f32, f32),
    Up(u8, f32, f32),
    Wheel(f32),
    Key(u32, bool),
    Resize(u32, u32),
    /// タッチ向けの詰めた画面か
    Compact(bool),
    /// 続く入力がタッチ操作か
    Touch(bool),
    /// 一覧を指でなぞったスクロール
    Scroll(f32),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Screen {
    Title,
    Play,
    Result,
}

/// 地図クリックの意味
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Normal,
    MoveOnly,
    AttackGround,
    Place(BK),
    Power(Power),
    Job(JobKind),
    /// 建物の集結地点（タッチ向け）
    Rally,
}

pub struct Ticker {
    pub text: String,
    pub t: f32,
    pub tone: u8,
    pub pos: Option<(f32, f32)>,
}

pub struct Game {
    pub screen: Screen,
    pub world: Option<World>,
    pub viewer: Option<u8>,
    // タイトルの設定
    pub cfg_side: i32,
    pub cfg_diff: usize,
    pub seed: u64,
    // カメラ
    pub cam: (f32, f32),
    pub zoom: f32,
    // 時間
    acc: f32,
    pub speed: u32,
    pub paused: bool,
    pub anim: f32,
    overlay_t: f32,
    // 入力
    pub mouse: (f32, f32),
    pub pressed: Option<(u8, f32, f32)>,
    keys: [bool; 256],
    drag: Option<(f32, f32)>,
    pan: Option<(f32, f32, f32, f32)>,
    job_start: Option<(f32, f32)>,
    pub minimap_drag: bool,
    last_click: (f32, f32, f32),
    // 選択と命令
    pub selected: Vec<u32>,
    /// 指揮中：自軍の部隊を選んでから命令を出すまで時間を止める
    pub cmd_pause: bool,
    last_sel: Vec<u32>,
    pub sel_bld: Option<u32>,
    pub groups: [Vec<u32>; 10],
    pub mode: Mode,
    pub tab: usize,
    pub tip: Option<String>,
    pub toast: Option<(String, f32)>,
    pub ticker: VecDeque<Ticker>,
    pub show_help: bool,
    pub show_brief: bool,
    pub show_chron: bool,
    pub show_save: bool,
    pub chron_scroll: f32,
    pub compact: bool,
    pub touch: bool,
    /// タッチでは説明を触れた直後だけ出す
    tip_until: f32,
    /// タッチで建物・支援の場所を見せている（もう一度タップで確定）
    pub place_tap: Option<(f32, f32)>,
    /// 今のフレームと前のフレームのボタンの範囲
    pub(crate) hit_rects: Vec<(f32, f32, f32, f32)>,
    pub(crate) hit_prev: Vec<(f32, f32, f32, f32)>,
    /// 地図の上に重ねたパネルの範囲
    pub(crate) bg_rects: Vec<(f32, f32, f32, f32)>,
    pub(crate) bg_prev: Vec<(f32, f32, f32, f32)>,
    pub show_prod: bool,
    pub ov_scroll: f32,
    /// 説明を出す位置（パネルの上端）
    pub(crate) panel_top: f32,
    autosave_t: f32,
    /// 年表のどこまで見せたか
    chron_seen: usize,
    /// 出来事の見出し（年表の番号, 残り秒）
    pub banner: VecDeque<(usize, f32)>,
    // 描画の資源
    terrain_tex: u32,
    overlay_tex: u32,
    terrain: Vec<u32>,
    overlay: Vec<u32>,
    pub particles: Vec<Particle>,
    pub streaks: Vec<Streak>,
    pub fx_rng: Rng,
    pub art: art::Art,
    pub clock: f32,
    pub quit: bool,
}

impl Game {
    pub fn new(seed: u64) -> Game {
        Game {
            screen: Screen::Title,
            world: None,
            viewer: None,
            cfg_side: 0,
            cfg_diff: 1,
            seed,
            cam: (W as f32 / 2.0, H as f32 / 2.0),
            zoom: 5.0,
            acc: 0.0,
            speed: 1,
            paused: false,
            anim: 0.0,
            overlay_t: 0.0,
            mouse: (-1.0, -1.0),
            pressed: None,
            keys: [false; 256],
            drag: None,
            pan: None,
            job_start: None,
            minimap_drag: false,
            last_click: (0.0, 0.0, -10.0),
            selected: Vec::new(),
            cmd_pause: false,
            last_sel: Vec::new(),
            sel_bld: None,
            groups: Default::default(),
            mode: Mode::Normal,
            tab: 0,
            tip: None,
            toast: None,
            ticker: VecDeque::new(),
            show_help: false,
            show_brief: false,
            show_chron: false,
            show_save: false,
            chron_scroll: 0.0,
            compact: false,
            touch: false,
            tip_until: 0.0,
            place_tap: None,
            hit_rects: Vec::new(),
            hit_prev: Vec::new(),
            bg_rects: Vec::new(),
            bg_prev: Vec::new(),
            show_prod: false,
            ov_scroll: 0.0,
            panel_top: 0.0,
            autosave_t: 0.0,
            chron_seen: 0,
            banner: VecDeque::new(),
            terrain_tex: 0,
            overlay_tex: 0,
            terrain: Vec::new(),
            overlay: Vec::new(),
            particles: Vec::new(),
            streaks: Vec::new(),
            fx_rng: Rng::new(seed ^ 77),
            art: art::Art::default(),
            clock: 0.0,
            quit: false,
        }
    }

    /// 自動テストと観戦向け：設定を直接決めて始める
    pub fn configure(&mut self, side: i32, diff: usize, seed: u64) {
        self.cfg_side = side;
        self.cfg_diff = diff;
        self.seed = seed;
    }

    pub fn start_game(&mut self, g: &mut dyn Gfx) {
        const DIFFS: [f32; 3] = [0.75, 1.0, 1.35];
        let player = if self.cfg_side < 0 { None } else { Some(self.cfg_side as u8) };
        let w = World::new(Settings { duration: WAR_SECONDS, seed: self.seed, ai_mult: DIFFS[self.cfg_diff], player });
        self.seed = self.seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1);
        self.viewer = player;
        if self.terrain.is_empty() {
            self.terrain = paint::paint_all(&w);
            self.terrain_tex = g.create_texture(paint::TW as u32, paint::TH as u32, &self.terrain);
            self.overlay = vec![0; W * H];
            paint::paint_overlay(&w, player, &mut self.overlay);
            self.overlay_tex = g.create_texture(W as u32, H as u32, &self.overlay);
        } else {
            self.terrain = paint::paint_all(&w);
            g.update_texture(self.terrain_tex, 0, 0, paint::TW as u32, paint::TH as u32, &self.terrain);
        }
        // 最初は激戦地のドンバ周辺を映す
        self.cam = if player == Some(1) { (225.0, 112.0) } else { (205.0, 105.0) };
        let (sw, _) = g.size();
        self.zoom = (sw / 150.0).max(5.0);
        self.selected.clear();
        self.sel_bld = None;
        self.groups = Default::default();
        self.mode = Mode::Normal;
        self.tab = 0;
        self.paused = false;
        self.speed = 1;
        self.acc = 0.0;
        self.particles.clear();
        self.streaks.clear();
        self.ticker.clear();
        self.toast = None;
        self.show_help = false;
        self.show_brief = player.is_some();
        self.show_chron = false;
        self.show_save = false;
        self.show_prod = false;
        self.place_tap = None;
        self.chron_scroll = 0.0;
        self.chron_seen = 0;
        self.autosave_t = 0.0;
        self.banner.clear();
        self.overlay_t = 0.0;
        self.world = Some(w);
        self.screen = Screen::Play;
    }

    pub fn screen_to_world(&self, g: &dyn Gfx, x: f32, y: f32) -> (f32, f32) {
        let (sw, sh) = g.size();
        ((x - sw / 2.0) / self.zoom + self.cam.0, (y - sh / 2.0) / self.zoom + self.cam.1)
    }

    pub fn world_to_screen(&self, g: &dyn Gfx, x: f32, y: f32) -> (f32, f32) {
        let (sw, sh) = g.size();
        ((x - self.cam.0) * self.zoom + sw / 2.0, (y - self.cam.1) * self.zoom + sh / 2.0)
    }

    pub fn message(&mut self, s: impl Into<String>) {
        self.toast = Some((s.into(), 3.0));
    }

    /// 1フレーム。true を返すと終了
    pub fn frame(&mut self, g: &mut dyn Gfx, evs: &[Ev], dt: f32) -> bool {
        self.clock += dt;
        self.pressed = None;
        self.tip = None;
        self.hit_prev = std::mem::take(&mut self.hit_rects);
        self.bg_prev = std::mem::take(&mut self.bg_rects);
        for e in evs {
            self.handle_event(g, *e);
        }
        if self.screen == Screen::Play {
            self.scroll_camera(g, dt);
            self.advance(g, dt);
        }
        g.begin([0.07, 0.08, 0.09, 1.0]);
        g.set_screen_xform();
        match self.screen {
            Screen::Title => {
                self.draw_title(g);
            }
            Screen::Play => {
                self.draw_world(g);
                g.set_screen_xform();
                self.draw_weather(g);
                self.draw_hud(g);
            }
            Screen::Result => {
                self.draw_result(g);
            }
        }
        g.present();
        self.quit
    }

    fn advance(&mut self, g: &mut dyn Gfx, dt: f32) {
        self.anim += dt;
        self.update_cmd_pause();
        let frozen = self.paused || self.modal_open() || self.cmd_hold();
        // 自動記録（2分ごと）
        if !frozen {
            self.autosave_t += dt;
            if self.autosave_t >= 120.0 {
                self.autosave_t = 0.0;
                self.save_to(g, 0);
            }
        }
        if !frozen {
            self.acc += dt * self.speed as f32;
            let mut steps = (self.acc / DT) as u32;
            let max = 12 * self.speed;
            if steps > max {
                steps = max;
                self.acc = 0.0;
            } else {
                self.acc -= steps as f32 * DT;
            }
            let Some(w) = self.world.as_mut() else { return };
            for _ in 0..steps {
                w.tick();
            }
        }
        self.absorb_fx();
        self.step_particles(if frozen { 0.0 } else { dt });
        let Some(w) = self.world.as_mut() else { return };
        // 地図の変更を絵に反映する
        for sc in std::mem::take(&mut w.scars).into_iter().take(40) {
            let seed = (sc.0 * 37.0) as u32 ^ (sc.1 * 91.0) as u32;
            let (x, y, rw, rh) = paint::stamp_scar(&mut self.terrain, sc.0, sc.1, sc.2, seed);
            upload(g, self.terrain_tex, &self.terrain, x, y, rw, rh);
        }
        for t in std::mem::take(&mut w.repaint) {
            let i = t as usize;
            paint::repaint_full(w, &mut self.terrain, i);
            let s = paint::SCALE as u32;
            let (x, y) = ((i % W) as u32 * s, (i / W) as u32 * s);
            upload(g, self.terrain_tex, &self.terrain, x, y, s, s);
        }
        let dirty = std::mem::take(&mut w.dirty);
        for t in dirty {
            let i = t as usize;
            paint::repaint_tile(w, &mut self.terrain, i);
            let s = paint::SCALE as u32;
            let (x, y) = ((i % W) as u32 * s, (i / W) as u32 * s);
            upload(g, self.terrain_tex, &self.terrain, x, y, s, s);
        }
        // 重ね絵（領土・霧）は少し間引いて更新する
        self.overlay_t -= dt;
        if self.overlay_t <= 0.0 {
            self.overlay_t = 0.2;
            paint::paint_overlay(w, self.viewer, &mut self.overlay);
            g.update_texture(self.overlay_tex, 0, 0, W as u32, H as u32, &self.overlay);
        }
        // 通知
        let viewer = self.viewer;
        for n in w.drain_notices() {
            if n.side.is_none() || n.side == viewer {
                let pos = if n.x >= 0.0 { Some((n.x, n.y)) } else { None };
                self.ticker.push_front(Ticker { text: n.text, t: 14.0, tone: n.tone, pos });
            }
        }
        // 史実の出来事は見出しを出す（説明の表示中は時間が止まるので待つ）
        while self.chron_seen < w.chronicle.len() {
            let c = &w.chronicle[self.chron_seen];
            let pos = c.pos;
            let tone = match (c.side, viewer) {
                (s, Some(v)) if s == v => 1,
                (2, _) | (_, None) => 0,
                _ => 2,
            };
            if c.happened {
                self.banner.push_back((self.chron_seen, 9.0));
            }
            let head = if c.happened { format!("{} {}", c.date, c.title) } else { format!("{} {}（この戦争では起きなかった）", c.date, c.title) };
            self.ticker.push_front(Ticker { text: head, t: 18.0, tone, pos });
            self.chron_seen += 1;
        }
        if !frozen {
            // 溜まっているときは早めに次へ
            let rate = if self.banner.len() > 2 { 2.0 } else { 1.0 };
            if let Some(b) = self.banner.front_mut() {
                b.1 -= dt * rate;
                if b.1 <= 0.0 {
                    self.banner.pop_front();
                }
            }
        }
        while self.ticker.len() > 7 {
            self.ticker.pop_back();
        }
        for t in &mut self.ticker {
            t.t -= dt;
        }
        self.ticker.retain(|t| t.t > 0.0);
        if let Some(t) = &mut self.toast {
            t.1 -= dt;
            if t.1 <= 0.0 {
                self.toast = None;
            }
        }
        let over = w.over;
        // 消えた部隊を選択から外す
        let alive: Vec<u32> = self.selected.iter().copied().filter(|id| w.unit(*id).is_some()).collect();
        self.selected = alive;
        if let Some(b) = self.sel_bld {
            if w.bld(b).is_none() {
                self.sel_bld = None;
            }
        }
        if over {
            self.screen = Screen::Result;
        }
    }

    fn scroll_camera(&mut self, g: &dyn Gfx, dt: f32) {
        let (sw, sh) = g.size();
        let sp = 900.0 / self.zoom * dt;
        let k = |c: u32| self.keys[c as usize];
        let (mut dx, mut dy) = (0.0, 0.0);
        if k(0x25) {
            dx -= sp;
        }
        if k(0x27) {
            dx += sp;
        }
        if k(0x26) {
            dy -= sp;
        }
        if k(0x28) {
            dy += sp;
        }
        // 画面の端でもスクロール
        let m = self.mouse;
        if !self.touch && self.drag.is_none() && self.pan.is_none() && m.0 >= 0.0 {
            if m.0 < 3.0 {
                dx -= sp;
            }
            if m.0 > sw - 3.0 {
                dx += sp;
            }
            if m.1 < 3.0 {
                dy -= sp;
            }
            if m.1 > sh - 3.0 {
                dy += sp;
            }
        }
        self.cam.0 += dx;
        self.cam.1 += dy;
        self.clamp_camera(g);
    }

    fn clamp_camera(&mut self, g: &dyn Gfx) {
        let (sw, sh) = g.size();
        let min_zoom = (sw / W as f32).max(sh / H as f32).max(2.0) * 0.9;
        self.zoom = self.zoom.clamp(min_zoom, 40.0);
        let (hw, hh) = (sw / 2.0 / self.zoom, sh / 2.0 / self.zoom);
        let clamp = |v: f32, half: f32, size: f32| if half * 2.0 >= size { size / 2.0 } else { v.clamp(half, size - half) };
        self.cam.0 = clamp(self.cam.0, hw, W as f32);
        self.cam.1 = clamp(self.cam.1, hh, H as f32 + 40.0 / self.zoom);
    }

    fn handle_event(&mut self, g: &mut dyn Gfx, e: Ev) {
        match e {
            Ev::Resize(w, h) => {
                g.resize(w, h);
            }
            Ev::MouseMove(x, y) => {
                self.mouse = (x, y);
                if let Some((sx, sy, cx, cy)) = self.pan {
                    self.cam = (cx - (x - sx) / self.zoom, cy - (y - sy) / self.zoom);
                    self.clamp_camera(g);
                }
                if self.minimap_drag && self.screen == Screen::Play {
                    self.minimap_jump(g, x, y);
                }
            }
            Ev::Wheel(d) => {
                if self.show_chron {
                    self.chron_scroll = (self.chron_scroll - d * 60.0).max(0.0);
                } else if self.show_help {
                    self.ov_scroll = (self.ov_scroll - d * 60.0).max(0.0);
                } else if self.screen == Screen::Play {
                    let before = self.screen_to_world(g, self.mouse.0, self.mouse.1);
                    self.zoom *= 1.15f32.powf(d);
                    self.clamp_camera(g);
                    let after = self.screen_to_world(g, self.mouse.0, self.mouse.1);
                    self.cam.0 += before.0 - after.0;
                    self.cam.1 += before.1 - after.1;
                    self.clamp_camera(g);
                }
            }
            Ev::Compact(c) => {
                self.compact = c;
                g.set_compact(c);
            }
            Ev::Touch(t) => self.touch = t,
            Ev::Scroll(d) => {
                if self.show_chron {
                    self.chron_scroll = (self.chron_scroll - d).max(0.0);
                } else {
                    self.ov_scroll = (self.ov_scroll - d).max(0.0);
                }
            }
            Ev::Down(b, x, y) => {
                let (x, y) = self.snap_touch(x, y);
                self.mouse = (x, y);
                self.tip_until = self.clock + 2.5;
                if self.screen != Screen::Play {
                    self.pressed = Some((b, x, y));
                    return;
                }
                if b == 2 {
                    self.pan = Some((x, y, self.cam.0, self.cam.1));
                    return;
                }
                if self.modal_open() || self.over_ui(g, x, y) {
                    self.pressed = Some((b, x, y));
                    return;
                }
                if b == 0 {
                    if let Mode::Job(_) = self.mode {
                        self.job_start = Some((x, y));
                    } else if self.mode == Mode::Normal {
                        self.drag = Some((x, y));
                    } else {
                        self.map_click(g, x, y);
                    }
                    if self.mode != Mode::Normal && !matches!(self.mode, Mode::Place(_) | Mode::Power(_)) {
                        self.place_tap = None;
                    }
                } else if b == 1 {
                    if self.mode != Mode::Normal {
                        self.mode = Mode::Normal;
                    } else {
                        self.map_right_click(g, x, y);
                    }
                }
            }
            Ev::Up(b, x, y) => {
                self.mouse = (x, y);
                if b == 2 {
                    self.pan = None;
                }
                if b == 0 {
                    self.minimap_drag = false;
                    if let Some(s) = self.job_start.take() {
                        self.finish_job(g, s, (x, y));
                    }
                    if let Some(s) = self.drag.take() {
                        self.finish_select(g, s, (x, y));
                    }
                }
            }
            Ev::Key(k, down) => {
                if (k as usize) < 256 {
                    self.keys[k as usize] = down;
                }
                if down {
                    self.key_press(g, k);
                }
            }
        }
    }

    /// 自軍の部隊を新たに選ぶと時間を止め、命令を出すと再開する
    fn update_cmd_pause(&mut self) {
        if self.selected != self.last_sel {
            // 戦死などで減っただけなら選び直しとはみなさない
            if self.selected.iter().any(|id| !self.last_sel.contains(id)) {
                self.cmd_pause = true;
            }
            self.last_sel = self.selected.clone();
        }
        if self.selected.is_empty() {
            self.cmd_pause = false;
        }
    }

    /// 時間を止めているか（部隊の選択中・範囲選択のドラッグ中）
    pub fn cmd_hold(&self) -> bool {
        self.cmd_pause || self.drag_box().is_some()
    }

    pub fn modal_open(&self) -> bool {
        self.show_help || self.show_brief || self.show_chron || self.show_save
    }

    fn side(&self) -> Option<u8> {
        self.viewer
    }

    fn key_press(&mut self, g: &mut dyn Gfx, k: u32) {
        match k {
            0x7A => g.toggle_fullscreen(),
            0xBB | 0x6B if self.keys[0x11] => g.adjust_scale(1),
            0xBD | 0x6D if self.keys[0x11] => g.adjust_scale(-1),
            _ => {}
        }
        if self.screen != Screen::Play {
            if k == 0x0D && self.screen == Screen::Title {
                self.start_game(g);
            }
            return;
        }
        match k {
            0x70 => self.show_help = !self.show_help,
            0x71 => self.show_chron = !self.show_chron,
            0x74 => self.show_save = !self.show_save,
            0x20 => {
                if self.show_brief {
                    self.show_brief = false;
                } else {
                    self.paused = !self.paused;
                }
            }
            0x1B => {
                if self.show_save {
                    self.show_save = false;
                } else if self.show_chron {
                    self.show_chron = false;
                } else if self.show_help {
                    self.show_help = false;
                } else if self.show_brief {
                    self.show_brief = false;
                } else if self.mode != Mode::Normal {
                    self.mode = Mode::Normal;
                } else {
                    self.selected.clear();
                    self.sel_bld = None;
                }
            }
            0xBB | 0x6B => {
                if !self.keys[0x11] {
                    self.cycle_speed(true);
                }
            }
            0xBD | 0x6D => {
                if !self.keys[0x11] {
                    self.cycle_speed(false);
                }
            }
            0x09 => self.tab = (self.tab + 1) % ui::TABS.len(),
            0x31..=0x39 => {
                let n = (k - 0x30) as usize;
                if self.keys[0x11] {
                    self.groups[n] = self.selected.clone();
                    self.message(format!("部隊を第{n}群に登録した"));
                } else {
                    let ids = self.groups[n].clone();
                    if !ids.is_empty() {
                        let same = ids == self.selected;
                        self.selected = ids;
                        self.sel_bld = None;
                        if same {
                            self.center_selection();
                        }
                    }
                }
            }
            _ => {
                if let Some(a) = ui::action_for_key(k) {
                    self.run_action(a);
                }
            }
        }
    }

    pub fn cycle_speed(&mut self, up: bool) {
        const S: [u32; 4] = [1, 2, 4, 8];
        let i = S.iter().position(|v| *v == self.speed).unwrap_or(0);
        let j = if up { (i + 1).min(3) } else { i.saturating_sub(1) };
        self.speed = S[j];
    }

    pub fn center_selection(&mut self) {
        let Some(w) = self.world.as_ref() else { return };
        let (mut x, mut y, mut n) = (0.0, 0.0, 0.0);
        for id in &self.selected {
            if let Some(u) = w.unit(*id) {
                x += u.x;
                y += u.y;
                n += 1.0;
            }
        }
        if n > 0.0 {
            self.cam = (x / n, y / n);
        }
    }

    pub fn run_action(&mut self, a: ui::Action) {
        use ui::Action::*;
        let ids = self.selected.clone();
        let Some(w) = self.world.as_mut() else { return };
        match a {
            Stop => {
                w.command(&ids, Cmd::Auto(false), 0.0, 0.0);
                w.command(&ids, Cmd::Stop, 0.0, 0.0);
                self.cmd_pause = false;
            }
            Hold => {
                w.command(&ids, Cmd::Auto(false), 0.0, 0.0);
                w.command(&ids, Cmd::Hold, 0.0, 0.0);
                self.cmd_pause = false;
            }
            MoveOnly => self.mode = Mode::MoveOnly,
            AttackGround => self.mode = Mode::AttackGround,
            Delegate => {
                let all = ids.iter().all(|id| w.unit(*id).is_some_and(|u| u.auto));
                w.command(&ids, Cmd::Auto(!all), 0.0, 0.0);
                self.cmd_pause = false;
                self.message(if all { "AI 委任を解除した" } else { "選んだ部隊を AI の将軍に委任した" });
            }
            Job(k) => {
                if ids.iter().any(|id| w.unit(*id).is_some_and(|u| u.kind == UK::Engineer)) {
                    self.mode = Mode::Job(k);
                }
            }
        }
    }

    fn finish_select(&mut self, g: &dyn Gfx, s: (f32, f32), e: (f32, f32)) {
        let Some(side) = self.side() else { return };
        let Some(w) = self.world.as_ref() else { return };
        let (ax, ay) = self.screen_to_world(g, s.0, s.1);
        let (bx, by) = self.screen_to_world(g, e.0, e.1);
        let shift = self.keys[0x10];
        let mut out: Vec<u32> = if shift { self.selected.clone() } else { Vec::new() };
        let is_click = (s.0 - e.0).abs() < if self.touch { 12.0 } else { 5.0 } && (s.1 - e.1).abs() < if self.touch { 12.0 } else { 5.0 };
        let grab = if self.touch { 18.0 } else { 9.0 } / self.zoom;
        // タッチ：部隊を選んでいるとき、自軍の部隊以外をタップすれば命令（右クリックと同じ）
        if self.touch && is_click && !self.selected.is_empty() {
            let own = w.units.iter().any(|u| u.side == side && !matches!(u.kind, UK::Fpv | UK::Cruise) && (u.x - ax).powi(2) + (u.y - ay).powi(2) < (grab + 0.5).powi(2));
            if !own {
                self.map_right_click(g, e.0, e.1);
                return;
            }
        }
        if is_click {
            let mut best = None;
            let mut bd = (grab + 0.5).powi(2);
            for u in &w.units {
                if u.side != side || matches!(u.kind, UK::Fpv | UK::Cruise) {
                    continue;
                }
                let dd = (u.x - ax).powi(2) + (u.y - ay).powi(2);
                if dd < bd {
                    bd = dd;
                    best = Some(u.id);
                }
            }
            let dbl = self.clock - self.last_click.2 < 0.35 && (self.last_click.0 - s.0).abs() < 6.0 && (self.last_click.1 - s.1).abs() < 6.0;
            self.last_click = (s.0, s.1, self.clock);
            if let Some(id) = best {
                if dbl {
                    // ダブルクリック：画面内の同じ種類を全部
                    let k = w.unit(id).map(|u| u.kind);
                    let (sw, sh) = g.size();
                    let (x0, y0) = self.screen_to_world(g, 0.0, 0.0);
                    let (x1, y1) = self.screen_to_world(g, sw, sh);
                    for u in &w.units {
                        if u.side == side && Some(u.kind) == k && u.x >= x0 && u.x <= x1 && u.y >= y0 && u.y <= y1 && !out.contains(&u.id) {
                            out.push(u.id);
                        }
                    }
                } else if !out.contains(&id) {
                    out.push(id);
                }
                self.sel_bld = None;
            } else {
                // 建物
                let b = w.blds.iter().find(|b| b.side == side && (b.x - ax).abs() < grab + 0.8 && (b.y - ay).abs() < grab + 0.8);
                self.sel_bld = b.map(|b| b.id);
                if !shift {
                    out.clear();
                }
            }
        } else {
            let (x0, x1) = (ax.min(bx), ax.max(bx));
            let (y0, y1) = (ay.min(by), ay.max(by));
            for u in &w.units {
                if u.side == side && !matches!(u.kind, UK::Fpv | UK::Cruise) && u.x >= x0 && u.x <= x1 && u.y >= y0 && u.y <= y1 && !out.contains(&u.id) {
                    out.push(u.id);
                }
            }
            self.sel_bld = None;
        }
        self.selected = out;
    }

    fn map_right_click(&mut self, g: &dyn Gfx, x: f32, y: f32) {
        let Some(side) = self.side() else { return };
        let (wx, wy) = self.screen_to_world(g, x, y);
        let ids = self.selected.clone();
        // 建物を選んでいれば集結地点（ここで生まれる部隊は直接指揮になる）
        if let Some(bid) = self.sel_bld {
            if let Some(w) = self.world.as_mut() {
                if let Some(b) = w.blds.iter_mut().find(|b| b.id == bid) {
                    b.rally = Some((wx, wy));
                    self.toast = Some(("集結地点を設定した".into(), 2.0));
                }
            }
            return;
        }
        if ids.is_empty() {
            return;
        }
        let grab = 9.0 / self.zoom + 0.5;
        let Some(w) = self.world.as_mut() else { return };
        // 敵部隊・敵建物の上なら攻撃
        let mut tgt = None;
        let mut bd = grab * grab;
        for u in &w.units {
            if u.side != side && w.can_see(side, u) {
                let dd = (u.x - wx).powi(2) + (u.y - wy).powi(2);
                if dd < bd {
                    bd = dd;
                    tgt = Some(Target::Unit(u.id));
                }
            }
        }
        if tgt.is_none() {
            tgt = w.blds.iter().find(|b| b.side != side && b.seen[side as usize] && (b.x - wx).abs() < grab + 0.6 && (b.y - wy).abs() < grab + 0.6).map(|b| Target::Bld(b.id));
        }
        // 委任を解いて直接指揮する
        w.command(&ids, Cmd::Auto(false), 0.0, 0.0);
        self.cmd_pause = false;
        if let Some(t) = tgt {
            w.attack_target(&ids, t);
            self.ping(wx, wy, [1.0, 0.4, 0.3]);
        } else {
            let (one_way, rest): (Vec<u32>, Vec<u32>) = ids.iter().partition(|id| w.unit(**id).is_some_and(|u| def(u.kind).suicide));
            w.attack_ground(&one_way, wx, wy);
            w.command(&rest, Cmd::AttackMove, wx, wy);
            self.ping(wx, wy, [0.5, 1.0, 0.5]);
        }
    }

    fn ping(&mut self, x: f32, y: f32, col: [f32; 3]) {
        self.particles.push(Particle { x, y, vx: 0.0, vy: 0.0, life: 0.45, max: 0.45, size: 0.3, grow: 3.0, col, alpha: 0.7 });
    }

    /// 特別なモードでの左クリック
    fn map_click(&mut self, g: &dyn Gfx, x: f32, y: f32) {
        let Some(side) = self.side() else { return };
        let (wx, wy) = self.screen_to_world(g, x, y);
        let ids = self.selected.clone();
        let mode = self.mode;
        // タッチ：1回目のタップは場所を見せるだけ（誤って建てたり撃ったりしないように）
        if self.touch && matches!(mode, Mode::Place(_) | Mode::Power(_)) && self.place_tap.is_none_or(|(px, py)| (px - x).abs() > 22.0 || (py - y).abs() > 22.0) {
            self.place_tap = Some((x, y));
            self.message("同じ場所をもう一度タップすると確定");
            return;
        }
        self.place_tap = None;
        let sel_bld = self.sel_bld;
        let Some(w) = self.world.as_mut() else { return };
        match mode {
            Mode::Rally => {
                if let Some(b) = w.blds.iter_mut().find(|b| Some(b.id) == sel_bld) {
                    b.rally = Some((wx, wy));
                }
                self.mode = Mode::Normal;
                self.message("集結地点を設定した");
            }
            Mode::MoveOnly => {
                w.command(&ids, Cmd::Auto(false), 0.0, 0.0);
                w.command(&ids, Cmd::Move, wx, wy);
                self.cmd_pause = false;
                self.ping(wx, wy, [0.7, 1.0, 1.0]);
                self.mode = Mode::Normal;
            }
            Mode::AttackGround => {
                w.command(&ids, Cmd::Auto(false), 0.0, 0.0);
                w.attack_ground(&ids, wx, wy);
                self.cmd_pause = false;
                self.ping(wx, wy, [1.0, 0.5, 0.3]);
                self.mode = Mode::Normal;
            }
            Mode::Place(k) => match w.place_building(side, k, wx, wy) {
                Ok(_) => {
                    if !self.keys[0x10] {
                        self.mode = Mode::Normal;
                    }
                }
                Err(e) => self.message(e),
            },
            Mode::Power(p) => match w.use_power(side, p, wx, wy) {
                Ok(()) => {
                    self.mode = Mode::Normal;
                    self.message(format!("{}を実行した", pdef(p).name));
                }
                Err(e) => self.message(e),
            },
            _ => {}
        }
    }

    /// 工兵の作業線：ドラッグした線上のタイルに予約する
    fn finish_job(&mut self, g: &dyn Gfx, s: (f32, f32), e: (f32, f32)) {
        let Mode::Job(kind) = self.mode else { return };
        let ids = self.selected.clone();
        let (ax, ay) = self.screen_to_world(g, s.0, s.1);
        let (bx, by) = self.screen_to_world(g, e.0, e.1);
        let Some(w) = self.world.as_mut() else { return };
        let dist = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
        let n = (dist.ceil() as usize).clamp(1, 40);
        let mut tiles: Vec<(u16, u16)> = Vec::new();
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let (x, y) = ((ax + (bx - ax) * t) as i32, (ay + (by - ay) * t) as i32);
            if !in_map(x, y) {
                continue;
            }
            let ter = w.map.terrain[tidx(x, y)];
            let ok = if kind == JobKind::Pontoon { ter == Terrain::River } else { ter.land() };
            if ok && !tiles.contains(&(x as u16, y as u16)) {
                tiles.push((x as u16, y as u16));
            }
        }
        if tiles.is_empty() {
            self.toast = Some((if kind == JobKind::Pontoon { "川の上をなぞってください".into() } else { "ここには作れない".into() }, 2.0));
            return;
        }
        w.command(&ids, Cmd::Auto(false), 0.0, 0.0);
        w.add_jobs(&ids, kind, &tiles);
        self.cmd_pause = false;
        if !self.keys[0x10] {
            self.mode = Mode::Normal;
        }
    }

    fn save_meta(&self) -> String {
        let Some(w) = self.world.as_ref() else { return String::new() };
        let side = w.player.map_or("観戦", |p| SIDE_NAMES[p as usize]);
        let share = w.player.map_or(w.share(0), |p| w.share(p)) * 100.0;
        let left = (w.duration - w.t).max(0.0) as u32;
        format!("{side}　{}　領土 {share:.1}%　残り {}:{:02}", w.date_text(), left / 3600, left / 60 % 60)
    }

    pub fn save_to(&mut self, g: &mut dyn Gfx, slot: u8) {
        let meta = self.save_meta();
        if let Some(w) = self.world.as_ref().filter(|w| !w.over) {
            let data = w.save_bytes();
            g.save_slot(slot, &data, &meta);
        }
    }

    /// 中断される前に自動記録する（アプリが裏に回ったときなど）
    pub fn autosave_now(&mut self, g: &mut dyn Gfx) {
        if self.screen == Screen::Play {
            self.save_to(g, 0);
        }
    }

    /// 記録を読み込んで再開する（一時停止した状態で始まる）
    pub fn apply_loaded(&mut self, g: &mut dyn Gfx, data: &[u8]) {
        let w = match World::load_bytes(data) {
            Ok(w) => w,
            Err(e) => {
                self.notify(&e);
                return;
            }
        };
        self.terrain = paint::paint_all(&w);
        for (i, sc) in w.scar_log.iter().enumerate() {
            paint::stamp_scar(&mut self.terrain, sc.0, sc.1, sc.2, i as u32);
        }
        if self.terrain_tex == 0 && self.overlay.is_empty() {
            self.terrain_tex = g.create_texture(paint::TW as u32, paint::TH as u32, &self.terrain);
            self.overlay = vec![0; W * H];
            self.overlay_tex = g.create_texture(W as u32, H as u32, &self.overlay);
        } else {
            g.update_texture(self.terrain_tex, 0, 0, paint::TW as u32, paint::TH as u32, &self.terrain);
        }
        self.viewer = w.player;
        // 前線の真ん中あたりを映す
        self.cam = (205.0, 105.0);
        let (sw, _) = g.size();
        self.zoom = (sw / 150.0).max(5.0);
        self.selected.clear();
        self.sel_bld = None;
        self.groups = Default::default();
        self.mode = Mode::Normal;
        self.paused = true;
        self.speed = 1;
        self.acc = 0.0;
        self.particles.clear();
        self.streaks.clear();
        self.ticker.clear();
        self.banner.clear();
        self.show_help = false;
        self.show_brief = false;
        self.show_chron = false;
        self.show_save = false;
        self.show_prod = false;
        self.place_tap = None;
        self.chron_seen = w.chronicle.len();
        self.overlay_t = 0.0;
        self.autosave_t = 0.0;
        let date = w.date_text();
        self.world = Some(w);
        self.screen = Screen::Play;
        self.toast = Some((format!("{date}から再開（一時停止中）"), 5.0));
    }

    /// 端末側からの知らせを出す
    pub fn notify(&mut self, s: &str) {
        self.toast = Some((s.to_string(), 5.0));
    }

    pub fn drag_box(&self) -> Option<((f32, f32), (f32, f32))> {
        self.drag.map(|d| (d, self.mouse)).filter(|(a, b)| (a.0 - b.0).abs() > 5.0 || (a.1 - b.1).abs() > 5.0)
    }

    pub fn job_line(&self) -> Option<((f32, f32), (f32, f32))> {
        self.job_start.map(|d| (d, self.mouse))
    }
}

fn upload(g: &mut dyn Gfx, tex: u32, buf: &[u32], x: u32, y: u32, w: u32, h: u32) {
    if w == 0 || h == 0 {
        return;
    }
    let mut sub = Vec::with_capacity((w * h) as usize);
    for row in y..y + h {
        let s = row as usize * paint::TW + x as usize;
        sub.extend_from_slice(&buf[s..s + w as usize]);
    }
    g.update_texture(tex, x, y, w, h, &sub);
}
