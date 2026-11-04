use crate::command::*;
use crate::defs::*;
use crate::world::*;

fn world() -> World {
    let mut w = World::new(Settings {
        seed: 17,
        duration: 7200.0,
        ai_mult: 1.0,
        player: Some(0),
    });
    for s in &mut w.sides {
        s.command=Default::default();
        s.ai = false;
        s.auto_buy = false;
        for g in &mut s.command.generals {
            g.buy_t = 1000.0;
        }
    }
    w.units.clear();
    w
}

#[test]
fn legacy_v3_fixture_loads_without_losing_the_old_armies() {
    let w = World::load_bytes(include_bytes!("../tests/fixtures/legacy_v3.bin")).unwrap();
    assert!(!w.units.is_empty());
    assert_eq!(w.player, Some(0));
    assert!(w.sides.iter().all(|s| s
        .command
        .generals
        .iter()
        .all(|g| g.budget == 0.0 && g.units.is_empty())));
    assert!(World::load_bytes(&w.save_bytes()).is_ok());
}

#[test]
fn new_campaign_deploys_all_generals_with_existing_units_and_keeps_a_reserve() {
    for player in [None,Some(0),Some(1)] {
        for seed in [5,17,42] {
            let w=World::new(Settings{seed,duration:7200.0,ai_mult:1.0,player});
            let mut owned=std::collections::HashSet::new();
            for side in 0..2u8 {
                let c=&w.sides[side as usize].command;
                for general in &c.generals {
                    assert!(general.region_assigned);
                    assert!(!general.units.is_empty(),"empty general on side {side}, seed {seed}");
                    assert!(general.units.len()<=4);
                    assert_eq!(general.budget,0.0);
                    assert_eq!(general.spent,0.0);
                    for id in &general.units {
                        let u=w.unit(*id).unwrap();
                        assert_eq!(u.side,side);assert!(u.auto);
                        assert!(general.region.contains(u.x,u.y),"initial unit starts outside its theater");
                        assert!(owned.insert(*id),"unit assigned to two generals");
                    }
                }
                assert!(w.units.iter().any(|u|u.side==side&&matches!(def(u.kind).pass,crate::geo::Pass::Inf|crate::geo::Pass::Veh)&&w.general_for(side,u.id).is_none()),"no ground reserve");
                assert!(w.units.iter().filter(|u|u.side==side&&matches!(u.kind,UK::Jet|UK::Heli|UK::Frigate)).all(|u|w.general_for(side,u.id).is_none()));
            }
            let copy=World::load_bytes(&w.save_bytes()).unwrap();
            assert_eq!(copy.units.len(),w.units.len());
            for side in 0..2 {
                assert_eq!(copy.sides[side].funds,w.sides[side].funds);
                for gi in 0..GENERAL_COUNT {
                    assert_eq!(copy.sides[side].command.generals[gi].units,w.sides[side].command.generals[gi].units);
                    assert_eq!(copy.sides[side].command.generals[gi].region,w.sides[side].command.generals[gi].region);
                }
            }
        }
    }
}

#[test]
fn loading_or_training_does_not_reassign_initial_generals() {
    let mut w=World::new(Settings{seed:17,duration:7200.0,ai_mult:1.0,player:Some(0)});
    let id=w.sides[0].command.generals[0].units[0];
    w.command(&[id],Cmd::Auto(false),0.0,0.0);
    w.sides[0].command.generals[0].region=Region{x:120.0,y:80.0,radius:28.0};
    let copy=World::load_bytes(&w.save_bytes()).unwrap();
    assert!(copy.general_for(0,id).is_none());
    assert_eq!(copy.sides[0].command.generals[0].region,w.sides[0].command.generals[0].region);
    for lesson in [TrainingLesson::Welcome,TrainingLesson::Select,TrainingLesson::Delegate] {
        let (training,_)=World::training(0,lesson);
        assert!(training.sides.iter().flat_map(|s|s.command.generals.iter()).all(|g|!g.region_assigned&&g.units.is_empty()));
    }
}

#[test]
fn legacy_v4_fixture_keeps_delegation_and_starts_proposals_later() {
    let w=World::load_bytes(include_bytes!("../tests/fixtures/legacy_v4.bin")).unwrap();
    assert_eq!(w.sides[0].command.generals[0].budget,100.0);
    assert_eq!(w.sides[0].command.generals[0].units.len(),1);
    assert!(w.sides[0].command.proposals.iter().all(Option::is_none));
    assert!(w.sides[0].command.proposal_t>0.0);
    assert!(World::load_bytes(&w.save_bytes()).is_ok());
}

