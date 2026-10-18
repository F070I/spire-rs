//! 蓝卡测试（批次 3.4）：充能球机制（通道/引动/Focus/球槽）、ID 错位卡、计数器。

use sts_core::battle::{BattleState, CardInst, Monster};
use sts_core::orb::OrbKind;
use sts_core::power::PowerKind;
use sts_data::gen;

fn sid(id: &str) -> usize {
    gen::CARDS.iter().position(|c| c.id == id).expect(id)
}

fn fixture(hand: &[&str]) -> BattleState {
    let deck: Vec<(usize, bool)> = hand.iter().map(|id| (sid(id), false)).collect();
    let mut b = BattleState::new(
        75,
        3,
        deck,
        vec![Monster::scripted("Jaw Worm", 90, vec![Some(10)])],
        5,
    );
    let cards: Vec<CardInst> = b
        .master_deck
        .group
        .iter()
        .map(|c| CardInst {
            uuid: 0,
            ..c.clone()
        })
        .collect();
    b.master_deck.group.clear();
    for mut c in cards {
        c.uuid = b.uuid_next;
        b.uuid_next += 1;
        b.hand.add_top(c);
    }
    b.max_orbs = 3; // 故障机器人基础球槽（preBattlePrep 等价）
    b.energy = 3;
    b
}

fn find_hand(b: &BattleState, id: &str) -> Option<usize> {
    b.hand.group.iter().position(|c| c.st().id == id)
}

fn play(b: &mut BattleState, id: &str, target: Option<usize>) {
    b.energy = b.energy.max(9);
    let i = find_hand(b, id).expect(id);
    b.play_card(i, target).unwrap();
    b.run_until_idle();
}

fn orb_counts(b: &BattleState) -> (i32, i32, i32, i32) {
    let mut l = 0;
    let mut f = 0;
    let mut d = 0;
    let mut p = 0;
    for o in &b.orbs {
        match o.kind {
            OrbKind::Lightning => l += 1,
            OrbKind::Frost => f += 1,
            OrbKind::Dark => d += 1,
            OrbKind::Plasma => p += 1,
            OrbKind::EmptySlot => {}
        }
    }
    (l, f, d, p)
}

#[test]
fn zap_channels_lightning() {
    let mut b = fixture(&["Zap"]);
    play(&mut b, "Zap", None);
    assert_eq!(orb_counts(&b).0, 1);
    // Zap 产出闪电牌机制 TODO 3.8（此处验证球）
}

#[test]
fn dualcast_evoke_without_remove_then_evoke() {
    let mut b = fixture(&["Zap", "Dualcast"]);
    play(&mut b, "Zap", None); // 1 闪电
    let hp0 = b.monsters[0].creature.current_health;
    play(&mut b, "Dualcast", None);
    // 双施法：第一引动不移除（球还在）+ 第二引动移除
    let (l, _f, _d, _p) = orb_counts(&b);
    assert_eq!(l, 0, "第二次引动移除闪电");
    assert!(b.monsters[0].creature.current_health < hp0, "引动伤害发生");
}

#[test]
fn capacitor_and_consume_slots() {
    let mut b = fixture(&["Capacitor", "Consume"]);
    play(&mut b, "Capacitor", None);
    assert_eq!(b.max_orbs, 5, "基础 3 + Capacitor 2");
    play(&mut b, "Consume", None);
    assert_eq!(b.max_orbs, 4);
}

#[test]
fn focus_modifies_channelled_orb() {
    let mut b = fixture(&["Defragment", "Zap"]);
    play(&mut b, "Defragment", None); // Focus +2
    play(&mut b, "Zap", None);
    let magic = gen::CARDS[sid("Defragment")].base_magic;
    // 闪电被动 3+Focus
    assert_eq!(b.orbs[0].passive_amount, 3 + magic);
    assert_eq!(b.orbs[0].evoke_amount, 8 + magic);
}

