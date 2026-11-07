//! ドローン戦線 — ブラウザ版の入口
//!
//! Win32 のメッセージループの代わりに、web/main.js から入力とフレーム更新を受け取る。
//! 描画命令は WebGL2（図形）と Canvas2D（文字）に橋渡しする。

use std::cell::RefCell;
use yydrone_game::gfx::{Col, Font, Gfx};
use yydrone_game::{Ev, Game};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn js_log(p: *const u8, l: usize);
    fn js_now() -> f64;
    fn js_random() -> u32;
    fn js_tex_create(w: u32, h: u32, p: *const u32) -> u32;
    fn js_tex_update(id: u32, x: u32, y: u32, w: u32, h: u32, p: *const u32);
    fn js_gl_begin(r: f32, g: f32, b: f32, a: f32);
    fn js_gl_xform(sx: f32, sy: f32, ox: f32, oy: f32);
    fn js_gl_draw(p: *const Vertex, n: u32, tex: u32, linear: u32);
    fn js_text_begin(ui: f32);
    fn js_text(p: *const u8, l: usize, x: f32, y: f32, w: f32, h: f32, f: u32, r: f32, g: f32, b: f32, a: f32);
    fn js_measure(p: *const u8, l: usize, f: u32) -> f32;
    fn js_measure_h(p: *const u8, l: usize, f: u32, w: f32) -> f32;
    fn js_fullscreen_toggle();
    fn js_save(slot: u32, p: *const u8, l: usize, mp: *const u8, ml: usize);
    fn js_load(slot: u32);
    fn js_slot_info(slot: u32, out: *mut u8, cap: usize) -> usize;
    fn js_export(slot: u32);
    fn js_import();
}

// wasm 以外では何もしない（cargo check --workspace のため）
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments, clippy::missing_safety_doc)]
mod stub {
    use super::Vertex;
    pub unsafe fn js_log(_: *const u8, _: usize) {}
    pub unsafe fn js_now() -> f64 {
        0.0
    }
    pub unsafe fn js_random() -> u32 {
        0
    }
    pub unsafe fn js_tex_create(_: u32, _: u32, _: *const u32) -> u32 {
        0
    }
    pub unsafe fn js_tex_update(_: u32, _: u32, _: u32, _: u32, _: u32, _: *const u32) {}
    pub unsafe fn js_gl_begin(_: f32, _: f32, _: f32, _: f32) {}
    pub unsafe fn js_gl_xform(_: f32, _: f32, _: f32, _: f32) {}
    pub unsafe fn js_gl_draw(_: *const Vertex, _: u32, _: u32, _: u32) {}
    pub unsafe fn js_text_begin(_: f32) {}
    pub unsafe fn js_text(_: *const u8, _: usize, _: f32, _: f32, _: f32, _: f32, _: u32, _: f32, _: f32, _: f32, _: f32) {}
    pub unsafe fn js_measure(_: *const u8, _: usize, _: u32) -> f32 {
        0.0
    }
    pub unsafe fn js_measure_h(_: *const u8, _: usize, _: u32, _: f32) -> f32 {
        0.0
    }
    pub unsafe fn js_fullscreen_toggle() {}
    pub unsafe fn js_save(_: u32, _: *const u8, _: usize, _: *const u8, _: usize) {}
    pub unsafe fn js_load(_: u32) {}
    pub unsafe fn js_slot_info(_: u32, _: *mut u8, _: usize) -> usize {
        0
    }
    pub unsafe fn js_export(_: u32) {}
    pub unsafe fn js_import() {}
}
#[cfg(not(target_arch = "wasm32"))]
use stub::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    c: Col,
}

const MAX_VERTS: usize = 300_000;

struct WebGfx {
    pw: u32,
    ph: u32,
    width: f32,
    height: f32,
    ui: f32,
    user_scale: f32,
    /// タッチ向けに詰めた画面（UI を約 1000×480 に収める）
    compact: bool,
    verts: Vec<Vertex>,
}

impl WebGfx {
    fn new(w: u32, h: u32) -> WebGfx {
        let mut g = WebGfx { pw: w, ph: h, width: w as f32, height: h as f32, ui: 1.0, user_scale: 1.0, compact: false, verts: Vec::with_capacity(MAX_VERTS) };
        g.update_scale();
        g
    }
    /// 画面の高さ 900px を基準に UI を拡大する（最小 1280×680 が収まる範囲で）
    fn update_scale(&mut self) {
        let (pw, ph) = (self.pw as f32, self.ph as f32);
        if self.compact {
            // スマートフォンは高さで、タブレットは幅で大きさが決まる（最小 900×420）
            let cap = (ph / 420.0).min(pw / 900.0).max(0.3);
            self.ui = ((ph / 480.0).min(pw / 1000.0) * self.user_scale).clamp(0.3, cap);
            self.width = (pw / self.ui).round().max(1.0);
            self.height = (ph / self.ui).round().max(1.0);
            return;
        }
        let cap = (ph / 680.0).min(pw / 1280.0).max(0.3);
        self.ui = (ph / 900.0 * self.user_scale).clamp(0.3, cap);
        self.width = (pw / self.ui).round().max(1.0);
        self.height = (ph / self.ui).round().max(1.0);
    }
}

