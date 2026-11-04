use crate::defs::*;
use crate::geo::*;
use crate::history::*;
use crate::world::*;

fn sim(seed: u64, duration: f32) -> World {
    World::new(Settings { duration, seed, ai_mult: 1.0, player: None })
}

#[test]
fn map_is_balanced_at_start() {
    let w = sim(1, 600.0);
    let s = w.share(0);
    println!("開戦時 ウクラヴィア {:.1}% 総タイル {}", s * 100.0, w.total_tiles);
    assert!((0.40..0.60).contains(&s), "開戦時の領土が偏っている: {s}");
}

#[test]
fn every_unit_starts_on_passable_ground() {
    let w = sim(2, 600.0);
    for u in &w.units {
        let p = def(u.kind).pass;
        assert!(w.map.cost(tidx(u.x as i32, u.y as i32), p).is_some(), "{:?} が通れない場所にいる", u.kind);
    }
}

#[test]
fn every_buyable_kind_has_a_name_for_its_sides() {
    for k in BUYABLE {
        let d = def(k);
        assert_eq!(d.kind, k);
        for s in 0..2 {
            assert_eq!(d.sides[s], !d.names[s].is_empty(), "{k:?}");
        }
    }
}

#[test]
fn ground_units_reach_goal_across_bridges() {
    let mut w = sim(3, 3000.0);
    w.sides[0].ai = false;
    w.sides[1].ai = false;
    // キエヴァ西岸からハルキアへ：川を橋で渡れる
    let id = w.spawn(UK::Tank, 0, 200.0, 40.0);
    w.units.iter_mut().for_each(|u| u.auto = false);
    let start = w.unit(id).map(|u| (u.x, u.y)).unwrap();
    let (kx, ky) = {
        let c = w.map.cities.iter().find(|c| c.name == "キエヴァ").unwrap();
        (c.x, c.y)
    };
    w.units.iter_mut().find(|u| u.id == id).unwrap().x = kx - 8.0;
    w.units.iter_mut().find(|u| u.id == id).unwrap().y = ky;
    let _ = start;
    let (hx, hy) = {
        let c = w.map.cities.iter().find(|c| c.name == "ポルタ").unwrap();
        (c.x, c.y)
    };
    w.command_one(id, Cmd::Move, hx, hy);
    for _ in 0..20 * 400 {
        w.tick();
        let u = w.unit(id).unwrap();
        if (u.x - hx).abs() + (u.y - hy).abs() < 3.0 {
            return;
        }
    }
    let u = w.unit(id).unwrap();
    panic!("着かなかった: ({}, {}) → ({hx}, {hy})", u.x, u.y);
}

#[test]
fn deterministic_with_same_seed() {
    let (mut a, mut b) = (sim(7, 200.0), sim(7, 200.0));
    for _ in 0..20 * 60 {
        a.tick();
        b.tick();
    }
    assert_eq!(a.sides[0].tiles, b.sides[0].tiles);
    assert_eq!(a.units.len(), b.units.len());
    assert_eq!(a.units.iter().map(|u| u.x.to_bits()).collect::<Vec<_>>(), b.units.iter().map(|u| u.x.to_bits()).collect::<Vec<_>>());
}

