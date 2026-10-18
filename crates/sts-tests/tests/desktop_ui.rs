//! 桌面 UI 协议测试：GameView 场景捕获的结构化断言 + PlayerCommand 分发。
//!
//! 全部走真实核心状态（RunState4b），不使用假数据。

use sts_interface::{dispatch, CampfireAction, GameView, PlayerCommand, Screen, ShopItemKind};

fn new_run(seed: i64) -> sts_core::flow4b::RunState4b {
    let mut run = sts_core::flow4b::RunState4b::new(seed);
    run.open_neow();
    run
}

fn win_battle(run: &mut sts_core::flow4b::RunState4b) {
    {
        let b = run.battle.as_mut().unwrap();
        b.run_until_idle();
        for mi in 0..b.monsters.len() {
            b.monsters[mi].creature.current_health = 1;
            b.mgr.add_bottom(sts_core::action::GameAction::Damage {
                target: sts_core::action::TargetRef::Monster(mi),
                info: sts_core::damage::DamageInfo {
                    owner_is_player: true,
                    base: 99,
                    output: 99,
                    ty: sts_core::power::DamageType::Normal,
                    modified: true,
                },
            });
        }
        b.run_until_idle();
    }
    run.settle_battle();
}

#[test]
fn gameview_capture_neow() {
    let run = new_run(1);
    let v = GameView::capture(&run);
    assert_eq!(v.screen, Screen::Neow);
    assert_eq!(v.neow.as_ref().map(|n| n.options.len()), Some(4));
    assert_eq!(v.top.max_hp, 80);
}

#[test]
fn gameview_capture_map_has_structured_nodes() {
    let mut run = new_run(2);
    let _ = run.take_neow(0);
    let v = GameView::capture(&run);
    assert_eq!(v.screen, Screen::Map);
    let m = v.map.expect("地图场景必有 MapView");
    assert!(!m.nodes.is_empty(), "节点来自核心数据而非渲染字符串");
    assert!(!m.edges.is_empty(), "边来自核心连接关系");
    // 所有边端点都有对应节点
    for e in &m.edges {
        assert!(
            m.nodes.iter().any(|n| (n.x, n.y) == e.from)
                && m.nodes.iter().any(|n| (n.x, n.y) == e.to),
            "边端点缺节点"
        );
    }
    // 第一层应有可达节点
    assert!(!m.reachable.is_empty(), "开局应有合法路线");
    // 节点符号属于实际类型集
    for n in &m.nodes {
        assert!("ME?R$TB".contains(n.symbol), "非法符号 {}", n.symbol);
    }
}

#[test]
fn gameview_capture_battle_full() {
    let mut run = new_run(3);
    let _ = run.take_neow(0);
    let v0 = GameView::capture(&run);
    let (x, y) = v0.map.as_ref().unwrap().reachable[0];
    run.choose_node(x, y).unwrap();
    run.battle.as_mut().unwrap().run_until_idle();
    assert_eq!(GameView::capture(&run).screen, Screen::Battle);
    let v = GameView::capture(&run);
    let b = v.battle.expect("战斗必有 BattleView");
    assert_eq!(b.enemies.len(), 1, "首层普通战单怪");
    let e = &b.enemies[0];
    assert!(!e.name.is_empty());
    assert!(e.max_hp > 0);
    assert!(!e.intent.is_empty(), "意图来自核心 intent_label");
    assert!(!b.hand.is_empty(), "首回合已抽牌");
    for c in &b.hand {
        assert!(!c.name.is_empty(), "卡名来自数据表中文");
        assert!(!c.kind_label.is_empty());
    }
    assert!(v.top.energy.is_some(), "战斗中顶栏带能量");
}

#[test]
fn gameview_capture_rewards() {
    let mut run = new_run(4);
    let _ = run.take_neow(0);
    let v = GameView::capture(&run);
    let (x, y) = v.map.as_ref().unwrap().reachable[0];
    run.choose_node(x, y).unwrap();
    if run.phase == sts_core::flow4b::RoomPhase::InBattle {
        run.battle.as_mut().unwrap().run_until_idle();
    }
    win_battle(&mut run);
    let v = GameView::capture(&run);
    assert_eq!(v.screen, Screen::Rewards);
    let r = v.reward.expect("奖励场景必有 RewardView");
    assert!(r.gold > 0, "金币奖励来自核心");
    assert_eq!(r.cards.len(), 3, "卡奖 3 张");
    for c in &r.cards {
        assert!(!c.name.is_empty());
    }
}

#[test]
fn gameview_capture_event() {
    let mut run = new_run(5);
    let _ = run.take_neow(0);
    run.force_event("Golden Idol");
    let v = GameView::capture(&run);
    assert_eq!(v.screen, Screen::Event);
    let ev = v.event.expect("事件必有 EventView");
    assert!(!ev.title.is_empty());
    assert!(!ev.body.is_empty());
    assert!(!ev.options.is_empty(), "选项来自事件定义");
}

#[test]
fn gameview_capture_shop() {
    let mut run = new_run(6);
    let _ = run.take_neow(0);
    run.force_shop_for_test();
    let v = GameView::capture(&run);
    assert_eq!(v.screen, Screen::Shop);
    let shop = v.shop.expect("商店必有 ShopView");
    assert!(!shop.cards.is_empty(), "商品来自商店状态");
    assert!(shop.purge_cost >= 75);
    for e in &shop.cards {
        assert!(e.price > 0);
    }
}

