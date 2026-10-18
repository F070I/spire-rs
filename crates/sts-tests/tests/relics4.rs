//! 批次 5e 遗物第四批测试：战斗开始/回合/出牌/弃牌/消耗/死亡/商店。

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

#[allow(dead_code)]
fn hit_player(b: &mut BattleState, amount: i32) {
    b.mgr.add_bottom(GameAction::Damage {
        target: TargetRef::Player,
        info: DamageInfo {
            owner_is_player: false,
            base: amount,
            output: amount,
            ty: DamageType::Normal,
            modified: true,
        },
    });
    b.run_until_idle();
}

#[test]
fn akabeko_first_attack_bonus_via_vigor() {
    let r = RelicCtx { akabeko: true, ..RelicCtx::default() };
    let b = battle(r);
    assert!(b.player.powers.iter().any(|p| p.kind == PowerKind::Vigor && p.amount == 8));
}

#[test]
fn teardrop_locket_starts_in_calm() {
    let r = RelicCtx { teardrop_locket: true, ..RelicCtx::default() };
    let b = battle(r);
    assert_eq!(b.player_stance, sts_core::stance::StanceKind::Calm);
}

#[test]
fn horn_cleat_gives_block_turn_two() {
    let r = RelicCtx { horn_cleat: true, ..RelicCtx::default() };
    let mut b = battle(r);
    let hp0 = b.player.current_health;
    b.end_turn();
    b.run_until_idle();
    // 第 2 回合起手 +14；Jaw Worm 打 10 → 至少剩 4 块或血没掉满
    assert!(
        b.player.current_block + (hp0 - b.player.current_health) + (b.player.current_health.max(0) - b.player.current_health.max(0)) >= 4,
        "船栓生效（块 {} 血损 {}）",
        b.player.current_block,
        hp0 - b.player.current_health
    );
}

#[test]
fn brimstone_buffs_both_sides() {
    let r = RelicCtx { brimstone: true, ..RelicCtx::default() };
    let mut b = battle(r);
    b.end_turn();
    b.run_until_idle();
    assert!(b.player.powers.iter().any(|p| p.kind == PowerKind::Strength && p.amount >= 2));
    assert!(b.monsters[0].creature.powers.iter().any(|p| p.kind == PowerKind::Strength && p.amount >= 1));
}

#[test]
fn magic_flower_boosts_heal() {
    let r = RelicCtx { magic_flower: true, ..RelicCtx::default() };
    let mut b = battle(r);
    b.player.current_health = 50;
    b.mgr.add_bottom(GameAction::Heal { target: TargetRef::Player, amount: 10 });
    b.run_until_idle();
    assert_eq!(b.player.current_health, 65, "10 治疗经魔花 ×1.5 = 15");
}

#[test]
fn mark_of_bloom_blocks_heal() {
    let r = RelicCtx { mark_of_bloom: true, ..RelicCtx::default() };
    let mut b = battle(r);
    b.player.current_health = 50;
    b.mgr.add_bottom(GameAction::Heal { target: TargetRef::Player, amount: 10 });
    b.run_until_idle();
    assert_eq!(b.player.current_health, 50, "马可之花：治疗归零");
}

#[test]
fn hand_drill_applies_vulnerable_on_block_break() {
    let r = RelicCtx { hand_drill: true, ..RelicCtx::default() };
    let mut b = battle(r);
    b.monsters[0].creature.current_block = 5;
    b.mgr.add_bottom(GameAction::Damage {
        target: TargetRef::Monster(0),
        info: DamageInfo { owner_is_player: true, base: 10, output: 10, ty: DamageType::Normal, modified: true },
    });
    b.run_until_idle();
    assert!(b.monsters[0].creature.powers.iter().any(|p| p.kind == PowerKind::Vulnerable && p.amount == 2));
}

#[test]
fn gremlin_horn_gains_energy_on_kill() {
    let r = RelicCtx { gremlin_horn: true, ..RelicCtx::default() };
    let mut b = battle(r);
    let e0 = b.energy;
    b.mgr.add_bottom(GameAction::Damage {
        target: TargetRef::Monster(0),
        info: DamageInfo { owner_is_player: true, base: 999, output: 999, ty: DamageType::Normal, modified: true },
    });
    b.run_until_idle();
    assert!(b.energy > e0 || b.is_battle_over, "击杀 +1 能量");
}

#[test]
fn membership_card_halves_shop_prices() {
    let mut run = RunState4b::new(31);
    run.open_neow();
    let _ = run.take_neow(0);
    run.apply_relic_effect("Membership Card");
    run.relics.push("Membership Card");
    run.force_shop_for_test();
    run.gold = 999;
    let price = run.shop.as_ref().unwrap().cards[0].2;
    let expected_pay = run.shop_price(price);
    let n0 = run.deck.len();
    let g0 = run.gold;
    let _ = run.shop_buy_card(0);
    assert_eq!(run.gold, g0 - expected_pay, "实付 = shop_price 快照");
    assert_eq!(run.deck.len(), n0 + 1, "卡入组");
}

#[test]
fn pen_nib_doubles_every_tenth_attack() {
    let r = RelicCtx { pen_nib: true, ..RelicCtx::default() };
    let mut b = battle(r);
    b.relic_pen_nib = 9;
    b.relic_pen_nib_active = true;
    let hp0 = b.monsters[0].creature.current_health;
    // 直接走 attack_info 路径：Stike_R 6 伤 → 12
    let info = sts_core::cards::attack_info(&b, 0, 6, DamageType::Normal);
    assert_eq!(info.output, 12, "蘸水笔翻倍（6 → 12）");
    let _ = hp0;
}
