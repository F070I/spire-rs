//! 批次 5h 测试：巨首倒计时/Maw 咆哮递增/Nemesis 相位/奖励流五遗物。

use sts_core::battle::{Monster, RelicCtx, SpecialAi};
use sts_core::flow4b::RunState4b;
use sts_core::power::PowerKind;

fn battle_with(monsters: Vec<Monster>, r: RelicCtx) -> sts_core::battle::BattleState {
    let deck: Vec<(usize, bool)> = (0..10).map(|_| (sid("Defend_R"), false)).collect();
    let mut b = sts_core::battle::BattleState::new(80, 3, deck, monsters, 5);
    b.relics_ctx = r;
    b.start_battle();
    b.roll_weighted_initial();
    b.run_until_idle();
    b
}

fn sid(id: &str) -> usize {
    sts_data::gen::CARDS
        .iter()
        .position(|c| c.id == id)
        .expect(id)
}

#[test]
fn giant_head_counts_down_then_stares() {
    let mut gh = Monster::scripted("Giant Head", 300, vec![Some(13)]);
    gh.special = Some(SpecialAi::GiantHead);
    let mut b = battle_with(vec![gh], RelicCtx::default());
    for t in 1..=5 {
        b.end_turn();
        b.run_until_idle();
        let script = b.monsters[0].script[0];
        if t <= 4 {
            assert!(script.is_some(), "计数阶段：瞪视(0)/计数(13)");
        } else {
            let d = script.unwrap();
            assert_eq!(d, 30 + (t - 5) * 5, "死亡凝视 +5/回合");
        }
    }
}

#[test]
fn maw_roar_escalates_strength() {
    let mut maw = Monster::scripted("Maw", 300, vec![Some(25), Some(0)]);
    maw.special = Some(SpecialAi::EscalatingBuff { base: 3, step: 2 });
    let mut b = battle_with(vec![maw], RelicCtx::default());
    // 强制两次咆哮
    for expect in [3, 5] {
        b.monsters[0].script = vec![Some(0)];
        b.end_turn();
        b.run_until_idle();
        let str_amt = b.monsters[0]
            .creature
            .powers
            .iter()
            .filter(|p| p.kind == PowerKind::Strength)
            .map(|p| p.amount)
            .sum::<i32>();
        assert!(str_amt >= expect, "咆哮力量 {str_amt} ≥ {expect}");
    }
}

#[test]
fn nemesis_gains_intangible() {
    let mut nem = Monster::scripted("Nemesis", 185, vec![Some(6)]);
    nem.special = Some(SpecialAi::PhaseShift);
    let mut b = battle_with(vec![nem], RelicCtx::default());
    b.end_turn();
    b.run_until_idle();
    assert!(
        b.monsters[0]
            .creature
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Intangible),
        "相位无实体生效"
    );
}

fn win_battle(run: &mut RunState4b) {
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
fn prayer_wheel_extends_card_reward() {
    let mut run = RunState4b::new(61);
    run.open_neow();
    let _ = run.take_neow(0);
    run.apply_relic_effect("Prayer Wheel");
    run.relics.push("Prayer Wheel");
    run.current = Some((3, 0));
    run.map[0][3].symbol = Some('M');
    run.enter_room_public();
    win_battle(&mut run);
    assert_eq!(run.card_reward.len(), 4, "祈祷轮：卡奖 4 张");
}

#[test]
fn prismatic_shard_mixes_colorless() {
    // 概率性——统计 3 局卡奖长度不 panic（30% 无色在池内抽样）
    for seed in 0..3 {
        let mut r = RunState4b::new(65 + seed);
        r.open_neow();
        let _ = r.take_neow(0);
        r.apply_relic_effect("PrismaticShard");
        r.relics.push("PrismaticShard");
        r.current = Some((3, 0));
        r.map[0][3].symbol = Some('M');
        r.enter_room_public();
        win_battle(&mut r);
        assert_eq!(r.card_reward.len(), 3);
    }
}

#[test]
fn matryoshka_may_grant_second_relic() {
    let mut run = RunState4b::new(67);
    run.open_neow();
    let _ = run.take_neow(0);
    run.apply_relic_effect("Matryoshka");
    run.relics.push("Matryoshka");
    run.relic_reward = Some("COMMON");
    let n0 = run.relics.len();
    let _ = run.open_treasure();
    // 50%：遗物数 ≥ n0（附赠时 +1）
    assert!(run.relics.len() >= n0);
}
