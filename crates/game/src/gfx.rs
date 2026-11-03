//! 描画の窓口。Windows 版（Direct3D11 / DirectWrite）とブラウザ版（WebGL2 / Canvas2D）が実装する
//!
//! 図形はすべて三角形バッチ、文字だけ別の層に描く。文字は常に図形の上に重なる。

pub type Col = [f32; 4];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Font {
    Small,
    Normal,
    NormalCenter,
    Big,
    BigCenter,
    Title,
    TitleMid,
}

pub trait Gfx {
    /// UI 座標での画面の幅と高さ
    fn size(&self) -> (f32, f32);
    fn begin(&mut self, clear: Col);
    /// pos * (sx, sy) + (ox, oy) が NDC になる変換
    fn set_xform(&mut self, sx: f32, sy: f32, ox: f32, oy: f32);
    fn tri(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Col);
    fn flush(&mut self);
    /// RGBA（下位バイトが R）の画像からテクスチャを作る
    fn create_texture(&mut self, w: u32, h: u32, data: &[u32]) -> u32;
    fn update_texture(&mut self, id: u32, x: u32, y: u32, w: u32, h: u32, data: &[u32]);
    fn draw_textured(&mut self, id: u32, x0: f32, y0: f32, x1: f32, y1: f32, col: Col, linear: bool);
    /// 図形を描き終えて文字の層を始める
    fn text_begin(&mut self);
    fn text(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: Col);
    fn text_end(&mut self);
    fn measure(&mut self, s: &str, f: Font) -> f32;
    fn measure_h(&mut self, s: &str, f: Font, w: f32) -> f32;
    fn present(&mut self);
    fn resize(&mut self, w: u32, h: u32);
    fn adjust_scale(&mut self, d: i32);
    fn toggle_fullscreen(&mut self);

    // ---- 端末の機能（対応しない版は何もしない）
    /// タッチ向けの詰めた画面に切り替える
    fn set_compact(&mut self, _c: bool) {}
    /// 記録を保存する（枠 0 は自動記録）
    fn save_slot(&mut self, _slot: u8, _data: &[u8], _meta: &str) {}
    /// 記録を読む。その場で読めなければ None を返し、あとで Game::apply_loaded に届ける
    fn load_slot(&mut self, _slot: u8) -> Option<Vec<u8>> {
        None
    }
    /// 枠の見出し（タブ区切りで内容と日時）
    fn slot_info(&mut self, _slot: u8) -> Option<String> {
        None
    }
    /// 記録をファイルとやりとりできるか
    fn has_files(&self) -> bool {
        false
    }
    fn export_slot(&mut self, _slot: u8) {}
    fn import_file(&mut self) {}

    fn set_screen_xform(&mut self) {
        let (w, h) = self.size();
        self.set_xform(2.0 / w, -2.0 / h, -1.0, 1.0);
    }
}

/// 三角形から組み立てる図形
pub trait Shapes {
    fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, c: Col);
    fn rect_outline(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, c: Col);
    fn circle(&mut self, cx: f32, cy: f32, r: f32, seg: usize, c: Col);
    fn ring(&mut self, cx: f32, cy: f32, r: f32, t: f32, seg: usize, c: Col);
    fn line(&mut self, a: (f32, f32), b: (f32, f32), t: f32, c: Col);
    /// 中心・向き・大きさを指定して凸多角形を描く
    fn poly(&mut self, cx: f32, cy: f32, ang: f32, s: f32, pts: &[(f32, f32)], c: Col);
    fn shadow_text(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: Col);
}

impl<T: Gfx + ?Sized> Shapes for T {
    fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, c: Col) {
        self.tri((x0, y0), (x1, y0), (x1, y1), c);
        self.tri((x0, y0), (x1, y1), (x0, y1), c);
    }
    fn rect_outline(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, c: Col) {
        self.rect(x0, y0, x1, y0 + t, c);
        self.rect(x0, y1 - t, x1, y1, c);
        self.rect(x0, y0 + t, x0 + t, y1 - t, c);
        self.rect(x1 - t, y0 + t, x1, y1 - t, c);
    }
    fn circle(&mut self, cx: f32, cy: f32, r: f32, seg: usize, c: Col) {
        let step = std::f32::consts::TAU / seg as f32;
        for i in 0..seg {
            let a0 = i as f32 * step;
            let a1 = a0 + step;
            self.tri((cx, cy), (cx + a0.cos() * r, cy + a0.sin() * r), (cx + a1.cos() * r, cy + a1.sin() * r), c);
        }
    }
    fn ring(&mut self, cx: f32, cy: f32, r: f32, t: f32, seg: usize, c: Col) {
        let step = std::f32::consts::TAU / seg as f32;
        let r2 = r - t;
        for i in 0..seg {
            let a0 = i as f32 * step;
            let a1 = a0 + step;
            let p0 = (cx + a0.cos() * r, cy + a0.sin() * r);
            let p1 = (cx + a1.cos() * r, cy + a1.sin() * r);
            let q0 = (cx + a0.cos() * r2, cy + a0.sin() * r2);
            let q1 = (cx + a1.cos() * r2, cy + a1.sin() * r2);
            self.tri(p0, p1, q1, c);
            self.tri(p0, q1, q0, c);
        }
    }
    fn line(&mut self, a: (f32, f32), b: (f32, f32), t: f32, c: Col) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-5);
        let (nx, ny) = (-dy / l * t * 0.5, dx / l * t * 0.5);
        self.tri((a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny), (b.0 - nx, b.1 - ny), c);
        self.tri((a.0 + nx, a.1 + ny), (b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny), c);
    }
    fn poly(&mut self, cx: f32, cy: f32, ang: f32, s: f32, pts: &[(f32, f32)], c: Col) {
        let (sn, cs) = ang.sin_cos();
        let m = |p: (f32, f32)| (cx + (p.0 * cs - p.1 * sn) * s, cy + (p.0 * sn + p.1 * cs) * s);
        let p0 = m(pts[0]);
        for w in pts[1..].windows(2) {
            self.tri(p0, m(w[0]), m(w[1]), c);
        }
    }
    fn shadow_text(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: Col) {
        self.text(s, x + 1.0, y + 1.0, w, h, f, [0.0, 0.0, 0.0, c[3] * 0.85]);
        self.text(s, x, y, w, h, f, c);
    }
}
