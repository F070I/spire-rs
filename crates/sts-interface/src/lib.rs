//! sts-interface：UI 与核心的统一协议（docs/04-UI规范与核心接口.md §2）。
//!
//! 冻结原则：
//! - UI 只提交玩家操作并展示核心计算结果；一切校验/结算/随机/状态变更由 sts-core 执行；
//! - 协议必须能表达完整游戏所需的全部交互（docs/01 清单），不得因 UI 未实现而删减；
//! - 阶段 0 先落枚举骨架（可编译、可序列化注记）；字段随批次 6.1 落地。
//!   serde 派生在存档模块（批次 5.1）引入 serde 依赖后统一补充。

/// 玩家操作（战斗中/外统一枚举）。
#[derive(Clone, Debug, PartialEq)]
pub enum PlayerCommand {
    // —— 战斗中 ——
    PlayCard {
        hand_index: usize,
        target: Option<usize>,
        x_energy: Option<u32>,
    },
    CancelTargeting,
    EndTurn,
    OpenPileView(PileKind),
    ClosePileView,
    InspectCard {
        card_index: usize,
    },
    UsePotion {
        slot: usize,
        target: Option<usize>,
    },
    DiscardPotion {
        slot: usize,
    },
    // —— 选择屏 ——
    HandSelectToggle {
        index: usize,
    },
    HandSelectConfirm,
    GridSelectToggle {
        index: usize,
    },
    GridSelectConfirm,
    GridSelectCancel,
    // —— 地图 ——
    ChooseMapNode {
        x: i32,
        y: i32,
    },
    // —— 奖励 ——
    ClaimReward {
        index: usize,
    },
    CardRewardPick {
        index: usize,
    },
    CardRewardSkip,
    SingingBowl,
    BossRelicPick {
        index: usize,
    },
    BossRelicSkip,
    // —— 房间 ——
    CampfireChoose(CampfireOption),
    ShopBuy {
        item: ShopItemKind,
        index: usize,
    },
    ShopPurge {
        deck_index: usize,
    },
    LeaveShop,
    EventChoose {
        index: usize,
    },
    // —— 桌面 UI 直连（对既有核心 API 的命令化包装）——
    OpenNeowPick {
        index: usize,
    },
    OpenTreasure,
    CampfireSmithPick {
        deck_index: usize,
    },
    CampfirePipePick {
        deck_index: usize,
    },
    // —— 查看 / 外壳 ——
    ViewDeck,
    CloseOverlay,
    OpenSettings,
    AbandonRun,
    ReturnToMenu,
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PileKind {
    Draw,
    Discard,
    Exhaust,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampfireOption {
    Rest,
    Smith,
    Dig,
    Lift,
    Recall,
    Toke,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopItemKind {
    Card,
    ColorlessCard,
    Relic,
    Potion,
}

/// 命令结果：接受/拒绝（含原版等价文案）+ 结算日志事件。
/// TODO(批次6.1)：LogEvent 完整结构（伤害/格挡/施加/触发……）。
#[derive(Clone, Debug)]
pub struct CommandResult {
    pub accepted: bool,
    pub rejection_reason: Option<String>,
}

impl CommandResult {
    pub fn accepted() -> Self {
        Self {
            accepted: true,
            rejection_reason: None,
        }
    }

    pub fn rejected(reason: impl Into<String>) -> Self {
        Self {
            accepted: false,
            rejection_reason: Some(reason.into()),
        }
    }
}

pub mod text;
pub mod view;
pub use text::{
    card_desc_raw, character_name_zhs, combat_eff, event_text, monster_name_zhs, potion_text,
    power_label, relic_text, stance_name_zhs, static_eff,
};
pub use view::{
    dispatch, BattleView, CampfireAction, CampfireView, CardView, EnemyView, EventView, GameView,
    MapEdge, MapNode, MapView, PileView, PotionView, PowerView, RewardView, Screen, ShopEntry,
    ShopView, TopBar,
};
