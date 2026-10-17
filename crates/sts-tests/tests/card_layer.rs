//! 批次 2.4 卡牌机制层测试：升级数值（机械抽取）、canUse 链（状态/诅咒/能量）、
//! X 费快照、虚无回合末消耗、自保留回手。

use sts_core::battle::{BattleState, CardInst, Monster};
use sts_data::gen;

fn sid(id: &str) -> usize {
    gen::CARDS.iter().position(|c| c.id == id).expect(id)
}

fn fixture(hand: &[(&str, bool)]) -> BattleState {
    let deck: Vec<(usize, bool)> = hand.iter().map(|(id, up)| (sid(id), *up)).collect();
    let mut b = BattleState::new(
        80,
        3,
        deck,
        vec![Monster::scripted("Jaw Worm", 60, vec![Some(10)])],
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
    b.energy = 3;
    b
}

fn find_hand(b: &BattleState, id: &str) -> Option<usize> {
    b.hand.group.iter().position(|c| c.st().id == id)
}

// ------------------------------------------------------------ 升级数值

#[test]
fn upgrade_deltas_extracted_mechanically() {
    // 出处：cards/*/X.java upgrade() 方法体（extract.py 机械抽取）
    let strike = &gen::CARDS[sid("Strike_R")];
    assert_eq!((strike.up_damage, strike.up_flags & gen::UPF_DMG != 0), (3, true));
    let defend = &gen::CARDS[sid("Defend_R")];
    assert_eq!((defend.up_block, defend.up_flags & gen::UPF_BLOCK != 0), (3, true));
    let bash = &gen::CARDS[sid("Bash")];
    assert_eq!(bash.up_damage, 2);
    assert_eq!(bash.up_magic, 1);
    // upgradeBaseCost（绝对值）：Corruption 3→2、Eruption 2→1
    assert_eq!(gen::CARDS[sid("Corruption")].up_cost, 2);
    assert_eq!(gen::CARDS[sid("Eruption")].up_cost, 1);
    // SearingBlow（4 + timesUpgraded）必须 custom
    assert!(gen::CARDS[sid("Searing Blow")].custom_upgrade);
    // 状态/诅咒：空 upgrade()
    assert!(!gen::CARDS[sid("Wound")].custom_upgrade);
}

#[test]
fn upgraded_card_uses_effective_values() {
    let strike = CardInst::new(sid("Strike_R"), true, 1);
    assert_eq!((strike.eff_damage(), strike.eff_cost()), (6 + 3, 1));
    let bash = CardInst::new(sid("Bash"), true, 2);
    assert_eq!((bash.eff_damage(), bash.eff_magic()), (8 + 2, 2 + 1));
    // 升级费用卡：Eruption+ = 1 费
    let eruption = CardInst::new(sid("Eruption"), true, 3);
    assert_eq!(eruption.eff_cost(), 1);
    assert_eq!(eruption.cost_for_turn, 1);
    // resetAttributes 回到生效值
    let mut b2 = eruption.clone();
    b2.cost_for_turn = 99;
    b2.reset_attributes();
    assert_eq!(b2.cost_for_turn, 1);
}

#[test]
fn upgraded_strike_and_bash_in_battle() {
    let mut b = fixture(&[("Strike_R", true), ("Bash", true)]);
    let hp0 = b.monsters[0].creature.current_health;
    let i = find_hand(&b, "Bash").unwrap();
    b.play_card(i, Some(0)).unwrap();
    b.run_until_idle();
    // Bash+：10 伤 + 易伤 3
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 10);
    assert_eq!(
        b.monsters[0]
            .creature
            .powers
            .iter()
            .find(|p| p.kind == vulnerable_kind())
            .map(|p| p.amount),
        Some(3)
    );
    // Strike+：9×1.5 = 13.5 → 13
    let i2 = find_hand(&b, "Strike_R").unwrap();
    b.play_card(i2, Some(0)).unwrap();
    b.run_until_idle();
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 10 - 13);
}

