//! 遭遇池三幕权重表（docs/source-inventory/04 §6 全表）。
//!
//! 权重对齐原版 AbstractDungeon populateMonsterList 的 MonsterInfo 体系。
//! 抽取规则：普通怪禁 A-B-A 三连、精英仅禁相邻重复（记录 04 §6）。

use crate::rng::StsRandom;

/// 弱怪（前 3 场普通战）池：key + weight。
pub struct Weights {
    pub key: &'static str,
    pub weight: f32,
}

pub struct ActPools {
    pub weak: &'static [Weights],
    pub strong: &'static [Weights],
    pub elites: &'static [Weights],
    pub bosses: &'static [&'static str],
    /// 强怪预生成数量（Exordium 3 / TheCity 2 / TheBeyond 2）
    pub weak_count: usize,
    pub strong_count: usize,
    pub elite_count: usize,
}

pub static EXORDIUM: ActPools = ActPools {
    weak: &[
        Weights { key: "Cultist", weight: 2.0 },
        Weights { key: "JawWorm", weight: 2.0 },
        Weights { key: "2 Louse", weight: 2.0 },
        Weights { key: "Small Slimes", weight: 2.0 },
    ],
    strong: &[
        Weights { key: "Blue Slaver", weight: 2.0 },
        Weights { key: "Gremlin Gang", weight: 1.0 },
        Weights { key: "Looter", weight: 2.0 },
        Weights { key: "Large Slime", weight: 2.0 },
        Weights { key: "Lots of Slimes", weight: 1.0 },
        Weights { key: "Exordium Thugs", weight: 1.5 },
        Weights { key: "Exordium Wildlife", weight: 1.5 },
        Weights { key: "Red Slaver", weight: 1.0 },
        Weights { key: "3 Louse", weight: 2.0 },
        Weights { key: "2 Fungi", weight: 2.0 },
    ],
    elites: &[
        Weights { key: "Gremlin Nob", weight: 1.0 },
        Weights { key: "Lagavulin", weight: 1.0 },
        Weights { key: "3 Sentries", weight: 1.0 },
    ],
    bosses: &["The Guardian", "Hexaghost", "Slime Boss"],
    weak_count: 3,
    strong_count: 12,
    elite_count: 10,
};

pub static THE_CITY: ActPools = ActPools {
    weak: &[
        Weights { key: "Spheric Guardian", weight: 2.0 },
        Weights { key: "Chosen", weight: 2.0 },
        Weights { key: "Shell Parasite", weight: 2.0 },
        Weights { key: "3 Byrds", weight: 2.0 },
        Weights { key: "2 Thieves", weight: 2.0 },
    ],
    strong: &[
        Weights { key: "Chosen & Byrds", weight: 2.0 },
        Weights { key: "Sentry & Sphere", weight: 2.0 },
        Weights { key: "Snake Plant", weight: 6.0 },
        Weights { key: "Snecko", weight: 4.0 },
        Weights { key: "Centurion & Healer", weight: 6.0 },
        Weights { key: "Cultist & Chosen", weight: 3.0 },
        Weights { key: "3 Cultists", weight: 3.0 },
        Weights { key: "Shelled Parasite & Fungi", weight: 3.0 },
    ],
    elites: &[
        Weights { key: "Gremlin Leader", weight: 1.0 },
        Weights { key: "Slavers", weight: 1.0 },
        Weights { key: "Book of Stabbing", weight: 1.0 },
    ],
    bosses: &["Bronze Automaton", "The Collector", "Champ"],
    weak_count: 2,
    strong_count: 12,
    elite_count: 10,
};

