//! 批次 4g 遗物第三批测试：出牌计数器/回合/承伤/战斗开始/拾取大件。

use sts_core::action::{GameAction, TargetRef};
use sts_core::battle::{Monster, RelicCtx};
use sts_core::damage::DamageInfo;
use sts_core::flow4b::RunState4b;
use sts_core::power::{DamageType, PowerKind};
use sts_data::gen;

fn battle(relics: RelicCtx) -> sts_core::battle::BattleState {
    let deck: Vec<(usize, bool)> = (0..10).map(|_| (sid("Strike_R"), false)).collect();
    let mut b = sts_core::battle::BattleState::new(
        80,
        3,
        deck,
        vec![Monster::scripted("Jaw Worm", 44, vec![Some(10)])],
        5,
    );
    b.relics_ctx = relics;
    b.start_battle();
    b.run_until_idle();
    b
}

fn sid(id: &str) -> usize {
    gen::CARDS.iter().position(|c| c.id == id).expect(id)
}

fn pure_player_dmg(amount: i32) -> GameAction {
    GameAction::Damage {
        target: TargetRef::Player,
        info: DamageInfo {
            owner_is_player: false,
            base: amount,
            output: amount,
            ty: DamageType::Normal,
            modified: true,
        },
    }
}

fn pure_monster_dmg(amount: i32) -> GameAction {
    GameAction::Damage {
        target: TargetRef::Monster(0),
        info: DamageInfo {
            owner_is_player: true,
            base: amount,
            output: amount,
            ty: DamageType::Normal,
            modified: true,
        },
    }
}

#[test]
fn kunai_shuriken_fan_every_third_attack() {
    let r = RelicCtx {
        kunai: true,
        shuriken: true,
        ornamental_fan: true,
        ..RelicCtx::default()
    };
    let mut b = battle(r);
    // 手动堆 2 次
    b.relic_attack_count = 2;
    let i = b
        .hand
        .group
        .iter()
        .position(|c| c.st().kind == gen::CardType::Attack)
        .unwrap();
    b.play_card(i, Some(0)).unwrap();
    b.run_until_idle();
    assert!(b
        .player
        .powers
        .iter()
        .any(|p| p.kind == PowerKind::Dexterity && p.amount >= 1));
    assert!(b
        .player
        .powers
        .iter()
        .any(|p| p.kind == PowerKind::Strength && p.amount >= 1));
    assert!(b.player.current_block >= 4, "装饰扇 +4 格挡");
}

#[test]
fn bird_faced_urn_heals_on_power() {
    let r = RelicCtx {
        bird_faced_urn: true,
        ..RelicCtx::default()
    };
    let mut b = battle(r);
    b.player.current_health = 40;
    // 塞一张能力牌入手
    let inflame = sid("Inflame");
    b.hand
        .add_top(sts_core::battle::CardInst::new(inflame, false, 99001));
    let i = b
        .hand
        .group
        .iter()
        .position(|c| c.st().id == "Inflame")
        .unwrap();
    b.play_card(i, None).unwrap();
    b.run_until_idle();
    assert!(b.player.current_health > 40, "鸟脸瓮打能力牌回 6");
}

#[test]
fn boot_upgrades_low_unblocked_damage() {
    let r = RelicCtx {
        boot: true,
        ..RelicCtx::default()
    };
    let mut b = battle(r);
    let hp0 = b.monsters[0].creature.current_health;
    b.mgr.add_bottom(pure_monster_dmg(3));
    b.run_until_idle();
    assert_eq!(hp0 - b.monsters[0].creature.current_health, 5, "3 伤 → 5");
}

#[test]
fn torii_caps_low_unblocked_damage() {
    let r = RelicCtx {
        torii: true,
        ..RelicCtx::default()
    };
    let mut b = battle(r);
    b.mgr.add_bottom(pure_player_dmg(4));
    b.run_until_idle();
    assert_eq!(b.player.current_health, 79, "4 伤 → 1");
}

#[test]
fn runic_cube_draws_on_hp_loss() {
    let r = RelicCtx {
        runic_cube: true,
        ..RelicCtx::default()
    };
    let mut b = battle(r);
    let n0 = b.hand.group.len();
    b.mgr.add_bottom(pure_player_dmg(6));
    b.run_until_idle();
    assert_eq!(b.hand.group.len(), n0 + 1, "掉血抽 1");
}

#[test]
fn bronze_scales_and_clockwork_at_start() {
    let r = RelicCtx {
        bronze_scales: true,
        clockwork_souvenir: true,
        ..RelicCtx::default()
    };
    let b = battle(r);
    assert!(b
        .player
        .powers
        .iter()
        .any(|p| p.kind == PowerKind::Thorns && p.amount == 3));
    assert!(b
        .player
        .powers
        .iter()
        .any(|p| p.kind == PowerKind::Artifact && p.amount == 1));
}

#[test]
fn twisted_funnel_poisons_all() {
    let r = RelicCtx {
        twisted_funnel: true,
        ..RelicCtx::default()
    };
    let b = battle(r);
    assert!(b.monsters[0]
        .creature
        .powers
        .iter()
        .any(|p| p.kind == PowerKind::Poison && p.amount == 4));
}

