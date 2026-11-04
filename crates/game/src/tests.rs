//! 画面なしで入力イベントを流して、指揮中の一時停止を確かめる

use super::*;

/// 何も描かない描画層
#[derive(Default)]
struct NullGfx {
    texture_uploads: Vec<(u32, u32, u64)>,
    dimensions: Option<(f32,f32)>,
    saves: usize,
    portrait_cards: usize,
    texts: Vec<(String,f32,f32,f32,f32)>,
}

impl Gfx for NullGfx {
    fn size(&self) -> (f32, f32) {
        self.dimensions.unwrap_or((1600.0, 900.0))
    }
    fn begin(&mut self, _: Col) {self.texts.clear();}
    fn set_xform(&mut self, _: f32, _: f32, _: f32, _: f32) {}
    fn tri(&mut self, _: (f32, f32), _: (f32, f32), _: (f32, f32), _: Col) {}
    fn flush(&mut self) {}
    fn create_texture(&mut self, width: u32, height: u32, pixels: &[u32]) -> u32 {
        use std::hash::{Hash, Hasher};
        assert_eq!(pixels.len(), (width * height) as usize);
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        pixels.hash(&mut hash);
        self.texture_uploads.push((width, height, hash.finish()));
        self.texture_uploads.len() as u32
    }
    fn update_texture(&mut self, _: u32, _: u32, _: u32, _: u32, _: u32, _: &[u32]) {}
    fn draw_textured(&mut self, _:u32,x0:f32,y0:f32,x1:f32,y1:f32,_:Col,_:bool) {if ((x1-x0)-44.0).abs()<0.1&&((y1-y0)-44.0).abs()<0.1 {self.portrait_cards+=1;}}
    fn text_begin(&mut self) {}
    fn text(&mut self, s:&str,x:f32,y:f32,w:f32,h:f32,_:Font,_:Col) {self.texts.push((s.into(),x,y,w,h));}
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
    fn save_slot(&mut self, _:u8, _:&[u8], _:&str) {self.saves+=1;}
}

fn train_ticks(game:&mut Game, seconds:f32) {
    for _ in 0..(seconds/DT).ceil() as usize {game.world.as_mut().unwrap().tick();}
    game.update_tutorial();
}

#[test]
fn tutorial_all_thirteen_tasks_are_achievable_for_both_sides() {
    use yydrone_core::command::{Doctrine,Region};
    for side in 0..2 {
        let mut g=NullGfx::default();let mut game=Game::new(1);game.cfg_side=side as i32;
        game.start_tutorial(&mut g);game.tutorial_next(&mut g,false);
        for lesson in TRAINING_LESSONS.iter().copied().skip(1).take(TRAINING_TASKS) {
            assert_eq!(game.tutorial.as_ref().unwrap().lesson,lesson);
            game.update_tutorial();
            assert!(!game.tutorial.as_ref().unwrap().ready,"premature completion {lesson:?}");
            game.tutorial_next(&mut g,false);
            assert_eq!(game.tutorial.as_ref().unwrap().lesson,lesson,"cannot advance before completing");
            let actors=game.tutorial.as_ref().unwrap().actors;
            match lesson {
                TrainingLesson::Camera => {game.cam.0+=5.0;game.zoom*=1.15;game.update_tutorial();}
                TrainingLesson::Select => {click_unit(&mut game,&mut g,actors.infantry);}
                TrainingLesson::Move | TrainingLesson::Capture => {
                    let x=if lesson==TrainingLesson::Move{98.0}else{110.0};
                    game.world.as_mut().unwrap().command(&[actors.infantry],Cmd::AttackMove,x,100.0);
                    train_ticks(&mut game,65.0);
                }
                TrainingLesson::Combat => {game.world.as_mut().unwrap().command(&[actors.tank.unwrap()],Cmd::AttackMove,120.0,100.0);train_ticks(&mut game,35.0);}
                TrainingLesson::Production | TrainingLesson::ReconProduction => {
                    let kind=if lesson==TrainingLesson::Production{UK::Infantry}else{UK::ReconDrone};
                    game.world.as_mut().unwrap().queue_unit(side,kind).unwrap();train_ticks(&mut game,35.0);
                }
                TrainingLesson::Construction => {game.world.as_mut().unwrap().place_building(side,BK::Depot,94.0,106.0).unwrap();train_ticks(&mut game,60.0);}
                TrainingLesson::ReconMove => {game.world.as_mut().unwrap().command(&[actors.recon.unwrap()],Cmd::AttackMove,126.0,86.0);train_ticks(&mut game,65.0);}
                TrainingLesson::Fpv => {
                    game.world.as_mut().unwrap().queue_unit(side,UK::FpvTeam).unwrap();train_ticks(&mut game,25.0);
                    let id=game.world.as_ref().unwrap().units.iter().find(|u|u.side==side&&u.kind==UK::FpvTeam).unwrap().id;
                    game.world.as_mut().unwrap().command(&[id],Cmd::AttackMove,108.0,104.0);train_ticks(&mut game,100.0);
                }
                TrainingLesson::Delegate => {
                    let w=game.world.as_mut().unwrap();
                    w.assign_general(side,0,&[actors.infantry]).unwrap();w.fund_general(side,0,100.0).unwrap();
                    w.sides[side as usize].command.generals[0].doctrine=Doctrine::Defend;
                    w.sides[side as usize].command.generals[0].region=Region{x:108.0,y:94.0,radius:28.0};
                    game.update_tutorial();
                }
                TrainingLesson::Staff => {game.show_command=true;game.command_tab=1;game.command_staff_tab=0;game.update_tutorial();}
                TrainingLesson::Report => {
                    train_ticks(&mut game,12.0);assert!(!game.tutorial.as_ref().unwrap().ready);
                    game.show_command=true;game.command_tab=1;game.command_staff_tab=1;game.update_tutorial();
                }
                _ => unreachable!(),
            }
            game.update_tutorial();
            assert!(game.tutorial.as_ref().unwrap().ready,"cannot finish side {side} {lesson:?}");
            game.tutorial_next(&mut g,false);
        }
        assert_eq!(game.tutorial.as_ref().unwrap().lesson,TrainingLesson::Complete);
        assert_eq!(game.tutorial.as_ref().unwrap().results,[1;TRAINING_TASKS]);
    }
}

