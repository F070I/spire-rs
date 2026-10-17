//! 真怪工厂（批次 4c）：五真怪状态机构造。
//!
//! 数值按原版构造器逐条定格（ascension 阈值 A2/A4/A7/A9/A17/A19）：
//! - Hexaghost   exordium/Hexaghost.java   HP 250（A9+: 264）
//! - Orb Walker  beyond/OrbWalker.java     HP 90-96（A7+: 92-102，走表 roll）
//! - Donu & Deca beyond/Donu.java/Deca.java HP 各 250（A9+: 265）
//! - AwakenedOne beyond/AwakenedOne.java   HP 300（A9+: 320）
//!
//! 遭遇名以 encounters.rs 池键为准（"Donu & Deca" 为单体遭遇键，展开双怪）。

use crate::battle::{Monster, SpecialAi};

pub fn build_real(id: &str, asc: u8, hp: i32) -> Option<Vec<Monster>> {
    let a2 = asc >= 2;
    let a4 = asc >= 4;
    let a9 = asc >= 9;
    let a17 = asc >= 17;
    let a19 = asc >= 19;
    match id {
        "Hexaghost" => Some(vec![Monster::real(
            "Hexaghost",
            if a9 { 264 } else { 250 },
            SpecialAi::HexaghostFull {
                fire_tackle: if a4 { 6 } else { 5 },
                inferno: if a4 { 3 } else { 2 },
                str_amt: if a19 { 3 } else { 2 },
                sear_burn: if a19 { 2 } else { 1 },
            },
        )]),
        "Orb Walker" => Some(vec![Monster::real(
            "Orb Walker",
            hp,
            SpecialAi::OrbWalkerAi {
                laser: if a2 { 11 } else { 10 },
                claw: if a2 { 16 } else { 15 },
                str_up: if a17 { 5 } else { 3 },
            },
        )]),
        "Donu & Deca" => {
            let beam = if a4 { 12 } else { 10 };
            let artifact = if a19 { 3 } else { 2 };
            let mut deca = Monster::real(
                "Deca",
                hp,
                SpecialAi::DecaAi {
                    beam,
                    plated: if a19 { 3 } else { 0 },
                    artifact,
                },
            );
            // Deca 开局攻击（isAttacking = true——与 Donu 反相）
            deca.brain.attacking = true;
            // 源码顺序：Deca 在前（MonsterHelper "Donu and Deca"）
            Some(vec![
                deca,
                Monster::real("Donu", hp, SpecialAi::DonuAi { beam, artifact }),
            ])
        }
        "Awakened One" => {
            // 源码组合：双教徒护卫（Cultist(-590), Cultist(-298), AwakenedOne(100)）
            let mut cultist = Monster::scripted("Cultist", 51, vec![Some(6)]);
            cultist.special = Some(SpecialAi::Ritual(3));
            Some(vec![
                cultist,
                Monster::real(
                    "Awakened One",
                    if a9 { 320 } else { 300 },
                    SpecialAi::AwakenedOneAi {
                        regen: if a19 { 15 } else { 10 },
                        curiosity: if a19 { 2 } else { 1 },
                        str_pre: if a4 { 2 } else { 0 },
                    },
                ),
            ])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_id_is_none() {
        assert!(build_real("Cultist", 0, 50).is_none());
    }

    #[test]
    fn donu_deca_is_pair_with_same_beam() {
        let v = build_real("Donu & Deca", 0, 250).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].creature.name, "Deca", "源码顺序 Deca 在前");
        assert_eq!(v[1].creature.name, "Donu");
        assert_eq!(v[0].creature.max_health, 250);
    }

    #[test]
    fn awakened_one_has_cultist_escort() {
        let v = build_real("Awakened One", 0, 300).unwrap();
        assert_eq!(v.len(), 2, "教徒 + Awakened One");
        assert_eq!(v[0].creature.name, "Cultist");
        assert_eq!(v[1].creature.name, "Awakened One");
    }

    #[test]
    fn ascension_scales() {
        let h = build_real("Hexaghost", 9, 250).unwrap();
        assert_eq!(h[0].creature.max_health, 264);
        let aw = build_real("Awakened One", 19, 300).unwrap();
        match aw.last().unwrap().special {
            Some(SpecialAi::AwakenedOneAi { regen, curiosity, str_pre }) => {
                assert_eq!((regen, curiosity, str_pre), (15, 2, 2));
            }
            _ => panic!("应有 AwakenedOneAi"),
        }
    }
}