fn vulnerable_kind() -> sts_core::power::PowerKind {
    sts_core::power::PowerKind::Vulnerable
}

// ------------------------------------------------------------ canUse 链

#[test]
fn status_unplayable_without_medical_kit() {
    let mut b = fixture(&[("Wound", false)]);
    let i = find_hand(&b, "Wound").unwrap();
    assert!(b.play_card(i, None).is_err(), "无医疗包：状态牌不可打出");
    b.relics_ctx.has_medical_kit = true;
    let i = find_hand(&b, "Wound").unwrap();
    assert!(b.play_card(i, None).is_ok());
    b.run_until_idle();
    // Wound：消耗型状态 → 消耗堆（静态 exhaust 标志）
    assert!(find_hand(&b, "Wound").is_none());
}

#[test]
fn curse_unplayable_without_blue_candle() {
    let mut b = fixture(&[("Regret", false), ("Defend_R", false)]);
    let i = find_hand(&b, "Regret").unwrap();
    assert!(b.play_card(i, None).is_err(), "无蓝蜡烛：诅咒不可打出");
    b.relics_ctx.has_blue_candle = true;
    let i = find_hand(&b, "Regret").unwrap();
    assert!(b.play_card(i, None).is_ok());
}

#[test]
fn x_cost_snapshot_and_zero_energy_pass() {
    let mut b = fixture(&[("Whirlwind", false)]);
    b.energy = 2;
    let i = find_hand(&b, "Whirlwind").unwrap();
    // X 费：能量校验恒过（cost=-1），energyOnUse 快照 2；WhirlwindAction 消耗全部快照能量
    assert!(b.play_card(i, Some(0)).is_ok());
    b.run_until_idle();
    assert_eq!(b.energy, 0, "X 费卡由其动作本体扣能");
    // 2 点能量 → 2 次全体伤害（baseDamage 5，怪 60 血）
    assert_eq!(b.monsters[0].creature.current_health, 60 - 10);
}

#[test]
fn energy_gate_blocks_expensive_card() {
    let mut b = fixture(&[("Bash", false)]);
    b.energy = 1;
    let i = find_hand(&b, "Bash").unwrap();
    assert_eq!(
        b.play_card(i, Some(0)).err(),
        Some("没有足够的能量")
    );
}

// ------------------------------------------------------------ 回合末：虚无/保留

#[test]
fn ethereal_exhausts_at_end_of_turn() {
    let mut b = fixture(&[("Dazed", false), ("Defend_R", false)]);
    b.game_hand_size = 0; // 隔离：新回合不抽牌，断言回合末状态本身
    b.end_turn();
    b.run_until_idle();
    // Dazed（虚无）→ 消耗堆；Defend → 弃牌堆
    assert!(b.exhaust_pile.group.iter().any(|c| c.st().id == "Dazed"));
    assert!(b.discard_pile.group.iter().any(|c| c.st().id == "Defend_R"));
    assert!(find_hand(&b, "Dazed").is_none());
}

#[test]
fn self_retain_returns_to_hand() {
    let mut b = fixture(&[("Perseverance", false), ("Defend_R", false)]);
    b.game_hand_size = 0; // 隔离：新回合不抽牌
    b.end_turn();
    b.run_until_idle();
    // Perseverance（selfRetain）留在手牌；Defend 弃牌
    assert!(find_hand(&b, "Perseverance").is_some(), "自保留卡回手");
    assert!(b.discard_pile.group.iter().any(|c| c.st().id == "Defend_R"));
    assert!(b.limbo.group.is_empty());
}

#[test]
fn runic_pyramid_skips_end_of_turn_discard() {
    let mut b = fixture(&[("Strike_R", false), ("Defend_R", false)]);
    b.relics_ctx.has_runic_pyramid = true;
    b.end_turn();
    b.run_until_idle();
    assert_eq!(b.hand.group.len(), 2, "拉尼卡金字塔：回合末不弃牌");
    assert!(b.discard_pile.group.is_empty());
}
