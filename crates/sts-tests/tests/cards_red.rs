//! 红卡批次测试（批次 3.2 试点）：15 张新转译卡的行为断言。
//! 测试模式 = 手动装配手牌（不走洗牌），与 g2_sandbox 同构。

use sts_core::battle::{BattleState, CardInst, Monster};
use sts_core::power::PowerKind;
use sts_data::gen;

fn sid(id: &str) -> usize {
    gen::CARDS.iter().position(|c| c.id == id).expect(id)
}

fn fixture(hand: &[&str]) -> BattleState {
    let deck: Vec<(usize, bool)> = hand.iter().map(|id| (sid(id), false)).collect();
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

fn fixture2(hand: &[(&str, bool)]) -> BattleState {
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

fn play(b: &mut BattleState, id: &str, target: Option<usize>) {
    let i = find_hand(b, id).expect(id);
    b.play_card(i, target).unwrap();
    b.run_until_idle();
}

#[test]
fn anger_deals_and_copies_to_discard() {
    let mut b = fixture(&["Anger"]);
    let hp0 = b.monsters[0].creature.current_health;
    play(&mut b, "Anger", Some(0));
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 6);
    // makeStatEquivalentCopy 进弃牌堆（Anger 循环机制）
    assert!(b.discard_pile.group.iter().any(|c| c.st().id == "Anger"));
}

#[test]
fn bloodletting_lose_hp_gain_energy() {
    let mut b = fixture(&["Bloodletting"]);
    play(&mut b, "Bloodletting", None);
    assert_eq!(b.player.current_health, 80 - 3);
    let cost = gen::CARDS[sid("Bloodletting")].cost;
    assert_eq!(b.energy, 3 - cost + 2, "扣费 + 回 2 能量");
}

#[test]
fn flex_strength_then_end_of_turn_loss() {
    let mut b = fixture(&["Flex"]);
    play(&mut b, "Flex", None);
    assert_eq!(
        b.player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Strength)
            .map(|p| p.amount),
        Some(2)
    );
    b.end_turn();
    b.run_until_idle();
    // 回合末：LoseStrength（原版 ID "Flex"）收回力量；Strength 归 0 → 移除
    assert!(b
        .player
        .powers
        .iter()
        .all(|p| p.kind != PowerKind::Strength));
}

#[test]
fn inflame_is_permanent_power() {
    let mut b = fixture(&["Inflame"]);
    play(&mut b, "Inflame", None);
    assert_eq!(
        b.player
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Strength)
            .map(|p| p.amount),
        Some(2)
    );
    // POWER 卡不入任何牌堆
    assert!(b.discard_pile.group.is_empty() && b.exhaust_pile.group.is_empty());
}

#[test]
fn dropkick_energy_draw_vs_vulnerable() {
    // 无易伤：仅伤害
    let mut b = fixture(&["Dropkick"]);
    let hp0 = b.monsters[0].creature.current_health;
    let hand0 = b.hand.group.len();
    play(&mut b, "Dropkick", Some(0));
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 5);
    assert_eq!(b.energy, 3 - 1, "无易伤不回能");
    assert_eq!(b.hand.group.len(), hand0 - 1);
    // 有易伤：+1 能量 +1 抽牌
    let mut b2 = fixture(&["Dropkick", "Bash"]);
    b2.monsters[0]
        .creature
        .powers
        .push(sts_core::power::Power::new(PowerKind::Vulnerable, 1));
    let hand1 = b2.hand.group.len();
    play(&mut b2, "Dropkick", Some(0));
    assert_eq!(b2.energy, 3, "3 - 1 费 + 1 回能");
    assert_eq!(b2.hand.group.len(), hand1, "打 1 抽 1");
}

#[test]
fn whirlwind_x_cost_multi_hits() {
    let mut b = fixture(&["Whirlwind"]);
    b.energy = 3;
    let hp0 = b.monsters[0].creature.current_health;
    play(&mut b, "Whirlwind", Some(0));
    assert_eq!(b.energy, 0);
    assert_eq!(
        b.monsters[0].creature.current_health,
        hp0 - 15,
        "3 点能量 × 5 伤"
    );
}

