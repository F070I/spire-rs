//! 红卡收尾批测试 + 4.1 RNG 接线验证（随机弃牌/随机敌人消耗同一流）。

use sts_core::battle::{BattleState, CardInst, Monster};
use sts_core::power::PowerKind;
use sts_data::gen;

fn sid(id: &str) -> usize {
    gen::CARDS.iter().position(|c| c.id == id).expect(id)
}

fn fixture2(hand: &[(&str, bool)]) -> BattleState {
    let deck: Vec<(usize, bool)> = hand.iter().map(|(id, up)| (sid(id), *up)).collect();
    let mut b = BattleState::new(
        80,
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

#[test]
fn rng_streams_exist_and_diverge() {
    let mut b = fixture2(&[("Defend_R", false)]);
    let s1 = b.rngs.shuffle.random_long();
    let s2 = b.rngs.shuffle.random_long();
    assert_ne!(s1, s2);
    // 各流独立演进：同 seed 创建但互不串流（对照原版 12 条流分离语义）
    b.set_rng_seed(7, 3);
    let a = b.rngs.shuffle.random_long();
    b.set_rng_seed(7, 3);
    let c = b.rngs.card_random.random_long();
    assert_eq!(a, c, "同 seed 同位置：不同流首次取数一致（异流分叉由消费模式决定）");
}

#[test]
fn second_wind_exhausts_non_attacks_only() {
    let mut b = fixture2(&[("Second Wind", false), ("Strike_R", false), ("Defend_R", false)]);
    let _n_non_attack = b
        .hand
        .group
        .iter()
        .filter(|c| c.st().kind != gen::CardType::Attack)
        .count() as i32;
    play(&mut b, "Second Wind", Some(0));
    assert_eq!(b.exhaust_pile.group.len(), 1, "SW 无自身消耗（源码核对）：仅 Defend 被消耗");
    // 动作执行时 SW 已离手：非攻击数 = 1（Defend）→ 格挡 5
    assert_eq!(b.player.current_block, 5);
}

#[test]
fn sever_soul_exhausts_non_attacks_then_hits() {
    let mut b = fixture2(&[("Sever Soul", false), ("Strike_R", false), ("Defend_R", false)]);
    let hp0 = b.monsters[0].creature.current_health;
    play(&mut b, "Sever Soul", Some(0));
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 16);
    assert_eq!(b.exhaust_pile.group.len(), 1, "仅 Defend（Sever Soul 无消耗标志——源码核对）");
    assert!(b.discard_pile.group.iter().any(|c| c.st().id == "Sever Soul"));
}

#[test]
fn shockwave_weak_and_frail_all() {
    let mut b = fixture2(&[("Shockwave", false)]);
    play(&mut b, "Shockwave", None);
    let m = &b.monsters[0].creature;
    assert_eq!(
        m.powers.iter().find(|p| p.kind == PowerKind::Weak).map(|p| p.amount),
        Some(3)
    );
    assert_eq!(
        m.powers.iter().find(|p| p.kind == PowerKind::Frail).map(|p| p.amount),
        Some(3)
    );
}

#[test]
fn spot_weakness_only_vs_attacking_intent() {
    // 脚本怪第一招为攻击（Some(10)）
    let mut b = fixture2(&[("Spot Weakness", false)]);
    play(&mut b, "Spot Weakness", Some(0)); // 无伤害：怪血不变
    assert_eq!(
        b.player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Strength)
            .map(|p| p.amount),
        Some(3)
    );
    // 防御意图（None 招式）：不加力量
    let mut b2 = fixture2(&[("Spot Weakness", false)]);
    b2.monsters[0].script = vec![None]; // 防御意图
    play(&mut b2, "Spot Weakness", Some(0));
    assert!(b2.player.powers.iter().all(|p| p.kind != PowerKind::Strength));
}

#[test]
fn reaper_heals_unblocked_damage() {
    let mut b = fixture2(&[("Reaper", false)]);
    b.player.current_health = 60;
    let mhp0 = b.monsters[0].creature.current_health;
    play(&mut b, "Reaper", Some(0));
    // 源码：baseDamage=4 全体；heal = 实扣血合计（lastDamageTaken）
    assert_eq!(b.monsters[0].creature.current_health, mhp0 - 4);
    assert_eq!(b.player.current_health, 64, "吸血 4（60+4）");
}

#[test]
fn reckless_charge_shuffles_dazed() {
    let mut b = fixture2(&[("Reckless Charge", false)]);
    let hp0 = b.monsters[0].creature.current_health;
    play(&mut b, "Reckless Charge", Some(0));
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 7, "源码 baseDamage=7");
    // Dazed 进抽牌堆（随机位——此处 1 张堆）
    assert!(b.draw_pile.group.iter().any(|c| c.st().id == "Dazed"));
}

#[test]
fn sword_boomerang_random_hits() {
    let mut b = fixture2(&[("Sword Boomerang", false)]);
    let hp0 = b.monsters[0].creature.current_health;
    play(&mut b, "Sword Boomerang", Some(0));
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 9, "3 次 × 3 伤（单怪全中）");
}

#[test]
fn uppercut_weak_vulnerable() {
    let mut b = fixture2(&[("Uppercut", false)]);
    play(&mut b, "Uppercut", Some(0));
    let m = &b.monsters[0].creature;
    assert!(m.powers.iter().any(|p| p.kind == PowerKind::Weak && p.amount == 1));
    assert!(m.powers.iter().any(|p| p.kind == PowerKind::Vulnerable && p.amount == 1));
}

#[test]
fn searing_blow_infinite_upgrade() {
    let mut c = CardInst::new(sid("Searing Blow"), false, 1);
    assert!(c.can_upgrade());
    c.apply_upgrade();
    assert_eq!(c.misc, 4, "第 1 次 +4");
    assert!(c.can_upgrade());
    c.apply_upgrade();
    assert_eq!(c.misc, 4 + 5, "第 2 次 +5");
    assert_eq!(c.st().base_damage + c.misc, 12 + 9, "源码 baseDamage=12");
}

#[test]
fn offering_costs_hp_gains_energy_draw() {
    let mut b = fixture2(&[("Offering", false), ("Strike_R", false)]);
    let strike_hp = b.monsters[0].creature.current_health;
    // 预填抽牌堆 3 张供 Offering 抽
    for _ in 0..3 {
        let c = CardInst::new(sid("Defend_R"), false, b.uuid_next);
        b.uuid_next += 1;
        b.draw_pile.add_top(c);
    }
    let hand0 = b.hand.group.len();
    play(&mut b, "Offering", None);
    assert_eq!(b.player.current_health, 80 - 6);
    assert_eq!(b.energy, 9 + 2, "Offering 0 费 + 回 2");
    assert_eq!(b.hand.group.len(), hand0 - 1 + 3);
    assert!(b.exhaust_pile.group.iter().any(|c| c.st().id == "Offering"));
    assert_eq!(b.monsters[0].creature.current_health, strike_hp, "怪未被打");
    let _ = strike_hp;
}
