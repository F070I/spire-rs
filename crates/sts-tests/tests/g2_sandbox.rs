//! G2 机制沙盘（docs/05 验证门 G2 的第一批）：
//! 真实数据表卡牌（Strike_R/Defend_R/Bash）+ 脚本怪，断言
//! 开场序列、打卡链、能量、回合循环、格挡/易伤/毒的完整交互。

use sts_core::action::{GameAction, TargetRef};
use sts_core::battle::{BattleState, CardInst, Monster, RelicCtx};
use sts_core::power::{DamageType, Power, PowerKind};
use sts_data::gen;

fn sid(id: &str) -> usize {
    gen::CARDS.iter().position(|c| c.id == id).expect(id)
}

fn rng(seed: i64) -> sts_data::rng::StsRandom {
    sts_data::rng::StsRandom::new(seed)
}

/// 手动装配的确定性战斗（不走洗牌）。
fn fixture(hand: &[(&str, bool)]) -> BattleState {
    let deck: Vec<(usize, bool)> = hand.iter().map(|(id, up)| (sid(id), *up)).collect();
    let mut b = BattleState::new(80, 3, deck, vec![Monster::scripted("Jaw Worm", 44, vec![Some(10), Some(10), None])], 5);
    b.relics_ctx = RelicCtx::default();
    // 确定性手牌：把 master_deck 全部以新 uuid 装入手牌（绕开洗牌）
    let cards: Vec<CardInst> = b.master_deck.group.iter().map(|c| CardInst { uuid: 0, ..c.clone() }).collect();
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

#[test]
fn battle_start_sequence() {
    let deck: Vec<(usize, bool)> = (0..5).map(|_| (sid("Strike_R"), false)).chain((0..5).map(|_| (sid("Defend_R"), false))).collect();
    let mut b = BattleState::new(80, 3, deck, vec![Monster::scripted("Jaw Worm", 44, vec![Some(10)])], 5);
    b.start_battle();
    b.run_until_idle();
    assert_eq!(b.energy, 3, "开场充能");
    assert_eq!(b.hand.group.len(), 5, "首回合抽 5");
    assert_eq!(b.draw_pile.group.len(), 5);
    assert_eq!(b.mgr.turn, 1);
}

#[test]
fn strike_via_full_play_chain() {
    let mut b = fixture(&[("Strike_R", false), ("Defend_R", false)]);
    let hp0 = b.monsters[0].creature.current_health;
    let i = find_hand(&b, "Strike_R").unwrap();
    b.play_card(i, Some(0)).unwrap();
    assert_eq!(b.energy, 3, "入队瞬间不扣能（记录 09 §1 阶段 A）");
    b.run_until_idle();
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 6);
    assert_eq!(b.energy, 2, "打出后扣能");
    assert_eq!(find_hand(&b, "Strike_R"), None, "离手");
    assert_eq!(b.discard_pile.group.len(), 1, "进弃牌堆");
}

#[test]
fn bash_applies_vulnerable_then_strike_benefit() {
    // Bash：8 伤 + 易伤 2（第一张正式转译的带效果攻击卡）
    let mut b = fixture(&[("Bash", false), ("Strike_R", false)]);
    let hp0 = b.monsters[0].creature.current_health;
    let i = find_hand(&b, "Bash").unwrap();
    b.play_card(i, Some(0)).unwrap();
    b.run_until_idle();
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 8);
    assert_eq!(
        b.monsters[0]
            .creature
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Vulnerable)
            .map(|p| p.amount),
        Some(2)
    );
    // 易伤下打打击：(6)×1.5 = 9
    let i2 = find_hand(&b, "Strike_R").unwrap();
    b.play_card(i2, Some(0)).unwrap();
    b.run_until_idle();
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 8 - 9);
}

#[test]
fn defend_blocks_monster_attack_and_clears_next_turn() {
    let mut b = fixture(&[("Defend_R", false), ("Strike_R", false)]);
    let i = find_hand(&b, "Defend_R").unwrap();
    b.play_card(i, None).unwrap();
    b.run_until_idle();
    assert_eq!(b.player.current_block, 5);
    // 结束回合：怪打 10 → 格挡吃 5 → 穿透 5
    let hp0 = b.player.current_health;
    b.end_turn();
    b.run_until_idle();
    assert_eq!(b.player.current_health, hp0 - 5);
    // 新回合：格挡清零、能量回满、手牌补 5
    assert_eq!(b.player.current_block, 0, "回合开始掉格挡（无 Barricade）");
    assert_eq!(b.energy, 3);
    assert_eq!(b.hand.group.len(), 2, "牌组共 2 张，全部重抽");
    assert_eq!(b.mgr.turn, 2);
}

