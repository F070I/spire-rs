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
