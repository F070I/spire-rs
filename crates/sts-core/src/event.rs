//! 事件系统首批（批次 4b 续）：文本 + 选项 + 效果。
//!
//! 结构对应 events/AbstractEvent.buttonEffect(int) 状态机（记录 06 §8）。
//! 首批收录 6 个高价值事件（金/血/卡权衡类）；其余事件批次后续。
//! 数据手写（事件文本中文），效果直接调 RunState4b API。

/// 事件选项效果所需的运行时操作集（由 RunState4b 实现副作用）。
pub struct EventOption {
    pub text: &'static str,
    /// 效果标记（供核心 match 分派）
    pub effect: Effect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// 获得金币
    GainGold(i32),
    /// 失去金币
    LoseGold(i32),
    /// 失去生命
    LoseHp(i32),
    /// 获得生命
    GainHp(i32),
    /// 升级一张随机卡
    UpgradeRandom,
    /// 移除一张卡
    RemoveCard,
    /// 获得随机普通遗物
    GainCommonRelic,
    /// 获得随机稀有遗物
    GainRareRelic,
    /// 加诅咒
    AddCurse,
    /// 无事发生
    Nothing,
}

pub struct EventDef {
    pub id: &'static str,
    pub title: &'static str,
    pub body: &'static str,
    pub options: &'static [EventOption],
}

