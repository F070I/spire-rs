//! 批次 4c 真怪状态机测试：Hexaghost 六火 / Orb Walker / Donu & Deca /
//! Awakened One 双形态 + 灼伤自动打出 / 虚空抽牌扣能。
//!
//! 数据对照 monsters/{exordium,beyond}/*.java（转译原版 takeTurn/getMove/
//! changeState/usePreBattleAction），数值断言以升阶 0 为基线。

use sts_core::action::{GameAction, TargetRef};
use sts_core::battle::{BattleState, Brain, CardInst, Monster, RelicCtx, SpecialAi};
use sts_core::damage::DamageInfo;
use sts_core::power::{DamageType, PowerKind};
use sts_data::gen;

fn sid(id: &str) -> usize {
    gen::CARDS.iter().position(|c| c.id == id).expect(id)
}

/// 真怪战斗装配：升阶 0，牌组 10 张打击，玩家 HP 自定。
fn fixture_real(id: &str, player_hp: i32) -> BattleState {
    let monsters = sts_core::monsters::build_real(id, 0, 93).expect(id);
    let deck: Vec<(usize, bool)> = (0..10).map(|_| (sid("Strike_R"), false)).collect();
    let mut b = BattleState::new(player_hp, 3, deck, monsters, 5);
    b.relics_ctx = RelicCtx::default();
    b.start_battle();
    b.setup_real_monsters();
    b.run_until_idle();
    b
}

/// 全堆计数（抽/弃/手/消耗——状态牌会被抽进手牌，单堆断言会漏）。
fn count_card(b: &BattleState, id: &str) -> usize {
    b.draw_pile
        .group
        .iter()
        .chain(b.discard_pile.group.iter())
        .chain(b.hand.group.iter())
        .chain(b.exhaust_pile.group.iter())
        .filter(|c| c.st().id == id)
        .count()
}

fn burn_count_all_upgraded(b: &BattleState) -> (usize, bool) {
    let burn_sid = sid("Burn");
    let burns: Vec<&CardInst> = b
        .discard_pile
        .group
        .iter()
        .chain(b.draw_pile.group.iter())
        .chain(b.hand.group.iter())
        .filter(|c| c.sid == burn_sid)
        .collect();
    let n = burns.len();
    let all_up = burns.iter().all(|c| c.upgraded);
    (n, all_up)
}

// ---------------------------------------------------------------- Hexaghost

#[test]
fn hexaghost_activate_then_divider() {
    let mut b = fixture_real("Hexaghost", 300);
    // 首招 ACTIVATE（六火全燃）
    assert_eq!(b.monsters[0].next_byte, 5);
    b.end_turn();
    b.run_until_idle();
    let m = &b.monsters[0];
    assert_eq!(m.brain.orb_count, 6, "六火全燃");
    // DIVIDER 每段伤 = 玩家当前 HP/12+1（300 → 26）
    assert_eq!(m.brain.divider_base, 300 / 12 + 1);
    assert_eq!(m.next_byte, 1, "ACTIVATE 后必接 DIVIDER");
    let hp0 = b.player.current_health;
    b.end_turn();
    b.run_until_idle();
    assert_eq!(
        hp0 - b.player.current_health,
        6 * (300 / 12 + 1),
        "DIVIDER 6 段定格伤"
    );
    // DIVIDER 后六火全灭 → 下一招 SEAR
    assert_eq!(b.monsters[0].brain.orb_count, 0);
    assert_eq!(b.monsters[0].next_byte, 4);
}

#[test]
fn hexaghost_full_burn_cycle() {
    let mut b = fixture_real("Hexaghost", 400);
    b.end_turn();
    b.run_until_idle(); // ACTIVATE
    b.end_turn();
    b.run_until_idle(); // DIVIDER
                        // 六火轮转：SEAR→TACKLE→SEAR→INFLAME→TACKLE→SEAR→INFERNO（7 回合）
    let expected = [4, 2, 4, 3, 2, 4, 6];
    for (t, byte) in expected.iter().enumerate() {
        assert_eq!(b.monsters[0].next_byte, *byte, "轮转第 {t} 招");
        let before = b.monsters[0].brain.orb_count;
        b.end_turn();
        b.run_until_idle();
        let after = b.monsters[0].brain.orb_count;
        if *byte == 6 {
            assert_eq!(after, 0, "INFERNO 后六火全灭");
        } else if *byte == 4 && before == 5 {
            assert_eq!(after, 6);
        } else if *byte == 3 {
            assert_eq!(after, before + 1, "INFLAME 点燃 1 火");
        }
    }
    // INFERNO 后：全场灼伤升级 + 3 张升级灼伤（SEAR×3 + 3 = 6）
    let (n, all_up) = burn_count_all_upgraded(&b);
    assert_eq!(n, 6, "SEAR×3（升级）+ INFERNO 3 张");
    assert!(all_up, "全部灼伤为升级版");
    assert!(b.monsters[0].brain.burn_up);
    // INFLAME 已给力量 +2
    assert!(b.monsters[0]
        .creature
        .powers
        .iter()
        .any(|p| p.kind == PowerKind::Strength && p.amount >= 2));
}