impl Gfx for WebGfx {
    fn size(&self) -> (f32, f32) {
        (self.width, self.height)
    }
    fn begin(&mut self, c: Col) {
        unsafe { js_gl_begin(c[0], c[1], c[2], c[3]) }
        self.verts.clear();
    }
    fn set_xform(&mut self, sx: f32, sy: f32, ox: f32, oy: f32) {
        self.flush();
        unsafe { js_gl_xform(sx, sy, ox, oy) }
    }
    #[inline]
    fn tri(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Col) {
        if self.verts.len() + 3 > MAX_VERTS {
            self.flush();
        }
        self.verts.push(Vertex { x: a.0, y: a.1, u: 0.0, v: 0.0, c: col });
        self.verts.push(Vertex { x: b.0, y: b.1, u: 0.0, v: 0.0, c: col });
        self.verts.push(Vertex { x: c.0, y: c.1, u: 0.0, v: 0.0, c: col });
    }
    fn flush(&mut self) {
        if !self.verts.is_empty() {
            unsafe { js_gl_draw(self.verts.as_ptr(), self.verts.len() as u32, 0, 0) }
            self.verts.clear();
        }
    }
    fn create_texture(&mut self, w: u32, h: u32, data: &[u32]) -> u32 {
        unsafe { js_tex_create(w, h, data.as_ptr()) }
    }
    fn update_texture(&mut self, id: u32, x: u32, y: u32, w: u32, h: u32, data: &[u32]) {
        unsafe { js_tex_update(id, x, y, w, h, data.as_ptr()) }
    }
    fn draw_textured(&mut self, id: u32, x0: f32, y0: f32, x1: f32, y1: f32, c: Col, linear: bool) {
        self.flush();
        let v = |x, y, u, vv| Vertex { x, y, u, v: vv, c };
        let q = [v(x0, y0, 0.0, 0.0), v(x1, y0, 1.0, 0.0), v(x1, y1, 1.0, 1.0), v(x0, y0, 0.0, 0.0), v(x1, y1, 1.0, 1.0), v(x0, y1, 0.0, 1.0)];
        unsafe { js_gl_draw(q.as_ptr(), 6, id, linear as u32) }
    }
    fn text_begin(&mut self) {
        self.flush();
        unsafe { js_text_begin(self.ui) }
    }
    fn text(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: Col) {
        unsafe { js_text(s.as_ptr(), s.len(), x, y, w, h, f as u32, c[0], c[1], c[2], c[3]) }
    }
    fn text_end(&mut self) {}
    fn measure(&mut self, s: &str, f: Font) -> f32 {
        unsafe { js_measure(s.as_ptr(), s.len(), f as u32) }
    }
    fn measure_h(&mut self, s: &str, f: Font, w: f32) -> f32 {
        unsafe { js_measure_h(s.as_ptr(), s.len(), f as u32, w) }
    }
    fn present(&mut self) {
        self.flush();
    }
    fn resize(&mut self, w: u32, h: u32) {
        self.pw = w.max(1);
        self.ph = h.max(1);
        self.update_scale();
    }
    fn adjust_scale(&mut self, d: i32) {
        self.user_scale = (self.user_scale * 1.12f32.powi(d)).clamp(0.6, 2.2);
        self.update_scale();
    }
    fn toggle_fullscreen(&mut self) {
        unsafe { js_fullscreen_toggle() }
    }
    fn set_compact(&mut self, c: bool) {
        if self.compact != c {
            self.compact = c;
            self.update_scale();
        }
    }
    fn save_slot(&mut self, slot: u8, data: &[u8], meta: &str) {
        unsafe { js_save(slot as u32, data.as_ptr(), data.len(), meta.as_ptr(), meta.len()) }
    }
    /// IndexedDB は非同期なので、読めたら yy_loaded に届く
    fn load_slot(&mut self, slot: u8) -> Option<Vec<u8>> {
        unsafe { js_load(slot as u32) }
        None
    }
    fn slot_info(&mut self, slot: u8) -> Option<String> {
        let mut buf = [0u8; 400];
        let n = unsafe { js_slot_info(slot as u32, buf.as_mut_ptr(), buf.len()) };
        if n == 0 {
            None
        } else {
            Some(String::from_utf8_lossy(&buf[..n]).to_string())
        }
    }
    fn has_files(&self) -> bool {
        true
    }
    fn export_slot(&mut self, slot: u8) {
        unsafe { js_export(slot as u32) }
    }
    fn import_file(&mut self) {
        unsafe { js_import() }
    }
}

