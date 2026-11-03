//! 画面なしで入力イベントを流して、指揮中の一時停止を確かめる

use super::*;

/// 何も描かない描画層
struct NullGfx;

impl Gfx for NullGfx {
    fn size(&self) -> (f32, f32) {
        (1600.0, 900.0)
    }
    fn begin(&mut self, _: Col) {}
    fn set_xform(&mut self, _: f32, _: f32, _: f32, _: f32) {}
    fn tri(&mut self, _: (f32, f32), _: (f32, f32), _: (f32, f32), _: Col) {}
    fn flush(&mut self) {}
    fn create_texture(&mut self, _: u32, _: u32, _: &[u32]) -> u32 {
        1
    }
    fn update_texture(&mut self, _: u32, _: u32, _: u32, _: u32, _: u32, _: &[u32]) {}
    fn draw_textured(&mut self, _: u32, _: f32, _: f32, _: f32, _: f32, _: Col, _: bool) {}
    fn text_begin(&mut self) {}
    fn text(&mut self, _: &str, _: f32, _: f32, _: f32, _: f32, _: Font, _: Col) {}
    fn text_end(&mut self) {}
    fn measure(&mut self, _: &str, _: Font) -> f32 {
        10.0
    }
    fn measure_h(&mut self, _: &str, _: Font, _: f32) -> f32 {
        20.0
    }
    fn present(&mut self) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn adjust_scale(&mut self, _: i32) {}
    fn toggle_fullscreen(&mut self) {}
}

fn game() -> (Game, NullGfx) {
    let mut g = NullGfx;
    let mut game = Game::new(5);
    game.configure(0, 1, 5);
    game.start_game(&mut g);
    game.show_brief = false;
    (game, g)
}

fn time(game: &Game) -> f32 {
    game.world.as_ref().unwrap().t
}

/// 数フレーム進めて、シミュレーションが進んだか返す
fn runs(game: &mut Game, g: &mut NullGfx) -> bool {
    let before = time(game);
    for _ in 0..6 {
        game.frame(g, &[], 0.05);
    }
    time(game) > before
}

/// 自軍の歩兵の id を返して、カメラをそこへ向ける
fn focus_own_unit(game: &mut Game, nth: usize) -> u32 {
    let w = game.world.as_ref().unwrap();
    let u = w.units.iter().filter(|u| u.side == 0 && u.kind == UK::Infantry).nth(nth).unwrap();
    let (id, x, y) = (u.id, u.x, u.y);
    game.cam = (x, y);
    id
}

fn own_unit_screen(game: &Game, g: &NullGfx, id: u32) -> (f32, f32) {
    let u = game.world.as_ref().unwrap().unit(id).unwrap();
    game.world_to_screen(g, u.x, u.y)
}

/// 左クリックでその部隊を選ぶ
fn click_unit(game: &mut Game, g: &mut NullGfx, id: u32) {
    let (x, y) = own_unit_screen(game, g, id);
    game.frame(g, &[Ev::MouseMove(x, y), Ev::Down(0, x, y), Ev::Up(0, x, y)], 0.0);
    assert!(game.selected.contains(&id), "選択できていない");
}

/// 部隊のいない地図の上の点（カメラの真上の方向で、パネルにかからない）
fn empty_spot(game: &Game, g: &NullGfx) -> (f32, f32) {
    let w = game.world.as_ref().unwrap();
    let (sw, _) = g.size();
    for sy in [240.0, 280.0, 340.0, 400.0, 460.0, 520.0] {
        for sx in [sw * 0.3, sw * 0.4, sw * 0.5, sw * 0.6, sw * 0.7] {
            let (wx, wy) = game.screen_to_world(g, sx, sy);
            if w.units.iter().all(|u| (u.x - wx).abs() > 6.0 || (u.y - wy).abs() > 6.0) {
                return (sx, sy);
            }
        }
    }
    (sw * 0.5, 320.0)
}

#[test]
fn time_runs_with_nothing_selected() {
    let (mut game, mut g) = game();
    assert!(runs(&mut game, &mut g));
}

#[test]
fn selecting_stops_time_and_an_order_restarts_it() {
    let (mut game, mut g) = game();
    let id = focus_own_unit(&mut game, 0);
    click_unit(&mut game, &mut g, id);
    assert!(!runs(&mut game, &mut g), "選択中に時間が進んでいる");
    // 右クリックの命令で再開し、選択は残る
    let (x, y) = empty_spot(&game, &g);
    game.frame(&mut g, &[Ev::MouseMove(x, y), Ev::Down(1, x, y), Ev::Up(1, x, y)], 0.0);
    assert!(runs(&mut game, &mut g), "命令しても時間が進まない");
    assert!(game.selected.contains(&id));
}

