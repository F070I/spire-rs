//! 抽取结果审计（批次 1.2/1.3，docs/05 完成条件 3+6）。
//!
//! 数量与关键事实对照 docs/source-inventory/ 的盘点记录；任何变化都必须先更新此处注释再重跑 extract.py。

use sts_data::gen::*;

#[test]
fn card_counts_match_source_tree() {
    // 对照盘点记录 02 §9：red 75 / green 75 / blue 76 / purple 77 / colorless 39 / curses 14 / status 5 / temp 9 / option 5
    let mut by_dir = std::collections::BTreeMap::new();
    for c in CARDS {
        *by_dir.entry(c.color_dir).or_insert(0) += 1;
    }
    assert_eq!(by_dir.get("red"), Some(&75));
    assert_eq!(by_dir.get("green"), Some(&75));
    assert_eq!(by_dir.get("blue"), Some(&76));
    assert_eq!(by_dir.get("purple"), Some(&77));
    assert_eq!(by_dir.get("colorless"), Some(&39));
    assert_eq!(by_dir.get("curses"), Some(&14));
    assert_eq!(by_dir.get("status"), Some(&5));
    assert_eq!(by_dir.get("tempCards"), Some(&9));
    assert_eq!(by_dir.get("optionCards"), Some(&5));
    assert_eq!(CARDS.len(), 375);
}

#[test]
fn card_ids_unique_and_spotchecked() {
    // ID 唯一性（原版 ID 是硬契约）
    let mut seen = std::collections::HashSet::new();
    for c in CARDS {
        assert!(seen.insert(c.id), "duplicate card id: {}", c.id);
    }
    // 抽查（出处：cards/ 源码 super(...) 调用）
    let strike = card("Strike_R").expect("Strike_R");
    assert_eq!((strike.kind, strike.color, strike.rarity), (CardType::Attack, CardColor::Red, CardRarity::Basic));
    assert_eq!(strike.cost, 1);
    assert_eq!(strike.base_damage, 6);
    assert_eq!(strike.tags & TAG_STRIKE, TAG_STRIKE);

    let anger = card("Anger").expect("Anger");
    assert_eq!((anger.cost, anger.base_damage), (0, 6));

    // X 费卡：Whirlwind cost = -1
    assert_eq!(card("Whirlwind").unwrap().cost, -1);
    // 状态牌：Burn cost = -2、类型 Status
    let burn = card("Burn").unwrap();
    assert_eq!((burn.cost, burn.kind), (-2, CardType::Status));

    // 升级数值：Bash baseDamage=8 baseBlock=0?（Bash 无格挡，是 vuln+2）
    let bash = card("Bash").unwrap();
    assert_eq!(bash.base_damage, 8);
}

#[test]
fn localization_coverage() {
    // eng 文本覆盖：423 键；375 张卡中除 Impulse（反编译资产缺口，docs/modules/extract.md 已记录）外全部有名
    let missing: Vec<&str> = CARDS.iter().filter(|c| c.name_en.is_none()).map(|c| c.id).collect();
    assert_eq!(missing, vec!["Impulse"]);
    // 中文（zhs）覆盖同样完整（Impulse 同缺）
    assert!(CARDS.iter().all(|c| c.name_zhs.is_some() || c.id == "Impulse"));

    // 遗物/药水/怪物/能力：抽查知名 ID 双语文本
    let vajra = RELICS.iter().find(|r| r.id == "Vajra").expect("Vajra");
    assert_eq!(vajra.name_en, Some("Vajra"));
    assert_eq!(vajra.name_zhs, Some("金刚杵"));
    let fire = POTIONS.iter().find(|p| p.id == "Fire Potion").expect("Fire Potion");
    assert_eq!(fire.rarity, PotionRarity::Common);
}

#[test]
fn relic_potion_power_monster_counts() {
    // 抽取计数（tools/extract.py 输出；与盘点记录的差异见 docs/modules/extract.md）
    assert_eq!(RELICS.len(), 181); // 含少量未注册/特殊遗物，177 为 RelicLibrary 注册数
    assert_eq!(POTIONS.len(), 42); // 排除 AbstractPotion/PotionSlot
    assert_eq!(POWERS.len(), 149); // 排除 AbstractPower 与 deprecated/
    assert_eq!(MONSTERS.len(), 68); // 排除 5 个核心类
    assert_eq!(EVENTS.len(), 52);
    assert!(KEYWORDS.len() >= 40); // GameDictionary ~45 词
}
