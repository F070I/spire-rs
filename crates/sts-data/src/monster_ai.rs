//! 怪物 AI 数据表（批次 3.9 第一批）：HP 区间 / 招式脚本 / 升级门槛。
//!
//! 数据来源：docs/source-inventory/04 §1-4（记录 04）。
//! AI 行为：脚本化招式序列（attack/defend/buff 轮转），升阶数值仅覆盖常用档。
//! 真怪差异化（如 Guardian 三态、Hexaghost 六火）在核心逐个转译——本表为
//! 遭遇工厂可用的 HP/招式基础数据，保证普通战可玩。

/// 怪物静态 AI 条目。
pub struct MonsterAi {
    pub id: &'static str,
    /// HP 区间（ascension 0）：随机取值用 monsterHpRng。
    pub hp_min: i32,
    pub hp_max: i32,
    /// 招式脚本：(伤害, 是否防御) 循环；None = 防御回合。
    /// 简化序列：首批覆盖 [攻,攻,防] 或单攻循环——真怪逐个转译时替换。
    pub moves: &'static [Option<i32>],
    /// 遭遇类型：N 普通怪 / E 精英 / B Boss / S 召唤物。
    pub kind: char,
}

pub static AI: &[MonsterAi] = &[
    // ============ 第一幕（Exordium）============
    MonsterAi { id: "Cultist", hp_min: 48, hp_max: 54, moves: &[Some(6)], kind: 'N' }, // 仪式：每回合+3力
    MonsterAi { id: "Jaw Worm", hp_min: 40, hp_max: 44, moves: &[Some(11), Some(7), None], kind: 'N' },
    MonsterAi { id: "2 Louse", hp_min: 10, hp_max: 15, moves: &[Some(5), Some(6)], kind: 'N' },
    MonsterAi { id: "Small Slimes", hp_min: 8, hp_max: 12, moves: &[Some(5), None], kind: 'N' },
    MonsterAi { id: "Blue Slaver", hp_min: 46, hp_max: 50, moves: &[Some(12), Some(8)], kind: 'N' },
    MonsterAi { id: "Red Slaver", hp_min: 46, hp_max: 50, moves: &[Some(13), Some(8), None], kind: 'N' },
    MonsterAi { id: "Fungi Beast", hp_min: 22, hp_max: 28, moves: &[Some(6), None], kind: 'N' },
    MonsterAi { id: "Large Slime", hp_min: 28, hp_max: 32, moves: &[Some(10), None], kind: 'N' },
    MonsterAi { id: "Looter", hp_min: 44, hp_max: 48, moves: &[Some(10), Some(12)], kind: 'N' },
    MonsterAi { id: "Gremlin Gang", hp_min: 14, hp_max: 18, moves: &[Some(5), Some(6)], kind: 'N' },
    // 精英
    MonsterAi { id: "Gremlin Nob", hp_min: 82, hp_max: 86, moves: &[None, Some(14), Some(8)], kind: 'E' },
    MonsterAi { id: "Lagavulin", hp_min: 109, hp_max: 111, moves: &[None, Some(18)], kind: 'E' },
    MonsterAi { id: "3 Sentries", hp_min: 38, hp_max: 42, moves: &[Some(9), Some(9), None], kind: 'E' },
    // Boss
    MonsterAi { id: "The Guardian", hp_min: 240, hp_max: 240, moves: &[Some(30), Some(12), None], kind: 'B' },
    MonsterAi { id: "Hexaghost", hp_min: 250, hp_max: 250, moves: &[Some(7), Some(6), Some(5)], kind: 'B' },
    MonsterAi { id: "Slime Boss", hp_min: 140, hp_max: 140, moves: &[Some(18), None, Some(16)], kind: 'B' },
    // ============ 第二幕（The City）============
    MonsterAi { id: "Spheric Guardian", hp_min: 50, hp_max: 54, moves: &[None, Some(11)], kind: 'N' },
    MonsterAi { id: "Chosen", hp_min: 95, hp_max: 99, moves: &[Some(7), Some(5)], kind: 'N' },
    MonsterAi { id: "Shell Parasite", hp_min: 68, hp_max: 72, moves: &[Some(10), None], kind: 'N' },
    MonsterAi { id: "Byrd", hp_min: 26, hp_max: 30, moves: &[Some(7), Some(4)], kind: 'N' },
    MonsterAi { id: "Centurion", hp_min: 76, hp_max: 80, moves: &[Some(12), None], kind: 'N' },
    MonsterAi { id: "Healer", hp_min: 54, hp_max: 58, moves: &[None, Some(8)], kind: 'N' },
    MonsterAi { id: "Cultist", hp_min: 48, hp_max: 54, moves: &[Some(6)], kind: 'N' },
    MonsterAi { id: "Snake Plant", hp_min: 48, hp_max: 52, moves: &[Some(7)], kind: 'N' },
    MonsterAi { id: "Snecko", hp_min: 112, hp_max: 118, moves: &[Some(8), Some(14)], kind: 'N' },
    MonsterAi { id: "Bandit", hp_min: 48, hp_max: 54, moves: &[Some(10), Some(6)], kind: 'N' },
    // 精英
    MonsterAi { id: "Gremlin Leader", hp_min: 140, hp_max: 144, moves: &[None, Some(7)], kind: 'E' },
    MonsterAi { id: "Slavers", hp_min: 50, hp_max: 54, moves: &[Some(12), Some(13)], kind: 'E' },
    MonsterAi { id: "Book of Stabbing", hp_min: 65, hp_max: 69, moves: &[Some(6)], kind: 'E' },
    // Boss
    MonsterAi { id: "Bronze Automaton", hp_min: 300, hp_max: 300, moves: &[None, Some(9)], kind: 'B' },
    MonsterAi { id: "The Collector", hp_min: 222, hp_max: 222, moves: &[Some(18), None], kind: 'B' },
    MonsterAi { id: "Champ", hp_min: 266, hp_max: 266, moves: &[Some(16), Some(12)], kind: 'B' },
    // ============ 第三幕（The Beyond）============
    MonsterAi { id: "3 Darklings", hp_min: 50, hp_max: 54, moves: &[Some(7), Some(8)], kind: 'N' },
    // Orb Walker：90-96（A7+: 92-102——monsters::build_real 按升阶覆盖）
    MonsterAi { id: "Orb Walker", hp_min: 90, hp_max: 96, moves: &[Some(9), Some(11)], kind: 'N' },
    MonsterAi { id: "Spiker", hp_min: 30, hp_max: 34, moves: &[Some(5), Some(6)], kind: 'N' },
    MonsterAi { id: "Repulsor", hp_min: 30, hp_max: 34, moves: &[Some(6), None], kind: 'N' },
    MonsterAi { id: "Exploder", hp_min: 30, hp_max: 34, moves: &[Some(9)], kind: 'N' },
    MonsterAi { id: "Transient", hp_min: 999, hp_max: 999, moves: &[Some(30), Some(40)], kind: 'N' },
    MonsterAi { id: "Serpent", hp_min: 68, hp_max: 72, moves: &[Some(11), None], kind: 'N' },
    MonsterAi { id: "Writhing Mass", hp_min: 160, hp_max: 160, moves: &[Some(12), None], kind: 'N' },
    MonsterAi { id: "Maw", hp_min: 300, hp_max: 300, moves: &[Some(25), Some(5)], kind: 'N' },
    MonsterAi { id: "Giant Head", hp_min: 320, hp_max: 320, moves: &[Some(13), Some(26)], kind: 'E' },
    MonsterAi { id: "Nemesis", hp_min: 90, hp_max: 94, moves: &[Some(6), Some(45)], kind: 'E' },
    MonsterAi { id: "Reptomancer", hp_min: 180, hp_max: 190, moves: &[None, Some(13)], kind: 'E' },
    // Boss（真怪状态机见 monsters.rs；此表为 fallback 脚本数据）
    MonsterAi { id: "Awakened One", hp_min: 300, hp_max: 300, moves: &[Some(16), None], kind: 'B' },
    MonsterAi { id: "Time Eater", hp_min: 454, hp_max: 454, moves: &[Some(7), Some(20)], kind: 'B' },
    // Donu & Deca：遭遇键为单体（encounters.rs 池），各 250（A9+: 265）
    MonsterAi { id: "Donu & Deca", hp_min: 250, hp_max: 250, moves: &[Some(10), None], kind: 'B' },
    // ============ 第四幕（The Ending）============
    MonsterAi { id: "Spire Shield", hp_min: 140, hp_max: 140, moves: &[None, Some(18)], kind: 'N' },
    MonsterAi { id: "Spire Spear", hp_min: 140, hp_max: 140, moves: &[Some(9), None], kind: 'N' },
    MonsterAi { id: "The Heart", hp_min: 750, hp_max: 750, moves: &[Some(30), Some(40), Some(45)], kind: 'B' },
    // ============ 批次 5a：个体怪条目（多怪遭遇成员；HP/招式按构造器转译）============
    MonsterAi { id: "Louse", hp_min: 10, hp_max: 15, moves: &[Some(6), Some(6), None], kind: 'N' }, // 咬 5-7 取 6；强化回合以防御近似
    MonsterAi { id: "Acid Slime L", hp_min: 65, hp_max: 69, moves: &[Some(16), Some(10), None], kind: 'N' },
    MonsterAi { id: "Acid Slime M", hp_min: 28, hp_max: 32, moves: &[Some(10), Some(7), None], kind: 'N' },
    MonsterAi { id: "Acid Slime S", hp_min: 8, hp_max: 12, moves: &[Some(3), None], kind: 'N' },
    MonsterAi { id: "Spike Slime L", hp_min: 64, hp_max: 70, moves: &[Some(8), None], kind: 'N' },
    MonsterAi { id: "Spike Slime M", hp_min: 28, hp_max: 32, moves: &[Some(7), None], kind: 'N' },
    MonsterAi { id: "Spike Slime S", hp_min: 10, hp_max: 14, moves: &[Some(5)], kind: 'N' },
    MonsterAi { id: "Taskmaster", hp_min: 48, hp_max: 52, moves: &[None, Some(7)], kind: 'N' },
    MonsterAi { id: "Mugger", hp_min: 48, hp_max: 52, moves: &[Some(10), Some(12)], kind: 'N' },
    MonsterAi { id: "Gremlin Warrior", hp_min: 20, hp_max: 24, moves: &[Some(4)], kind: 'N' },
    MonsterAi { id: "Gremlin Thief", hp_min: 10, hp_max: 14, moves: &[Some(9)], kind: 'N' },
    MonsterAi { id: "Gremlin Fat", hp_min: 13, hp_max: 17, moves: &[Some(4), None], kind: 'N' },
    MonsterAi { id: "Gremlin Tsundere", hp_min: 12, hp_max: 15, moves: &[None, Some(6)], kind: 'N' },
    MonsterAi { id: "Gremlin Wizard", hp_min: 21, hp_max: 25, moves: &[None, None, Some(25)], kind: 'N' },
    MonsterAi { id: "Sentry", hp_min: 38, hp_max: 42, moves: &[Some(9), None], kind: 'N' },
    MonsterAi { id: "Bandit Pointy", hp_min: 30, hp_max: 30, moves: &[Some(5)], kind: 'N' },
    MonsterAi { id: "Bandit Leader", hp_min: 35, hp_max: 39, moves: &[Some(15), Some(10)], kind: 'N' },
    MonsterAi { id: "Bandit Bear", hp_min: 38, hp_max: 42, moves: &[Some(18), None, Some(9)], kind: 'N' },
    MonsterAi { id: "Darkling", hp_min: 48, hp_max: 56, moves: &[Some(8), Some(5), None], kind: 'N' },
    MonsterAi { id: "Spire Growth", hp_min: 170, hp_max: 170, moves: &[Some(14), None, Some(16)], kind: 'E' },
    MonsterAi { id: "Snake Dagger", hp_min: 20, hp_max: 24, moves: &[Some(9)], kind: 'S' },
    // ============ 召唤物 ============
    MonsterAi { id: "Dagger", hp_min: 10, hp_max: 12, moves: &[Some(9)], kind: 'S' },
    MonsterAi { id: "Bronze Orb", hp_min: 46, hp_max: 50, moves: &[None, Some(10)], kind: 'S' },
    MonsterAi { id: "Apology Slime", hp_min: 1, hp_max: 1, moves: &[None], kind: 'S' },
];

