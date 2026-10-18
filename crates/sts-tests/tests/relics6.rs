//! 批次 5g 测试：卡上下文三件套（打击假人/腕刃/化学 X）+ 破冠奖励 + 噩梦捕手。

use sts_core::battle::{BattleState, Monster, RelicCtx};
use sts_core::power::DamageType;
use sts_core::flow4b::RunState4b;
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
fn strike_dummy_boosts_strike_named_attacks() {
    let r = RelicCtx { strike_dummy: true, ..RelicCtx::default() };
    let mut b = battle(r);
    let strike_sid = sid("Strike_R");
    b.playing_card = Some((strike_sid, 1));
    let info = sts_core::cards::attack_info(&b, 0, 6, DamageType::Normal);
    assert_eq!(info.output, 9, "打击假人：打击 +3");
    // 非 Strike 名不受影响
    b.playing_card = Some((sid("Bash"), 2));
    let info2 = sts_core::cards::attack_info(&b, 0, 8, DamageType::Normal);
    assert_eq!(info2.output, 8, "非打击名 +0");
}

#[test]
fn wrist_blade_boosts_zero_cost_attacks() {
    let r = RelicCtx { wrist_blade: true, ..RelicCtx::default() };
    let mut b = battle(r);
    let shiv = sid("Shiv");
    b.playing_card = Some((shiv, 0));
    let info = sts_core::cards::attack_info(&b, 0, 4, DamageType::Normal);
    assert_eq!(info.output, 8, "腕刃：0 费攻击 +4");
    b.playing_card = Some((sid("Strike_R"), 1));
    let info2 = sts_core::cards::attack_info(&b, 0, 6, DamageType::Normal);
    assert_eq!(info2.output, 6, "非 0 费 +0");
}

#[test]
fn chemical_x_boosts_x_value() {
    let r = RelicCtx { chemical_x: true, ..RelicCtx::default() };
    let b = battle(r);
    assert_eq!(sts_core::cards::x_value(&b), b.energy + 2);
    let r2 = RelicCtx::default();
    let b2 = battle(r2);
    assert_eq!(sts_core::cards::x_value(&b2), b2.energy);
}

#[test]
fn busted_crown_shrinks_card_reward() {
    let mut run = RunState4b::new(51);
    run.open_neow();
    let _ = run.take_neow(0);
    run.apply_relic_effect("Busted Crown");
    run.relics.push("Busted Crown");
    run.current = Some((3, 0));
    run.map[0][3].symbol = Some('M');
    run.enter_room_public();
    assert!(run.card_reward.len() <= 1, "破冠：卡奖励 -2");
}

#[test]
fn dream_catcher_adds_card_on_rest() {
    let mut run = RunState4b::new(53);
    run.open_neow();
    let _ = run.take_neow(0);
    run.apply_relic_effect("Dream Catcher");
    run.relics.push("Dream Catcher");
    let n0 = run.deck.len();
    run.phase = sts_core::flow4b::RoomPhase::CampfireChoice;
    let _ = run.campfire_rest();
    assert_eq!(run.deck.len(), n0 + 1, "休息获得一张牌");
}
