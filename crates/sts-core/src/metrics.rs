//! 解锁与运行统计（批次 5 余量）：记忆式 prefs + 运行统计记录。
//!
//! 对应原版 UnlockTracker + StatsScreen（记录 07 §3/§9）：
//! - 角色解锁链（Silent 首结算 / Defect 用 Silent 通关 / Watcher 三宝石）
//! - 进阶等级（每角色，最高 20）
//! - 运行统计（胜场/到场层数/金币累计——本地记录，不上传）

use std::collections::HashMap;

/// 角色。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Character {
    Ironclad,
    TheSilent,
    Defect,
    Watcher,
}

impl Character {
    pub fn key(&self) -> &'static str {
        match self {
            Character::Ironclad => "IRONCLAD",
            Character::TheSilent => "THE_SILENT",
            Character::Defect => "DEFECT",
            Character::Watcher => "WATCHER",
        }
    }

    pub fn all() -> [Character; 4] {
        [
            Character::Ironclad,
            Character::TheSilent,
            Character::Defect,
            Character::Watcher,
        ]
    }
}

/// 解锁/统计 prefs（内存式；持久化由存档层合并）。
#[derive(Default, Clone, Debug)]
pub struct Prefs {
    /// 角色是否已解锁（默认仅铁甲）
    pub unlocked: HashMap<&'static str, bool>,
    /// 每角色进阶等级（0-20）
    pub ascension: HashMap<&'static str, u8>,
    /// 每角色胜场
    pub wins: HashMap<&'static str, u32>,
    /// 全局：最高到达层
    pub best_floor: i64,
    /// 全局：总胜场
    pub total_wins: u32,
}

impl Prefs {
    pub fn new() -> Self {
        let mut p = Self {
            unlocked: HashMap::new(),
            ascension: HashMap::new(),
            wins: HashMap::new(),
            best_floor: 0,
            total_wins: 0,
        };
        p.unlocked.insert("IRONCLAD", true); // 铁甲默认解锁
        for c in Character::all() {
            p.ascension.entry(c.key()).or_insert(0);
            p.wins.entry(c.key()).or_insert(0);
        }
        p
    }

    pub fn is_unlocked(&self, c: Character) -> bool {
        self.unlocked.get(c.key()).copied().unwrap_or(false)
    }

    /// 结算钩子：更新解锁链（记录 07 §3）。
    pub fn settle(&mut self, played: Character, won: bool, floor_reached: i64) {
        // 1) Silent：任意结算解锁
        if !self.is_unlocked(Character::TheSilent) {
            self.unlocked.insert("THE_SILENT", true);
        }
        // 2) Defect：用 Silent 通关解锁
        if won && played == Character::TheSilent && !self.is_unlocked(Character::Defect) {
            self.unlocked.insert("DEFECT", true);
        }
        // 3) Watcher：Defect 已解锁且用任意角色通关（宝石成就简化为胜利）
        if won && self.is_unlocked(Character::Defect) && !self.is_unlocked(Character::Watcher) {
            self.unlocked.insert("WATCHER", true);
        }
        // 4) 进阶：胜利后 +1（正在玩最高级且非种子局——简化）
        if won {
            let cur = self.ascension.entry(played.key()).or_insert(0);
            if *cur < 20 {
                *cur += 1;
            }
            self.total_wins += 1;
            *self.wins.entry(played.key()).or_insert(0) += 1;
        }
        // 5) 最高层
        if floor_reached > self.best_floor {
            self.best_floor = floor_reached;
        }
    }
}

/// 运行统计（MetricData 简化集；runs/ 文件输出 TODO）。
#[derive(Default, Clone, Debug)]
pub struct RunMetrics {
    pub floors: Vec<i64>,
    pub hp_per_floor: Vec<i32>,
    pub gold_per_floor: Vec<i32>,
    pub battles_won: u32,
    pub cards_picked: u32,
    pub cards_skipped: u32,
    pub potions_used: u32,
    pub relics_obtained: Vec<String>,
    pub campfire_rests: u32,
    pub campfire_smiths: u32,
    pub shop_purchases: u32,
    pub shop_purges: u32,
}

impl RunMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_floor(&mut self, floor: i64, hp: i32, gold: i32) {
        self.floors.push(floor);
        self.hp_per_floor.push(hp);
        self.gold_per_floor.push(gold);
    }

    pub fn on_victory(&mut self) {
        self.battles_won += 1;
    }

    pub fn on_card_pick(&mut self) {
        self.cards_picked += 1;
    }

    pub fn on_card_skip(&mut self) {
        self.cards_skipped += 1;
    }

    pub fn summary(&self) -> String {
        format!(
            "战斗胜 {} · 拾卡 {} 弃 {} · 篝火 休{} 锻{} · 商店 购{} 移{}",
            self.battles_won,
            self.cards_picked,
            self.cards_skipped,
            self.campfire_rests,
            self.campfire_smiths,
            self.shop_purchases,
            self.shop_purges,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlock_chain_progression() {
        let mut p = Prefs::new();
        assert!(p.is_unlocked(Character::Ironclad), "铁甲默认解锁");
        assert!(!p.is_unlocked(Character::TheSilent));
        // 首次结算解锁 Silent
        p.settle(Character::Ironclad, false, 3);
        assert!(p.is_unlocked(Character::TheSilent));
        // Silent 通关解锁 Defect
        p.settle(Character::TheSilent, true, 50);
        assert!(p.is_unlocked(Character::Defect));
        // 再通关解锁 Watcher
        p.settle(Character::TheSilent, true, 50);
        assert!(p.is_unlocked(Character::Watcher));
        // 进阶递增（Silent 胜 2 次 → +2）
        assert_eq!(p.ascension["THE_SILENT"], 2);
        assert_eq!(p.total_wins, 2);
    }

    #[test]
    fn ascension_caps_at_20() {
        let mut p = Prefs::new();
        for _ in 0..25 {
            p.settle(Character::Ironclad, true, 50);
        }
        assert_eq!(p.ascension["IRONCLAD"], 20);
    }

    #[test]
    fn metrics_track_actions() {
        let mut m = RunMetrics::new();
        m.on_floor(1, 80, 99);
        m.on_victory();
        m.on_card_pick();
        assert_eq!(m.floors.len(), 1);
        assert_eq!(m.battles_won, 1);
        assert!(m.summary().contains("战斗胜 1"));
    }
}