#[test]
fn generals_proactively_request_budget_and_theaters_without_taking_resources() {
    let mut w=world();let funds=w.sides[0].funds;
    for _ in 0..5 {w.command_tick(0);}
    assert_eq!(w.sides[0].funds,funds);
    assert!(w.sides[0].command.generals.iter().all(|g|g.budget==0.0&&!g.region_assigned));
    let proposals=&w.sides[0].command.proposals;
    assert!(proposals.iter().all(Option::is_some));
    assert!(proposals.iter().flatten().all(|p|p.budget>0.0&&p.region.is_some()));
    assert!(proposals[3].as_ref().unwrap().budget>proposals[1].as_ref().unwrap().budget);
    assert_ne!(proposals[0].as_ref().unwrap().reason,proposals[5].as_ref().unwrap().reason);
    assert!(proposals.iter().flatten().map(|p|p.city).any(|city|city!=proposals[0].as_ref().unwrap().city));
    assert_eq!(w.notices.iter().filter(|n|n.text.contains("人から提案")).count(),1);
    w.notices.clear();w.sides[0].command.proposal_t=0.0;w.command_tick(0);
    assert!(w.notices.iter().all(|n|!n.text.contains("人から提案")),"unchanged requests should not repeat notifications");
}

#[test]
fn proposals_use_visible_enemies_only_and_cover_both_factions() {
    for side in 0..2 {
        let mut w=world();w.vis[side as usize].fill(0);
        let before:Vec<_>=(0..GENERAL_COUNT).map(|gi|w.general_proposal(side,gi)).collect();
        for _ in 0..20 {w.spawn(UK::Tank,1-side,205.0,100.0);}
        let hidden:Vec<_>=(0..GENERAL_COUNT).map(|gi|w.general_proposal(side,gi)).collect();
        assert_eq!(before,hidden,"hidden enemies changed side {side} requests");
        w.vis[side as usize].fill(255);
        let visible:Vec<_>=(0..GENERAL_COUNT).map(|gi|w.general_proposal(side,gi)).collect();
        assert_ne!(hidden,visible);
    }
}

#[test]
fn proposal_approval_is_atomic_partial_and_accounts_for_existing_funding() {
    let mut w=world();w.sides[0].command.proposal_t=0.0;w.command_tick(0);
    let p=w.sides[0].command.proposals[3].clone().unwrap();let original=w.sides[0].command.generals[3].region;
    w.sides[0].funds=0.0;
    assert!(w.accept_general_proposal(0,3,ProposalPart::Both).is_err());
    assert_eq!(w.sides[0].command.generals[3].region,original);
    assert_eq!(w.sides[0].command.proposals[3],Some(p.clone()));
    w.accept_general_proposal(0,3,ProposalPart::Theater).unwrap();
    assert_eq!(w.sides[0].command.generals[3].region,p.region.unwrap());
    assert!(w.sides[0].command.generals[3].region_assigned);
    assert!(w.sides[0].command.proposals[3].as_ref().unwrap().region.is_none());
    w.sides[0].funds=10000.0;w.fund_general(0,3,100.0).unwrap();
    assert_eq!(w.proposal_budget_needed(0,3),p.budget-100.0);
    let funds=w.sides[0].funds;
    let paid=w.accept_general_proposal(0,3,ProposalPart::Budget).unwrap();
    assert_eq!(paid,p.budget-100.0);assert_eq!(w.sides[0].funds,funds-paid);
    assert!(w.sides[0].command.proposals[3].is_none());
    assert!(w.accept_general_proposal(0,3,ProposalPart::Both).is_err());
    let needed=w.general_proposal(0,3).map_or(0.0,|p|p.budget);
    assert_eq!(needed,0.0);
    w.queue_command_unit(0,3,UK::Tank).unwrap();
    assert_eq!(w.general_proposal(0,3).map_or(0.0,|p|p.budget),0.0,"queued reinforcements were charged twice");
}

#[test]
fn proposal_state_and_dismissal_cooldown_survive_saving() {
    let mut w=world();w.sides[0].command.proposal_t=0.0;w.command_tick(0);
    w.accept_general_proposal(0,0,ProposalPart::Theater).unwrap();
    w.dismiss_general_proposal(0,1);
    let copy=World::load_bytes(&w.save_bytes()).unwrap();
    assert_eq!(copy.sides[0].command.proposals,w.sides[0].command.proposals);
    assert_eq!(copy.sides[0].command.proposal_after,w.sides[0].command.proposal_after);
    assert!(copy.sides[0].command.generals[0].region_assigned);
    w.sides[0].command.proposal_t=0.0;w.command_tick(0);
    assert!(w.sides[0].command.proposals[1].is_none());
    w.t=91.0;w.sides[0].command.proposal_t=0.0;w.command_tick(0);
    assert!(w.sides[0].command.proposals[1].is_some());
}