#[test]
fn ice_cream_keeps_energy() {
    let r = RelicCtx {
        ice_cream: true,
        ..RelicCtx::default()
    };
    let mut b = battle(r);
    b.energy = 5; // 本回合剩 5
    b.end_turn();
    b.run_until_idle();
    assert_eq!(b.energy, 8, "剩 5 + 充能 3 = 8（不清零）");
}

#[test]
fn art_of_war_energy_if_no_attack() {
    let r = RelicCtx {
        art_of_war: true,
        ..RelicCtx::default()
    };
    let mut b = battle(r);
    // 回合 1 不打攻击直接结束 → 回合 2 +1 能量
    b.end_turn();
    b.run_until_idle();
    let e = b.energy;
    assert!(e >= 4, "战争艺术：上回合未打攻击 +1 能量");
}

#[test]
fn lizard_tail_revives_once() {
    let r = RelicCtx {
        lizard_tail: true,
        ..RelicCtx::default()
    };
    let mut b = battle(r);
    b.mgr.add_bottom(pure_player_dmg(999));
    b.run_until_idle();
    assert_eq!(b.player.current_health, 40, "死亡→回 50% 上限");
    assert!(!b.player.is_dying);
    b.mgr.add_bottom(pure_player_dmg(999));
    b.run_until_idle();
    assert!(b.player.is_dying, "第二次死亡正常");
}

// ------------------------------------------------ RunState4b 层

fn run() -> RunState4b {
    let mut run = RunState4b::new(7);
    run.open_neow();
    let _ = run.take_neow(0);
    run
}

#[test]
fn regal_pillow_extra_heal() {
    let mut run = run();
    run.apply_relic_effect("Regal Pillow");
    run.relics.push("Regal Pillow");
    run.player_hp = 40;
    run.phase = sts_core::flow4b::RoomPhase::CampfireChoice;
    let _ = run.campfire_rest();
    assert_eq!(run.player_hp, 40 + 12 + 15, "30%(12) + 枕头 15");
}

#[test]
fn omamori_resists_curses() {
    let mut run = run();
    run.apply_relic_effect("Omamori");
    run.relics.push("Omamori");
    run.force_event("The Mausoleum");
    let n0 = run.deck.len();
    let _ = run.choose_event(0); // +诅咒
    assert_eq!(run.deck.len(), n0, "御守抵消第一张诅咒");
    run.force_event("The Mausoleum");
    let _ = run.choose_event(0); // 第二张也被抵消（御守 2 次）
    assert_eq!(run.deck.len(), n0, "御守共 2 次");
    run.force_event("The Mausoleum");
    let _ = run.choose_event(0); // 第三张正常加入
    assert_eq!(run.deck.len(), n0 + 1);
}

#[test]
fn empty_cage_removes_two() {
    let mut run = run();
    let n0 = run.deck.len();
    run.apply_relic_effect("Empty Cage");
    run.relics.push("Empty Cage");
    assert_eq!(run.deck.len(), n0 - 2);
}

#[test]
fn astrolabe_transforms_three() {
    let mut run = run();
    let n0 = run.deck.len();
    run.apply_relic_effect("Astrolabe");
    run.relics.push("Astrolabe");
    assert_eq!(run.deck.len(), n0, "变换 3 张：数量不变");
}

#[test]
fn black_blood_upgrades_victory_heal() {
    let mut run = run();
    run.apply_relic_effect("Black Blood");
    run.relics.push("Black Blood");
    assert_eq!(run.relics_ctx.burning_blood_heal, 12);
}

#[test]
fn necronomicon_adds_curse_on_pickup() {
    let mut run = run();
    let curses0 = run
        .deck
        .iter()
        .filter(|c| c.st().kind == gen::CardType::Curse)
        .count();
    run.apply_relic_effect("Necronomicon");
    run.relics.push("Necronomicon");
    let curses = run
        .deck
        .iter()
        .filter(|c| c.st().kind == gen::CardType::Curse)
        .count();
    assert_eq!(curses, curses0 + 1, "拾取附死灵之诅咒");
}

#[test]
fn calling_bell_relic_and_two_curses() {
    let mut run = run();
    let n0 = run.deck.len();
    run.apply_relic_effect("Calling Bell");
    run.relics.push("Calling Bell");
    assert!(run.relics.contains(&"Girya"));
    assert_eq!(run.deck.len(), n0 + 2);
}

#[test]
fn maw_bank_gold_per_room_until_shop() {
    let mut run = run();
    run.apply_relic_effect("MawBank");
    run.relics.push("MawBank");
    let g0 = run.gold;
    // 手摆一个普通节点进入
    run.current = Some((3, 0));
    run.map[0][3].symbol = Some('M');
    run.enter_room_public();
    assert_eq!(run.gold, g0 + 12);
    assert!(run.relics_ctx.maw_bank, "商店前有效");
}

#[test]
fn ceramic_fish_gold_on_card_reward() {
    let mut run = run();
    run.apply_relic_effect("CeramicFish");
    run.relics.push("CeramicFish");
    run.card_reward = vec![(sid("Strike_R"), false)];
    run.phase = sts_core::flow4b::RoomPhase::Rewards;
    let g0 = run.gold;
    run.take_card_reward(Some(0));
    assert_eq!(run.gold, g0 + 9);
}