// ---------------------------------------------------------------- Orb Walker

#[test]
fn orb_walker_laser_adds_burns_and_strength() {
    let mut b = fixture_real("Orb Walker", 80);
    assert!(
        (90..=96).contains(&b.monsters[0].creature.max_health),
        "升阶 0：90-96 表内 roll"
    );
    b.monsters[0].next_byte = 1; // 强制 LASER
    b.end_turn();
    b.run_until_idle();
    // 灼伤×2：弃牌堆 1 + 抽牌堆 1（MakeTempCardInDiscardAndDeck）
    assert_eq!(count_card(&b, "Burn"), 2, "灼伤×2：弃牌堆+抽牌堆各 1");
    // GenericStrengthUpPower：回合末 +3 力
    assert!(
        b.monsters[0]
            .creature
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Strength && p.amount >= 3),
        "回合末力量成长"
    );
}

// ---------------------------------------------------------------- Donu & Deca

#[test]
fn donu_deca_phases_and_buffs() {
    let mut b = fixture_real("Donu & Deca", 300);
    // 开局 Artifact 2；相位相反：Deca 攻击先行（源码顺序 Deca 在前）
    assert_eq!(b.monsters[0].creature.name, "Deca");
    assert_eq!(b.monsters[0].next_byte, 0);
    assert_eq!(b.monsters[1].creature.name, "Donu");
    assert_eq!(b.monsters[1].next_byte, 2);
    for m in &b.monsters {
        assert!(m
            .creature
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Artifact && p.amount == 2));
    }
    b.end_turn();
    b.run_until_idle();
    // 源码顺序 Deca 在前：光束 2×10=20（Donu 后手，强化下轮才生效）
    assert_eq!(b.player.current_health, 280);
    assert_eq!(count_card(&b, "Dazed"), 2);
    for m in &b.monsters {
        assert!(m
            .creature
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Strength && p.amount >= 3));
    }
    // 相位翻转：Deca → 方阵，Donu → 光束
    assert_eq!(b.monsters[0].next_byte, 2);
    assert_eq!(b.monsters[1].next_byte, 0);
    b.end_turn();
    b.run_until_idle();
    // Donu 光束：2×(10+3)=26（光环上轮已 +3）；Deca 方阵：全体 +16 格挡
    assert_eq!(b.player.current_health, 280 - 26);
    assert_eq!(b.monsters[0].creature.current_block, 16);
    assert_eq!(b.monsters[1].creature.current_block, 16);
}

// ---------------------------------------------------------------- Awakened One

#[test]
fn awakened_one_rebirth_transition() {
    let mut b = fixture_real("Awakened One", 300);
    // 开局：Regenerate 10 + Curiosity 1（升阶 0 无先行力量）；首招 Pounce
    let m = &b.monsters[1];
    assert!(m
        .creature
        .powers
        .iter()
        .any(|p| p.kind == PowerKind::Regenerate && p.amount == 10));
    assert!(m
        .creature
        .powers
        .iter()
        .any(|p| p.kind == PowerKind::Curiosity && p.amount == 1));
    assert_eq!(m.next_byte, 1);
    b.end_turn();
    b.run_until_idle();
    // AO Pounce 20 + 教徒（仪式 +3 力）咬 9
    assert_eq!(b.player.current_health, 271, "Pounce 20 + 教徒 9");
    // 一击归零：不判死 → 半死 + 二形态转换 + 下一招 REBIRTH
    b.mgr.add_bottom(GameAction::Damage {
        target: TargetRef::Monster(1),
        info: DamageInfo {
            owner_is_player: true,
            base: 999,
            output: 999,
            ty: DamageType::Normal,
            modified: true,
        },
    });
    b.run_until_idle();
    let m = &b.monsters[1];
    assert!(!m.creature.is_dying, "一形态归零不判死");
    assert!(m.creature.half_dead);
    assert!(m.brain.form2);
    assert_eq!(m.next_byte, 3, "转换后下招 REBIRTH");
    assert!(
        !m.creature
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Curiosity),
        "转换时去除 Curiosity"
    );
    assert!(
        m.creature
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Regenerate),
        "增益保留"
    );
    assert!(!b.is_battle_over, "半死不算胜利");
    // REBIRTH：回满血 → 二形态首招 Dark Echo
    b.end_turn();
    b.run_until_idle();
    let m = &b.monsters[1];
    assert_eq!(m.creature.current_health, m.creature.max_health);
    assert!(!m.creature.half_dead);
    assert_eq!(m.next_byte, 5);
    // Dark Echo 40 + 教徒咬（仪式 3 层 +9 力 → 15）
    let hp0 = b.player.current_health;
    b.end_turn();
    b.run_until_idle();
    assert_eq!(hp0 - b.player.current_health, 55);
}

