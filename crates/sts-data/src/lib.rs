//! sts-data：静态游戏数据、ID、随机数算法、文本协议常量。
//!
//! 本 crate 不含游戏行为；只含数据与纯函数。
//! 数值常量的出处逐条标注原版文件（见 docs/source-inventory/）。

pub mod rng;

/// 原版规则常量（出处标注）。
pub mod config {
    /// `core/Settings.java:204`
    pub const MAX_HAND_SIZE: usize = 10;
    /// `core/Settings.java`
    pub const NUM_POTIONS: usize = 3;
    /// `core/Settings.java`
    pub const MAX_ASCENSION_LEVEL: u8 = 20;
    /// `dungeons/AbstractDungeon.java:202-205`
    pub const MAP_HEIGHT: usize = 15;
    pub const MAP_WIDTH: usize = 7;
    pub const MAP_DENSITY: usize = 6;
    pub const FINAL_ACT_MAP_HEIGHT: usize = 3;
    /// 房间概率（三幕一致，AbstractDungeon.generateRoomTypes:662）
    pub const SHOP_ROOM_CHANCE: f32 = 0.05;
    pub const REST_ROOM_CHANCE: f32 = 0.12;
    pub const EVENT_ROOM_CHANCE: f32 = 0.22;
    pub const ELITE_ROOM_CHANCE: f32 = 0.08;
    pub const TREASURE_ROOM_CHANCE: f32 = 0.0;
    /// 卡奖升级概率（Exordium 恒 0；City/Beyond 按 ascension 分档，见各幕构造）
    pub const COLORLESS_RARE_CHANCE: f32 = 0.3;
    /// 神龛概率（AbstractDungeon.generateEvent:2097）
    pub const SHRINE_CHANCE: f32 = 0.25;
    /// 药水稀有度（helpers/PotionHelper.java）
    pub const POTION_COMMON_CHANCE: i32 = 65;
    pub const POTION_UNCOMMON_CHANCE: i32 = 25;
    /// 药水掉落基础概率与保底步进（rooms/AbstractRoom.addPotionToRewards:644-670）
    pub const POTION_BASE_CHANCE: i32 = 40;
    pub const POTION_BLIZZARD_STEP: i32 = 10;
    /// 卡奖保底（AbstractDungeon:1625 附近）
    pub const CARD_BLIZZ_START_OFFSET: i32 = 5;
    pub const CARD_BLIZZ_RANDOMIZER_INIT: i32 = 5;
    pub const CARD_BLIZZ_GROWTH: i32 = 1;
    pub const CARD_BLIZZ_MAX_OFFSET: i32 = -40;
    /// 战后金币区间（rooms/AbstractRoom:304-347）
    pub const GOLD_MONSTER_MIN: i32 = 10;
    pub const GOLD_MONSTER_MAX: i32 = 20;
    pub const GOLD_ELITE_MIN: i32 = 25;
    pub const GOLD_ELITE_MAX: i32 = 35;
    pub const GOLD_BOSS_BASE: i32 = 100;
    /// 宝箱金币量（rewards/chests/*）
    pub const GOLD_SMALL_CHEST: i32 = 25;
    pub const GOLD_MEDIUM_CHEST: i32 = 50;
    pub const GOLD_LARGE_CHEST: i32 = 75;
    /// 商店（shop/ShopScreen.java）
    pub const PURGE_COST_BASE: i32 = 75;
    pub const PURGE_COST_RAMP: i32 = 25;
    pub const COLORLESS_PRICE_BUMP: f32 = 1.2;
}

/// 顶层游戏模式（对应 core/CardCrawlGame.GameMode，文字版外壳等价）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    CharSelect,
    Gameplay,
}

/// 屏幕枚举 —— 对应 `dungeons/AbstractDungeon.CurrentScreen`（23 值）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenKind {
    None,
    MasterDeckView,
    Settings,
    InputSettings,
    Grid,
    Map,
    Ftue,
    ChooseOne,
    HandSelect,
    Shop,
    CombatReward,
    DiscardView,
    ExhaustView,
    GameDeckView,
    BossReward,
    Death,
    CardReward,
    Transform,
    Victory,
    Unlock,
    DoorUnlock,
    Credits,
    NoInteract,
    NeowUnlock,
}