/// AI 同士で最後まで戦わせて、勝敗がつき、数値が暴れないことを確かめる
#[test]
fn long_sim_finishes_with_winner() {
    let dur: f32 = std::env::var("SIM_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(WAR_SECONDS);
    let seeds: u64 = std::env::var("SIM_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    for seed in 0..seeds {
        let mut w = sim(seed + 11, dur);
        let t0 = std::time::Instant::now();
        let mut worst = 0.0f64;
        while !w.over {
            let t1 = std::time::Instant::now();
            w.tick();
            worst = worst.max(t1.elapsed().as_secs_f64());
            if w.ticks % (20 * 120) == 0 {
                let cnt = |s: u8| w.units.iter().filter(|u| u.side == s).count();
                println!(
                    "{:5.0}s {} U {:4.1}% R {:4.1}% 部隊 {}/{} 資金 {:.0}/{:.0} 砲弾 {:.0}/{:.0} 撃破 {}/{} 建物 {}",
                    w.t, w.date_text(), w.share(0) * 100.0, w.share(1) * 100.0, cnt(0), cnt(1), w.sides[0].funds, w.sides[1].funds, w.sides[0].shells, w.sides[1].shells, w.sides[0].kills, w.sides[1].kills, w.blds.len()
                );
            }
            w.drain_notices();
            w.fx.clear();
            w.scars.clear();
            w.dirty.clear();
        }
        println!("seed {} 勝者 {:?} U {:.1}% 実時間 {:.1}s 最悪 1tick {:.1}ms", seed + 11, w.winner, w.share(0) * 100.0, t0.elapsed().as_secs_f32(), worst * 1000.0);
        assert!(w.units.iter().all(|u| u.x.is_finite() && u.y.is_finite()));
        assert!(w.sides[0].tiles + w.sides[1].tiles == w.total_tiles);
        if dur >= WAR_SECONDS {
            assert!((0.05..0.95).contains(&w.share(0)), "2時間で片方が消えかけた: {:.1}%", w.share(0) * 100.0);
        }
    }
}

/// プレイヤーが何もしなくても、委任された部隊で序盤の前線が保たれる
#[test]
fn delegated_player_holds_the_opening() {
    for side in 0..2u8 {
        let mut w = World::new(Settings { duration: WAR_SECONDS, seed: 5, ai_mult: 1.0, player: Some(side) });
        let before = w.units.iter().filter(|u| u.side == side).count();
        for _ in 0..20 * 45 {
            w.tick();
            w.drain_notices();
            w.fx.clear();
        }
        let after = w.units.iter().filter(|u| u.side == side).count();
        println!("陣営{side} 部隊 {before}→{after} 領土 {:.1}%", w.share(side) * 100.0);
        assert!(after * 3 >= before, "何も買わなくても序盤で全滅に近づかない");
        assert!(w.share(side) > 0.35);
    }
}

#[test]
#[ignore]
fn breakdown() {
    let mut w = sim(11, WAR_SECONDS);
    let mut spent = [[0.0f32; 2]; 2];
    let mut last_blds: Vec<u32> = Vec::new();
    while w.t < 800.0 {
        w.tick();
        for b in &w.blds {
            if !last_blds.contains(&b.id) {
                last_blds.push(b.id);
                spent[b.side as usize][0] += bdef(b.kind).cost;
            }
        }
        w.drain_notices();
        w.fx.clear();
        if w.ticks % (20 * 200) == 0 {
            for s in 0..2u8 {
                let mut line = format!("{:.0}s side{} inc {:.0} tiles {:.0}% q{}:", w.t, s, w.sides[s as usize].income, w.share(s) * 100.0, w.blds.iter().filter(|b| b.side == s).map(|b| b.queue.len()).sum::<usize>());
                for k in BUYABLE {
                    let c = w.units.iter().filter(|u| u.side == s && u.kind == k).count();
                    if c > 0 {
                        line += &format!(" {}{}", def(k).short, c);
                    }
                }
                line += &format!(" | 建物{}:", spent[s as usize][0]);
                for k in BK_ALL {
                    line += &format!(" {}{}", &bdef(k).name[..3], w.blds.iter().filter(|b| b.side == s && b.kind == k).count());
                }
                println!("{line}");
            }
        }
    }
}

fn quiet(seed: u64) -> World {
    // AI を止めて、指定した部隊だけを動かす
    let mut w = sim(seed, 5000.0);
    w.sides[0].ai = false;
    w.sides[1].ai = false;
    for s in &mut w.sides {s.command=Default::default();}
    w.units.clear();
    w
}

fn city(w: &World, name: &str) -> (f32, f32) {
    let c = w.map.cities.iter().find(|c| c.name == name).unwrap();
    (c.x, c.y)
}

#[test]
fn engineer_digs_trench_and_lays_mines() {
    let mut w = quiet(21);
    let (x, y) = city(&w, "ポルタ");
    let id = w.spawn(UK::Engineer, 0, x, y);
    w.units.iter_mut().for_each(|u| u.auto = false);
    let spot = (0..30).map(|d| (x as i32 + 4 + d, y as i32)).find(|p| w.map.terrain[tidx(p.0, p.1)] == Terrain::Plain).unwrap();
    let spot2 = (spot.0 as u16 + 1, spot.1 as u16);
    w.add_jobs(&[id], JobKind::Trench, &[(spot.0 as u16, spot.1 as u16)]);
    w.add_jobs(&[id], JobKind::Mine, &[spot2]);
    for _ in 0..20 * 40 {
        w.tick();
    }
    assert_ne!(w.feat[tidx(spot.0, spot.1)] & crate::path::FEAT_TRENCH, 0, "塹壕ができていない");
    assert_eq!(w.mines[tidx(spot2.0 as i32, spot2.1 as i32)], 1, "地雷が置かれていない");
}

#[test]
fn enemy_vehicle_triggers_mine() {
    let mut w = quiet(22);
    let (x, y) = city(&w, "ポルタ");
    let (mx, my) = (x as i32 + 3, y as i32);
    let id = w.spawn(UK::Tank, 1, mx as f32 - 2.0, my as f32 + 0.5);
    w.units.iter_mut().for_each(|u| u.auto = false);
    for dx in 0..4 {
        for dy in -2..=2 {
            w.mines[tidx(mx + dx, my + dy)] = 1;
        }
    }
    let hp = w.unit(id).unwrap().hp;
    w.command_one(id, Cmd::Move, mx as f32 + 5.0, my as f32 + 0.5);
    for _ in 0..20 * 12 {
        w.tick();
    }
    assert!(w.unit(id).map_or(true, |u| u.hp < hp), "地雷で傷ついていない");
}

#[test]
fn destroying_a_bridge_blocks_vehicles_and_pontoon_reopens() {
    let mut w = quiet(23);
    // 川にかかる橋をひとつ選ぶ
    let bi = w.map.bridges.iter().position(|b| b.under == Terrain::River && b.tiles.len() >= 2).unwrap();
    let (bx, by) = (w.map.bridges[bi].x, w.map.bridges[bi].y);
    let id = w.spawn(UK::Mlrs, 0, bx + 12.0, by);
    w.units.iter_mut().for_each(|u| u.auto = false);
    w.sides[0].shells = 1000.0;
    w.attack_ground(&[id], bx, by);
    for _ in 0..20 * 120 {
        w.tick();
        if w.map.bridges[bi].hp <= 0.0 {
            break;
        }
    }
    assert!(w.map.bridges[bi].hp <= 0.0, "ロケット砲で橋を落とせない");
    let t = w.map.bridges[bi].tiles[0] as usize;
    assert_eq!(w.map.terrain[t], Terrain::River);
    assert!(w.map.cost(t, crate::geo::Pass::Veh).is_none(), "落ちた橋を車両が通れる");

    // 工兵が浮橋を架ければまた通れる
    let eng = w.spawn(UK::Engineer, 0, bx + 3.0, by);
    w.units.iter_mut().for_each(|u| u.auto = false);
    let tile = ((t % W) as u16, (t / W) as u16);
    w.add_jobs(&[eng], JobKind::Pontoon, &[tile]);
    for _ in 0..20 * 60 {
        w.tick();
        if w.map.terrain[t] == Terrain::Bridge {
            return;
        }
    }
    panic!("浮橋が架からない");
}

#[test]
fn jet_sorties_and_returns_home() {
    let mut w = quiet(24);
    let (x, y) = city(&w, "クルスカ");
    let id = w.spawn(UK::Jet, 1, x + 4.0, y + 2.0);
    w.units.iter_mut().for_each(|u| u.auto = false);
    let start = w.unit(id).map(|u| u.home).unwrap();
    w.attack_ground(&[id], x - 40.0, y + 20.0);
    let mut left = false;
    for _ in 0..20 * 60 {
        w.tick();
        let u = w.unit(id).unwrap();
        if u.sortie == 2 {
            left = true;
        }
        if left && u.sortie == 0 {
            assert!((u.x - start.0).abs() < 8.0 && (u.y - start.1).abs() < 8.0, "基地に戻っていない");
            return;
        }
    }
    panic!("戦闘機が帰還しなかった: left={left}");
}

#[test]
fn fpv_team_launches_at_visible_enemy_and_jammer_stops_it() {
    // 妨害なしでは敵に当たる
    let mut w = quiet(25);
    let (x, y) = city(&w, "ポルタ");
    let team = w.spawn(UK::FpvTeam, 0, x, y);
    let tank = w.spawn(UK::Tank, 1, x + 8.0, y);
    w.units.iter_mut().for_each(|u| u.auto = false);
    let hp = w.unit(tank).unwrap().hp;
    for _ in 0..20 * 12 {
        w.tick();
    }
    assert!(w.unit(team).is_some());
    let damaged = w.unit(tank).map_or(0.0, |u| u.hp);
    assert!(damaged < hp, "FPV が当たっていない");

    // 電子戦車の近くでは多くが墜落する
    let run = |with_ew: bool| {
        let mut w = quiet(26);
        let (x, y) = city(&w, "ポルタ");
        w.spawn(UK::FpvTeam, 0, x, y);
        let tank = w.spawn(UK::Tank, 1, x + 16.0, y);
        if with_ew {
            w.spawn(UK::Ew, 1, x + 14.0, y);
        }
        w.units.iter_mut().for_each(|u| u.auto = false);
        for _ in 0..20 * 30 {
            w.tick();
        }
        w.unit(tank).map_or(0.0, |u| u.hp)
    };
    assert!(run(true) > run(false), "電子戦車の効果がない");
}

#[test]
fn supply_cut_when_depots_and_cities_are_gone() {
    let mut w = quiet(27);
    w.update_supply();
    let (x, y) = city(&w, "ドニプロヴァ");
    assert!(w.supply[0][tidx(x as i32, y as i32)] > 0);
    w.blds.retain(|b| b.side != 0);
    for c in w.map.cities.iter_mut() {
        if c.owner == 0 {
            c.owner = 1;
        }
    }
    w.update_supply();
    assert_eq!(w.supply[0][tidx(x as i32, y as i32)], 0);
}

#[test]
fn time_up_decides_by_territory() {
    let mut w = sim(28, 5.0);
    for o in w.owner.iter_mut() {
        if *o == 1 {
            *o = 0;
        }
    }
    w.count_tiles();
    while !w.over {
        w.tick();
    }
    assert_eq!(w.winner, Some(0));
}

#[test]
fn calendar_spans_the_war() {
    assert_eq!(date_to_day(2022, 2, 24), 0);
    assert_eq!(date_to_day(2022, 2, 28), 4);
    assert_eq!(date_to_day(2022, 3, 1), 5);
    assert_eq!(date_to_day(2023, 2, 24), 365);
    assert_eq!(date_to_day(2024, 3, 1), 736);
    for d in [0, 4, 5, 37, 310, 365, 736, 1041, 1461, 1693, 1710] {
        let (y, m, dd) = day_to_date(d);
        assert_eq!(date_to_day(y, m, dd), d, "{y}/{m}/{dd}");
    }
    assert_eq!(day_to_date(frac_to_day(0.0)), (2022, 2, 24));
    assert_eq!(day_to_date(frac_to_day(1.0)), (2026, 10, 31));
    let mut last = 0;
    for i in 0..=1000 {
        let d = frac_to_day(i as f32 / 1000.0);
        assert!(d >= last, "暦が戻っている");
        last = d;
    }
}

#[test]
fn history_is_sorted_and_names_resolve() {
    let w = sim(31, 600.0);
    let mut last = 0;
    let has = |n: &str| w.map.cities.iter().any(|c| c.name == n);
    let check = |p: &Place| {
        if let Place::City(n) = p {
            assert!(has(n), "都市がない: {n}");
        }
    };
    for e in HISTORY {
        let d = date_to_day(e.date.0, e.date.1, e.date.2);
        assert!(d >= last, "年表の順序: {}", e.title);
        last = d;
        if let Some(p) = &e.place {
            check(p);
        }
        if let Some((c, _)) = e.requires {
            assert!(has(c), "{c}");
        }
        for f in e.effects {
            match f {
                Eff::Spawn(_, _, _, p) | Eff::Revert(_, p, _) | Eff::Halt(_, p, _, _) => check(p),
                Eff::Retreat(_, a, _, b) => {
                    check(a);
                    check(b);
                }
                Eff::Mod(_, _, Some((p, _)), _) => check(p),
                Eff::Axes(_, names, _) => names.iter().for_each(|n| assert!(has(n), "{n}")),
                _ => {}
            }
        }
    }
    assert_eq!(HISTORY[0].title, "全面侵攻が始まる");
    assert!(HISTORY.iter().any(|e| e.title == "マスカ氏の接続制限"));
}

#[test]
fn every_event_fires_once_in_a_full_war() {
    let mut w = sim(32, 300.0);
    assert_eq!(w.chronicle[0].title, "全面侵攻が始まる", "開戦と同時に侵攻が始まる");
    while !w.over {
        w.tick();
        w.drain_notices();
        w.fx.clear();
    }
    assert_eq!(w.chronicle.len(), HISTORY.len());
    for c in &w.chronicle {
        println!("{} {} {}", c.date, if c.happened { "○" } else { "×" }, c.title);
    }
}

/// キエヴァは進撃を食い止めて持ちこたえ、解放の日に北部が戻る
#[test]
fn kyiv_holds_until_the_withdrawal() {
    let target = date_to_day(2022, 4, 2);
    let mut held = 0;
    for seed in 0..6 {
        let mut w = sim(40 + seed, WAR_SECONDS);
        let mut ros_near_kyiv = 0;
        while w.day() < target {
            w.tick();
            w.drain_notices();
            w.fx.clear();
        }
        let kyiv = w.map.cities.iter().find(|c| c.name == "キエヴァ").unwrap();
        let (kx, ky) = (kyiv.x, kyiv.y);
        if kyiv.owner == 0 {
            held += 1;
        }
        for u in &w.units {
            if u.side == 1 && def(u.kind).cap_r > 0.0 && (u.x - kx).powi(2) + (u.y - ky).powi(2) < 25.0 {
                ros_near_kyiv += 1;
            }
        }
        println!("seed {seed}: キエヴァ {} 近くの敵 {ros_near_kyiv} U {:.1}%", kyiv.owner, w.share(0) * 100.0);
        assert!(w.chronicle.iter().any(|c| c.title == "キエヴァ攻防戦"));
    }
    assert_eq!(held, 6, "キエヴァが落ちた");
}

#[test]
fn starlink_whitelist_cuts_enemy_drones() {
    // 同じ条件で、通信途絶があると FPV が届きにくい
    let run = |outage: bool| {
        let mut w = quiet(33);
        w.chronicle.clear();
        w.mods.clear();
        let (x, y) = city(&w, "ポルタ");
        w.spawn(UK::FpvTeam, 0, x, y);
        let tank = w.spawn(UK::Tank, 1, x + 16.0, y);
        w.units.iter_mut().for_each(|u| u.auto = false);
        if outage {
            w.mods.push(Mod { side: 0, kind: ModKind::Outage, zone: None, until: 100000 });
        }
        w.duration = 1e7; // 暦を進めない
        for _ in 0..20 * 30 {
            w.tick();
        }
        w.unit(tank).map_or(0.0, |u| u.hp)
    };
    assert!(run(true) > run(false), "通信途絶の効果がない");
}

#[test]
fn kakhovka_flood_blocks_and_recedes() {
    let mut w = sim(34, 600.0);
    let before = w.map.terrain.iter().filter(|t| **t == Terrain::River).count();
    let d = date_to_day(2023, 6, 6);
    w.next_event = HISTORY.iter().position(|e| e.title == "カホヴァ・ダムの決壊").unwrap();
    while w.day() < d {
        w.t += 1.0;
    }
    w.run_history_for_test();
    let during = w.map.terrain.iter().filter(|t| **t == Terrain::River).count();
    assert!(during > before + 50, "水没が足りない: {before}→{during}");
    while w.day() < d + 91 {
        w.t += 1.0;
    }
    w.next_event = HISTORY.len();
    w.run_history_for_test();
    let after = w.map.terrain.iter().filter(|t| **t == Terrain::River).count();
    assert_eq!(after, before, "水が引いていない");
}

#[test]
fn save_and_load_round_trip() {
    let mut a = sim(51, WAR_SECONDS);
    // ダム決壊と橋の破壊のあとまで進める（暦を飛ばして出来事を起こす）
    while a.day() < date_to_day(2023, 6, 20) {
        a.tick();
        a.drain_notices();
        a.fx.clear();
    }
    let data = a.save_bytes();
    println!("記録の大きさ {} KB", data.len() / 1024);
    assert!(data.len() < 1 << 20, "記録が大きすぎる");
    let mut b = World::load_bytes(&data).expect("読み込めない");
    assert_eq!(a.t, b.t);
    assert_eq!(a.sides[0].tiles, b.sides[0].tiles);
    assert_eq!(a.sides[1].funds, b.sides[1].funds);
    assert_eq!(a.units.len(), b.units.len());
    assert_eq!(a.blds.len(), b.blds.len());
    assert_eq!(a.chronicle.len(), b.chronicle.len());
    assert_eq!(a.chronicle.last().map(|c| c.title), b.chronicle.last().map(|c| c.title));
    assert_eq!(a.mods.len(), b.mods.len());
    assert_eq!(a.flooded.len(), b.flooded.len());
    assert!(!b.flooded.is_empty(), "決壊の最中の記録のはず");
    assert!(a.owner == b.owner && a.feat == b.feat && a.mines == b.mines);
    assert!(a.map.terrain == b.map.terrain);
    for (x, y) in a.units.iter().zip(&b.units) {
        assert_eq!((x.id, x.kind, x.side, x.x.to_bits(), x.hp.to_bits()), (y.id, y.kind, y.side, y.x.to_bits(), y.hp.to_bits()));
        assert_eq!(b.unit_index(y.id).map(|i| b.units[i].id), Some(y.id));
    }
    // 読み込んだ戦争は最後まで走り、水も引く
    while !b.over {
        b.tick();
        b.drain_notices();
        b.fx.clear();
    }
    assert!(b.winner.is_some());
    assert_eq!(b.chronicle.len(), HISTORY.len());
    assert!(b.flooded.is_empty());
    // 再保存も同じように読める
    assert!(World::load_bytes(&b.save_bytes()).is_ok());
}

#[test]
fn pontoon_survives_save() {
    let mut w = quiet(52);
    let bi = w.map.bridges.len();
    let t = (0..W * H).find(|&i| w.map.terrain[i] == Terrain::River).unwrap();
    let (x, y) = ((t % W) as f32, (t / W) as f32);
    let id = w.spawn(UK::Engineer, 0, x + 1.0, y);
    w.add_jobs(&[id], JobKind::Pontoon, &[((t % W) as u16, (t / W) as u16)]);
    for _ in 0..20 * 30 {
        w.tick();
    }
    assert_eq!(w.map.bridges.len(), bi + 1, "浮橋が架かっていない");
    let b = World::load_bytes(&w.save_bytes()).unwrap();
    assert_eq!(b.map.bridges.len(), bi + 1);
    assert_eq!(b.map.terrain[t], Terrain::Bridge);
    assert_eq!(b.map.bridge_at[t] as usize, bi);
}

#[test]
fn broken_saves_are_rejected_not_crashes() {
    let data = sim(53, 600.0).save_bytes();
    assert!(World::load_bytes(b"hello").is_err());
    let mut wrong = data.clone();
    wrong[4] = 99;
    assert!(World::load_bytes(&wrong).err().unwrap().contains("版"));
    for cut in [5, 30, 1000, data.len() / 2, data.len() - 1] {
        assert!(World::load_bytes(&data[..cut]).is_err(), "途中で切れた記録 {cut}");
    }
    // 数値が壊れていても落ちない（結果は問わない）
    for i in (9..data.len()).step_by(data.len() / 200 + 1) {
        let mut d = data.clone();
        d[i] ^= 0xA5;
        let _ = World::load_bytes(&d);
    }
}

fn at_date(w: &mut World, y: u32, m: u32, d: u32) {
    let target = date_to_day(y, m, d);
    while w.day() < target {
        w.t += 1.0;
    }
    w.mud_until = 0;
    w.update_season();
}

/// 平地の上の部隊の速さを測る
fn plain_speed(w: &mut World, k: UK) -> f32 {
    let p = (0..W * H).find(|&i| w.map.terrain[i] == Terrain::Plain && !w.map.road[i]).unwrap();
    let id = w.spawn(k, 0, (p % W) as f32 + 0.5, (p / W) as f32 + 0.5);
    let u = w.unit(id).unwrap();
    let s = w.speed_of(u);
    let i = w.unit_index(id).unwrap();
    w.units[i].dead = true;
    s
}

#[test]
fn ground_follows_the_calendar() {
    assert_eq!(World::ground_of_date(1, 10), Ground::Frozen);
    assert_eq!(World::ground_of_date(3, 20), Ground::Mud);
    assert_eq!(World::ground_of_date(7, 1), Ground::Normal);
    assert_eq!(World::ground_of_date(10, 20), Ground::Mud);
    assert_eq!(World::ground_of_date(12, 1), Ground::Normal);
    assert_eq!(World::ground_of_date(12, 20), Ground::Frozen);
}

#[test]
fn mud_hurts_wheels_most_and_frost_helps() {
    let mut w = quiet(61);
    at_date(&mut w, 2023, 7, 1);
    let (tank, truck) = (plain_speed(&mut w, UK::Tank) / def(UK::Tank).speed, plain_speed(&mut w, UK::Mlrs) / def(UK::Mlrs).speed);
    assert_eq!(tank, truck);
    let dry = tank;
    at_date(&mut w, 2023, 10, 20);
    let (tank, truck) = (plain_speed(&mut w, UK::Tank) / def(UK::Tank).speed, plain_speed(&mut w, UK::Mlrs) / def(UK::Mlrs).speed);
    assert!(truck < tank && tank < dry, "泥濘で装輪が最も遅いはず: 乾 {dry} 装軌 {tank} 装輪 {truck}");
    at_date(&mut w, 2024, 1, 10);
    assert!(plain_speed(&mut w, UK::Tank) / def(UK::Tank).speed > dry, "凍結で速くなるはず");
}

#[test]
fn mud_keeps_vehicles_on_roads_when_pathing() {
    let mut w = quiet(62);
    let i = (0..W * H).find(|&i| w.map.terrain[i] == Terrain::Plain && !w.map.road[i]).unwrap();
    let r = (0..W * H).find(|&i| w.map.terrain[i] == Terrain::Plain && w.map.road[i] && !w.map.rail[i]).unwrap();
    at_date(&mut w, 2023, 7, 1);
    let dry = w.map.cost(i, crate::geo::Pass::Veh).unwrap();
    at_date(&mut w, 2023, 10, 20);
    let wet = w.map.cost(i, crate::geo::Pass::Veh).unwrap();
    assert!(wet > dry * 2.0);
    assert_eq!(w.map.cost(r, crate::geo::Pass::Veh), Some(0.6), "道路は泥濘でも変わらない");
}

#[test]
fn map_has_marsh_treelines_and_hills() {
    let w = quiet(63);
    let count = |t: Terrain| w.map.terrain.iter().filter(|x| **x == t).count();
    println!("湿地 {} 樹林帯 {} 高地 {} 森 {} 平地 {}", count(Terrain::Marsh), count(Terrain::Treeline), count(Terrain::Hill), count(Terrain::Forest), count(Terrain::Plain));
    assert!(count(Terrain::Marsh) > 100);
    assert!(count(Terrain::Treeline) > 500);
    assert!(count(Terrain::Hill) > 200);
}

#[test]
fn frozen_marsh_opens_to_vehicles() {
    let mut w = quiet(64);
    let i = (0..W * H).find(|&i| w.map.terrain[i] == Terrain::Marsh && !w.map.road[i]).unwrap();
    at_date(&mut w, 2023, 7, 1);
    let summer = w.map.cost(i, crate::geo::Pass::Veh).unwrap();
    at_date(&mut w, 2024, 1, 10);
    let frozen = w.map.cost(i, crate::geo::Pass::Veh).unwrap();
    assert!(frozen < summer / 3.0);
}

#[test]
fn foliage_hides_infantry_from_drones_only_when_in_leaf() {
    let seen = |y: u32, m: u32| {
        let mut w = quiet(65);
        at_date(&mut w, y, m, 10);
        w.weather = Weather::Clear;
        w.weather_t = 1e6;
        let t = (0..W * H).find(|&i| w.map.terrain[i] == Terrain::Treeline && w.map.start_owner[i] == 1).unwrap();
        let (x, y) = ((t % W) as f32 + 0.5, (t / W) as f32 + 0.5);
        let inf = w.spawn(UK::Infantry, 1, x, y);
        // 偵察機は少し離れて飛ぶ（真下ではない）
        w.spawn(UK::ReconDrone, 0, x + 10.0, y);
        w.units.iter_mut().for_each(|u| {
            u.auto = false;
            u.still = 5.0;
        });
        w.update_vision();
        let u = w.unit(inf).unwrap();
        w.can_see(0, u)
    };
    assert!(!seen(2023, 7), "夏の樹林帯の歩兵は上空から見つけにくい");
    assert!(seen(2024, 1), "冬の樹林帯は丸見え");
}

#[test]
fn fog_blinds_drones() {
    let reach = |wx: Weather| {
        let mut w = quiet(66);
        w.weather = wx;
        w.weather_t = 1e6;
        let t = (0..W * H).find(|&i| w.map.start_owner[i] == 1 && w.map.terrain[i] == Terrain::Plain).unwrap();
        let (x, y) = ((t % W) as f32 + 0.5, (t / W) as f32 + 0.5);
        let tank = w.spawn(UK::Tank, 1, x, y);
        w.spawn(UK::ReconDrone, 0, x + 9.0, y);
        w.units.iter_mut().for_each(|u| u.auto = false);
        w.update_vision();
        let u = w.unit(tank).unwrap();
        w.can_see(0, u)
    };
    assert!(reach(Weather::Clear));
    assert!(!reach(Weather::Fog));
}

#[test]
fn frozen_ground_slows_digging() {
    let time_to_dig = |y: u32, m: u32| {
        let mut w = quiet(67);
        at_date(&mut w, y, m, 10);
        let t = (0..W * H).find(|&i| w.map.terrain[i] == Terrain::Plain && w.map.start_owner[i] == 0 && !w.map.road[i]).unwrap();
        let id = w.spawn(UK::Infantry, 0, (t % W) as f32 + 0.5, (t / W) as f32 + 0.5);
        w.units.iter_mut().for_each(|u| u.auto = false);
        let start = w.t;
        for _ in 0..20 * 400 {
            w.tick();
            if w.unit(id).is_some_and(|u| u.dig >= 1.0) {
                break;
            }
        }
        w.t - start
    };
    assert!(time_to_dig(2024, 1) > time_to_dig(2023, 7) * 2.0);
}

#[test]
fn hills_see_farther() {
    let sees = |terrain: Terrain| {
        let mut w = quiet(68);
        w.weather = Weather::Clear;
        w.weather_t = 1e6;
        let t = (0..W * H).find(|&i| w.map.terrain[i] == terrain && w.map.start_owner[i] == 1).unwrap();
        let (x, y) = ((t % W) as f32 + 0.5, (t / W) as f32 + 0.5);
        w.spawn(UK::Infantry, 0, x, y);
        w.update_vision();
        let far = (x as i32 + 10).min(W as i32 - 1);
        w.vis[0][tidx(far, y as i32)] != 0
    };
    assert!(sees(Terrain::Hill) && !sees(Terrain::Plain));
}

#[test]
fn snow_only_falls_in_the_cold() {
    let mut w = quiet(69);
    at_date(&mut w, 2023, 7, 1);
    for _ in 0..300 {
        w.weather_t = 0.0;
        w.update_weather(1.0);
        assert_ne!(w.weather, Weather::Snow, "真夏に雪");
    }
    w.notices.clear();
    at_date(&mut w, 2024, 1, 10);
    let mut snow = 0;
    for _ in 0..300 {
        w.weather_t = 0.0;
        w.update_weather(1.0);
        if w.weather == Weather::Snow {
            snow += 1;
        }
    }
    assert!(snow > 30);
}

#[test]
fn weather_and_ground_survive_save() {
    let mut w = sim(70, WAR_SECONDS);
    w.mud_until = 500;
    w.weather = Weather::Fog;
    w.weather_t = 42.0;
    let b = World::load_bytes(&w.save_bytes()).unwrap();
    assert_eq!(b.weather, Weather::Fog);
    assert_eq!(b.weather_t, 42.0);
    assert_eq!(b.mud_until, 500);
    assert!(b.mud());
    assert!(b.notices.is_empty());
}

fn rail_reach(w: &World, from: &str, to: &str) -> bool {
    let c = |n: &str| {
        let c = w.map.cities.iter().find(|c| c.name == n).unwrap();
        (c.x as i32, c.y as i32)
    };
    let (a, b) = (c(from), c(to));
    let near = |p: (i32, i32)| {
        for r in 0..5i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if in_map(p.0 + dx, p.1 + dy) && w.map.rail[tidx(p.0 + dx, p.1 + dy)] {
                        return Some(tidx(p.0 + dx, p.1 + dy));
                    }
                }
            }
        }
        None
    };
    let (Some(s), Some(e)) = (near(a), near(b)) else { return false };
    let mut seen = vec![false; W * H];
    let mut st = vec![s];
    seen[s] = true;
    while let Some(i) = st.pop() {
        if i == e {
            return true;
        }
        let (x, y) = ((i % W) as i32, (i / W) as i32);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if in_map(x + dx, y + dy) {
                let t = tidx(x + dx, y + dy);
                if w.map.rail[t] && !seen[t] {
                    seen[t] = true;
                    st.push(t);
                }
            }
        }
    }
    false
}

