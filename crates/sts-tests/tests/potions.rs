//! 批次 4f 药水实装测试：数值强化/目标类/全体类/卡牌生成/特殊机制。

use sts_core::action::{GameAction, TargetRef};
use sts_core::battle::Monster;
use sts_core::damage::DamageInfo;
use sts_core::flow4b::{RunState4b, RoomPhase};
use sts_core::power::{DamageType, PowerKind};
use sts_data::gen;

fn run_with_potion(id: &'static str) -> RunState4b {
    let mut run = RunState4b::new(7);
    run.open_neow();
    let _ = run.take_neow(0);
    run.potions[0] = Some(id);
    run
}

fn battle_run(id: &'static str) -> RunState4b {
    let mut run = run_with_potion(id);
    run.force_battle_for_test(vec![Monster::scripted("Jaw Worm", 44, vec![Some(10)])]);
    run
}

#[test]
fn strength_potion_applies_in_battle() {
    let mut run = battle_run("Strength Potion");
    let _ = run.use_potion(0, None);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    assert!(
        b.player
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Strength && p.amount >= 2),
        "力量药水 +2 力"
    );
}

#[test]
fn fire_potion_thorns_ignores_block() {
    let mut run = battle_run("Fire Potion");
    {
        let b = run.battle.as_mut().unwrap();
        b.monsters[0].creature.current_block = 99;
        b.mgr.add_bottom(GameAction::Damage {
            target: TargetRef::Monster(0),
            info: DamageInfo::pure(true, 20, DamageType::Thorns),
        });
        b.run_until_idle();
    }
    assert_eq!(
        run.battle.as_ref().unwrap().monsters[0].creature.current_health,
        24,
        "THORNS 20 无视 99 格挡"
    );
}

#[test]
fn speed_and_steroid_gain_then_lose() {
    let mut run = battle_run("SpeedPotion");
    let _ = run.use_potion(0, None);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    assert!(
        b.player
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Dexterity && p.amount == 5)
    );
    assert!(
        b.player
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::LoseDexterity && p.amount == 5)
    );
}

#[test]
fn regen_potion_heals_and_decays() {
    let mut run = run_with_potion("Regen Potion");
    run.force_battle_for_test(vec![Monster::scripted("Jaw Worm", 44, vec![None])]);
    let _ = run.use_potion(0, None);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    assert!(b.player.powers.iter().any(|p| p.kind == PowerKind::Regen && p.amount == 5));
    // 回合推进：回合末回 5 → -1
    b.player.current_health = 40;
    b.end_turn();
    b.run_until_idle();
    let regen = b
        .player
        .powers
        .iter()
        .find(|p| p.kind == PowerKind::Regen)
        .map(|p| p.amount);
    assert_eq!(regen, Some(4), "回合并递减");
    assert!(b.player.current_health > 40, "回合末回血");
}

#[test]
fn block_and_energy_potions() {
    let mut run = battle_run("Block Potion");
    let _ = run.use_potion(0, None);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    assert!(b.player.current_block >= 12);

    let mut run = battle_run("Energy Potion");
    let e0 = run.battle.as_ref().unwrap().energy;
    let _ = run.use_potion(0, None);
    run.battle.as_mut().unwrap().run_until_idle();
    assert!(run.battle.as_ref().unwrap().energy > e0);
}

#[test]
fn cunning_potion_makes_shivs() {
    let mut run = battle_run("CunningPotion");
    let _ = run.use_potion(0, None);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    let shiv_sid = gen::CARDS.iter().position(|c| c.id == "Shiv").unwrap();
    assert_eq!(b.hand.group.iter().filter(|c| c.sid == shiv_sid).count(), 3);
}

#[test]
fn elixir_exhausts_status_and_curses() {
    let mut run = battle_run("ElixirPotion");
    {
        let b = run.battle.as_mut().unwrap();
        for id in ["Burn", "Doubt", "Strike_R"] {
            let sid = gen::CARDS.iter().position(|c| c.id == id).unwrap();
            b.hand.add_top(sts_core::battle::CardInst::new(sid, false, 9000 + sid as u64));
        }
    }
    let _ = run.use_potion(0, None);
    let b = run.battle.as_ref().unwrap();
    assert!(
        !b.hand.group.iter().any(|c| c.st().id == "Burn" || c.st().id == "Doubt"),
        "状态/诅咒被消耗"
    );
    assert!(b.hand.group.iter().any(|c| c.st().id == "Strike_R"));
}

#[test]
fn entropic_brew_fills_slots() {
    let mut run = battle_run("EntropicBrew");
    run.potions[1] = None;
    run.potions[2] = None;
    let _ = run.use_potion(0, None);
    assert!(
        run.potions.iter().all(|s| s.is_some()),
        "全部药水槽填满"
    );
}

#[test]
fn fruit_juice_raises_max_hp() {
    let mut run = battle_run("Fruit Juice");
    let max0 = run.player_max_hp;
    let _ = run.use_potion(0, None);
    assert_eq!(run.player_max_hp, max0 + 5);
    assert_eq!(run.player_hp, max0 + 5);
}

#[test]
fn smoke_bomb_ends_non_boss_battle() {
    let mut run = battle_run("SmokeBomb");
    let _ = run.use_potion(0, None);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    assert!(b.is_battle_over, "非 Boss 战立即结束");
}

#[test]
fn fairy_potion_manual_heal() {
    let mut run = run_with_potion("FairyPotion");
    run.player_hp = 20;
    let _ = run.use_potion(0, None);
    assert_eq!(run.player_hp, 20 + 80 * 30 / 100);
}

#[test]
fn ancient_potion_gives_artifact() {
    let mut run = battle_run("Ancient Potion");
    let _ = run.use_potion(0, None);
    let b = run.battle.as_mut().unwrap();
    b.run_until_idle();
    assert!(b.player.powers.iter().any(|p| p.kind == PowerKind::Artifact && p.amount == 1));
}

#[test]
fn every_potion_usable_without_panic() {
    for p in gen::potions::POTIONS.iter() {
        let mut run = battle_run(p.id);
        let _ = run.use_potion(0, Some(0));
        run.potions[0] = Some(p.id);
        let _ = run.use_potion(0, None); // 无目标路径也不崩
    }
}

#[test]
fn non_battle_potions_are_safe() {
    let mut run = run_with_potion("Strength Potion");
    run.phase = RoomPhase::RoomDone;
    let _ = run.use_potion(0, None); // 非战斗：战斗类药水静默消耗
    assert!(run.potions[0].is_none());
}
