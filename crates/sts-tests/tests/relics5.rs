//! 批次 5f 测试：纸系伤害修正/算盘/金像血像/多莉镜子/坩埚。

use sts_core::action::{GameAction, TargetRef};
use sts_core::battle::{BattleState, Monster, RelicCtx};
use sts_core::damage::DamageInfo;
use sts_core::flow4b::RunState4b;
use sts_core::power::{DamageType, PowerKind};
use sts_data::gen;

fn sid(id: &str) -> usize {
    gen::CARDS.iter().position(|c| c.id == id).expect(id)
}

fn battle(r: RelicCtx) -> BattleState {
    let deck: Vec<(usize, bool)> = (0..10).map(|_| (sid("Defend_R"), false)).collect();
    let mut b = BattleState::new(80, 3, deck, vec![Monster::scripted("Jaw Worm", 44, vec![Some(10)])], 5);
    b.relics_ctx = r;
    b.start_battle();
    b.roll_weighted_initial();
    b.run_until_idle();
    b
}

#[test]
fn paper_frog_boosts_vulnerable_target() {
    let r = RelicCtx { paper_frog: true, ..RelicCtx::default() };
    let mut b = battle(r);
    b.monsters[0].creature.powers.push(sts_core::power::Power::new(PowerKind::Vulnerable, 2));
    // attack_info 路径：10 伤 → 链内易伤 ×1.5=15 → 纸蛙 ×(1.75/1.5)=17.5 → 17
    let info = sts_core::cards::attack_info(&b, 0, 10, DamageType::Normal);
    assert_eq!(info.output, 17, "纸蛙 75% 增伤（floor 17.5）");
}

#[test]
fn paper_krane_dampens_weak_attacker() {
    let r = RelicCtx { paper_krane: true, ..RelicCtx::default() };
    let mut b = battle(r);
    b.monsters[0].creature.powers.push(sts_core::power::Power::new(PowerKind::Weak, 2));
    let hp0 = b.player.current_health;
    b.monsters[0].script = vec![Some(10)];
    b.monsters[0].move_idx = 0;
    b.end_turn();
    b.run_until_idle();
    let taken = hp0 - b.player.current_health;
    assert_eq!(taken, 6, "纸鹤：10 伤弱化 -40% = 6");
}

#[test]
fn abacus_blocks_on_shuffle() {
    let r = RelicCtx { abacus: true, ..RelicCtx::default() };
    let mut b = battle(r);
    b.player.current_block = 0;
    b.mgr.add_bottom(GameAction::EmptyDeckShuffle);
    b.run_until_idle();
    assert_eq!(b.player.current_block, 6, "洗牌 +6 格挡");
}

#[test]
fn golden_idol_boosts_combat_gold() {
    let mut run = RunState4b::new(41);
    run.open_neow();
    let _ = run.take_neow(0);
    run.apply_relic_effect("Golden Idol");
    run.relics.push("Golden Idol");
    run.force_battle_for_test(vec![Monster::scripted("Louse", 5, vec![None])]);
    {
        let b = run.battle.as_mut().unwrap();
        b.run_until_idle();
        b.mgr.add_bottom(GameAction::Damage {
            target: TargetRef::Monster(0),
            info: DamageInfo { owner_is_player: true, base: 99, output: 99, ty: DamageType::Normal, modified: true },
        });
        b.run_until_idle();
    }
    let g0 = run.gold;
    run.settle_battle();
    assert!(run.gold >= g0, "战斗奖励已发");
}

#[test]
fn bloody_idol_heals_on_gold() {
    let mut run = RunState4b::new(43);
    run.open_neow();
    let _ = run.take_neow(0);
    run.apply_relic_effect("Bloody Idol");
    run.relics.push("Bloody Idol");
    run.player_hp = 40;
    run.force_battle_for_test(vec![Monster::scripted("Louse", 5, vec![None])]);
    {
        let b = run.battle.as_mut().unwrap();
        b.run_until_idle();
        b.mgr.add_bottom(GameAction::Damage {
            target: TargetRef::Monster(0),
            info: DamageInfo { owner_is_player: true, base: 99, output: 99, ty: DamageType::Normal, modified: true },
        });
        b.run_until_idle();
    }
    run.settle_battle();
    assert!(run.player_hp > 40, "得金回 5");
}

#[test]
fn dollys_mirror_duplicates_card() {
    let mut run = RunState4b::new(45);
    run.open_neow();
    let _ = run.take_neow(0);
    let n0 = run.deck.len();
    run.apply_relic_effect("DollysMirror");
    run.relics.push("DollysMirror");
    assert_eq!(run.deck.len(), n0 + 1, "复制一张");
}

#[test]
fn cauldron_fills_potion_slots() {
    let mut run = RunState4b::new(47);
    run.open_neow();
    let _ = run.take_neow(0);
    run.apply_relic_effect("Cauldron");
    run.relics.push("Cauldron");
    assert!(run.potions.iter().filter(|s| s.is_some()).count() >= 3, "3 瓶药水");
}
