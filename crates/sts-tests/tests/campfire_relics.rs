//! 批次 5i 测试：篝火四扩展选项 + 赌徒筹码 + 冻结之眼 + 奥瑞瑞。

use sts_core::flow4b::{RunState4b, RoomPhase};

fn new_run() -> RunState4b {
    let mut run = RunState4b::new(71);
    run.open_neow();
    let _ = run.take_neow(0);
    run
}

#[test]
fn peace_pipe_removes_card_at_campfire() {
    let mut run = new_run();
    run.apply_relic_effect("Peace Pipe");
    run.relics.push("Peace Pipe");
    run.phase = RoomPhase::CampfireChoice;
    let n0 = run.deck.len();
    let _ = run.campfire_pipe(0);
    assert_eq!(run.deck.len(), n0 - 1);
    // 无烟斗拒绝
    let mut run2 = new_run();
    run2.phase = RoomPhase::CampfireChoice;
    assert!(run2.campfire_pipe(0).is_err());
}

#[test]
fn shovel_digs_relic() {
    let mut run = new_run();
    run.apply_relic_effect("Shovel");
    run.relics.push("Shovel");
    run.phase = RoomPhase::CampfireChoice;
    let n0 = run.relics.len();
    let _ = run.campfire_dig();
    assert_eq!(run.relics.len(), n0 + 1);
}

#[test]
fn singing_bowl_raises_max_hp() {
    let mut run = new_run();
    run.apply_relic_effect("Singing Bowl");
    run.relics.push("Singing Bowl");
    run.phase = RoomPhase::CampfireChoice;
    let max0 = run.player_max_hp;
    let _ = run.campfire_bowl();
    assert_eq!(run.player_max_hp, max0 + 2);
}

#[test]
fn question_card_adds_card() {
    let mut run = new_run();
    run.apply_relic_effect("Question Card");
    run.relics.push("Question Card");
    run.phase = RoomPhase::CampfireChoice;
    let n0 = run.deck.len();
    let _ = run.campfire_question();
    assert_eq!(run.deck.len(), n0 + 1);
}

#[test]
fn orrery_grants_two_cards() {
    let mut run = new_run();
    let n0 = run.deck.len();
    run.apply_relic_effect("Orrery");
    run.relics.push("Orrery");
    assert_eq!(run.deck.len(), n0 + 2);
}

#[test]
fn gambling_chip_redraws_once_per_turn() {
    let mut run = new_run();
    run.apply_relic_effect("Gambling Chip");
    run.relics.push("Gambling Chip");
    run.force_battle_for_test(vec![sts_core::battle::Monster::scripted(
        "Jaw Worm",
        44,
        vec![Some(10)],
    )]);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    let n = b.hand.group.len();
    assert!(run.gambling_redraw().is_ok(), "首用成功");
    let b2 = run.battle.as_mut().unwrap();
    b2.run_until_idle();
    assert_eq!(b2.hand.group.len(), n, "重抽同数");
    assert!(run.gambling_redraw().is_err(), "本回合二用拒绝");
}
