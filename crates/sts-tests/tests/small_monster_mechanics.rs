//! 批次 5c 小怪特殊机制测试：蜷缩/分裂/召唤/逃跑/连击钳制。

use sts_core::action::{GameAction, TargetRef};
use sts_core::battle::{BattleState, Monster, RelicCtx};
use sts_core::damage::DamageInfo;
use sts_core::flow4b::RunState4b;
use sts_core::power::{DamageType, PowerKind};

fn sid(id: &str) -> usize {
    sts_data::gen::CARDS
        .iter()
        .position(|c| c.id == id)
        .expect(id)
}

fn battle(monsters: Vec<Monster>) -> BattleState {
    let deck: Vec<(usize, bool)> = (0..10).map(|_| (sid("Defend_R"), false)).collect();
    let mut b = BattleState::new(80, 3, deck, monsters, 5);
    b.relics_ctx = RelicCtx::default();
    b.start_battle();
    b.roll_weighted_initial();
    b.run_until_idle();
    b
}

fn hit_monster(b: &mut BattleState, mi: usize, amount: i32) {
    b.mgr.add_bottom(GameAction::Damage {
        target: TargetRef::Monster(mi),
        info: DamageInfo {
            owner_is_player: true,
            base: amount,
            output: amount,
            ty: DamageType::Normal,
            modified: true,
        },
    });
    b.run_until_idle();
}

#[test]
fn louse_defensive_curls_up_once() {
    let mut louse = Monster::scripted("Louse Defensive", 15, vec![Some(6)]);
    louse
        .creature
        .powers
        .push(sts_core::power::Power::new(PowerKind::CurlUp, 5));
    let mut b = battle(vec![louse]);
    assert_eq!(b.monsters[0].creature.current_block, 0);
    hit_monster(&mut b, 0, 2);
    assert_eq!(b.monsters[0].creature.current_block, 5, "蜷缩 +5 格挡");
    assert!(
        !b.monsters[0]
            .creature
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::CurlUp),
        "蜷缩一次性移除"
    );
    let block_before = b.monsters[0].creature.current_block;
    hit_monster(&mut b, 0, 2);
    assert!(
        b.monsters[0].creature.current_block < block_before + 5,
        "二次受击不再 +5（蜷缩已消耗）"
    );
}

#[test]
fn slime_boss_splits_at_half() {
    let mut boss = Monster::scripted("Slime Boss", 140, vec![Some(18), None, Some(16)]);
    boss.special = Some(sts_core::battle::SpecialAi::SplitOnHalf);
    boss.creature.current_health = 60; // <50%
    let mut b = battle(vec![boss]);
    b.end_turn();
    b.run_until_idle();
    // 分裂：原 Boss 离场，双大软泥继承 60 血
    assert!(b.monsters.len() >= 3, "分裂出两只大软泥");
    let slimes: Vec<&Monster> = b
        .monsters
        .iter()
        .filter(|m| m.creature.name.contains("Slime L"))
        .collect();
    assert_eq!(slimes.len(), 2, "Spike Slime L + Acid Slime L");
    for m in &slimes {
        assert_eq!(m.creature.max_health, 60, "继承分裂时 Boss 当前血");
    }
    assert!(!b.is_battle_over, "分裂后战斗继续");
}

#[test]
fn gremlin_leader_summons_every_third_turn() {
    let mut leader = Monster::scripted("Gremlin Leader", 140, vec![Some(7)]);
    leader.special = Some(sts_core::battle::SpecialAi::Summoner);
    let mut b = battle(vec![leader]);
    let n0 = b.monsters.len();
    // 三回合后召唤
    for _ in 0..3 {
        b.end_turn();
        b.run_until_idle();
    }
    assert!(b.monsters.len() > n0, "第 3 回合召唤哥布林");
    let minions = b
        .monsters
        .iter()
        .filter(|m| m.creature.name.starts_with("Gremlin") && m.creature.name != "Gremlin Leader")
        .count();
    assert!(minions >= 2, "一次召 2 只");
}

#[test]
fn gremlin_gang_survivors_flee_after_death() {
    let mut run = RunState4b::new(21);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_battle_for_test(vec![
        Monster::scripted("Gremlin Thief", 10, vec![None]),
        Monster::scripted("Gremlin Fat", 10, vec![None]),
        Monster::scripted("Gremlin Warrior", 10, vec![None]),
    ]);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    hit_monster(b, 0, 99); // 杀死一只
    assert!(
        b.monsters[1].escape_next && b.monsters[2].escape_next,
        "幸存同伴标记逃跑"
    );
}

#[test]
fn last_two_constraint_blocks_triple_repeat() {
    let wm = sts_data::monster_ai::get_weighted("Louse Defensive").unwrap();
    // 咬击权重 75% 且连击禁位：上两招都是咬时，掷中咬应顺延到强化(-1)
    let (dmg, _, _) = sts_data::monster_ai::pick_weighted(wm, 0, true);
    assert_eq!(dmg, -1, "连击钳制：第三咬被禁止");
    let (dmg, _, _) = sts_data::monster_ai::pick_weighted(wm, 0, false);
    assert_eq!(dmg, 6, "无钳制时正常咬");
}

#[test]
fn escape_ends_battle_when_all_flee() {
    let mut run = RunState4b::new(23);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_battle_for_test(vec![
        Monster::scripted("Gremlin Thief", 10, vec![None]),
        Monster::scripted("Gremlin Fat", 10, vec![None]),
    ]);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    hit_monster(b, 0, 99);
    // 幸存者下回合逃跑 → 全员离场 → 战斗结束
    for _ in 0..3 {
        if b.is_battle_over {
            break;
        }
        b.end_turn();
        b.run_until_idle();
    }
    assert!(b.is_battle_over, "全员逃跑即胜利");
}