#[test]
fn clearing_the_selection_restarts_time() {
    let (mut game, mut g) = game();
    let id = focus_own_unit(&mut game, 0);
    click_unit(&mut game, &mut g, id);
    assert!(!runs(&mut game, &mut g));
    game.frame(&mut g, &[Ev::Key(0x1B, true), Ev::Key(0x1B, false)], 0.0);
    assert!(game.selected.is_empty());
    assert!(runs(&mut game, &mut g));
}

#[test]
fn selecting_another_unit_after_an_order_stops_time_again() {
    let (mut game, mut g) = game();
    let id = focus_own_unit(&mut game, 0);
    click_unit(&mut game, &mut g, id);
    game.frame(&mut g, &[Ev::Key(0x53, true), Ev::Key(0x53, false)], 0.0); // S：停止
    assert!(runs(&mut game, &mut g), "停止の命令でも再開するはず");
    let other = focus_own_unit(&mut game, 1);
    game.selected.clear();
    game.frame(&mut g, &[], 0.0);
    click_unit(&mut game, &mut g, other);
    assert!(!runs(&mut game, &mut g), "選び直すと止まるはず");
}

#[test]
fn losing_a_selected_unit_does_not_stop_time() {
    let (mut game, mut g) = game();
    let (a, b) = (focus_own_unit(&mut game, 0), focus_own_unit(&mut game, 1));
    game.selected = vec![a, b];
    game.frame(&mut g, &[], 0.0);
    assert!(!runs(&mut game, &mut g));
    game.frame(&mut g, &[Ev::Key(0x53, true), Ev::Key(0x53, false)], 0.0);
    assert!(runs(&mut game, &mut g));
    // 片方が戦死して選択が減っても、再び止まらない
    let w = game.world.as_mut().unwrap();
    let i = w.unit_index(a).unwrap();
    w.units[i].dead = true;
    assert!(runs(&mut game, &mut g), "戦死で時間が止まった");
    assert!(!game.selected.contains(&a));
}

#[test]
fn dragging_a_box_stops_time() {
    let (mut game, mut g) = game();
    focus_own_unit(&mut game, 0);
    let (x, y) = empty_spot(&game, &g);
    game.frame(&mut g, &[Ev::MouseMove(x, y), Ev::Down(0, x, y), Ev::MouseMove(x + 40.0, y + 30.0)], 0.0);
    assert!(!runs(&mut game, &mut g), "ドラッグ中に時間が進んでいる");
}

#[test]
fn choosing_a_target_mode_keeps_time_stopped_until_the_order() {
    let (mut game, mut g) = game();
    let id = focus_own_unit(&mut game, 0);
    click_unit(&mut game, &mut g, id);
    // Q（移動）を押しただけでは止まったまま
    game.frame(&mut g, &[Ev::Key(0x51, true), Ev::Key(0x51, false)], 0.0);
    assert_eq!(game.mode, Mode::MoveOnly);
    assert!(!runs(&mut game, &mut g));
    let (x, y) = empty_spot(&game, &g);
    game.frame(&mut g, &[Ev::MouseMove(x, y), Ev::Down(0, x, y), Ev::Up(0, x, y)], 0.0);
    assert_eq!(game.mode, Mode::Normal);
    assert!(runs(&mut game, &mut g), "移動先を決めたら再開するはず");
}

#[test]
fn touch_tap_order_restarts_time() {
    let (mut game, mut g) = game();
    game.touch = true;
    game.compact = true;
    let id = focus_own_unit(&mut game, 0);
    click_unit(&mut game, &mut g, id);
    assert!(!runs(&mut game, &mut g));
    let (x, y) = empty_spot(&game, &g);
    game.frame(&mut g, &[Ev::MouseMove(x, y), Ev::Down(0, x, y), Ev::Up(0, x, y)], 0.0);
    assert!(runs(&mut game, &mut g), "タップの命令で再開するはず");
}

#[test]
fn selecting_a_building_does_not_stop_time() {
    let (mut game, mut g) = game();
    let w = game.world.as_ref().unwrap();
    let id = w.blds.iter().find(|b| b.side == 0).unwrap().id;
    game.sel_bld = Some(id);
    assert!(runs(&mut game, &mut g));
}
