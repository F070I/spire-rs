//! 自动生成 —— 勿手改（tools/extract.py）。
//! 由 tools/extract.py 从反编译源码与本地化 JSON（eng + zhs）机械抽取。
//! 静态数据在此；行为（use()/升级/钩子方法体）在 sts-core 按批次人工语义转译。
#![allow(dead_code)]

pub mod cards;
pub mod events;
pub mod keywords;
pub mod monsters;
pub mod potions;
pub mod powers;
pub mod relics;
pub mod types;

pub use cards::{CardStatic, CARDS};
pub use cards::{
    FLAG_ETHEREAL, FLAG_EXHAUST, FLAG_EXHAUST_ON_USE_ONCE, FLAG_INNATE, FLAG_PURGE_ON_USE,
    FLAG_RETAIN, FLAG_RETURN_TO_HAND, FLAG_SELF_RETAIN, FLAG_SHUFFLE_BACK, TAG_EMPTY, TAG_HEALING,
    TAG_STARTER_DEFEND, TAG_STARTER_STRIKE, TAG_STRIKE, UPF_BLOCK, UPF_COST, UPF_DMG, UPF_MAGIC,
};
pub use events::{EventId, EVENTS};
pub use keywords::{Keyword, KEYWORDS};
pub use monsters::{MonsterId, MONSTERS};
pub use potions::{PotionStatic, POTIONS};
pub use powers::{PowerId, POWERS};
pub use relics::{RelicStatic, RELICS};
pub use types::*;

/// 按规范 ID 查卡（原版 ID 字符串是硬契约：存档/本地化键/字面量判断均依赖）。
pub fn card(id: &str) -> Option<&'static CardStatic> {
    CARDS.iter().find(|c| c.id == id)
}

pub fn relic(id: &str) -> Option<&'static RelicStatic> {
    RELICS.iter().find(|r| r.id == id)
}
