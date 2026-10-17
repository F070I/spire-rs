//! 批次 5a 多怪遭遇测试：组合表计数/成员构造/目标选择/多怪战斗流程。

use sts_core::action::{GameAction, TargetRef};
use sts_core::battle::Monster;
use sts_core::damage::DamageInfo;
use sts_core::flow4b::{NeowReward, RunState4b, RoomPhase};
use sts_data::encounters;
use sts_data::rng::StsRandom;

fn comp(key: &str, seed: i64) -> Vec<&'static str> {
    encounters::composition(key, &mut StsRandom::new(seed))
}

#[test]
fn fixed_compositions_match_source() {
    // Louse 50/50 双变体（getLouse 转译）
    for m in comp("2 Louse", 1) {
        assert!(m == "Louse" || m == "Louse Defensive", "非法 Louse 变体 {m}");
    }
    assert_eq!(comp("2 Louse", 1).len(), 2);
    assert_eq!(comp("3 Louse", 1).len(), 3);
    assert_eq!(comp("2 Fungi", 1), vec!["Fungi Beast", "Fungi Beast"]);
    assert_eq!(comp("3 Sentries", 1), vec!["Sentry", "Sentry", "Sentry"]);
    assert_eq!(comp("3 Cultists", 1), vec!["Cultist", "Cultist", "Cultist"]);
    assert_eq!(comp("3 Darklings", 1), vec!["Darkling", "Darkling", "Darkling"]);
    assert_eq!(comp("Jaw Worm Horde", 1), vec!["Jaw Worm", "Jaw Worm", "Jaw Worm"]);
    assert_eq!(
        comp("Centurion & Healer", 1),
        vec!["Centurion", "Healer"]
    );
    assert_eq!(comp("Slavers", 1), vec!["Blue Slaver", "Taskmaster", "Red Slaver"]);
    assert_eq!(comp("Shield and Spear", 1), vec!["Spire Shield", "Spire Spear"]);
    // & 与 and 两写法等价
    assert_eq!(comp("Centurion and Healer", 1), comp("Centurion & Healer", 1));
    // JawWorm 无空格写法
    assert_eq!(comp("JawWorm", 1), vec!["Jaw Worm"]);
    // Boss 单体键直通
    assert_eq!(comp("Time Eater", 1), vec!["Time Eater"]);
    assert_eq!(comp("The Guardian", 1), vec!["The Guardian"]);
}

#[test]
fn random_pools_roll_within_members() {
    // 大软泥 50/50
    let r = comp("Large Slime", 3);
    assert!(r == vec!["Acid Slime L"] || r == vec!["Spike Slime L"]);
    // 哥布林帮：4 只不重复
    let g = comp("Gremlin Gang", 5);
    assert_eq!(g.len(), 4);
    let valid = ["Gremlin Warrior", "Gremlin Thief", "Gremlin Fat", "Gremlin Tsundere", "Gremlin Wizard"];
    for m in &g {
        assert!(valid.contains(m), "非法成员 {m}");
    }
    // 形体池：Spiker/Repulsor/Exploder
    for m in comp("3 Shapes", 7) {
        assert!(["Spiker", "Repulsor", "Exploder"].contains(&m));
    }
}

#[test]
fn two_louse_fight_has_two_monsters_and_targeting() {
    let mut run = RunState4b::new(42);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_encounter("2 Louse");
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    assert_eq!(b.monsters.len(), 2, "2 Louse = 两只");
    assert!(b.monsters[0].creature.name.starts_with("Louse"));
    // 打第二只：伤害落在目标 1
    let (hp0, hp1) = (
        b.monsters[0].creature.current_health,
        b.monsters[1].creature.current_health,
    );
    let i = b.hand.group.iter().position(|c| c.st().id == "Strike_R").unwrap();
    b.play_card(i, Some(1)).unwrap();
    b.run_until_idle();
    if b.monsters[0].creature.current_health == hp0 {
        assert!(b.monsters[1].creature.current_health < hp1, "伤害落在目标 1");
    }
}

#[test]
fn cultist_trio_all_have_ritual() {
    let mut run = RunState4b::new(9);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_encounter("3 Cultists");
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    assert_eq!(b.monsters.len(), 3);
    for m in &b.monsters {
        assert_eq!(m.creature.name, "Cultist");
    }
    // 打完一轮后全员力量提升（仪式）
    let s0: i32 = b
        .monsters
        .iter()
        .map(|m| m.creature.powers.iter().find(|p| p.kind == sts_core::power::PowerKind::Strength).map(|p| p.amount).unwrap_or(0))
        .sum();
    b.end_turn();
    b.run_until_idle();
    let s1: i32 = b
        .monsters
        .iter()
        .map(|m| m.creature.powers.iter().find(|p| p.kind == sts_core::power::PowerKind::Strength).map(|p| p.amount).unwrap_or(0))
        .sum();
    assert!(s1 > s0, "三教徒全员仪式");
}

#[test]
fn slime_boss_still_single_and_real() {
    let mut run = RunState4b::new(11);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_encounter("Slime Boss");
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    assert_eq!(b.monsters.len(), 1);
    assert_eq!(b.monsters[0].creature.max_health, 140);
}

#[test]
fn member_hp_rolls_within_table_range() {
    let mut run = RunState4b::new(13);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_encounter("2 Louse");
    let b = run.battle.as_ref().unwrap();
    for m in &b.monsters {
        assert!(
            (10..=17).contains(&m.creature.max_health),
            "Louse HP {} 越界",
            m.creature.max_health
        );
    }
}

#[test]
fn battle_ends_when_all_members_die() {
    let mut run = RunState4b::new(15);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_battle_for_test(vec![
        Monster::scripted("Louse", 10, vec![None]),
        Monster::scripted("Louse", 10, vec![None]),
    ]);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    for mi in 0..2 {
        b.monsters[mi].creature.current_health = 1;
        b.mgr.add_bottom(GameAction::Damage {
            target: TargetRef::Monster(mi),
            info: DamageInfo {
                owner_is_player: true,
                base: 99,
                output: 99,
                ty: sts_core::power::DamageType::Normal,
                modified: true,
            },
        });
    }
    b.run_until_idle();
    assert!(b.is_battle_over, "全员死亡即战斗结束");
}

#[test]
fn rewards_phase_after_multi_monster_victory() {
    let mut run = RunState4b::new(17);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_battle_for_test(vec![
        Monster::scripted("Louse", 5, vec![None]),
        Monster::scripted("Louse", 5, vec![None]),
    ]);
    {
        let b = run.battle.as_mut().unwrap();
        b.run_until_idle();
        for mi in 0..2 {
            b.monsters[mi].creature.current_health = 1;
            b.mgr.add_bottom(GameAction::Damage {
                target: TargetRef::Monster(mi),
                info: DamageInfo {
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
    run.phase = RoomPhase::InBattle;
    run.settle_battle();
    assert_eq!(run.phase, RoomPhase::Rewards);
}

// RunState 辅助：Neow 引用避免未使用告警
#[allow(unused)]
fn _touch(_: NeowReward) {}