/// 按 ID 查 AI（未收录 id → Cultist 兜底——原版 fallback 语义）。
pub fn get(id: &str) -> &'static MonsterAi {
    AI.iter().find(|m| m.id == id).unwrap_or(&AI[0])
}

/// 按升阶修正 HP（记录 04 §1：ascension 2 起各怪加成——简化：统一 +7%/阶）。
pub fn scaled_hp(ai: &MonsterAi, ascension: u8, rng: &mut crate::rng::StsRandom) -> i32 {
    let base = rng.random_int_inclusive(ai.hp_max - ai.hp_min) + ai.hp_min;
    if ascension >= 2 {
        base + (base as f32 * 0.07 * (ascension as f32 - 1.0)) as i32
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_acts_have_bosses() {
        for boss in ["The Guardian", "Bronze Automaton", "Awakened One", "The Heart"] {
            let ai = get(boss);
            assert_eq!(ai.kind, 'B', "{} 应为 Boss", boss);
            assert!(ai.hp_min >= 140, "{} Boss HP 过低", boss);
        }
    }

    #[test]
    fn fallback_for_unknown_id() {
        let ai = get("Not A Monster");
        assert_eq!(ai.id, "Cultist"); // AI[0] 兜底
    }

    #[test]
    fn transient_hp_is_999() {
        assert_eq!(get("Transient").hp_min, 999);
    }
}
