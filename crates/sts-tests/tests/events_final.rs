//! 批次 4d 事件收尾测试：新效果变体的功能验证（走 RunState4b.choose_event）。

use sts_core::flow4b::RunState4b;

/// 起一局进到事件房（直达：手摆 boss 事件流——用事件房入口）。
fn run_with_event() -> RunState4b {
    let mut run = RunState4b::new(42);
    run.open_neow();
    let _ = run.take_neow(0); // ThreeCards：避免改变 HP 的祝福
    run.force_event("MindBloom");
    run
}

#[test]
fn mindbloom_upgrade_all_and_bathe() {
    // 生长：全部可升级卡升级
    let mut run = run_with_event();
    let n_upgradeable = run.deck.iter().filter(|c| c.can_upgrade()).count();
    assert!(n_upgradeable > 0, "初始牌组应有可升级卡");
    let _ = run.choose_event(1); // 生长
    assert_eq!(run.deck.iter().filter(|c| c.can_upgrade()).count(), 0);

    // 沐浴：回满血 + 999 金 + 2 张 Normality
    let mut run = run_with_event();
    run.player_hp = 10;
    let gold0 = run.gold;
    let normality = sts_data::gen::CARDS
        .iter()
        .position(|c| c.id == "Normality")
        .unwrap();
    let curses_before = run.deck.iter().filter(|c| c.sid == normality).count();
    let _ = run.choose_event(2); // 沐浴
    assert_eq!(run.player_hp, run.player_max_hp);
    assert_eq!(run.gold, gold0 + 999);
    assert_eq!(
        run.deck.iter().filter(|c| c.sid == normality).count(),
        curses_before + 2
    );
}

#[test]
fn fountain_removes_all_curses_keeps_ascendersbane() {
    let mut run = run_with_event();
    run.force_event("Fountain of Cleansing");
    // 手动塞诅咒（含不可移除的 AscendersBane）
    for id in ["Decay", "Doubt", "AscendersBane"] {
        let sid = sts_data::gen::CARDS
            .iter()
            .position(|c| c.id == id)
            .unwrap();
        run.deck
            .push(sts_core::battle::CardInst::new(sid, false, run.uuid_next));
        run.uuid_next += 1;
    }
    let _ = run.choose_event(0); // 饮用
    assert!(!run.deck.iter().any(|c| c.st().id == "Decay"));
    assert!(!run.deck.iter().any(|c| c.st().id == "Doubt"));
    assert!(
        run.deck.iter().any(|c| c.st().id == "AscendersBane"),
        "开局诅咒豁免"
    );
}

#[test]
fn tomb_red_mask_pays_all_gold() {
    let mut run = run_with_event();
    run.force_event("Tomb of Lord Red Mask");
    run.gold = 333;
    let _ = run.choose_event(1); // 献上全部金币
    assert_eq!(run.gold, 0);
    assert!(run.relics.contains(&"Red Mask"));
}

#[test]
fn sensory_stone_gives_colorless_cards() {
    let mut run = run_with_event();
    run.force_event("SensoryStone");
    let n0 = run.deck.len();
    let _ = run.choose_event(2); // 回忆三段
    assert_eq!(run.deck.len(), n0 + 3);
    // 三张都是无色且可打出类型
    for c in run.deck.iter().skip(n0) {
        assert_eq!(c.st().color, sts_data::gen::CardColor::Colorless);
    }
}

#[test]
fn back_to_basics_upgrades_strikes_defends_only() {
    let mut run = run_with_event();
    run.force_event("Back to Basics");
    // 塞一张非 S/D 可升级卡（Bash）与一张 Strike_R
    let bash = sts_data::gen::CARDS
        .iter()
        .position(|c| c.id == "Bash")
        .unwrap();
    let strike = sts_data::gen::CARDS
        .iter()
        .position(|c| c.id == "Strike_R")
        .unwrap();
    run.deck
        .push(sts_core::battle::CardInst::new(bash, false, run.uuid_next));
    run.uuid_next += 1;
    run.deck.push(sts_core::battle::CardInst::new(
        strike,
        false,
        run.uuid_next,
    ));
    run.uuid_next += 1;
    let _ = run.choose_event(0); // 返璞归真
    assert!(
        run.deck.iter().any(|c| c.st().id == "Bash" && !c.upgraded),
        "非 S/D 不升级"
    );
    assert!(
        run.deck
            .iter()
            .any(|c| c.st().id == "Strike_R" && c.upgraded),
        "打击升级"
    );
}

#[test]
fn bonfire_rare_offering_heals_full_and_max_hp() {
    let mut run = run_with_event();
    run.force_event("Bonfire Elementals");
    run.player_hp = 20;
    let max0 = run.player_max_hp;
    let n0 = run.deck.len();
    let _ = run.choose_event(2); // 献上稀有卡
    assert_eq!(run.player_hp, run.player_max_hp);
    assert_eq!(run.player_max_hp, max0 + 10);
    assert_eq!(run.deck.len(), n0 - 1);
}

#[test]
fn compound_effects_apply_in_order() {
    let mut run = run_with_event();
    run.force_event("Liars Game");
    let gold0 = run.gold;
    let doubt = sts_data::gen::CARDS
        .iter()
        .position(|c| c.id == "Doubt")
        .unwrap();
    let before = run.deck.iter().filter(|c| c.sid == doubt).count();
    let _ = run.choose_event(0); // +175 金 + 疑虑
    assert_eq!(run.gold, gold0 + 175);
    assert_eq!(
        run.deck.iter().filter(|c| c.sid == doubt).count(),
        before + 1
    );
}

#[test]
fn every_event_every_option_applies_without_panic() {
    // 全事件全选项冒烟：随机池 + 终幕事件都走一遍
    let ids: Vec<&'static str> = sts_core::event::EVENTS.iter().map(|e| e.id).collect();
    for id in ids {
        let mut run = run_with_event();
        run.force_event(id);
        let n = sts_core::event::get(id).options.len();
        for i in 0..n {
            let mut r = run_with_event();
            r.force_event(id);
            let _ = r.choose_event(i);
        }
    }
}