#[test]
fn awakened_one_curiosity_on_power_card() {
    // 牌组含能力牌（Inflame）→ 打出时 Awakened One +1 力
    let monsters = sts_core::monsters::build_real("Awakened One", 0, 300).unwrap();
    let deck: Vec<(usize, bool)> = (0..10)
        .map(|_| (sid("Strike_R"), false))
        .chain(std::iter::once((sid("Inflame"), false)))
        .collect();
    let mut b = BattleState::new(80, 3, deck, monsters, 5);
    b.relics_ctx = RelicCtx::default();
    b.start_battle();
    b.setup_real_monsters();
    b.run_until_idle();
    // 把 Inflame 抽到手（确定性）
    if let Some(pos) = b
        .master_deck
        .group
        .iter()
        .position(|c| c.st().id == "Inflame")
    {
        let c = b.master_deck.group.remove(pos);
        let mut c = CardInst {
            uuid: b.uuid_next,
            ..c
        };
        b.uuid_next += 1;
        c.uuid = b.uuid_next; // 占位避免 warning
        b.hand.add_top(c);
    }
    let i = b
        .hand
        .group
        .iter()
        .position(|c| c.st().id == "Inflame")
        .unwrap();
    b.play_card(i, None).unwrap();
    b.run_until_idle();
    assert!(
        b.monsters[1]
            .creature
            .powers
            .iter()
            .any(|p| p.kind == PowerKind::Strength && p.amount >= 1),
        "Curiosity：能力牌打出 +1 力"
    );
}

#[test]
fn awakened_one_sludge_adds_void() {
    let mut b = fixture_real("Awakened One", 300);
    // 推进到二形态（杀→REBIRTH→Dark Echo）后强制 Sludge
    b.mgr.add_bottom(GameAction::Damage {
        target: TargetRef::Monster(1),
        info: DamageInfo {
            owner_is_player: true,
            base: 999,
            output: 999,
            ty: DamageType::Normal,
            modified: true,
        },
    });
    b.run_until_idle();
    b.end_turn();
    b.run_until_idle(); // REBIRTH
    b.monsters[1].next_byte = 6; // SLUDGE
    b.end_turn();
    b.run_until_idle();
    assert_eq!(count_card(&b, "Void"), 1, "虚空洗入抽牌堆");
}

// ------------------------------------------------- 灼伤自动打出 / 虚空扣能

#[test]
fn burn_in_hand_autoplays_at_end_of_turn() {
    let mut b = fixture_real("Hexaghost", 80);
    // 手牌顶加一张升级灼伤（THORNS 4，无视格挡）
    let mut c = CardInst {
        uuid: b.uuid_next,
        sid: sid("Burn"),
        upgraded: true,
        times_upgraded: 1,
        cost_for_turn: -2,
        misc: 0,
    };
    b.uuid_next += 1;
    c.uuid = b.uuid_next;
    b.hand.add_top(c);
    b.player.current_block = 10;
    let hp0 = b.player.current_health;
    b.end_turn();
    b.run_until_idle();
    assert_eq!(hp0 - b.player.current_health, 4, "灼伤 THORNS 无视格挡");
}

#[test]
fn void_on_draw_loses_energy() {
    let mut b = fixture_real("Awakened One", 80);
    let mut c = CardInst {
        uuid: b.uuid_next,
        sid: sid("Void"),
        upgraded: false,
        times_upgraded: 0,
        cost_for_turn: -2,
        misc: 0,
    };
    b.uuid_next += 1;
    c.uuid = b.uuid_next;
    b.draw_pile.add_top(c);
    let e0 = b.energy;
    b.mgr.add_bottom(GameAction::DrawCard {
        amount: 1,
        end_turn_draw: false,
    });
    b.run_until_idle();
    assert_eq!(b.energy, e0 - 1, "抽到虚空失去 1 能量");
}

// ------------------------------------------------------------- Brain 默认值

#[test]
fn brain_defaults_match_scripted_monsters() {
    let m = Monster::scripted("Cultist", 50, vec![Some(6)]);
    assert_eq!(m.brain, Brain::default());
    assert!(m.special.is_none());
}

#[test]
fn hexaghost_variants_by_ascension() {
    let a4 = sts_core::monsters::build_real("Hexaghost", 4, 250).unwrap();
    match a4[0].special {
        Some(SpecialAi::HexaghostFull {
            fire_tackle,
            inferno,
            str_amt,
            sear_burn,
        }) => {
            assert_eq!((fire_tackle, inferno, str_amt, sear_burn), (6, 3, 2, 1));
        }
        _ => panic!("应有 HexaghostFull"),
    }
    let a19 = sts_core::monsters::build_real("Hexaghost", 19, 250).unwrap();
    match a19[0].special {
        Some(SpecialAi::HexaghostFull {
            str_amt, sear_burn, ..
        }) => {
            assert_eq!((str_amt, sear_burn), (3, 2));
        }
        _ => panic!("应有 HexaghostFull"),
    }
}