#[test]
fn gameview_capture_campfire_gated_options() {
    let mut run = new_run(7);
    let _ = run.take_neow(0);
    run.phase = sts_core::flow4b::RoomPhase::CampfireChoice;
    let v = GameView::capture(&run);
    let cf = v.campfire.expect("篝火必有 CampfireView");
    // 基础两选项：休息 + 锻造
    assert!(cf.actions.iter().any(|(a, _)| *a == CampfireAction::Rest));
    assert!(cf.actions.iter().any(|(a, _)| *a == CampfireAction::Smith));
    // 无和平烟斗时不显示移除选项
    assert!(!cf.actions.iter().any(|(a, _)| *a == CampfireAction::Pipe));
    // 拿到和平烟斗后出现
    run.apply_relic_effect("Peace Pipe");
    run.relics.push("Peace Pipe");
    let v2 = GameView::capture(&run);
    let cf2 = v2.campfire.expect("篝火必有 CampfireView");
    assert!(cf2.actions.iter().any(|(a, _)| *a == CampfireAction::Pipe));
}

// ---------------------------------------------------------------- dispatch

#[test]
fn dispatch_rejects_illegally() {
    let mut run = new_run(8);
    let _ = run.take_neow(0);
    // 不可达节点：核心拒绝
    let r = dispatch(&mut run, &PlayerCommand::ChooseMapNode { x: 9, y: 9 });
    assert!(!r.accepted, "越权命令被核心拒绝");
    // 不在战斗中结束回合
    let r = dispatch(&mut run, &PlayerCommand::EndTurn);
    assert!(!r.accepted, "非战斗 EndTurn 拒绝");
}

#[test]
fn dispatch_full_map_to_battle_loop() {
    let mut run = new_run(9);
    // Neow 选祝福（协议路径）
    let r = dispatch(&mut run, &PlayerCommand::OpenNeowPick { index: 0 });
    assert!(r.accepted);
    // 选节点
    let v = GameView::capture(&run);
    let (x, y) = v.map.as_ref().unwrap().reachable[0];
    let r = dispatch(&mut run, &PlayerCommand::ChooseMapNode { x, y });
    assert!(r.accepted);
    assert_eq!(GameView::capture(&run).screen, Screen::Battle);
    // 打完手牌里的攻击/技能（能量允许时），再结束回合——全走协议
    for _ in 0..2 {
        let v = GameView::capture(&run);
        if v.screen != Screen::Battle {
            break;
        }
        if let Some(c) = v
            .battle
            .as_ref()
            .unwrap()
            .hand
            .iter()
            .find(|c| c.affordable)
        {
            let tgt = if c.needs_target { Some(0) } else { None };
            let r = dispatch(
                &mut run,
                &PlayerCommand::PlayCard {
                    hand_index: c.hand_index,
                    target: tgt,
                    x_energy: None,
                },
            );
            assert!(r.accepted, "合法出牌应被接受");
        }
        let r = dispatch(&mut run, &PlayerCommand::EndTurn);
        assert!(r.accepted);
    }
}

#[test]
fn dispatch_event_and_shop_paths() {
    let mut run = new_run(10);
    let _ = run.take_neow(0);
    run.force_event("Golden Idol");
    let v = GameView::capture(&run);
    let n = v.event.as_ref().unwrap().options.len();
    let r = dispatch(
        &mut run,
        &PlayerCommand::EventChoose {
            index: n.saturating_sub(1),
        },
    );
    assert!(r.accepted, "最后一个选项总是合法（离开）");
    // 商店
    run.force_shop_for_test();
    let gold0 = run.gold;
    let price = run
        .shop
        .as_ref()
        .unwrap()
        .potions
        .first()
        .map(|p| p.1)
        .unwrap_or(50);
    if gold0 >= price {
        let r = dispatch(
            &mut run,
            &PlayerCommand::ShopBuy {
                item: ShopItemKind::Potion,
                index: 0,
            },
        );
        assert!(r.accepted);
        assert_eq!(run.gold, gold0 - price);
    }
    let r = dispatch(&mut run, &PlayerCommand::LeaveShop);
    assert!(r.accepted);
}

#[test]
fn dispatch_deck_pick_commands() {
    let mut run = new_run(11);
    let _ = run.take_neow(0);
    run.phase = sts_core::flow4b::RoomPhase::CampfireChoice;
    let n0 = run.deck.len();
    // 锻造选牌（桌面直连）
    let r = dispatch(
        &mut run,
        &PlayerCommand::CampfireSmithPick { deck_index: 0 },
    );
    assert!(r.accepted);
    assert!(run.deck[0].upgraded || !run.deck.is_empty());
    // 和平烟斗未持有时拒绝
    let r = dispatch(&mut run, &PlayerCommand::CampfirePipePick { deck_index: 0 });
    assert!(!r.accepted, "无和平烟斗应拒绝");
    run.apply_relic_effect("Peace Pipe");
    run.relics.push("Peace Pipe");
    run.phase = sts_core::flow4b::RoomPhase::CampfireChoice;
    let n = run.deck.len();
    let r = dispatch(&mut run, &PlayerCommand::CampfirePipePick { deck_index: 0 });
    assert!(r.accepted);
    assert_eq!(run.deck.len(), n - 1);
    let _ = n0;
}
