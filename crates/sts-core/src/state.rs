//! 状态模型骨架 —— 替代原版 AbstractDungeon/CardCrawlGame 的全局静态海。
//!
//! 层级（docs/03-架构冻结.md §3.1）：
//! `Game`（外壳）→ `RunState`（一局）→ `DungeonState`（一幕）/ `BattleState`（一场战斗）。
//! 阶段 0 只落类型骨架；字段随批次 1.4 / 2.x / 4.x 逐批补全，补全时保持本文件结构。

use sts_data::rng::StsRandom;
use sts_data::{GameMode, ScreenKind};

/// 一条 RNG 流的句柄名（12 条 + map）。
/// 对应原版 AbstractDungeon 的静态 RNG 字段（记录 01 §4）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RngStream {
    Monster,
    Event,
    Merchant,
    Card,
    Treasure,
    Relic,
    MonsterHp,
    Potion,
    Ai,
    Shuffle,
    CardRandom,
    Misc,
    Map,
}

/// 屏幕栈：对应原版 screen / previousScreen / screenSwap（记录 08 §1）。
#[derive(Clone, Debug)]
pub struct ScreenStack {
    pub screen: ScreenKind,
    pub previous: Option<ScreenKind>,
    pub swap: bool,
}

impl Default for ScreenStack {
    fn default() -> Self {
        Self {
            screen: ScreenKind::None,
            previous: None,
            swap: false,
        }
    }
}

/// 一局爬塔的持久状态。
/// TODO(批次1.4)：player / actNum / floorNum / 12 条 RNG / 遭遇队列 / 三钥匙 / 模式标志。
pub struct RunState {
    pub seed: i64,
    /// RNG 注册表：流 →（种子，counter）。阶段 0 骨架：仅占位。
    /// 实际落地时每条流为 `StsRandom`，存档时序列化 (seed, counter) 全集（决策 D-02）。
    pub rngs: Vec<(RngStream, StsRandom)>,
}

/// 顶层游戏状态（原版 CardCrawlGame + AbstractDungeon 的收敛）。
pub struct Game {
    pub mode: GameMode,
    pub screens: ScreenStack,
    pub run: Option<RunState>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self {
            mode: GameMode::CharSelect,
            screens: ScreenStack::default(),
            run: None,
        }
    }

    /// TODO(批次1.4)：初始化一局（角色选择确认 → 12 条 RNG 同 seed 创建，记录 01 §4）。
    pub fn start_run(&mut self, seed: i64) {
        self.run = Some(RunState {
            seed,
            rngs: RngStream::all()
                .into_iter()
                .map(|s| (s, StsRandom::new(seed)))
                .collect(),
        });
        self.mode = GameMode::Gameplay;
    }
}

impl RngStream {
    pub fn all() -> [RngStream; 13] {
        [
            RngStream::Monster,
            RngStream::Event,
            RngStream::Merchant,
            RngStream::Card,
            RngStream::Treasure,
            RngStream::Relic,
            RngStream::MonsterHp,
            RngStream::Potion,
            RngStream::Ai,
            RngStream::Shuffle,
            RngStream::CardRandom,
            RngStream::Misc,
            RngStream::Map,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_run_creates_13_streams_same_seed() {
        let mut g = Game::new();
        g.start_run(424242);
        let run = g.run.as_ref().unwrap();
        assert_eq!(run.rngs.len(), 13);
        // 同 seed 下不同流的初始状态相同（对应原版"12 条流同 seed 异 counter"），
        // 差异在 counter 的演进；map 流另建。
        for (s, r) in &run.rngs {
            assert_eq!(r.counter, 0, "stream {:?} should start at counter 0", s);
        }
    }
}