#[test]
fn rail_network_is_connected() {
    let w = quiet(81);
    for (a, b) in [("リヴィア", "キエヴァ"), ("キエヴァ", "ハルキア"), ("キエヴァ", "オデスカ"), ("ハルキア", "ロストヴァ"), ("ドニプロヴァ", "シンフェラ"), ("ヴォロニア", "ロストヴァ"), ("ケルチャ", "クラスノーダ"), ("セヴァストラ", "ケルチャ")] {
        assert!(rail_reach(&w, a, b), "{a}〜{b} が鉄道でつながっていない");
    }
}

#[test]
fn rail_carries_supply_and_cutting_starves_the_line() {
    let mut w = quiet(82);
    // ウクラヴィアの建物と、キエヴァ以外の都市を消して、鉄道だけで届くか調べる
    w.blds.clear();
    for c in w.map.cities.iter_mut() {
        if c.owner == 0 && c.name != "キエヴァ" {
            c.owner = 1;
        }
    }
    // ポルタの先の線路（キエヴァから遠い）
    let k = w.map.cities.iter().find(|c| c.name == "キエヴァ").map(|c| (c.x, c.y)).unwrap();
    let line = w.map.rail_lines.iter().find(|l| {
        let a = l[0];
        (a.0 - k.0).abs() < 2.0 && (a.1 - k.1).abs() < 2.0 && l.last().is_some_and(|e| e.0 > k.0 + 20.0)
    }).unwrap().clone();
    let far = line[line.len() - 8];
    let ti = tidx(far.0 as i32, far.1 as i32);
    w.update_supply();
    let dist = ((far.0 - k.0).powi(2) + (far.1 - k.1).powi(2)).sqrt();
    assert!(dist > 26.0, "都市の補給だけでは届かない距離のはず: {dist}");
    assert!(w.supply[0][ti] > 0, "線路で補給が届いていない");
    // 線路網には迂回路があるので、全部切ると届かなくなることで確かめる
    w.cut_rail(W as f32 / 2.0, H as f32 / 2.0, 400.0, 45.0);
    w.update_supply();
    assert_eq!(w.supply[0][ti], 0, "線路を切っても補給が届いている");
    assert!(!w.rail_cuts.is_empty());
    // 時間がたつと復旧する
    w.t += 46.0;
    w.repair_rails_for_test();
    w.update_supply();
    assert!(w.supply[0][ti] > 0, "復旧していない");
}