#[test]
fn tutorial_skips_restore_checkpoints_and_never_save_campaign_slots() {
    let mut g=NullGfx::default();let mut game=Game::new(1);
    game.start_game(&mut g);let normal=game.world.as_ref().unwrap().save_bytes();
    game.start_tutorial(&mut g);game.save_to(&mut g,1);game.autosave_now(&mut g);
    game.tutorial_next(&mut g,false);game.speed=8;
    for _ in 0..125 {game.advance(&mut g,1.0);}
    assert_eq!(g.saves,0);assert!(game.world.as_ref().unwrap().chronicle.is_empty());
    assert!(!game.world.as_ref().unwrap().over);
    game.tutorial_next(&mut g,true);
    assert_eq!(game.tutorial.as_ref().unwrap().results[0],2);
    assert_eq!(game.world.as_ref().unwrap().t,0.0);
    game.apply_loaded(&mut g,&normal);
    assert!(game.tutorial.is_none());assert!(!game.world.as_ref().unwrap().is_training());
    game.save_to(&mut g,1);assert_eq!(g.saves,1);
    game.start_tutorial(&mut g);game.start_game(&mut g);
    assert!(game.tutorial.is_none());assert!(!game.world.as_ref().unwrap().is_training());
}

fn game() -> (Game, NullGfx) {
    let mut g = NullGfx::default();
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

#[test]
fn texture_cache_keeps_faction_models_and_facilities_separate() {
    let mut art = crate::art::Art::default();
    let mut gfx = NullGfx::default();
    // Either faction may be encountered first, including in spectator mode.
    let russian = art.unit_texture(&mut gfx, UK::Shorad, 1).unwrap();
    let ukrainian = art.unit_texture(&mut gfx, UK::Shorad, 0).unwrap();
    assert_ne!(russian, ukrainian);
    assert_ne!(gfx.texture_uploads[0].2, gfx.texture_uploads[1].2);
    assert_eq!(art.unit_texture(&mut gfx, UK::Shorad, 1), Some(russian));
    assert_eq!(art.unit_texture(&mut gfx, UK::Shorad, 0), Some(ukrainian));
    assert_eq!(gfx.texture_uploads.len(), 2, "cached models were uploaded again");

    let workshop = art.bld_texture(&mut gfx, BK::Workshop).unwrap();
    let factory = art.bld_texture(&mut gfx, BK::Factory).unwrap();
    let depot = art.bld_texture(&mut gfx, BK::Depot).unwrap();
    let port = art.bld_texture(&mut gfx, BK::Port).unwrap();
    assert_ne!(workshop, factory);
    assert_ne!(depot, port);
    assert_ne!(gfx.texture_uploads[2].2, gfx.texture_uploads[3].2);
    assert_ne!(gfx.texture_uploads[4].2, gfx.texture_uploads[5].2);
    assert_eq!(art.bld_texture(&mut gfx, BK::Port), Some(port));
    assert_eq!(gfx.texture_uploads.len(), 6);
}

#[test]
fn headquarters_opens_pauses_and_sets_a_region_from_the_map() {
    let (mut game,mut gfx)=game();
    game.key_press(&mut gfx,0x72);assert!(game.show_command);assert!(game.modal_open());assert!(!runs(&mut game,&mut gfx));
    game.key_press(&mut gfx,0x1B);assert!(!game.show_command);
    game.mode=Mode::CommandArea(1);let expected=game.screen_to_world(&gfx,800.0,300.0);
    game.map_click(&gfx,800.0,300.0);
    assert!(game.show_command);assert_eq!(game.mode,Mode::Normal);
    let region=game.world.as_ref().unwrap().sides[0].command.generals[1].region;
    assert_eq!((region.x,region.y),expected);
}

#[test]
fn all_headquarters_pages_fit_the_compact_viewport() {
    let (mut game,mut gfx)=game();game.compact=true;gfx.dimensions=Some((1000.0,480.0));game.show_command=true;
    for (tab,detail,staff,general,side) in [(0,0,0,0,0),(0,1,0,4,0),(0,0,0,8,0),(1,0,0,0,0),(1,0,1,0,0),(0,0,0,0,1),(0,1,0,4,1),(0,0,0,8,1),(1,0,0,0,1),(1,0,1,0,1)] {
        game.command_tab=tab;game.command_detail_tab=detail;game.command_staff_tab=staff;game.command_general=general;game.viewer=Some(side);
        game.hit_rects.clear();let mut text=crate::ui::TextQ::default();
        game.draw_command(&mut gfx,&mut text,1000.0,480.0);
        for (x,y,x1,y1) in &game.hit_rects{assert!(*x>=0.0&&*y>=0.0&&*x1<=1000.0&&*y1<=480.0,"button outside viewport: {x},{y},{x1},{y1}");}
        for (s,x,y,w,h,_,_,_) in text.0{assert!(x>=0.0&&y>=0.0&&x+w<=1001.0&&y+h<=480.0,"text outside viewport: {s} at {x},{y},{w},{h}");}
    }
}

#[test]
fn disabled_controls_do_not_execute_headquarters_actions() {
    let (mut game,mut gfx)=game();let mut text=crate::ui::TextQ::default();
    game.pressed=Some((0,25.0,25.0));
    assert!(game.button(&mut gfx,&mut text,"disabled",10.0,10.0,100.0,32.0,false,false).is_none());
}

#[test]
fn phone_top_bar_opens_headquarters_with_no_generals_and_while_producing_or_selecting() {
    for dimensions in [(900.0,420.0),(1000.0,480.0)] {
        for production in [false,true] {
            for selection in [false,true] {
                let (mut game,mut gfx)=game();game.compact=true;game.touch=true;gfx.dimensions=Some(dimensions);
                game.show_prod=production;game.command_tab=1;game.command_detail_tab=2;
                if selection {game.selected.push(game.world.as_ref().unwrap().units.iter().find(|u|u.side==0).unwrap().id);}
                game.frame(&mut gfx,&[],0.0);
                let (_,x,y,w,h)=gfx.texts.iter().find(|(s,_,y,_,_)|s=="司令部"&&*y<game.top_h()).expect("phone needs a permanent headquarters entry").clone();
                assert!(x>=0.0&&x+w<=dimensions.0&&y+h<=game.top_h());
                let (x,y)=(x+w/2.0,y+h/2.0);
                game.frame(&mut gfx,&[Ev::Down(0,x,y),Ev::Up(0,x,y)],0.0);
                assert!(game.show_command,"top-bar tap did not open headquarters");
                assert_eq!(game.command_tab,0);assert_eq!(game.command_detail_tab,0);
                game.frame(&mut gfx,&[],0.0);
                assert!(gfx.texts.iter().any(|(s,_,_,_,_)|s=="積極攻勢"));
                assert!(gfx.texts.iter().any(|(s,_,_,_,_)|s=="全力防御"));
            }
        }
    }
}

#[test]
fn touch_command_card_opens_orders_and_all_controls_work_at_minimum_phone_size() {
    use yydrone_core::command::*;
    fn tap(game:&mut Game,gfx:&mut NullGfx,label:&str) {
        game.frame(gfx,&[],0.0);
        let (_,x,y,w,h)=gfx.texts.iter().find(|(s,_,_,_,_)|s==label).unwrap_or_else(||panic!("missing phone button {label}")).clone();
        let (x,y)=(x+w/2.0,y+h/2.0);
        game.frame(gfx,&[Ev::Touch(true),Ev::MouseMove(x,y),Ev::Down(0,x,y),Ev::Up(0,x,y)],0.0);
        game.frame(gfx,&[],0.0);
    }
    for dimensions in [(900.0,420.0),(1000.0,480.0)] {
        for side in 0..2 {
            let (mut game,mut gfx)=game();game.compact=true;game.touch=true;gfx.dimensions=Some(dimensions);game.viewer=Some(side);
            game.world.as_mut().unwrap().fund_general(side,0,100.0).unwrap();
            game.command_detail_tab=2;
            tap(&mut game,&mut gfx,&format!("1 {}",GENERAL_NAMES[side as usize][0]));
            assert!(game.show_command);assert_eq!(game.command_detail_tab,0,"phone card must open orders directly");
            for (label,doctrine) in [("全力防御",Doctrine::Defend),("積極攻勢",Doctrine::Assault),("臨機応変",Doctrine::Flexible)] {
                tap(&mut game,&mut gfx,label);
                assert_eq!(game.world.as_ref().unwrap().sides[side as usize].command.generals[0].doctrine,doctrine);
                assert!(gfx.texts.iter().any(|(s,_,_,_,_)|s.contains(&format!("指示：{label}"))));
            }
            tap(&mut game,&mut gfx,"予算 +100");
            assert_eq!(game.world.as_ref().unwrap().sides[side as usize].command.generals[0].budget,200.0);
            tap(&mut game,&mut gfx,"地域と部隊");
            for (_,x,y,w,h) in &gfx.texts {assert!(*x>=0.0&&*y>=0.0&&*x+*w<=dimensions.0+1.0&&*y+*h<=dimensions.1,"phone control outside screen");}
            tap(&mut game,&mut gfx,"地図で中心を指定");assert!(!game.show_command);
            let (x,y)=(dimensions.0*0.45,dimensions.1*0.6);let expected=game.screen_to_world(&gfx,x,y);
            game.frame(&mut gfx,&[Ev::Down(0,x,y),Ev::Up(0,x,y)],0.0);
            assert!(game.show_command);
            let r=game.world.as_ref().unwrap().sides[side as usize].command.generals[0].region;
            assert!((r.x-expected.0).abs()<0.01&&(r.y-expected.1).abs()<0.01);
            let id=game.world.as_ref().unwrap().units.iter().find(|u|u.side==side&&u.kind==UK::Infantry).unwrap().id;
            game.selected=vec![id];tap(&mut game,&mut gfx,"選択 1 部隊を委任");
            assert_eq!(game.world.as_ref().unwrap().general_for(side,id),Some(0));
        }
    }
}

#[test]
fn proposals_and_all_nine_region_labels_fit_compact_and_use_the_viewed_side() {
    use yydrone_core::command::*;
    let (mut game,mut gfx)=game();game.compact=true;gfx.dimensions=Some((1000.0,480.0));
    for side in 0..2 {
        for gi in 0..GENERAL_COUNT {
            let p=game.world.as_ref().unwrap().general_proposal(side as u8,gi);
            let c=&mut game.world.as_mut().unwrap().sides[side].command;
            c.proposals[gi]=p;c.generals[gi].region_assigned=true;
            c.generals[gi].region=Region{x:205.0,y:100.0,radius:42.0};
        }
    }
    game.cam=(205.0,100.0);game.zoom=5.0;
    for side in 0..2 {
        game.viewer=Some(side);
        for gi in 0..GENERAL_COUNT {
            game.command_tab=0;game.command_detail_tab=2;game.command_general=gi;game.show_command=true;
            let mut text=crate::ui::TextQ::default();game.hit_rects.clear();
            game.draw_command(&mut gfx,&mut text,1000.0,480.0);
            for (s,x,y,w,h,_,_,_) in text.0 {assert!(x>=0.0&&y>=0.0&&x+w<=1001.0&&y+h<=480.0,"proposal text outside viewport: {s}");}
        }
        game.show_command=false;game.bg_prev.clear();game.bg_rects.clear();game.hit_rects.clear();
        let portraits_before=gfx.portrait_cards;
        let mut text=crate::ui::TextQ::default();game.draw_command_map(&mut gfx,&mut text);
        for name in GENERAL_NAMES[side as usize] {assert!(text.0.iter().any(|(s,_,_,_,_,_,_,_)|s.contains(name)),"missing {name}");}
        for name in GENERAL_NAMES[1-side as usize] {assert!(!text.0.iter().any(|(s,_,_,_,_,_,_,_)|s.split_whitespace().nth(1)==Some(name)),"enemy commander leaked {name}");}
        assert!(gfx.portrait_cards>=portraits_before+GENERAL_COUNT,"map cards have no portrait images");
        for (s,x,y,w,h,_,_,_) in text.0 {assert!(x>=0.0&&y>=0.0&&x+w<=1001.0&&y+h<=480.0,"map text outside viewport: {s}");}
    }
    assert_eq!(crate::command_map::GENERAL_COLORS.len(),GENERAL_COUNT);
    game.show_regions=false;
    let mut text=crate::ui::TextQ::default();game.draw_command_map(&mut gfx,&mut text);
    assert!(text.0.iter().all(|(s,_,_,_,_,_,_,_)|!s.contains("部隊")));
}