#[test]
fn hyperbeam_focus_loss() {
    let mut b = fixture(&["Hyperbeam", "Defragment"]);
    play(&mut b, "Defragment", None);
    let focus0 = b
        .player
        .powers
        .iter()
        .find(|p| p.kind == PowerKind::Focus)
        .map(|p| p.amount);
    let hp0 = b.monsters[0].creature.current_health;
    play(&mut b, "Hyperbeam", Some(0));
    // 全体 26 伤 + Focus -3
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 26);
    let focus1 = b
        .player
        .powers
        .iter()
        .find(|p| p.kind == PowerKind::Focus)
        .map(|p| p.amount);
    assert_eq!(focus1, focus0.map(|f| f - 3));
}

#[test]
fn machine_learning_increases_hand_size() {
    let mut b = fixture(&["Machine Learning"]);
    let hs0 = b.game_hand_size;
    let magic = gen::CARDS[sid("Machine Learning")].base_magic;
    play(&mut b, "Machine Learning", None);
    assert_eq!(b.game_hand_size, hs0 + magic);
    // 移除还原（回合结束等路径）——直接移除验证配对
    b.mgr.add_bottom(sts_core::action::GameAction::RemovePower {
        target: sts_core::action::TargetRef::Player,
        kind: PowerKind::MachineLearning,
    });
    b.run_until_idle();
    assert_eq!(b.game_hand_size, hs0, "DrawPower.onRemove 还原");
}

#[test]
fn thunder_strike_scales_with_lightning() {
    let mut b = fixture(&["Zap", "Ball Lightning", "Thunder Strike"]);
    play(&mut b, "Zap", None); // 1 闪电
    play(&mut b, "Ball Lightning", Some(0)); // 又 1 闪电 + 伤害
    let hp1 = b.monsters[0].creature.current_health;
    let ts = gen::CARDS[sid("Thunder Strike")].base_damage;
    let expected_ts = ts * 2;
    play(&mut b, "Thunder Strike", Some(0));
    assert_eq!(b.monsters[0].creature.current_health, hp1 - expected_ts);
}

#[test]
fn melt_strips_block_then_hits() {
    let mut b = fixture(&["Melter"]);
    b.monsters[0].creature.add_block(50);
    let hp0 = b.monsters[0].creature.current_health;
    let expected = gen::CARDS[sid("Melter")].base_damage;
    play(&mut b, "Melter", Some(0));
    assert_eq!(
        b.monsters[0].creature.current_health,
        hp0 - expected,
        "清格挡后全额命中"
    );
    assert_eq!(b.monsters[0].creature.current_block, 0);
}

#[test]
fn consume_reduces_slots_negatively() {
    let mut b = fixture(&["Consume"]);
    b.max_orbs = 3;
    b.orbs = vec![
        sts_core::orb::Orb {
            kind: OrbKind::Lightning,
            passive_amount: 3,
            evoke_amount: 8,
        },
        sts_core::orb::Orb {
            kind: OrbKind::Frost,
            passive_amount: 2,
            evoke_amount: 5,
        },
        sts_core::orb::Orb {
            kind: OrbKind::Plasma,
            passive_amount: 1,
            evoke_amount: 2,
        },
    ];
    play(&mut b, "Consume", None);
    assert_eq!(b.max_orbs, 2);
    assert_eq!(b.orbs.len(), 2, "最右球槽（含球）移除");
}

#[test]
fn id_contract_blue_mismatches() {
    // ID↔类名错位确认（批次 3.4 实录）
    assert_eq!(gen::CARDS[sid("Gash")].class, "Claw");
    assert_eq!(gen::CARDS[sid("Undo")].class, "Equilibrium");
    assert_eq!(gen::CARDS[sid("Redo")].class, "Recursion");
    assert_eq!(gen::CARDS[sid("Steam Power")].class, "Overclock");
    assert_eq!(gen::CARDS[sid("Defend_B")].class, "Defend_Blue");
    assert_eq!(gen::CARDS[sid("Night Terror")].color_dir, "green"); // Nightmare
}