#[test]
fn own_rail_moves_units_faster() {
    let mut w = quiet(83);
    at_date(&mut w, 2023, 7, 1);
    let t = (0..W * H).find(|&i| w.map.rail[i] && w.map.start_owner[i] == 0 && !w.map.road[i] && w.map.terrain[i] == Terrain::Plain).unwrap();
    let (x, y) = ((t % W) as f32 + 0.5, (t / W) as f32 + 0.5);
    let own = w.spawn(UK::Tank, 0, x, y);
    let enemy = w.spawn(UK::Tank, 1, x, y);
    let road = w.unit(enemy).map(|u| w.speed_of(u)).unwrap();
    let train = w.unit(own).map(|u| w.speed_of(u)).unwrap();
    assert!(train > road, "自国の線路の方が速いはず");
}

#[test]
fn rail_cuts_survive_save() {
    let mut w = quiet(84);
    let t = (0..W * H).find(|&i| w.map.rail[i]).unwrap();
    w.cut_rail((t % W) as f32, (t / W) as f32, 1.0, 45.0);
    assert!(!w.rail_cuts.is_empty());
    let b = World::load_bytes(&w.save_bytes()).unwrap();
    assert_eq!(b.rail_cuts.len(), w.rail_cuts.len());
    assert!(b.map.rail == w.map.rail);
}