pub static THE_BEYOND: ActPools = ActPools {
    weak: &[
        Weights { key: "3 Darklings", weight: 2.0 },
        Weights { key: "Orb Walker", weight: 2.0 },
        Weights { key: "3 Shapes", weight: 2.0 },
    ],
    strong: &[
        Weights { key: "Spire Growth", weight: 1.0 },
        Weights { key: "Transient", weight: 1.0 },
        Weights { key: "4 Shapes", weight: 1.0 },
        Weights { key: "Maw", weight: 1.0 },
        Weights { key: "Sphere & 2 Shapes", weight: 1.0 },
        Weights { key: "Jaw Worm Horde", weight: 1.0 },
        Weights { key: "3 Darklings", weight: 1.0 },
        Weights { key: "Writhing Mass", weight: 1.0 },
    ],
    elites: &[
        Weights { key: "Giant Head", weight: 2.0 },
        Weights { key: "Nemesis", weight: 2.0 },
        Weights { key: "Reptomancer", weight: 2.0 },
    ],
    bosses: &["Awakened One", "Time Eater", "Donu & Deca"],
    weak_count: 2,
    strong_count: 12,
    elite_count: 10,
};

pub static THE_ENDING: ActPools = ActPools {
    weak: &[],
    strong: &[],
    elites: &[Weights { key: "Shield and Spear", weight: 1.0 }],
    bosses: &["The Heart"],
    weak_count: 0,
    strong_count: 0,
    elite_count: 1,
};

/// 幕编号（0=Exordium, 1=TheCity, 2=TheBeyond, 3=TheEnding）→ 池。
pub fn act_pools(act: usize) -> &'static ActPools {
    match act {
        0 => &EXORDIUM,
        1 => &THE_CITY,
        2 => &THE_BEYOND,
        _ => &THE_ENDING,
    }
}

/// 加权抽取（monsterRng 驱动，记录 04 §6）。
pub fn roll(pools: &[Weights], rng: &mut StsRandom) -> &'static str {
    let total: f32 = pools.iter().map(|w| w.weight).sum();
    let roll = rng.random_f32() * total;
    let mut acc = 0.0;
    for w in pools {
        acc += w.weight;
        if roll < acc {
            return w.key;
        }
    }
    pools.last().map(|w| w.key).unwrap_or("Cultist")
}

/// populateMonsterList：普通怪禁 A-B-A 三连（i-- 重掷）、精英禁相邻重复。
pub fn populate_list(
    pools: &[Weights],
    count: usize,
    is_elite: bool,
    rng: &mut StsRandom,
) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    let mut i = 0;
    while i < count {
        let key = roll(pools, rng);
        if is_elite {
            // 精英：禁相邻重复
            if out.last() == Some(&key) {
                continue; // i-- 重掷语义
            }
        } else {
            // 普通怪：禁 A-B-A 三连（前两位相同且当前也相同 → 重掷）
            if out.len() >= 2 && out[out.len() - 1] == key && out[out.len() - 2] == key {
                continue;
            }
        }
        out.push(key);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exordium_pools_match_source() {
        // 出处：dungeons/Exordium.java:128-175
        assert_eq!(EXORDIUM.weak.len(), 4);
        assert_eq!(EXORDIUM.strong.len(), 10);
        assert_eq!(EXORDIUM.elites.len(), 3);
        assert_eq!(EXORDIUM.bosses.len(), 3);
        assert_eq!(EXORDIUM.weak_count, 3);
        assert_eq!(EXORDIUM.strong_count, 12);
        assert_eq!(EXORDIUM.elite_count, 10);
    }

    #[test]
    fn populate_no_aba_triples_for_normal() {
        let mut rng = StsRandom::new(42);
        let list = populate_list(&EXORDIUM.strong, 50, false, &mut rng);
        for i in 2..list.len() {
            assert!(
                !(list[i] == list[i - 1] && list[i - 1] == list[i - 2]),
                "A-B-A 三连违反 at index {i}"
            );
        }
    }

    #[test]
    fn populate_elite_no_adjacent_dups() {
        let mut rng = StsRandom::new(42);
        let list = populate_list(&EXORDIUM.elites, 20, true, &mut rng);
        for i in 1..list.len() {
            assert!(list[i] != list[i - 1], "精英相邻重复 at index {i}");
        }
    }
}