#[test]
fn all_nine_generals_and_direct_recruits_survive_save_load() {
    let mut w = world();
    for index in 0..GENERAL_COUNT {
        let id = w.spawn(UK::Infantry, 0, 120.0 + index as f32, 80.0);
        w.assign_general(0, index, &[id]).unwrap();
        w.fund_general(0, index, 100.0).unwrap();
        assert!(w.sides[0].command.generals[index]
            .threshold(index)
            .is_finite());
    }
    w.queue_command_unit(0, (GENERAL_COUNT - 1) as u8, UK::Infantry)
        .unwrap();
    w.queue_command_unit(0, DIRECT_COMMAND, UK::Engineer)
        .unwrap();
    let copy = World::load_bytes(&w.save_bytes()).unwrap();
    assert_eq!(copy.sides[0].command.generals.len(), 9);
    assert!(copy.sides[0]
        .command
        .generals
        .iter()
        .all(|g| g.units.len() == 1));
    assert_eq!(copy.sides[0].command.pending.len(), 2);
    assert!(copy.sides[0]
        .command
        .pending
        .iter()
        .any(|p| p.general == DIRECT_COMMAND));
}

#[test]
fn delegated_campaign_keeps_accounts_and_assignments_consistent() {
    let mut w = World::new(Settings {
        seed: 37,
        duration: 7200.0,
        ai_mult: 1.0,
        player: Some(0),
    });
    for index in 0..GENERAL_COUNT {
        let ids: Vec<_> = w
            .units
            .iter()
            .filter(|u| {
                u.side == 0
                    && w.general_for(0, u.id).is_none()
                    && w.sides[0].command.generals[index].region.contains(u.x, u.y)
            })
            .take(8)
            .map(|u| u.id)
            .collect();
        w.assign_general(0, index, &ids).unwrap();
        w.fund_general(0, index, 100.0).unwrap();
        w.sides[0].command.generals[index].doctrine =
            [Doctrine::Assault, Doctrine::Defend, Doctrine::Flexible][index % 3];
    }
    for _ in 0..4000 {
        w.tick();
        w.fx.clear();
        w.notices.clear();
        w.dirty.clear();
        w.scars.clear();
    }
    for general in &w.sides[0].command.generals {
        assert!(general.budget >= 0.0);
        assert!((general.budget + general.spent - 100.0).abs() < 0.01);
        assert!(general
            .units
            .iter()
            .all(|id| w.unit(*id).is_some_and(|u| u.side == 0 && u.auto)));
    }
    assert!(!w.sides[0].command.reports.is_empty());
    assert!(World::load_bytes(&w.save_bytes()).is_ok());
}

#[test]
fn general_budget_is_escrowed_and_queue_cancellation_refunds_the_owner() {
    let mut w = world();
    let start = w.sides[0].funds;
    assert!(w.fund_general(0, 0, start + 1.0).is_err());
    w.fund_general(0, 0, 500.0).unwrap();
    assert_eq!(w.sides[0].funds, start - 500.0);
    w.queue_command_unit(0, 0, UK::Infantry).unwrap();
    w.queue_unit(0, UK::Tank).unwrap();
    w.queue_command_unit(0, 0, UK::Infantry).unwrap();
    let cost = unit_cost(0, UK::Infantry);
    assert_eq!(w.sides[0].command.generals[0].budget, 500.0 - 2.0 * cost);
    assert!(w.cancel_unit(0, UK::Infantry));
    assert_eq!(w.sides[0].command.generals[0].budget, 500.0 - cost);
    assert!(w.cancel_unit(0, UK::Tank));
    assert_eq!(w.sides[0].funds, start - 500.0);
    assert_eq!(w.sides[0].command.pending.len(), 1);
    w.fund_general(0, 0, -10000.0).unwrap();
    assert_eq!(w.sides[0].command.generals[0].budget, 0.0);
    assert_eq!(w.sides[0].funds, start - cost);
}

#[test]
fn assignments_are_exclusive_and_direct_orders_revoke_delegation() {
    let mut w = world();
    let own = w.spawn(UK::Infantry, 0, 120.0, 80.0);
    let enemy = w.spawn(UK::Tank, 1, 130.0, 80.0);
    assert_eq!(w.assign_general(0, 0, &[own, own, enemy]).unwrap(), 1);
    assert_eq!(w.sides[0].command.generals[0].units, vec![own]);
    w.assign_general(0, 1, &[own]).unwrap();
    assert!(w.sides[0].command.generals[0].units.is_empty());
    w.command(&[own], Cmd::Auto(false), 0.0, 0.0);
    w.command(&[own], Cmd::Move, 118.0, 80.0);
    w.command_tick(0);
    assert!(w.general_for(0, own).is_none());
    assert_eq!(w.unit(own).unwrap().order, Order::Move(118.0, 80.0));
}

