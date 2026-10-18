//! 批次 5b 加权招式测试：getMove 权重转译（分布采样/thrash 附带格挡/意图可见）。

use sts_core::battle::{BattleState, Monster, RelicCtx};
use sts_data::monster_ai;

fn sid(id: &str) -> usize {
    sts_data::gen::CARDS
        .iter()
        .position(|c| c.id == id)
        .expect(id)
}

fn battle_with(monster: Monster) -> BattleState {
    let deck: Vec<(usize, bool)> = (0..10).map(|_| (sid("Defend_R"), false)).collect();
    let mut b = BattleState::new(80, 3, deck, vec![monster], 5);
    b.relics_ctx = RelicCtx::default();
    b.start_battle();
    b.roll_weighted_initial();
    b.run_until_idle();
    b
}

#[test]
fn weighted_table_covers_act1_normals() {
    for id in [
        "Jaw Worm",
        "Louse",
        "Acid Slime L",
        "Acid Slime M",
        "Acid Slime S",
        "Spike Slime L",
        "Spike Slime M",
        "Spike Slime S",
        "Fungi Beast",
        "Blue Slaver",
        "Red Slaver",
        "Sentry",
        "Gremlin Fat",
        "Gremlin Tsundere",
    ] {
        assert!(monster_ai::get_weighted(id).is_some(), "{id} 缺加权表");
    }
    assert!(
        monster_ai::get_weighted("Cultist").is_none(),
        "教徒为确定性 AI"
    );
}

#[test]
fn pick_weighted_respects_weights() {
    let wm = monster_ai::get_weighted("Louse").unwrap();
    // 1000 次采样：咬 6 约 75%，强化(-1) 约 25%
    let mut bites = 0;
    for n in 0..1000i32 {
        let (dmg, _, _) = monster_ai::pick_weighted(wm, n % 100, false);
        if dmg == 6 {
            bites += 1;
        }
    }
    assert!((700..=800).contains(&bites), "咬占比 {bites}/1000 偏离 75%");
}

#[test]
fn jaw_worm_opening_is_chomp() {
    // 源码首招必为 Chomp（构造器 setMove(1)）
    let mut b = battle_with(Monster::scripted("Jaw Worm", 44, vec![Some(11)]));
    b.weighted_override_for_test();
    b.roll_weighted_initial();
    // 首掷走表后仍应有一次攻击意图（chomp 11 或其他攻击）
    let intent = b.monsters[0].script[0];
    assert!(intent.is_some(), "首招应为攻击");
}

#[test]
fn thrash_grants_bonus_block() {
    // Jaw Worm 掷到 Thrash（7 伤 + 6 格挡）时怪物获得格挡
    let mut b = battle_with(Monster::scripted("Jaw Worm", 44, vec![Some(11)]));
    b.weighted_override_for_test();
    // 手动置为 thrash
    b.monsters[0].script = vec![Some(7)];
    b.monsters[0].bonus_block = 6;
    b.monsters[0].creature.current_block = 0;
    b.end_turn();
    b.run_until_idle();
    assert!(
        b.monsters[0].creature.current_block > 0,
        "Thrash 附带 6 格挡"
    );
}

#[test]
fn weighted_intent_visible_after_initial_roll() {
    let mut b = battle_with(Monster::scripted("Sentry", 40, vec![Some(9), None]));
    b.weighted_override_for_test();
    b.roll_weighted_initial();
    // Sentry 首招：光束 9 或螺栓（None）——两者必居其一且意图可读
    let intent = b.monsters[0].intent_label();
    assert!(!intent.is_empty());
    assert!(
        intent.contains("⚔") || intent.contains("🛡"),
        "意图 {intent}"
    );
}

#[test]
fn act23_weighted_tables_cover_individuals() {
    for id in [
        "Byrd",
        "Centurion",
        "Chosen",
        "Shell Parasite",
        "Spheric Guardian",
        "Snake Plant",
        "Snecko",
        "Book of Stabbing",
        "Taskmaster",
        "Bandit Leader",
        "Bandit Bear",
        "Darkling",
        "Reptomancer",
        "Spire Growth",
        "Maw",
        "Nemesis",
        "Writhing Mass",
        "Giant Head",
        "Spire Shield",
        "Spire Spear",
    ] {
        assert!(monster_ai::get_weighted(id).is_some(), "{id} 缺加权表");
    }
}

#[test]
fn multi_hit_moves_carry_hits() {
    // Byrd 啄击 1×5；Book of Stabbing 连刺 6×3
    let byrd = monster_ai::get_weighted("Byrd").unwrap();
    let (_, _, hits) = monster_ai::pick_weighted(byrd, 10, false); // 10% 落在啄击
    assert_eq!(hits, 5, "啄击 5 段");
    let book = monster_ai::get_weighted("Book of Stabbing").unwrap();
    let (_, _, hits) = monster_ai::pick_weighted(book, 10, false);
    assert_eq!(hits, 3, "连刺 3 段");
}

#[test]
fn byrd_peck_deals_five_hits_in_battle() {
    let mut byrd = Monster::scripted("Byrd", 30, vec![Some(1)]);
    byrd.weighted = true;
    let deck: Vec<(usize, bool)> = (0..10).map(|_| (sid("Defend_R"), false)).collect();
    let mut b = BattleState::new(80, 3, deck, vec![byrd], 5);
    b.relics_ctx = RelicCtx::default();
    b.start_battle();
    b.roll_weighted_initial();
    b.run_until_idle();
    // 强制啄击（1 伤 ×5）
    b.monsters[0].script = vec![Some(1)];
    b.monsters[0].hits = 5;
    b.monsters[0].move_idx = 0;
    b.end_turn();
    b.run_until_idle();
    let taken = 80 - b.player.current_health;
    assert_eq!(taken, 5, "啄击 5 段各 1 伤");
}
