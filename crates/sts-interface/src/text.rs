//! 官方中文文本的展示插值层（§15.5）。
//!
//! 原则：数据层保存官方原文（含 `!D!`/`!B!`/`!M!`/`NL`/`*关键词*`/`[E]` 占位），
//! 本模块在展示前用**实际结算值**插值；不翻译、不改写、不新增措辞。
//! 战斗内：!D! 含力量加成、!B! 含敏捷加成（原版 AbstractCard.applyPowers 语义）。

use sts_data::gen::{self, zhs_text};

/// 查官方遗物文本：id → (名称, 描述, 风味)。未收录返回 None。
pub fn relic_text(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    zhs_text::RELIC_TEXT
        .iter()
        .find(|(k, _, _, _)| *k == id)
        .map(|(_, n, d, f)| (*n, *d, *f))
}

/// 查官方药水文本：id → (名称, 描述)。
pub fn potion_text(id: &str) -> Option<(&'static str, &'static str)> {
    zhs_text::POTION_TEXT
        .iter()
        .find(|(k, ..)| *k == id)
        .map(|(_, n, d)| (*n, *d))
}

/// 查官方事件文本：id → (名称, 正文, 选项数组)。
/// 选项在生成时以 " | " 连接存储；未收录返回 None。
pub fn event_text(id: &str) -> Option<(&'static str, String, Vec<String>)> {
    zhs_text::EVENT_TEXT
        .iter()
        .find(|(k, ..)| *k == id)
        .map(|(_, n, body, opts)| {
            (
                *n,
                body.to_string(),
                opts.split(" | ").map(|s| s.to_string()).collect(),
            )
        })
}

/// 怪物显示名 → 官方中文名（生成数据键为原版类键）。
pub fn monster_name_zhs(display_name: &str) -> Option<&'static str> {
    let key = match display_name {
        "Jaw Worm" => "JawWorm",
        "Cultist" => "Cultist",
        "Louse" => "LouseNormal",
        "Louse Defensive" => "LouseDefensive",
        "Acid Slime L" => "AcidSlime_L",
        "Acid Slime M" => "AcidSlime_M",
        "Acid Slime S" => "AcidSlime_S",
        "Spike Slime L" => "SpikeSlime_L",
        "Spike Slime M" => "SpikeSlime_M",
        "Spike Slime S" => "SpikeSlime_S",
        "Blue Slaver" => "SlaverBlue",
        "Red Slaver" => "SlaverRed",
        "Looter" => "Looter",
        "Mugger" => "Mugger",
        "Fungi Beast" => "FungiBeast",
        "Gremlin Nob" => "GremlinNob",
        "Lagavulin" => "Lagavulin",
        "Sentry" => "Sentry",
        "The Guardian" => "TheGuardian",
        "Hexaghost" => "Hexaghost",
        "Slime Boss" => "SlimeBoss",
        "Spheric Guardian" => "SphericGuardian",
        "Chosen" => "Chosen",
        "Shell Parasite" => "ShelledParasite",
        "Byrd" => "Byrd",
        "Centurion" => "Centurion",
        "Snake Plant" => "SnakePlant",
        "Snecko" => "Snecko",
        "Book of Stabbing" => "BookOfStabbing",
        "Gremlin Leader" => "GremlinLeader",
        "Taskmaster" => "Taskmaster",
        "Bandit Pointy" => "BanditPointy",
        "Bandit Leader" => "BanditLeader",
        "Bandit Bear" => "BanditBear",
        "Bronze Automaton" => "BronzeAutomaton",
        "The Collector" => "TheCollector",
        "Champ" => "Champ",
        "Darkling" => "Darkling",
        "Reptomancer" => "Reptomancer",
        "Spire Growth" => "SpireGrowth",
        "Transient" => "Transient",
        "Maw" => "Maw",
        "Nemesis" => "Nemesis",
        "Writhing Mass" => "WrithingMass",
        "Giant Head" => "GiantHead",
        "Orb Walker" => "OrbWalker",
        "Awakened One" => "AwakenedOne",
        "Time Eater" => "TimeEater",
        "Donu" => "Donu",
        "Deca" => "Deca",
        "The Heart" => "CorruptHeart",
        "Spire Shield" => "SpireShield",
        "Spire Spear" => "SpireSpear",
        "Gremlin Warrior" => "GremlinWarrior",
        "Gremlin Thief" => "GremlinThief",
        "Gremlin Fat" => "GremlinFat",
        "Gremlin Tsundere" => "GremlinTsundere",
        "Gremlin Wizard" => "GremlinWizard",
        "Snake Dagger" => "SnakeDagger",
        _ => return None,
    };
    zhs_text::MONSTER_NAMES
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, n)| *n)
}

