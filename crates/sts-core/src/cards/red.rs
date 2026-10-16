//! 铁甲战士卡牌效果（cards/red/，75 张）。
//!
//! 翻译规范见 docs/guides/card-translation.md；每条匹配arm的注释标注原版方法体依据。
//! HANDLED 列表 = 已转译卡（灌注进度核对）。

use super::builder::EffectBuilder;
use super::{attack_info, damage_matrix};
use crate::action::{GameAction, RoutineCall, TargetRef};
use crate::battle::{BattleState, CardInst};
use sts_data::gen;
use crate::power::{DamageType, Power, PowerKind};

pub const HANDLED: &[&str] = &[
    "Bash",
    "Body Slam",
    "Clash",
    "Clothesline",
    "Headbutt",
    "Heavy Blade",
    "Iron Wave",
    "Perfected Strike",
    "Pommel Strike",
    "Shrug It Off",
    "Thunderclap",
    "Twin Strike",
    "Warcry",
    "Wild Strike",
];

pub fn effects(b: &BattleState, card: &CardInst, target: Option<usize>, eb: &mut EffectBuilder) {
    match card.st().id {
        // cards/red/Bash.java use()：DamageAction + ApplyPowerAction(Vulnerable, magic)
        "Bash" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Vulnerable, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Body Slam：伤害 = 当前格挡（calculateCardDamage 重写：baseDamage = currentBlock）
        "Body Slam" => {
            if let Some(mi) = target {
                let base = b.player.current_block;
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, base, DamageType::Normal),
                });
            }
        }
        // Clash：本回合未打出非攻击牌才可用（canUse 重写）——打出效果：Damage
        "Clash" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Clothesline：Damage + ApplyPower(Weak, magic)
        "Clothesline" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Weak, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Headbutt：Damage + 把弃牌堆一张牌置于抽牌堆顶（HeadbuttAction 选择屏）→ 批次 6 交互
        "Headbutt" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "HeadbuttAction",
                        target: None,
                        amount: 0,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Heavy Blade：伤害 = baseDamage + 力量×(magic=3/5)（calculateCardDamage 重写）
        "Heavy Blade" => {
            if let Some(mi) = target {
                let str_amt = b
                    .player
                    .powers
                    .iter()
                    .find(|p| p.kind == PowerKind::Strength)
                    .map(|p| p.amount)
                    .unwrap_or(0);
                let mult = card.eff_magic(); // 3（+升级 5）
                let base = card.st().base_damage + str_amt * mult;
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, base, DamageType::Normal),
                });
            }
        }
        // Iron Wave：Damage + GainBlock（baseDamage 与 baseBlock 同为 magic 5/7）
        "Iron Wave" => {
            if let Some(mi) = target {
                let amt = card.eff_magic();
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, amt, DamageType::Normal),
                });
                eb.bot(GameAction::GainBlock {
                    target: TargetRef::Player,
                    amount: amt,
                });
            }
        }
        // Perfected Strike：baseDamage += magic × 全牌组 STRIKE 数（applyPowers 重写，记录 02 §7）
        "Perfected Strike" => {
            if let Some(mi) = target {
                let strikes = b
                    .master_deck
                    .group
                    .iter()
                    .filter(|c| c.st().tags & gen::TAG_STRIKE != 0)
                    .count() as i32;
                let base = card.st().base_damage + card.eff_magic() * strikes;
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, base, DamageType::Normal),
                });
            }
        }
        // Pommel Strike：Damage + DrawCard(magic)
        "Pommel Strike" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::DrawCard {
                    amount: card.eff_magic(),
                    end_turn_draw: false,
                });
            }
        }
        // Shrug It Off：GainBlock + DrawCard(1)
        "Shrug It Off" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::DrawCard {
                amount: 1,
                end_turn_draw: false,
            });
        }
        // Thunderclap：DamageAllEnemies + ApplyPower(Vulnerable 1) 全体
        "Thunderclap" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
            for mi in 0..b.monsters.len() {
                if !b.monsters[mi].creature.is_dying && !b.monsters[mi].creature.is_dead {
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Vulnerable, 1),
                        from_monster: false,
                    });
                }
            }
        }
        // Twin Strike：两次 DamageAction
        "Twin Strike" => {
            if let Some(mi) = target {
                for _ in 0..2 {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
            }
        }
        // Warcry：抽 1 弃任意张 + 置于抽牌堆任意位（WarcryAction 选择屏）→ 批次 6
        "Warcry" => {
            eb.bot(GameAction::DrawCard {
                amount: 1,
                end_turn_draw: false,
            });
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "WarcryAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Wild Strike：Damage + 洗入一张 Wound 到抽牌堆随机位
        "Wild Strike" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::MakeTempCardInDrawPile {
                    sid: super::sid_of("Wound"),
                    upgraded: false,
                    count: 1,
                    random_spot: true,
                });
            }
        }
        _ => {}
    }
}