#[test]
fn turn_loop_two_rounds_and_discard_flow() {
    let mut b = fixture(&[("Strike_R", false); 3]);
    b.game_hand_size = 3;
    b.hand.group.truncate(3);
    let hp0 = b.monsters[0].creature.current_health;
    for t in 0..2 {
        let i = find_hand(&b, "Strike_R").unwrap();
        b.play_card(i, Some(0)).unwrap();
        b.run_until_idle();
        b.end_turn();
        b.run_until_idle();
    }
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 12);
    assert_eq!(b.mgr.turn, 3);
    // 牌循环：弃→洗回→重抽，两回合后弃牌堆清空、手牌回满
    assert_eq!(b.discard_pile.group.len(), 0);
    assert_eq!(b.hand.group.len(), 3);
}

#[test]
fn poison_ticks_at_round_end_hp_loss() {
    let mut b = fixture(&[("Defend_R", false)]);
    // 毒（PoisonPower 语义：atEndOfRound 伤害 + 递减，HP_LOSS 无视格挡）
    b.monsters[0].creature.powers.push(Power::new(PowerKind::Poison, 5));
    let hp0 = b.monsters[0].creature.current_health;
    b.end_turn();
    b.run_until_idle();
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 5, "毒按层数扣血");
    assert_eq!(
        b.monsters[0]
            .creature
            .powers
            .iter()
            .find(|p| p.kind == PowerKind::Poison)
            .map(|p| p.amount),
        Some(4)
    );
    // 玩家未被毒波及（HP_LOSS 不吃格挡——用毒打有格挡的怪验证在下一条）
}

#[test]
fn monster_block_wiped_at_its_turn_then_poison_bypasses() {
    let mut b = fixture(&[("Defend_R", false)]);
    b.monsters[0].creature.add_block(50);
    b.monsters[0].creature.powers.push(Power::new(PowerKind::Poison, 5));
    let hp0 = b.monsters[0].creature.current_health;
    b.end_turn();
    b.run_until_idle();
    // MonsterStartTurn：怪物回合开始掉格挡（applyPreTurnLogic，记录 01 §8）
    assert_eq!(b.monsters[0].creature.current_block, 0);
    // 毒 HP_LOSS 扣血（decrement_block 对 HP_LOSS 的旁路另有单元测试）
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 5);
}

#[test]
fn monster_debuffs_just_applied_skip_first_tick() {
    let mut b = fixture(&[("Defend_R", false)]);
    // 怪施加的易伤（justApplied）：本回合结束不递减
    b.mgr.add_bottom(GameAction::ApplyPower {
        target: TargetRef::Player,
        source: Some(TargetRef::Monster(0)),
        power: Power::new(PowerKind::Vulnerable, 2),
        from_monster: true,
    });
    b.run_until_idle();
    assert_eq!(
        b.player.powers.iter().find(|p| p.kind == PowerKind::Vulnerable).map(|p| p.amount),
        Some(2)
    );
    b.end_turn();
    b.run_until_idle();
    // 本回合结束时 justApplied 跳过 → 仍 2；下回合结束 → 1
    assert_eq!(
        b.player.powers.iter().find(|p| p.kind == PowerKind::Vulnerable).map(|p| p.amount),
        Some(2),
        "justApplied 首跳"
    );
    b.end_turn();
    b.run_until_idle();
    assert_eq!(
        b.player.powers.iter().find(|p| p.kind == PowerKind::Vulnerable).map(|p| p.amount),
        Some(1)
    );
}

#[test]
fn player_strength_and_weak_on_attacker() {
    let mut b = fixture(&[("Strike_R", false)]);
    // 力量在玩家（give 段加法），虚弱在玩家（give 段 ×0.75）——怪身上的 Weak 不影响被打
    b.player
        .powers
        .push(Power::new(PowerKind::Strength, 3));
    b.player
        .powers
        .push(Power::new(PowerKind::Weak, 1));
    let hp0 = b.monsters[0].creature.current_health;
    let i = find_hand(&b, "Strike_R").unwrap();
    b.play_card(i, Some(0)).unwrap();
    b.run_until_idle();
    // (6+3)×0.75 = 6.75 → 6
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 6);
}

#[test]
fn wrath_stance_doubles_outgoing() {
    let mut b = fixture(&[("Strike_R", false)]);
    b.player_stance = sts_core::stance::StanceKind::Wrath;
    let hp0 = b.monsters[0].creature.current_health;
    let i = find_hand(&b, "Strike_R").unwrap();
    b.play_card(i, Some(0)).unwrap();
    b.run_until_idle();
    assert_eq!(b.monsters[0].creature.current_health, hp0 - 12);
}

#[test]
fn battle_over_when_monster_dies() {
    let mut b = fixture(&[("Strike_R", false)]);
    b.monsters[0].creature.current_health = 5;
    let i = find_hand(&b, "Strike_R").unwrap();
    b.play_card(i, Some(0)).unwrap();
    b.run_until_idle();
    assert!(b.is_battle_over, "怪死亡 → 战斗结束");
    assert!(b.monsters[0].creature.is_dying);
    // 死亡后额外伤害动作被 clearPostCombat 保留白名单放行（此处验证状态不再推进）
    let _ = b.tick();
    assert!(b.is_battle_over);
    let _ = DamageType::Normal; // 引用保持导入
}