/// 能力官方中文名（PowerKind → 原版 Power ID → powers.json NAME）。
/// 未映射的能力回退到接口层手写标签（在审计报告中登记）。
pub fn power_label(kind: sts_core::power::PowerKind) -> String {
    use sts_core::power::PowerKind as K;
    let official_id: &str = match kind {
        K::Strength => "Strength",
        K::Dexterity => "Dexterity",
        K::Vulnerable => "Vulnerable",
        K::Weak => "Weakened",
        K::Frail => "Frail",
        K::Poison => "Poison",
        K::Artifact => "Artifact",
        K::Metallicize => "Metallicize",
        K::Intangible | K::IntangiblePlayer => "Intangible",
        K::Thorns => "Thorns",
        K::Regenerate => "Regenerate",
        K::PlatedArmor => "Plated Armor",
        K::CurlUp => "Curl Up",
        K::Ritual => "Ritual",
        K::Curiosity => "Curiosity",
        K::Barricade => "Barricade",
        K::DemonForm => "Demon Form",
        K::Mantra => "Mantra",
        K::Focus => "Focus",
        K::Rage => "Rage",
        K::AfterImage => "After Image",
        K::NoxiousFumes => "Noxious Fumes",
        K::Combust => "Combust",
        K::DarkEmbrace => "Dark Embrace",
        K::FeelNoPain => "Feel No Pain",
        K::Corruption => "Corruption",
        K::Echo => "Echo Form",
        K::Buffer => "Buffer",
        K::LikeWater => "Like Water",
        K::WreathOfFlame => "Wreath of Flame",
        K::Establishment => "Establishment",
        K::FlameBarrier => "Flame Barrier",
        K::DodgeAndRoll => "Dodge and Roll",
        K::Berserk => "Berserk",
        K::DoubleTap => "Double Tap",
        K::Evolve => "Evolve",
        K::Juggernaut => "Juggernaut",
        _ => return fallback_label(kind),
    };
    if let Some((_, n)) = zhs_text::POWER_NAMES
        .iter()
        .find(|(k, _)| *k == official_id)
    {
        return n.to_string();
    }
    fallback_label(kind)
}

fn fallback_label(kind: sts_core::power::PowerKind) -> String {
    use sts_core::power::PowerKind as K;
    match kind {
        K::CurlUp => "蜷缩",
        K::Ritual => "仪式",
        K::Curiosity => "好奇",
        K::DrawCardNextTurn => "下回合抽牌",
        K::GainStrength => "力量收回",
        _ => return format!("{kind:?}"),
    }
    .to_string()
}

/// 姿态官方中文名。
pub fn stance_name_zhs(stance: sts_core::stance::StanceKind) -> &'static str {
    use sts_core::stance::StanceKind as S;
    match stance {
        S::Neutral => "中立",
        S::Calm => "平静",
        S::Wrath => "愤怒",
        S::Divinity => "神格",
    }
}

/// 角色官方中文名。
pub fn character_name_zhs(id: &str) -> &'static str {
    zhs_text::CHARACTER_NAMES
        .iter()
        .find(|(k, _)| *k == id)
        .map(|(_, n)| *n)
        .unwrap_or("")
}

/// 卡牌描述插值（核心显示值 → 官方中文模板）。
///
/// - `dmg/blk/magic`：调用方传入的**实际显示值**（战斗外=基础/升级值；战斗内
///   由 capture 先叠加力量/敏捷——原版 applyPowers 语义）。
/// - `NL` → 换行；`*关键词*` 去星号（终端风格以颜色/字重区分由 UI 决定）。
/// - `[E]`/`[B]` 等图标占位原样保留（等宽终端字符，非变量未解析）。
pub fn card_desc_raw(sid: usize, upgraded: bool, dmg: i32, blk: i32, magic: i32) -> String {
    let st = &gen::CARDS[sid];
    let base = st.desc_zhs.or(st.desc_en).unwrap_or("");
    let raw = if upgraded {
        gen::zhs_text::CARD_UPG_DESC
            .iter()
            .find(|(k, _)| *k == st.id)
            .map(|(_, u)| *u)
            .filter(|u| !u.is_empty())
            .unwrap_or(base)
    } else {
        base
    };
    raw.replace("!D!", &dmg.to_string())
        .replace("!B!", &blk.to_string())
        .replace("!M!", &magic.to_string())
        .replace("NL", "\n")
        .replace('*', "")
}

/// 静态（战斗外）显示值：升级delta 合成，与 CardInst::eff_* 同规则。
pub fn static_eff(sid: usize, upgraded: bool) -> (i32, i32, i32, i32) {
    let st = &gen::CARDS[sid];
    let has = |f: u8| upgraded && st.up_flags & f != 0;
    let dmg = if st.base_damage >= 0 && (has(gen::UPF_DMG) || upgraded && st.up_damage > 0) {
        st.base_damage + if has(gen::UPF_DMG) { st.up_damage } else { 0 }
    } else {
        st.base_damage
    };
    let blk = if st.base_block >= 0 && (has(gen::UPF_BLOCK) || upgraded && st.up_block > 0) {
        st.base_block + if has(gen::UPF_BLOCK) { st.up_block } else { 0 }
    } else {
        st.base_block
    };
    let magic = if st.base_magic >= 0 && (has(gen::UPF_MAGIC) || upgraded && st.up_magic > 0) {
        st.base_magic + if has(gen::UPF_MAGIC) { st.up_magic } else { 0 }
    } else {
        st.base_magic
    };
    (dmg, blk, magic, st.cost)
}

/// 战斗内显示值：静态值 + 力量（攻击）/敏捷（技能）（原版 applyPowers 展示语义）。
pub fn combat_eff(
    sid: usize,
    upgraded: bool,
    powers: &[sts_core::power::Power],
) -> (i32, i32, i32) {
    use sts_core::power::PowerKind as K;
    let (mut dmg, mut blk, magic, _) = static_eff(sid, upgraded);
    let st = &gen::CARDS[sid];
    let str_amt = powers
        .iter()
        .find(|p| p.kind == K::Strength)
        .map(|p| p.amount)
        .unwrap_or(0);
    let dex_amt = powers
        .iter()
        .find(|p| p.kind == K::Dexterity)
        .map(|p| p.amount)
        .unwrap_or(0);
    if st.kind == gen::CardType::Attack && dmg >= 0 {
        dmg += str_amt;
    }
    if st.kind == gen::CardType::Skill && blk > 0 {
        blk += dex_amt;
    }
    (dmg, blk, magic)
}