#[test]
fn rampage_grows_via_misc() {
    let mut b = fixture2(&[("Rampage", false)]);
    let hp0 = b.monsters[0].creature.current_health;
    play(&mut b, "Rampage", Some(0));
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 8);
    // 第二次（卡已循环回手？在弃牌堆）——misc +3 已记录
    let rampage_misc = b
        .discard_pile
        .group
        .iter()
        .find(|c| c.st().id == "Rampage")
        .map(|c| c.misc);
    assert_eq!(rampage_misc, Some(5));
}

#[test]
fn true_grit_block_and_exhaust() {
    let mut b = fixture(&["True Grit", "Strike_R"]);
    let hand0 = b.hand.group.len();
    play(&mut b, "True Grit", None);
    assert_eq!(b.player.current_block, 7, "源码 baseBlock=7");
    assert_eq!(
        b.hand.group.len(),
        hand0 - 2,
        "打出的卡 + 随机消耗占位（取顶）"
    );
    assert_eq!(b.exhaust_pile.group.len(), 1);
}

#[test]
fn armaments_upgraded_upgrades_all_hand() {
    let mut b = fixture2(&[
        ("Armaments", true),
        ("Strike_R", false),
        ("Defend_R", false),
    ]);
    play(&mut b, "Armaments", None);
    assert_eq!(b.player.current_block, 5);
    // Armaments+ 不升级自己（POWER? Armaments 是 SKILL 且已升级不可再升）——其余手牌全升级
    let strike = b
        .hand
        .group
        .iter()
        .find(|c| c.st().id == "Strike_R")
        .unwrap();
    let defend = b
        .hand
        .group
        .iter()
        .find(|c| c.st().id == "Defend_R")
        .unwrap();
    assert!(strike.upgraded && strike.eff_damage() == 9);
    assert!(defend.upgraded && defend.eff_block() == 8);
}

#[test]
fn intimidate_weak_all_and_cleave_damage_all() {
    let mut b = fixture(&["Intimidate", "Cleave", "Strike_R"]);
    b.monsters
        .push(Monster::scripted("Cultist", 50, vec![None]));
    play(&mut b, "Intimidate", None);
    for m in &b.monsters {
        assert_eq!(
            m.creature
                .powers
                .iter()
                .find(|p| p.kind == PowerKind::Weak)
                .map(|p| p.amount),
            Some(1)
        );
    }
    let hp0: Vec<i32> = b
        .monsters
        .iter()
        .map(|m| m.creature.current_health)
        .collect();
    play(&mut b, "Cleave", None);
    for (i, m) in b.monsters.iter().enumerate() {
        // 怪身上的 Weak 只减它自己的攻击（give 段）；被打全额
        let expected = hp0[i] - 8;
        assert_eq!(m.creature.current_health, expected, "monster {i}");
    }
}

#[test]
fn blood_for_blood_cost_reduces_on_exhaust() {
    let mut b = fixture(&["Blood for Blood", "Strike_R", "True Grit"]); // 占位消耗取手牌顶=Strike_R
    assert_eq!(
        b.hand
            .group
            .iter()
            .find(|c| c.st().id == "Blood for Blood")
            .unwrap()
            .cost_for_turn,
        4
    );
    // True Grit 随机消耗触发 B4B 降费
    play(&mut b, "True Grit", None);
    let b4b = b
        .hand
        .group
        .iter()
        .find(|c| c.st().id == "Blood for Blood")
        .expect("B4B in hand");
    assert_eq!(b4b.cost_for_turn, 3, "每次消耗 -1");
}

#[test]
fn burning_pact_exhaust_then_draw() {
    let mut b = fixture(&["Burning Pact", "Strike_R"]);
    let hand0 = b.hand.group.len();
    play(&mut b, "Burning Pact", None);
    // 消耗 1（占位取手牌第一张可消耗对象）+ 抽 1
    assert_eq!(b.exhaust_pile.group.len(), 1);
    assert_eq!(b.hand.group.len(), hand0 - 1 - 1 + 1);
}

#[test]
fn rage_and_intimidate_use_magic_from_upgrade_data() {
    // Rage+ / Intimidate+ 的 magic 升级 delta 抽取验证
    let rage = &gen::CARDS[sid("Rage")];
    let intim = &gen::CARDS[sid("Intimidate")];
    assert!(rage.up_magic > 0 && intim.up_magic > 0);
}