#[test]
fn traits_and_defense_change_actual_orders_and_regions_bound_objectives() {
    let mut w = world();
    let a = w.spawn(UK::Infantry, 0, 120.0, 80.0);
    let b = w.spawn(UK::Infantry, 1, 126.0, 80.0);
    w.vis[0].fill(255);
    w.units.iter_mut().for_each(|u| u.supplied = true);
    for g in &mut w.sides[0].command.generals {
        g.region = Region {
            x: 120.0,
            y: 80.0,
            radius: 28.0,
        };
    }
    w.owner[crate::geo::tidx(126, 80)] = 1;
    w.assign_general(0, 0, &[a]).unwrap();
    w.command_tick(0);
    assert_eq!(w.unit(a).unwrap().order, Order::Attack(Target::Unit(b)));
    w.assign_general(0, 1, &[a]).unwrap();
    w.command_tick(0);
    assert!(!matches!(w.unit(a).unwrap().order, Order::Attack(_)));
    w.assign_general(0, 0, &[a]).unwrap();
    w.sides[0].command.generals[0].doctrine = Doctrine::Defend;
    w.command_tick(0);
    assert!(!matches!(w.unit(a).unwrap().order, Order::Attack(_)));
    w.sides[0].command.generals[0].region = Region {
        x: 100.0,
        y: 80.0,
        radius: 10.0,
    };
    w.units.iter_mut().find(|u| u.id == a).unwrap().ai_t = 0.0;
    w.command_tick(0);
    let goal = w.unit(a).unwrap().goal.unwrap();
    assert!(w.sides[0].command.generals[0]
        .region
        .contains(goal.0, goal.1));
}

#[test]
fn reinforcements_keep_general_or_direct_command_ownership() {
    let mut w = world();
    w.fund_general(0, 0, 500.0).unwrap();
    w.queue_command_unit(0, 0, UK::Infantry).unwrap();
    w.queue_command_unit(0, DIRECT_COMMAND, UK::Engineer)
        .unwrap();
    for _ in 0..320 {
        w.tick();
    }
    let a = w
        .units
        .iter()
        .find(|u| u.kind == UK::Infantry && u.side == 0)
        .unwrap();
    assert_eq!(w.general_for(0, a.id), Some(0));
    let b = w
        .units
        .iter()
        .find(|u| u.kind == UK::Engineer && u.side == 0)
        .unwrap();
    assert!(!b.auto);
    assert!(w.general_for(0, b.id).is_none());
}

#[test]
fn staff_does_not_reveal_hidden_enemies_and_reports_are_bounded() {
    let mut w = world();
    w.spawn(UK::Infantry, 0, 120.0, 80.0);
    w.spawn(UK::Jet, 1, 126.0, 80.0);
    w.vis[0].fill(0);
    assert!(w.front_summary(0, 1).contains("確認敵 0"));
    assert!(!w
        .staff_plan(0)
        .advice
        .iter()
        .any(|s| s.contains("敵航空戦力を確認")));
    w.vis[0].fill(255);
    assert!(w
        .staff_plan(0)
        .advice
        .iter()
        .any(|s| s.contains("敵航空戦力を確認")));
    for _ in 0..9 {
        w.sides[0].command.report_t = 0.0;
        w.command_tick(0);
    }
    assert_eq!(w.sides[0].command.reports.len(), 18);
    assert_eq!(w.sides[0].command.reports[0].sector, 0);
}

#[test]
fn command_state_round_trips_and_destroyed_facility_clears_pending_orders() {
    let mut w = world();
    let id = w.spawn(UK::Infantry, 0, 120.0, 80.0);
    w.assign_general(0, 2, &[id]).unwrap();
    w.fund_general(0, 2, 500.0).unwrap();
    w.queue_command_unit(0, 2, UK::Infantry).unwrap();
    w.sides[0].command.generals[2].doctrine = Doctrine::Assault;
    w.sides[0].command.report_t = 0.0;
    w.command_tick(0);
    let loaded = World::load_bytes(&w.save_bytes()).unwrap();
    assert_eq!(loaded.general_for(0, id), Some(2));
    assert_eq!(
        loaded.sides[0].command.generals[2].doctrine,
        Doctrine::Assault
    );
    assert_eq!(
        loaded.sides[0].command.generals[2].budget,
        w.sides[0].command.generals[2].budget
    );
    assert_eq!(loaded.sides[0].command.pending.len(), 1);
    assert_eq!(
        loaded.sides[0].command.reports[0].text,
        w.sides[0].command.reports[0].text
    );
    let bid = w.sides[0].command.pending[0].building;
    w.blds.iter_mut().find(|b| b.id == bid).unwrap().dead = true;
    w.tick();
    assert!(w.sides[0].command.pending.is_empty());
    assert!(World::load_bytes(&w.save_bytes()).is_ok());
}
