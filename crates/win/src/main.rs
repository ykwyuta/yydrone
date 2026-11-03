#![cfg_attr(windows, windows_subsystem = "windows")]
//! ドローン戦線 — Windows 版（Rust + Direct3D11 / Direct2D・DirectWrite）

#[cfg(windows)]
mod gfx;

#[cfg(not(windows))]
fn main() {
    eprintln!("このクレートは Windows 専用です。ブラウザ版は build.sh でビルドしてください。");
}

#[cfg(windows)]
fn main() {
    if let Err(e) = app::run() {
        app::show_error(&format!("{e:?}"));
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod app {
    use crate::gfx::WinGfx;
    use std::cell::RefCell;
    use windows::core::*;
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::HiDpi::*;
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use yydrone_game::gfx::Gfx;
    use yydrone_game::{Ev, Game};

    thread_local! {
        static EVENTS: RefCell<Vec<Ev>> = const { RefCell::new(Vec::new()) };
        /// UI 拡大率（マウス座標の換算に使う）
        static UI: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
    }

    fn push(e: Ev) {
        EVENTS.with(|v| v.borrow_mut().push(e));
    }

    fn lp_xy(lp: LPARAM) -> (f32, f32) {
        let x = (lp.0 & 0xffff) as i16 as f32;
        let y = ((lp.0 >> 16) & 0xffff) as i16 as f32;
        let u = UI.with(|u| u.get());
        (x / u, y / u)
    }

    extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        unsafe {
            match msg {
                WM_DESTROY => {
                    PostQuitMessage(0);
                    LRESULT(0)
                }
                WM_SIZE => {
                    push(Ev::Resize((lp.0 & 0xffff) as u32, ((lp.0 >> 16) & 0xffff) as u32));
                    LRESULT(0)
                }
                WM_MOUSEMOVE => {
                    let (x, y) = lp_xy(lp);
                    push(Ev::MouseMove(x, y));
                    LRESULT(0)
                }
                WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
                    let (x, y) = lp_xy(lp);
                    SetCapture(hwnd);
                    let b = match msg {
                        WM_LBUTTONDOWN => 0,
                        WM_RBUTTONDOWN => 1,
                        _ => 2,
                    };
                    push(Ev::Down(b, x, y));
                    LRESULT(0)
                }
                WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
                    let (x, y) = lp_xy(lp);
                    let _ = ReleaseCapture();
                    let b = match msg {
                        WM_LBUTTONUP => 0,
                        WM_RBUTTONUP => 1,
                        _ => 2,
                    };
                    push(Ev::Up(b, x, y));
                    LRESULT(0)
                }
                WM_MOUSEWHEEL => {
                    push(Ev::Wheel(((wp.0 >> 16) & 0xffff) as i16 as f32 / 120.0));
                    LRESULT(0)
                }
                WM_KEYDOWN | WM_SYSKEYDOWN => {
                    if wp.0 as u32 == VK_F4.0 as u32 && msg == WM_SYSKEYDOWN {
                        return DefWindowProcW(hwnd, msg, wp, lp);
                    }
                    push(Ev::Key(wp.0 as u32, true));
                    LRESULT(0)
                }
                WM_KEYUP | WM_SYSKEYUP => {
                    push(Ev::Key(wp.0 as u32, false));
                    LRESULT(0)
                }
                WM_ERASEBKGND => LRESULT(1),
                _ => DefWindowProcW(hwnd, msg, wp, lp),
            }
        }
    }

    pub fn show_error(s: &str) {
        unsafe {
            MessageBoxW(None, &HSTRING::from(s), w!("ドローン戦線"), MB_OK | MB_ICONERROR);
        }
    }

    /// 自動確認用のコマンドライン：--smoke <秒> [--shot 出力.bmp] [--side 0|1|-1] [--dur 戦争の秒数] [--speed 倍率] [--chron] [--load 枠]
    struct Smoke {
        secs: f32,
        shot: Option<String>,
        side: i32,
        dur: Option<f32>,
        speed: Option<u32>,
        chron: bool,
        save: Option<u8>,
        load: Option<u8>,
    }

    fn parse_smoke() -> Option<Smoke> {
        let args: Vec<String> = std::env::args().collect();
        let pos = args.iter().position(|a| a == "--smoke")?;
        let secs = args.get(pos + 1)?.parse().ok()?;
        let val = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1).cloned());
        Some(Smoke { secs, shot: val("--shot"), side: val("--side").and_then(|s| s.parse().ok()).unwrap_or(-1), dur: val("--dur").and_then(|s| s.parse().ok()), speed: val("--speed").and_then(|s| s.parse().ok()), chron: args.iter().any(|a| a == "--chron"), save: val("--save").and_then(|s| s.parse().ok()), load: val("--load").and_then(|s| s.parse().ok()) })
    }

    pub fn run() -> Result<()> {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let inst = GetModuleHandleW(None)?;
            let class = w!("YYDroneWnd");
            let icon = |sx: i32, sy: i32| -> HICON { LoadImageW(Some(inst.into()), PCWSTR(1 as *const u16), IMAGE_ICON, sx, sy, LR_DEFAULTCOLOR).map(|h| HICON(h.0)).unwrap_or_default() };
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                hInstance: inst.into(),
                hIcon: icon(GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON)),
                hIconSm: icon(GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON)),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassExW(&wc);
            let (w, h) = (1600, 900);
            let mut rc = RECT { left: 0, top: 0, right: w, bottom: h };
            let _ = AdjustWindowRect(&mut rc, WS_OVERLAPPEDWINDOW, false);
            let hwnd = CreateWindowExW(WINDOW_EX_STYLE::default(), class, w!("ドローン戦線"), WS_OVERLAPPEDWINDOW | WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT, rc.right - rc.left, rc.bottom - rc.top, None, None, Some(inst.into()), None)?;
            let mut cr = RECT::default();
            GetClientRect(hwnd, &mut cr)?;
            let mut gfx = WinGfx::new(hwnd, (cr.right - cr.left).max(1) as u32, (cr.bottom - cr.top).max(1) as u32)?;
            let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
            let mut game = Game::new(seed);
            let smoke = parse_smoke();
            if let Some(s) = &smoke {
                game.configure(s.side, 1, 42);
                game.start_game(&mut gfx);
                game.show_brief = false;
                if let (Some(d), Some(w)) = (s.dur, game.world.as_mut()) {
                    w.duration = d;
                }
                if let Some(v) = s.speed {
                    game.speed = v;
                }
                if let Some(data) = s.load.and_then(|n| gfx.load_slot(n)) {
                    game.apply_loaded(&mut gfx, &data);
                    game.paused = false;
                }
            }
            let mut last = std::time::Instant::now();
            let started = last;
            let mut msg = MSG::default();
            'outer: loop {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        break 'outer;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                UI.with(|u| u.set(gfx.ui));
                // ウィンドウの外や非アクティブ時は画面端スクロールを止める
                let mut cp = POINT::default();
                let mut crc = RECT::default();
                let _ = GetCursorPos(&mut cp);
                let _ = ScreenToClient(hwnd, &mut cp);
                let _ = GetClientRect(hwnd, &mut crc);
                if GetForegroundWindow() != hwnd || cp.x < 0 || cp.y < 0 || cp.x >= crc.right || cp.y >= crc.bottom {
                    push(Ev::MouseMove(-1.0, -1.0));
                }
                let evs: Vec<Ev> = EVENTS.with(|v| std::mem::take(&mut *v.borrow_mut()));
                let now = std::time::Instant::now();
                let dt = (now - last).as_secs_f32().min(0.1);
                last = now;
                if game.frame(&mut gfx, &evs, dt) {
                    break;
                }
                if let Some(s) = &smoke {
                    if s.chron && (now - started).as_secs_f32() >= s.secs - 1.0 {
                        game.show_chron = true;
                    }
                    if (now - started).as_secs_f32() >= s.secs {
                        if let Some(p) = &s.shot {
                            gfx.save_backbuffer(p)?;
                        }
                        if let Some(n) = s.save {
                            game.save_to(&mut gfx, n);
                        }
                        break;
                    }
                }
                if IsIconic(hwnd).as_bool() {
                    std::thread::sleep(std::time::Duration::from_millis(30));
                }
            }
            // 閉じるときに自動記録する（自動確認のときは記録を荒らさない）
            if smoke.is_none() {
                game.autosave_now(&mut gfx);
            }
        }
        Ok(())
    }
}