pub static EVENTS: &[EventDef] = &[
    EventDef {
        id: "Golden Idol",
        title: "黄金神像",
        body: "你看到一个黄金神像嵌在石座里。拿走它可能会触发陷阱……",
        options: &[
            EventOption { text: "拿走神像（+100金，获得遗物，失去 15 生命）", effect: Effect::GainGold(100) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Bonfire Spirits",
        title: "篝火之灵",
        body: "火堆旁的低语邀请你献祭一张卡。",
        options: &[
            EventOption { text: "献祭（移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Cleric",
        title: "牧师",
        body: "一名牧师愿意为你提供服务。",
        options: &[
            EventOption { text: "治疗（35 金，回 20% 最大生命）", effect: Effect::GainHp(20) },
            EventOption { text: "祈愿（50 金，升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Ancient Writing",
        title: "古老文字",
        body: "石碑上的文字向你低语。",
        options: &[
            EventOption { text: "研读（升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "抄录（+诅咒）", effect: Effect::AddCurse },
        ],
    },
    EventDef {
        id: "Winding Halls",
        title: "回廊",
        body: "扭曲的回廊在你面前展开。",
        options: &[
            EventOption { text: "继续走（失去 5 生命，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "原路返回", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Big Fish",
        title: "大鱼",
        body: "水中有什么东西闪闪发光……",
        options: &[
            EventOption { text: "钓鱼（+50% 概率遗物，失去 5 生命）", effect: Effect::GainCommonRelic },
            EventOption { text: "吃掉它（回 12 生命）", effect: Effect::GainHp(12) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Library",
        title: "图书馆",
        body: "一座漂浮的图书馆静候访客。",
        options: &[
            EventOption { text: "阅读（升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "抄写（+诅咒）", effect: Effect::AddCurse },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Moai Head",
        title: "石像",
        body: "一座石像凝视着你，似乎在索求什么。",
        options: &[
            EventOption { text: "献祭（失去 7 生命，+普通遗物）", effect: Effect::GainCommonRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "GoopPuddle",
        title: "黏液坑",
        body: "地上的黏液泛着微光。",
        options: &[
            EventOption { text: "捡起（失去 3 生命，+50 金）", effect: Effect::GainGold(50) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Wing Statue",
        title: "有翼石像",
        body: "石像的手中握着什么东西。",
        options: &[
            EventOption { text: "祈祷（+最大生命 5）", effect: Effect::GainHp(5) },
            EventOption { text: "砸开（+100 金，失去 7 生命）", effect: Effect::GainGold(100) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Golden Shrine",
        title: "黄金神龛",
        body: "神龛上的黄金反射着你的脸。",
        options: &[
            EventOption { text: "祈祷（+75 金）", effect: Effect::GainGold(75) },
            EventOption { text: "破坏（+150 金，+诅咒）", effect: Effect::AddCurse },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Accursed Blacksmith",
        title: "被诅咒的铁匠",
        body: "铁匠愿意为你打造，但需要献祭。",
        options: &[
            EventOption { text: "锻造（升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Forgotten Altar",
        title: "被遗忘的祭坛",
        body: "血迹斑斑的祭坛等待着献祭。",
        options: &[
            EventOption { text: "献祭（失去 7 生命，移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Designer",
        title: "设计师",
        body: "一位神秘设计师提出交易。",
        options: &[
            EventOption { text: "购买服务（75 金，升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Living Wall",
        title: "活墙",
        body: "墙上浮现出一张脸，要求你献祭。",
        options: &[
            EventOption { text: "献祭卡（移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "献祭血（失去 6 生命）", effect: Effect::LoseHp(6) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Mausoleum",
        title: "陵墓",
        body: "陵墓的门微微敞开着。",
        options: &[
            EventOption { text: "进入（+诅咒）", effect: Effect::AddCurse },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Purifier",
        title: "净化者",
        body: "一位旅人愿意为你净化一张卡。",
        options: &[
            EventOption { text: "净化（75 金，移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Boost Trap",
        title: "增幅陷阱",
        body: "你看到一个奇怪的装置。",
        options: &[
            EventOption { text: "触发（失去 5 生命，+50 金）", effect: Effect::GainGold(50) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "TheIDol",
        title: "偶人",
        body: "一个偶人挡住了去路。",
        options: &[
            EventOption { text: "推动（失去 5 生命，+普通遗物）", effect: Effect::GainCommonRelic },
            EventOption { text: "绕行", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Masked Bandits",
        title: "蒙面强盗",
        body: "强盗们要求你交出金币。",
        options: &[
            EventOption { text: "战斗（失去 10 生命，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "交钱（失去全部金币）", effect: Effect::LoseGold(999) },
        ],
    },
    EventDef {
        id: "Wandering Apprentice",
        title: "流浪学徒",
        body: "一名学徒请求指导。",
        options: &[
            EventOption { text: "指导（升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: " Knowing Skull",
        title: "知之颅骨",
        body: "一颗颅骨低声提出交易。",
        options: &[
            EventOption { text: "交易（失去 6 生命，+50 金）", effect: Effect::GainGold(50) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Woman in Blue",
        title: "蓝衣女子",
        body: "蓝衣女子出售她的药剂。",
        options: &[
            EventOption { text: "买药水（50 金，+药水）", effect: Effect::Nothing },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Scrap Ooze",
        title: "废料软泥怪",
        body: "软泥怪身上缠着什么金属。",
        options: &[
            EventOption { text: "伸手（失去 4 生命，+50 金）", effect: Effect::GainGold(50) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Gremlin Wheel Game",
        title: "哥布林轮盘",
        body: "哥布林邀请你转轮盘。",
        options: &[
            EventOption { text: "转一次（+25% 概率赢金）", effect: Effect::GainGold(60) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Shining Light",
        title: "闪亮之光",
        body: "一束光从裂缝中透出。",
        options: &[
            EventOption { text: "进入（失去 4 生命，升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Tomb of Lord Red Mask",
        title: "红面具勋爵之墓",
        body: "墓室里堆满了陪葬品。",
        options: &[
            EventOption { text: "搜刮（+100 金）", effect: Effect::GainGold(100) },
            EventOption { text: "敬畏（+稀有遗物）", effect: Effect::GainRareRelic },
        ],
    },
    EventDef {
        id: "The Colosseum",
        title: "竞技场",
        body: "观众的欢呼声从竞技场传来。",
        options: &[
            EventOption { text: "参加（+75 金，+稀有遗物，失去 10 生命）", effect: Effect::GainRareRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Moai Head",
        title: "摩艾石像",
        body: "石像的嘴里流出金色液体。",
        options: &[
            EventOption { text: "喝下（+最大生命 5）", effect: Effect::GainHp(5) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Nest",
        title: "巢穴",
        body: "一个巨大的巢穴在头顶。",
        options: &[
            EventOption { text: "偷蛋（+普通遗物，失去 8 生命）", effect: Effect::GainCommonRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Adventurer",
        title: "冒险者",
        body: "一名冒险者倒在路上。",
        options: &[
            EventOption { text: "搜身（+随机卡，失去 8 生命）", effect: Effect::GainGold(25) },
            EventOption { text: "帮助（回 10 生命）", effect: Effect::GainHp(10) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Mine",
        title: "矿坑",
        body: "废弃的矿坑深处有金属反光。",
        options: &[
            EventOption { text: "挖掘（+75 金）", effect: Effect::GainGold(75) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Graveyard",
        title: "墓地",
        body: "墓碑林立，似乎有东西埋在浅土里。",
        options: &[
            EventOption { text: "挖掘（+诅咒，+50 金）", effect: Effect::GainGold(50) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Castle",
        title: "城堡",
        body: "一座废弃的城堡矗立在山丘上。",
        options: &[
            EventOption { text: "探索（+稀有遗物，失去 12 生命）", effect: Effect::GainRareRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Dead Adventurer",
        title: "死去的冒险者",
        body: "冒险者的尸体旁散落着物品。",
        options: &[
            EventOption { text: "搜刮（+50 金，失去 6 生命）", effect: Effect::GainGold(50) },
            EventOption { text: "埋葬（+普通遗物）", effect: Effect::GainCommonRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Spirits",
        title: "低语之灵",
        body: "灵体的低语围绕着你。",
        options: &[
            EventOption { text: "倾听（移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Mysterious Sphere",
        title: "神秘球体",
        body: "球体内部有什么在流动。",
        options: &[
            EventOption { text: "触摸（失去 5 生命，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Top",
        title: "陀螺",
        body: "一个精美的陀螺静静躺在地上。",
        options: &[
            EventOption { text: "捡起（+普通遗物）", effect: Effect::GainCommonRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Ghosts",
        title: "鬼影",
        body: "鬼影在雾中若隐若现。",
        options: &[
            EventOption { text: "跟随（+50 金，失去 8 生命）", effect: Effect::GainGold(50) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Test",
        title: "试炼",
        body: "一个声音要求你证明自己。",
        options: &[
            EventOption { text: "接受（失去 12 生命，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "拒绝", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Bird Cave",
        title: "鸟洞",
        body: "洞穴深处传来鸟鸣。",
        options: &[
            EventOption { text: "探索（+普通遗物，失去 4 生命）", effect: Effect::GainCommonRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Vampires",
        title: "吸血鬼",
        body: "吸血鬼提出了一个危险的交易。",
        options: &[
            EventOption { text: "接受（移除打击卡，+诅咒）", effect: Effect::AddCurse },
            EventOption { text: "拒绝", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Joust",
        title: "马上比武",
        body: "两名骑士在远处比武。",
        options: &[
            EventOption { text: "下注（失去 50 金，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "旁观", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "World of Goop",
        title: "黏液世界",
        body: "地上全是黏液。",
        options: &[
            EventOption { text: "捡起（失去 5 生命，+75 金）", effect: Effect::GainGold(75) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Cleric 2",
        title: "牧师的祝福",
        body: "牧师再次出现。",
        options: &[
            EventOption { text: "治疗（回 25% 最大生命）", effect: Effect::GainHp(20) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "White Beast Statue",
        title: "白兽像",
        body: "白兽像的眼睛发出光芒。",
        options: &[
            EventOption { text: "触摸（+药水）", effect: Effect::Nothing },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Gold Idol Trap",
        title: "金像陷阱",
        body: "又一个金像，但这次你更小心了。",
        options: &[
            EventOption { text: "小心拿取（+150 金）", effect: Effect::GainGold(150) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "KnowSP",
        title: "知识",
        body: "一本古书摊开在面前。",
        options: &[
            EventOption { text: "阅读（移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Wheel of Change",
        title: "命运之轮",
        body: "一个巨大的轮盘在转动。",
        options: &[
            EventOption { text: "转动（+100 金，+50 金，+诅咒）", effect: Effect::GainGold(150) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Secret Portal",
        title: "秘密传送门",
        body: "一道传送门在你面前打开。",
        options: &[
            EventOption { text: "进入（+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
];

/// 按 ID 查事件（未知 → 首个事件兜底）。
pub fn get(id: &str) -> &'static EventDef {
    EVENTS.iter().find(|e| e.id == id).unwrap_or(&EVENTS[0])
}

/// 随机选取事件。
pub fn roll(rng: &mut sts_data::rng::StsRandom) -> &'static EventDef {
    &EVENTS[rng.random_int_inclusive(EVENTS.len() as i32 - 1) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_events_have_options() {
        for e in EVENTS {
            assert!(!e.options.is_empty(), "{} 无选项", e.id);
            for o in e.options {
                assert!(!o.text.is_empty());
            }
        }
    }

    #[test]
    fn unknown_id_falls_back() {
        assert_eq!(get("Not An Event").id, "Golden Idol");
    }
}