struct App {
    gfx: WebGfx,
    game: Game,
    events: Vec<Ev>,
    /// ブラウザから届いた記録（次のフレームで読み込む）
    pending_load: Option<Vec<u8>>,
    pending_notice: Vec<String>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn hook(info: &std::panic::PanicHookInfo) {
    let s = info.to_string();
    unsafe { js_log(s.as_ptr(), s.len()) }
}

#[no_mangle]
pub extern "C" fn yy_init(w: u32, h: u32, compact: u32, touch: u32) {
    std::panic::set_hook(Box::new(hook));
    let seed = unsafe { (js_random() as u64) << 20 ^ js_now() as u64 };
    let mut gfx = WebGfx::new(w.max(1), h.max(1));
    let mut game = Game::new(seed);
    gfx.set_compact(compact != 0);
    game.compact = compact != 0;
    game.touch = touch != 0;
    let app = App { gfx, game, events: Vec::new(), pending_load: None, pending_notice: Vec::new() };
    APP.with(|a| *a.borrow_mut() = Some(app));
}

/// kind: 0=移動(x,y) 1=押下(b,x,y) 2=離す(b,x,y) 3=ホイール(d) 4=キー(vk,down) 6=リサイズ(w,h)
/// 7=なぞったスクロール(実ピクセル) 8=詰めた画面(0/1) 9=タッチ操作(0/1)
#[no_mangle]
pub extern "C" fn yy_event(kind: u32, a: f32, b: f32, c: f32) {
    APP.with(|cell| {
        let mut cell = cell.borrow_mut();
        let Some(app) = cell.as_mut() else { return };
        let ui = app.gfx.ui;
        let e = match kind {
            // 画面の外（-1）はそのまま伝える
            0 => {
                if a < 0.0 {
                    Ev::MouseMove(-1.0, -1.0)
                } else {
                    Ev::MouseMove(a / ui, b / ui)
                }
            }
            1 => Ev::Down(a as u8, b / ui, c / ui),
            2 => Ev::Up(a as u8, b / ui, c / ui),
            3 => Ev::Wheel(a),
            4 => Ev::Key(a as u32, b != 0.0),
            6 => Ev::Resize(a as u32, b as u32),
            7 => Ev::Scroll(a / ui),
            8 => Ev::Compact(a != 0.0),
            9 => Ev::Touch(a != 0.0),
            _ => return,
        };
        app.events.push(e);
    });
}

#[no_mangle]
pub extern "C" fn yy_frame(dt: f32) {
    APP.with(|cell| {
        let mut cell = cell.borrow_mut();
        let Some(app) = cell.as_mut() else { return };
        let evs = std::mem::take(&mut app.events);
        if let Some(data) = app.pending_load.take() {
            app.game.apply_loaded(&mut app.gfx, &data);
        }
        for n in std::mem::take(&mut app.pending_notice) {
            app.game.notify(&n);
        }
        app.game.frame(&mut app.gfx, &evs, dt);
    });
}

/// Current screen for browser controls (independent of overlays and hit targets).
#[no_mangle]
pub extern "C" fn yy_is_title() -> u32 {
    APP.with(|cell| cell.borrow().as_ref().map_or(1, |app| (app.game.screen == yydrone_game::Screen::Title) as u32))
}

/// タッチした場所に何があるか（実ピクセル）。0=地図 1=画面部品 2=押してすぐ反応 3=なぞれる一覧
#[no_mangle]
pub extern "C" fn yy_hit(x: f32, y: f32) -> u32 {
    APP.with(|cell| {
        let cell = cell.borrow();
        let Some(app) = cell.as_ref() else { return 1 };
        app.game.hit_kind(&app.gfx, x / app.gfx.ui, y / app.gfx.ui)
    })
}

/// ブラウザが記録のデータを書き込むための領域を確保する
#[no_mangle]
pub extern "C" fn yy_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// 記録のデータが届いた（yy_alloc で確保した領域。所有権を受け取る）
///
/// # Safety
/// ptr は yy_alloc(len) が返した領域で、len バイト書き込まれていること
#[no_mangle]
pub unsafe extern "C" fn yy_loaded(ptr: *mut u8, len: usize) {
    let data = Vec::from_raw_parts(ptr, len, len.max(1));
    APP.with(|cell| {
        if let Some(app) = cell.borrow_mut().as_mut() {
            app.pending_load = Some(data);
        }
    });
}

/// ブラウザ側からの通知（yy_alloc で確保した UTF-8 文字列）
///
/// # Safety
/// ptr は yy_alloc(len) が返した領域で、len バイト書き込まれていること
#[no_mangle]
pub unsafe extern "C" fn yy_notice(ptr: *mut u8, len: usize) {
    let data = Vec::from_raw_parts(ptr, len, len.max(1));
    let s = String::from_utf8_lossy(&data).to_string();
    APP.with(|cell| {
        if let Some(app) = cell.borrow_mut().as_mut() {
            app.pending_notice.push(s);
        }
    });
}

/// 画面が隠れた（スマートフォンでは裏に回ると閉じられることがある）：すぐ自動記録する
#[no_mangle]
pub extern "C" fn yy_autosave() {
    APP.with(|cell| {
        if let Some(app) = cell.borrow_mut().as_mut() {
            app.game.autosave_now(&mut app.gfx);
        }
    });
}
