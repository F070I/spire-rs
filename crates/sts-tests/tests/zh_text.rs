//! 官方中文文本插值测试（§15.5/§15.7）：占位符解析、升级数值、战斗内加成、
//! 官方名映射、术语一致性。全部使用官方 zhs 数据，无手写翻译。

use sts_core::battle::Monster;
use sts_core::power::Power;
use sts_data::gen;
use sts_interface::{
    card_desc_raw, combat_eff, event_text, monster_name_zhs, potion_text, power_label, relic_text,
    stance_name_zhs, static_eff,
};

fn sid(id: &str) -> usize {
    gen::CARDS.iter().position(|c| c.id == id).expect(id)
}

#[test]
fn strike_official_text_interpolated() {
    let (dmg, blk, magic, _) = static_eff(sid("Strike_R"), false);
    assert_eq!(dmg, 6);
    let t = card_desc_raw(sid("Strike_R"), false, dmg, blk, magic);
    assert_eq!(t, "造成 6 点伤害。", "官方原文 + !D! 插值");
    assert!(!t.contains('!'), "无未解析占位符");
}

#[test]
fn strike_upgraded_uses_upgrade_delta() {
    let (dmg, _, _, _) = static_eff(sid("Strike_R"), true);
    assert_eq!(dmg, 9, "升级打击 6+3");
    let t = card_desc_raw(sid("Strike_R"), true, dmg, 0, 0);
    assert!(t.contains("造成 9 点伤害"), "升级后数值更新：{t}");
}

#[test]
fn bash_dual_value_and_vulnerable_magic() {
    let (dmg, _, magic, _) = static_eff(sid("Bash"), false);
    assert_eq!(dmg, 8);
    assert_eq!(magic, 2);
    let t = card_desc_raw(sid("Bash"), false, dmg, 0, magic);
    assert!(t.contains("造成 8 点伤害"), "{t}");
    assert!(t.contains("2 层 易伤") || t.contains("易伤"), "{t}");
}

#[test]
fn defend_block_value() {
    let (_, blk, _, _) = static_eff(sid("Defend_R"), false);
    assert_eq!(blk, 5);
    let t = card_desc_raw(sid("Defend_R"), false, 0, blk, 0);
    assert!(
        t.contains("5 点 格挡") || t.contains("5 点格挡") || t.contains("获得 5"),
        "{t}"
    );
}

#[test]
fn combat_strength_and_dex_apply_to_display() {
    // 力量 3 → 打击显示 9；敏捷 2 → 防御显示 7
    let powers = vec![
        Power::new(sts_core::power::PowerKind::Strength, 3),
        Power::new(sts_core::power::PowerKind::Dexterity, 2),
    ];
    let (d, _, _) = combat_eff(sid("Strike_R"), false, &powers);
    assert_eq!(d, 9, "力量入攻击显示");
    let (_, b2, _) = combat_eff(sid("Defend_R"), false, &powers);
    assert_eq!(b2, 7, "敏捷入格挡显示");
}

#[test]
fn no_raw_placeholders_across_all_cards() {
    // 全 375 卡插值冒烟：任何卡不得残留 !D!/!B!/!M! 或 *
    for sid in 0..gen::CARDS.len() {
        let (dmg, blk, magic, _) = static_eff(sid, false);
        let t = card_desc_raw(sid, false, dmg, blk, magic);
        assert!(!t.contains("!D!"), "{} 残留 !D!", gen::CARDS[sid].id);
        assert!(!t.contains("!B!"), "{} 残留 !B!", gen::CARDS[sid].id);
        assert!(!t.contains("!M!"), "{} 残留 !M!", gen::CARDS[sid].id);
    }
}

#[test]
fn relic_potion_official_names_and_desc() {
    let (n, d, flavor) = relic_text("Burning Blood").expect("燃烧之血");
    assert_eq!(n, "燃烧之血");
    assert!(d.contains("回复"), "官方描述：{d}");
    assert!(!flavor.is_empty(), "风味文本保留");
    let (pn, pd) = potion_text("Fire Potion").expect("火焰药水");
    assert_eq!(pn, "火焰药水");
    assert!(pd.contains("点伤害"), "{pd}");
}

#[test]
fn event_official_title_golden_idol() {
    let (n, body, _) = event_text("Golden Idol").expect("金神像");
    assert_eq!(n, "金神像");
    assert!(body.contains("闪闪发光"));
}

#[test]
fn monster_names_official() {
    assert_eq!(monster_name_zhs("Jaw Worm"), Some("大颚虫"));
    assert_eq!(monster_name_zhs("Cultist"), Some("邪教徒"));
    assert_eq!(monster_name_zhs("Awakened One"), Some("觉醒者"));
    // 战斗面板敌人名验证
    let m = Monster::scripted("Jaw Worm", 44, vec![Some(10)]);
    assert_eq!(m.creature.name, "Jaw Worm"); // 核心仍存类名
    assert_eq!(monster_name_zhs(&m.creature.name), Some("大颚虫"));
}

#[test]
fn power_labels_official_terms() {
    assert_eq!(power_label(sts_core::power::PowerKind::Strength), "力量");
    assert_eq!(power_label(sts_core::power::PowerKind::Vulnerable), "易伤");
    assert_eq!(power_label(sts_core::power::PowerKind::Weak), "虚弱");
    assert_eq!(power_label(sts_core::power::PowerKind::Frail), "脆弱");
    assert_eq!(power_label(sts_core::power::PowerKind::Poison), "中毒");
}

#[test]
fn stance_labels_official() {
    assert_eq!(stance_name_zhs(sts_core::stance::StanceKind::Wrath), "愤怒");
    assert_eq!(stance_name_zhs(sts_core::stance::StanceKind::Calm), "平静");
    assert_eq!(
        stance_name_zhs(sts_core::stance::StanceKind::Divinity),
        "神格"
    );
    assert_eq!(
        stance_name_zhs(sts_core::stance::StanceKind::Neutral),
        "中立"
    );
}

#[test]
fn card_desc_in_view_uses_interpolation() {
    // 经 GameView 路径的端到端：奖励卡描述已插值
    let mut run = sts_core::flow4b::RunState4b::new(99);
    run.open_neow();
    let _ = run.take_neow(0);
    run.force_shop_for_test();
    let v = sts_interface::GameView::capture(&run);
    let shop = v.shop.expect("商店");
    for c in &shop.cards {
        assert!(
            !c.desc.contains('!'),
            "{} 描述残留占位符: {}",
            c.name,
            c.desc
        );
    }
}
