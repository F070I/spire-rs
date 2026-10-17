//! 批次 4h 诅咒/状态手牌触发测试（Regret/Shame/Decay/Pain/Normality）。

use sts_core::battle::Monster;
use sts_core::flow4b::RunState4b;
use sts_data::gen;

fn run_with_hand(ids: &[&str]) -> RunState4b {
    let mut run = RunState4b::new(7);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_battle_for_test(vec![Monster::scripted("Jaw Worm", 44, vec![None])]);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    for id in ids {
        let sid = gen::CARDS.iter().position(|c| c.id == *id).unwrap();
        b.hand.add_top(sts_core::battle::CardInst::new(sid, false, 88000 + sid as u64));
    }
    run
}

#[test]
fn regret_loses_hp_equal_to_hand_size() {
    let mut run = run_with_hand(&["Regret"]);
    let b = run.battle.as_mut().unwrap();
    b.player.current_health = 80;
    let hand_n = b.hand.group.len() as i32;
    b.end_turn();
    b.run_until_idle();
    assert_eq!(b.player.current_health, 80 - hand_n, "悔恨：失去手牌数生命");
}

#[test]
fn decay_thorns_two() {
    let mut run = run_with_hand(&["Decay"]);
    let b = run.battle.as_mut().unwrap();
    b.player.current_health = 80;
    b.end_turn();
    b.run_until_idle();
    assert_eq!(b.player.current_health, 78, "腐朽：回合末 THORNS 2");
}

#[test]
fn shame_grants_frail() {
    let mut run = run_with_hand(&["Shame"]);
    let b = run.battle.as_mut().unwrap();
    b.end_turn();
    b.run_until_idle();
    assert!(
        b.player
            .powers
            .iter()
            .any(|p| p.kind == sts_core::power::PowerKind::Frail && p.amount == 1)
    );
}

#[test]
fn pain_loses_one_per_card() {
    let mut run = run_with_hand(&["Pain", "Strike_R", "Strike_R"]);
    let b = run.battle.as_mut().unwrap();
    b.player.current_health = 80;
    let i = b.hand.group.iter().position(|c| c.st().id == "Strike_R").unwrap();
    b.play_card(i, Some(0)).unwrap();
    b.run_until_idle();
    assert_eq!(b.player.current_health, 79, "苦痛：出牌掉 1 血");
}

#[test]
fn normality_caps_three_cards() {
    let mut run = run_with_hand(&["Normality", "Strike_R", "Strike_R", "Strike_R", "Strike_R"]);
    let b = run.battle.as_mut().unwrap();
    b.cards_played_this_turn = 3;
    let i = b.hand.group.iter().position(|c| c.st().id == "Strike_R").unwrap();
    assert!(b.play_card(i, Some(0)).is_err(), "凡庸：3 张上限");
}
