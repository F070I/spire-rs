//! 事件系统（批次 4b 续 → 4d 收尾）：文本 + 选项 + 效果。
//!
//! 结构对应 events/AbstractEvent.buttonEffect(int) 状态机（记录 06 §8）。
//! 52 个数据表 id（gen/events.rs）全部覆盖；效果按各事件 Java 源转译，
//! 多段/分支流程（读书多页、配对翻牌等）收敛为单决策近似——注释标注。
//! `act` 字段：1/2/3 幕专属，0 = 神龛类（全幕可遇），4 = 终幕（不参与随机池）。

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
    /// 失去全部金币（Tomb of Lord Red Mask 付费）
    LoseGoldAll,
    /// 失去生命（事件伤害不致死）
    LoseHp(i32),
    /// 失去最大生命的 N%（Forgotten Altar 放血）
    LoseHpMaxPct(i32),
    /// 获得生命
    GainHp(i32),
    /// 回复 N% 最大生命（Mushrooms 吃蘑菇）
    HealMaxPct(i32),
    /// 回满血
    HealFull,
    /// 最大生命 +N（同步当前生命——原版 increaseMaxHp）
    GainMaxHp(i32),
    /// 升级一张随机卡（神龛/牧师——原版多为选择，简化随机）
    UpgradeRandom,
    /// 升级牌组全部可升级卡（MindBloom 生长）
    UpgradeAll,
    /// 升级全部打击与防御（Back to Basics）
    UpgradeStrikesDefends,
    /// 移除一张卡（末尾；原版多为选择，简化）
    RemoveCard,
    /// 移除全部可移除诅咒（Fountain of Cleansing）
    RemoveAllCurses,
    /// 各移除一张技能/能力/攻击（Falling）
    RemoveTypedCards,
    /// 移除一件遗物（N'loth/WeMeetAgain——不回退数值效果，近似）
    RemoveRelic,
    /// 复制牌组中随机一张卡（Duplicator）
    DuplicateCard,
    /// 变换（移除随机卡并获得随机卡）（Transmogrifier/Drug Dealer）
    TransformCard,
    /// 获得随机普通遗物
    GainCommonRelic,
    /// 获得随机稀有遗物
    GainRareRelic,
    /// 获得随机稀有度遗物（miscRng 定稀有度——原版 returnRandomRelicTier）
    GainRandomRelic,
    /// 获得指定遗物（apply_relic_effect 接效果）
    GainRelic(&'static str),
    /// 获得一瓶药水（miscRng 抽——原版 PotionHelper）
    GainPotion,
    /// 失去一瓶药水（WeMeetAgain）
    LosePotion,
    /// 获得一张随机卡（常规色池：A/S/P）
    GainRandomCard,
    /// 获得 N 张随机无色卡（SensoryStone）
    GainColorlessCards(i32),
    /// 加默认诅咒（Regret——旧行为保留）
    AddCurse,
    /// 加指定诅咒
    AddCurseCard(&'static str),
    /// 复合效果（依序应用）
    And(&'static [Effect]),
    /// 无事发生
    Nothing,
}

pub struct EventDef {
    pub id: &'static str,
    pub title: &'static str,
    pub body: &'static str,
    pub options: &'static [EventOption],
    /// 所属幕：1/2/3 专属，0=神龛全幕，4=终幕（随机池排除）
    pub act: u8,
}

pub static EVENTS: &[EventDef] = &[
    // ================= Act 1（The Exordium）=================
    EventDef {
        id: "Golden Idol",
        act: 1,
        title: "黄金神像",
        body: "你看到一个黄金神像嵌在石座里。拿走它可能会触发陷阱……",
        options: &[
            EventOption { text: "拿走神像（+100金，获得遗物，失去 15 生命）", effect: Effect::GainGold(100) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Big Fish",
        act: 1,
        title: "大鱼",
        body: "水中有什么东西闪闪发光……",
        options: &[
            EventOption { text: "钓鱼（+50% 概率遗物，失去 5 生命）", effect: Effect::GainCommonRelic },
            EventOption { text: "吃掉它（回 12 生命）", effect: Effect::GainHp(12) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Cleric",
        act: 1,
        title: "牧师",
        body: "一名牧师愿意为你提供服务。",
        options: &[
            EventOption { text: "治疗（35 金，回 20% 最大生命）", effect: Effect::GainHp(20) },
            EventOption { text: "祈愿（50 金，升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Dead Adventurer",
        act: 1,
        title: "死去的冒险者",
        body: "冒险者的尸体旁散落着物品。",
        options: &[
            EventOption { text: "搜刮（+50 金，失去 6 生命）", effect: Effect::GainGold(50) },
            EventOption { text: "埋葬（+普通遗物）", effect: Effect::GainCommonRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Golden Wing",
        act: 1,
        title: "金翼石像",
        body: "石像展开一对金翼，水晶在翼下闪烁。（GoldenWing.java：50-80 金随机，取 65 近似）",
        options: &[
            EventOption { text: "敲碎水晶（失去 10 生命，+50~80 金）", effect: Effect::And(&[Effect::LoseHp(10), Effect::GainGold(65)]) },
            EventOption { text: "摘下金翼（移除一张卡，失去 10 生命）", effect: Effect::And(&[Effect::RemoveCard, Effect::LoseHp(10)]) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "World of Goop",
        act: 1,
        title: "黏液世界",
        body: "地上全是黏液。",
        options: &[
            EventOption { text: "捡起（失去 5 生命，+75 金）", effect: Effect::GainGold(75) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Living Wall",
        act: 1,
        title: "活墙",
        body: "墙上浮现出一张脸，要求你献祭。",
        options: &[
            EventOption { text: "献祭卡（移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "献祭血（失去 6 生命）", effect: Effect::LoseHp(6) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Mushrooms",
        act: 1,
        title: "蘑菇",
        body: "一丛发光的蘑菇。（Mushrooms.java：治疗 25% + 寄生诅咒；踩碎→战斗+奇菇遗物，战斗 TODO）",
        options: &[
            EventOption { text: "吃掉它们（回复 25% 最大生命，+寄生诅咒）", effect: Effect::And(&[Effect::HealMaxPct(25), Effect::AddCurseCard("Parasite")]) },
            EventOption { text: "踩碎它们（战斗→+普通遗物）", effect: Effect::And(&[Effect::LoseHp(8), Effect::GainCommonRelic]) },
        ],
    },
    EventDef {
        id: "Liars Game",
        act: 1,
        title: "骗子游戏",
        body: "蛇形商人摆出壳游戏。（Sssserpent.java：175 金（A15+: 150）+ 疑虑）",
        options: &[
            EventOption { text: "答应（+175 金，+疑虑诅咒）", effect: Effect::And(&[Effect::GainGold(175), Effect::AddCurseCard("Doubt")]) },
            EventOption { text: "拒绝", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Lab",
        act: 1,
        title: "实验室",
        body: "一间废弃的实验室，药剂台上摆满了瓶子。（Lab.java：3 瓶随机药水入奖励）",
        options: &[
            EventOption { text: "搜刮药剂（+3 瓶药水）", effect: Effect::And(&[Effect::GainPotion, Effect::GainPotion, Effect::GainPotion]) },
        ],
    },
    EventDef {
        id: "NoteForYourself",
        act: 1,
        title: "给自己的笔记",
        body: "一张泛黄的便签钉在树上。（NoteForYourself.java：获得铁斩波+，可选移除一张）",
        options: &[
            EventOption { text: "读笔记（+1 张随机卡）", effect: Effect::GainRandomCard },
            EventOption { text: "收好便签（移除一张卡）", effect: Effect::RemoveCard },
        ],
    },
    EventDef {
        id: "Scrap Ooze",
        act: 1,
        title: "废料软泥怪",
        body: "软泥怪身上缠着什么金属。",
        options: &[
            EventOption { text: "伸手（失去 4 生命，+50 金）", effect: Effect::GainGold(50) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Shining Light",
        act: 1,
        title: "闪亮之光",
        body: "一束光从裂缝中透出。",
        options: &[
            EventOption { text: "进入（失去 4 生命，升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Wheel of Change",
        act: 1,
        title: "哥布林轮盘",
        body: "哥布林邀请你转轮盘。（数据表 id = Wheel of Change，类 GremlinWheelGame）",
        options: &[
            EventOption { text: "转一次（+25% 概率赢金）", effect: Effect::GainGold(60) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Adventurer",
        act: 1,
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
        act: 1,
        title: "矿坑",
        body: "废弃的矿坑深处有金属反光。",
        options: &[
            EventOption { text: "挖掘（+75 金）", effect: Effect::GainGold(75) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    // ================= Act 2（The City）=================
    EventDef {
        id: "The Library",
        act: 2,
        title: "图书馆",
        body: "一座漂浮的图书馆静候访客。",
        options: &[
            EventOption { text: "阅读（升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "抄写（+诅咒）", effect: Effect::AddCurse },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Mausoleum",
        act: 2,
        title: "陵墓",
        body: "陵墓的门微微敞开着。",
        options: &[
            EventOption { text: "进入（+诅咒）", effect: Effect::AddCurse },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Vampires",
        act: 2,
        title: "吸血鬼",
        body: "吸血鬼提出了一个危险的交易。",
        options: &[
            EventOption { text: "接受（移除打击卡，+诅咒）", effect: Effect::AddCurse },
            EventOption { text: "拒绝", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Joust",
        act: 2,
        title: "马上比武",
        body: "两名骑士在远处比武。",
        options: &[
            EventOption { text: "下注（失去 50 金，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "旁观", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Masked Bandits",
        act: 2,
        title: "蒙面强盗",
        body: "强盗们要求你交出金币。",
        options: &[
            EventOption { text: "战斗（失去 10 生命，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "交钱（失去全部金币）", effect: Effect::LoseGoldAll },
        ],
    },
    EventDef {
        id: "Nest",
        act: 2,
        title: "巢穴",
        body: "一个巨大的巢穴在头顶。",
        options: &[
            EventOption { text: "偷蛋（+普通遗物，失去 8 生命）", effect: Effect::GainCommonRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Woman in Blue",
        act: 2,
        title: "蓝衣女子",
        body: "蓝衣女子出售她的药剂。",
        options: &[
            EventOption { text: "买药水（50 金，+药水）", effect: Effect::GainPotion },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "White Beast Statue",
        act: 2,
        title: "白兽像",
        body: "白兽像的眼睛发出光芒。",
        options: &[
            EventOption { text: "触摸（+药水）", effect: Effect::GainPotion },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Addict",
        act: 2,
        title: "瘾君子",
        body: "一个瘾君子向你兜售他的存货。（Addict.java：85 金→随机稀有度遗物；偷药→J.A.X.+随机遗物）",
        options: &[
            EventOption { text: "注射（-85 金，+随机遗物）", effect: Effect::And(&[Effect::LoseGold(85), Effect::GainRandomRelic]) },
            EventOption { text: "偷药（+J.A.X.，+随机遗物）", effect: Effect::And(&[Effect::GainRandomCard, Effect::GainRandomRelic]) },
        ],
    },
    EventDef {
        id: "Back to Basics",
        act: 2,
        title: "返璞归真",
        body: "一位隐士向你传授基础之道。（BackToBasics.java：upgradeStrikeAndDefends）",
        options: &[
            EventOption { text: "返璞归真（升级全部打击与防御）", effect: Effect::UpgradeStrikesDefends },
            EventOption { text: "断舍离（移除一张卡）", effect: Effect::RemoveCard },
        ],
    },
    EventDef {
        id: "Beggar",
        act: 2,
        title: "乞丐",
        body: "一名乞丐请求施舍。（Beggar.java：75 金→移除一张卡）",
        options: &[
            EventOption { text: "施舍（-75 金，移除一张卡）", effect: Effect::And(&[Effect::LoseGold(75), Effect::RemoveCard]) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Colosseum",
        act: 2,
        title: "竞技场",
        body: "观众的欢呼声从竞技场传来。（Colosseum.java：两连战+罕见/稀有遗物，战斗 TODO 近似）",
        options: &[
            EventOption { text: "参加（连战两场，失去 10 生命，+稀有遗物）", effect: Effect::And(&[Effect::LoseHp(10), Effect::GainRareRelic]) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Cursed Tome",
        act: 2,
        title: "诅咒之书",
        body: "一本漆黑的书摊开着。（CursedTome.java：逐页 1/2/3/3 伤害；取书 +10（A2+: 15）——收敛为两选项）",
        options: &[
            EventOption { text: "读到头（共失去 19 生命，+随机书遗物）", effect: Effect::And(&[Effect::LoseHp(19), Effect::GainRandomRelic]) },
            EventOption { text: "停手（失去 3 生命）", effect: Effect::LoseHp(3) },
        ],
    },
    EventDef {
        id: "Drug Dealer",
        act: 2,
        title: "毒贩",
        body: "阴影里的毒贩招了招手。（DrugDealer.java：变异之力遗物 / J.A.X. / 变换 2 张）",
        options: &[
            EventOption { text: "注射变异原体（+突变之力）", effect: Effect::GainRelic("MutagenicStrength") },
            EventOption { text: "吸一口（+J.A.X.）", effect: Effect::GainRandomCard },
            EventOption { text: "成为实验体（变换 2 张卡）", effect: Effect::And(&[Effect::TransformCard, Effect::TransformCard]) },
        ],
    },
    EventDef {
        id: "Forgotten Altar",
        act: 2,
        title: "被遗忘的祭坛",
        body: "血迹斑斑的祭坛等待着献祭。（ForgottenAltar.java：放血 25%（A15+: 35%）最大生命 +5 上限；亵渎→腐朽；金像→血之偶像）",
        options: &[
            EventOption { text: "献上金像（+血之偶像）", effect: Effect::GainRelic("Bloody Idol") },
            EventOption { text: "放血（失去 25% 最大生命，+5 最大生命）", effect: Effect::And(&[Effect::LoseHpMaxPct(25), Effect::GainMaxHp(5)]) },
            EventOption { text: "亵渎（+腐朽诅咒）", effect: Effect::AddCurseCard("Decay") },
        ],
    },
    EventDef {
        id: "Knowing Skull",
        act: 2,
        title: "知之颅骨",
        body: "一颗颅骨低声提出交易。（KnowingSkull.java：药 6/金 8（90）/卡 12 生命；离开付剩余——收敛为单轮）",
        options: &[
            EventOption { text: "买情报·药水（失去 6 生命，+药水）", effect: Effect::And(&[Effect::LoseHp(6), Effect::GainPotion]) },
            EventOption { text: "买情报·金币（失去 8 生命，+90 金）", effect: Effect::And(&[Effect::LoseHp(8), Effect::GainGold(90)]) },
            EventOption { text: "买情报·卡（失去 12 生命，+随机卡）", effect: Effect::And(&[Effect::LoseHp(12), Effect::GainRandomCard]) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "N'loth",
        act: 2,
        title: "恩洛斯",
        body: "一只奇异的生物盯着你的背包。（Nloth.java：移除一件遗物换 N'loth's Gift——简化为随机遗物）",
        options: &[
            EventOption { text: "献上遗物（移除一件遗物，+随机遗物）", effect: Effect::And(&[Effect::RemoveRelic, Effect::GainRandomRelic]) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "WeMeetAgain",
        act: 2,
        title: "又见面了",
        body: "一张熟悉的面孔出现在集市。（WeMeetAgain.java：药水/金币（≤150）/卡→随机稀有度遗物）",
        options: &[
            EventOption { text: "给药水（-1 瓶药水，+随机遗物）", effect: Effect::And(&[Effect::LosePotion, Effect::GainRandomRelic]) },
            EventOption { text: "付钱（-150 金，+随机遗物）", effect: Effect::And(&[Effect::LoseGold(150), Effect::GainRandomRelic]) },
            EventOption { text: "给卡（移除一张卡，+随机遗物）", effect: Effect::And(&[Effect::RemoveCard, Effect::GainRandomRelic]) },
        ],
    },
    EventDef {
        id: "The Graveyard",
        act: 2,
        title: "墓地",
        body: "墓碑林立，似乎有东西埋在浅土里。",
        options: &[
            EventOption { text: "挖掘（+诅咒，+50 金）", effect: Effect::GainGold(50) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    // ================= Act 3（The Beyond）=================
    EventDef {
        id: "Winding Halls",
        act: 3,
        title: "回廊",
        body: "扭曲的回廊在你面前展开。",
        options: &[
            EventOption { text: "继续走（失去 5 生命，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "原路返回", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Mysterious Sphere",
        act: 3,
        title: "神秘球体",
        body: "球体内部有什么在流动。",
        options: &[
            EventOption { text: "触摸（失去 5 生命，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Tomb of Lord Red Mask",
        act: 3,
        title: "红面具勋爵之墓",
        body: "墓室里堆满了陪葬品。（TombRedMask.java：222 金 / 全部金币换红面具）",
        options: &[
            EventOption { text: "搜刮（+222 金）", effect: Effect::GainGold(222) },
            EventOption { text: "献上全部金币（+红面具）", effect: Effect::And(&[Effect::LoseGoldAll, Effect::GainRelic("Red Mask")]) },
        ],
    },
    EventDef {
        id: "Ghosts",
        act: 3,
        title: "鬼影",
        body: "鬼影在雾中若隐若现。",
        options: &[
            EventOption { text: "跟随（+50 金，失去 8 生命）", effect: Effect::GainGold(50) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Moai Head",
        act: 3,
        title: "摩艾石像",
        body: "石像的嘴里流出金色液体。",
        options: &[
            EventOption { text: "喝下（+最大生命 5）", effect: Effect::GainMaxHp(5) },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Top",
        act: 3,
        title: "陀螺",
        body: "一个精美的陀螺静静躺在地上。",
        options: &[
            EventOption { text: "捡起（+普通遗物）", effect: Effect::GainCommonRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Falling",
        act: 3,
        title: "坠落",
        body: "脚下的大地突然崩塌……（Falling.java：技能/能力/攻击各移除顶卡一张；无牌则无事）",
        options: &[
            EventOption { text: "抓住岩壁（各移除 1 张技能/能力/攻击）", effect: Effect::RemoveTypedCards },
            EventOption { text: "放弃", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "MindBloom",
        act: 3,
        title: "心灵绽放",
        body: "一朵巨大的花在雾中绽放。（MindBloom.java：挖掘→稀有遗物+马可之花；生长→全升级+马可之花；沐浴→回满血+999金+2 凡庸。马可之花的治疗封锁未接入 3.6）",
        options: &[
            EventOption { text: "挖掘（+稀有遗物，+马可之花）", effect: Effect::GainRareRelic },
            EventOption { text: "生长（全部可升级卡升级）", effect: Effect::UpgradeAll },
            EventOption { text: "沐浴（回满血，+999 金，+2 张凡庸）", effect: Effect::And(&[Effect::HealFull, Effect::GainGold(999), Effect::AddCurseCard("Normality"), Effect::AddCurseCard("Normality")]) },
        ],
    },
    EventDef {
        id: "SensoryStone",
        act: 3,
        title: "记忆之石",
        body: "一块 stone 记录着无数冒险者的记忆。（SensoryStone.java：回忆 N 段→N 张无色卡；回忆 2/3 额外 5/10 伤害）",
        options: &[
            EventOption { text: "回忆一段（+1 张无色卡）", effect: Effect::GainColorlessCards(1) },
            EventOption { text: "回忆两段（+2 张无色卡，失去 5 生命）", effect: Effect::And(&[Effect::GainColorlessCards(2), Effect::LoseHp(5)]) },
            EventOption { text: "回忆三段（+3 张无色卡，失去 10 生命）", effect: Effect::And(&[Effect::GainColorlessCards(3), Effect::LoseHp(10)]) },
        ],
    },
    EventDef {
        id: "SecretPortal",
        act: 3,
        title: "秘密传送门",
        body: "一道传送门在你面前打开。（SecretPortal.java：进入→直达 Boss 房；传送语义 TODO 4.x）",
        options: &[
            EventOption { text: "进入（直达 Boss）", effect: Effect::Nothing },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Duplicator",
        act: 3,
        title: "复制者",
        body: "一台奇怪的机器闪烁着微光。（Duplicator.java：选 1 张牌组卡获得复制——简化随机）",
        options: &[
            EventOption { text: "复制（+1 张牌组卡的复制）", effect: Effect::DuplicateCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "FaceTrader",
        act: 3,
        title: "换脸商人",
        body: "帷幕后面传出窃窃私语。（FaceTrader.java：触摸→金+伤（50-99/3-7，取 75/5）；换脸→面具遗物）",
        options: &[
            EventOption { text: "触摸（+75 金，失去 5 生命）", effect: Effect::And(&[Effect::GainGold(75), Effect::LoseHp(5)]) },
            EventOption { text: "换脸（+邪教徒头套）", effect: Effect::GainRelic("CultistMask") },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Test",
        act: 3,
        title: "试炼",
        body: "一个声音要求你证明自己。",
        options: &[
            EventOption { text: "接受（失去 12 生命，+稀有遗物）", effect: Effect::GainRareRelic },
            EventOption { text: "拒绝", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Bird Cave",
        act: 3,
        title: "鸟洞",
        body: "洞穴深处传来鸟鸣。",
        options: &[
            EventOption { text: "探索（+普通遗物，失去 4 生命）", effect: Effect::GainCommonRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Castle",
        act: 3,
        title: "城堡",
        body: "一座废弃的城堡矗立在山丘上。",
        options: &[
            EventOption { text: "探索（+稀有遗物，失去 12 生命）", effect: Effect::GainRareRelic },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Bonfire Spirits",
        act: 3,
        title: "篝火之灵",
        body: "火堆旁的低语邀请你献祭一张卡。",
        options: &[
            EventOption { text: "献祭（移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "The Spirits",
        act: 3,
        title: "低语之灵",
        body: "灵体的低语围绕着你。",
        options: &[
            EventOption { text: "倾听（移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    // ================= 神龛（全幕可遇，act 0）=================
    EventDef {
        id: "Ancient Writing",
        act: 0,
        title: "古老文字",
        body: "石碑上的文字向你低语。",
        options: &[
            EventOption { text: "研读（升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "抄录（+诅咒）", effect: Effect::AddCurse },
        ],
    },
    EventDef {
        id: "Golden Shrine",
        act: 0,
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
        act: 0,
        title: "被诅咒的铁匠",
        body: "铁匠愿意为你打造，但需要献祭。",
        options: &[
            EventOption { text: "锻造（升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Designer",
        act: 0,
        title: "设计师",
        body: "一位神秘设计师提出交易。",
        options: &[
            EventOption { text: "购买服务（75 金，升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Purifier",
        act: 0,
        title: "净化者",
        body: "一位旅人愿意为你净化一张卡。",
        options: &[
            EventOption { text: "净化（75 金，移除一张卡）", effect: Effect::RemoveCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Wandering Apprentice",
        act: 0,
        title: "流浪学徒",
        body: "一名学徒请求指导。",
        options: &[
            EventOption { text: "指导（升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Transmorgrifier",
        act: 0,
        title: "变形祭坛",
        body: "祭坛上的符文缓缓旋转。（Transmogrifier.java：选 1 张变形——简化随机）",
        options: &[
            EventOption { text: "变形（变换一张卡）", effect: Effect::TransformCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Upgrade Shrine",
        act: 0,
        title: "升级神龛",
        body: "神龛闪耀着升阶之力。（UpgradeShrine.java：选 1 张升级——简化随机）",
        options: &[
            EventOption { text: "祈祷（升级一张卡）", effect: Effect::UpgradeRandom },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Fountain of Cleansing",
        act: 0,
        title: "净化之泉",
        body: "泉水泛着圣洁的光。（FountainOfCurseRemoval.java：移除全部可移除诅咒）",
        options: &[
            EventOption { text: "饮用（移除全部诅咒）", effect: Effect::RemoveAllCurses },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    EventDef {
        id: "Bonfire Elementals",
        act: 0,
        title: "篝火元素",
        body: "篝火中浮现出两张面孔。（Bonfire.java：献卡按稀有度回报——普通→精灵便便/罕见→回满血/稀有→回满血+10 上限）",
        options: &[
            EventOption { text: "献上普通卡（+精灵便便诅咒）", effect: Effect::And(&[Effect::RemoveCard, Effect::AddCurseCard("Spirit Poop")]) },
            EventOption { text: "献上罕见卡（回满血）", effect: Effect::And(&[Effect::RemoveCard, Effect::HealFull]) },
            EventOption { text: "献上稀有卡（回满血，+10 最大生命）", effect: Effect::And(&[Effect::RemoveCard, Effect::HealFull, Effect::GainMaxHp(10)]) },
        ],
    },
    EventDef {
        id: "Match and Keep!",
        act: 0,
        title: "配对游戏",
        body: "一群哥布林在玩翻牌游戏。（GremlinMatchGame.java：翻牌+配对——简化为随机卡）",
        options: &[
            EventOption { text: "参与（+1 张随机卡）", effect: Effect::GainRandomCard },
            EventOption { text: "离开", effect: Effect::Nothing },
        ],
    },
    // ================= Act 4（The Ending）=================
    EventDef {
        id: "Spire Heart",
        act: 4,
        title: "尖塔之心",
        body: "尖塔的心脏在你面前搏动。（SpireHeart.java：触摸→最终战斗——由楼层流程直入 Boss）",
        options: &[
            EventOption { text: "触摸尖塔之心（最终战斗）", effect: Effect::Nothing },
        ],
    },
];

/// 按 ID 查事件（未知 → 首个事件兜底）。
pub fn get(id: &str) -> &'static EventDef {
    EVENTS.iter().find(|e| e.id == id).unwrap_or(&EVENTS[0])
}

/// 随机选取事件（终幕事件排除——SpireHeart 只在 Act 4 流程直入）。
pub fn roll(rng: &mut sts_data::rng::StsRandom) -> &'static EventDef {
    let pool: Vec<&EventDef> = EVENTS.iter().filter(|e| e.act != 4).collect();
    pool[rng.random_int_inclusive(pool.len() as i32 - 1) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_data::gen;

    /// 数据表 52 个事件 id 全覆盖对账（批次 4d 收尾验收）。
    #[test]
    fn all_table_ids_covered() {
        for ev in gen::EVENTS.iter() {
            let def = EVENTS.iter().find(|e| e.id == ev.id);
            assert!(def.is_some(), "数据表事件 {} 未转译", ev.id);
        }
    }

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

    #[test]
    fn roll_excludes_act4() {
        let mut rng = sts_data::rng::StsRandom::new(42);
        for _ in 0..200 {
            assert_ne!(roll(&mut rng).act, 4);
        }
    }
}
