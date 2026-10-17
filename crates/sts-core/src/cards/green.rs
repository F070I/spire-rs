//! 静默猎手卡牌效果（cards/green/，75 张）——批次 3.3。
//! 翻译规范见 docs/guides/card-translation.md；注释标注原版依据。

use super::builder::EffectBuilder;
use super::{attack_info, damage_matrix};
use crate::action::{GameAction, RoutineCall, TargetRef};
use crate::battle::{BattleState, CardInst};
use crate::power::{DamageType, Power, PowerKind};

pub const HANDLED: &[&str] = &[
    "Accuracy", "Acrobatics", "All Out Attack", "Backflip", "Backstab", "Bane", "Blade Dance",
    "Blur", "Caltrops", "Choke", "Cloak And Dagger", "Concentrate", "Dagger Spray",
    "Dagger Throw", "Dash", "Deadly Poison", "Defend_G", "Deflect", "Die Die Die",
    "Dodge and Roll", "Endless Agony", "Escape Plan", "Expertise", "Flying Knee", "Footwork",
];

pub fn effects(b: &BattleState, card: &CardInst, target: Option<usize>, eb: &mut EffectBuilder) {
    match card.st().id {
        // Accuracy：ApplyPower(Accuracy, magic)（Shiv +伤：钩子 TODO 3.5）
        "Accuracy" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Accuracy, card.eff_magic()),
                from_monster: false,
            });
        }
        // Acrobatics：DrawCard(magic) + DiscardAction(1, 非随机任意张)
        "Acrobatics" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
            eb.bot(GameAction::Discard {
                amount: 1,
                is_random: false,
                any_number: true,
            });
        }
        // All Out Attack：DamageAll + Discard(1, 随机)
        "All Out Attack" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
            eb.bot(GameAction::Discard {
                amount: 1,
                is_random: true,
                any_number: false,
            });
        }
        // Backflip：GainBlock + DrawCard 2
        "Backflip" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::DrawCard {
                amount: 2,
                end_turn_draw: false,
            });
        }
        // Backstab：Damage 11（固有：静态标志）
        "Backstab" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Bane：DamageAction 总是执行 + BaneAction（目标中毒 → 二段 Damage）
        "Bane" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                let poisoned = b.monsters[mi]
                    .creature
                    .powers
                    .iter()
                    .any(|p| p.kind == PowerKind::Poison);
                if poisoned {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
            }
        }
        // Blade Dance：MakeTempCardInHand(Shiv, magic 3)
        "Blade Dance" => {
            eb.bot(GameAction::MakeTempCardInHand {
                sid: super::sid_of("Shiv"),
                upgraded: card.upgraded,
                count: card.eff_magic(),
            });
        }
        // Blur：GainBlock + ApplyPower(Blur, 1)（不掉格挡：begin_player_turn 已接）
        "Blur" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Blur, 1),
                from_monster: false,
            });
        }
        // Caltrops：ApplyPower(Thorns, magic)（受击反伤钩子 TODO 3.5）
        "Caltrops" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Thorns, card.eff_magic()),
                from_monster: false,
            });
        }
        // Choke：Damage + ApplyPower(Choke, magic)（出牌掉血钩子 TODO 3.5）
        "Choke" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Choke, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Cloak And Dagger：GainBlock + MakeTempCardInHand(Shiv, magic)
        "Cloak And Dagger" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::MakeTempCardInHand {
                sid: super::sid_of("Shiv"),
                upgraded: card.upgraded,
                count: card.eff_magic(),
            });
        }
        // Concentrate：Discard(magic 3, 任意张) + GainEnergy 2
        "Concentrate" => {
            eb.bot(GameAction::Discard {
                amount: card.eff_magic(),
                is_random: false,
                any_number: true,
            });
            eb.bot(GameAction::GainEnergy { amount: 2 });
        }
        // Dagger Spray：DamageAll
        "Dagger Spray" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
        }
        // Dagger Throw：Damage + DrawCard 1 + Discard(1, 非随机任意)
        "Dagger Throw" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
            eb.bot(GameAction::DrawCard {
                amount: 1,
                end_turn_draw: false,
            });
            eb.bot(GameAction::Discard {
                amount: 1,
                is_random: false,
                any_number: true,
            });
        }
        // Dash：GainBlock + Damage（同 magic 值 10/13）
        "Dash" => {
            if let Some(mi) = target {
                eb.bot(GameAction::GainBlock {
                    target: TargetRef::Player,
                    amount: card.eff_block(),
                });
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Deadly Poison：ApplyPower(Poison, magic 5/7)
        "Deadly Poison" => {
            if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Poison, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Defend_G：GainBlock（基础卡显式化）
        "Defend_G" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Deflect：GainBlock 7（0 费）
        "Deflect" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Die Die Die：DamageAll 13
        "Die Die Die" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
        }
        // Dodge and Roll：GainBlock + NextTurnBlockPower(block)（抽牌后同值格挡）
        "Dodge and Roll" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::DodgeAndRoll, card.eff_block()),
                from_monster: false,
            });
        }
        // Endless Agony：Damage（消耗+抽到再抽：静态标志 + draw 钩子）
        "Endless Agony" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Escape Plan：DrawCard(1, followUp=EscapePlanAction(block))——抽到技能 +block
        "Escape Plan" => {
            eb.bot(GameAction::DrawCard {
                amount: 1,
                end_turn_draw: false,
            });
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "EscapePlanAction",
                    target: None,
                    amount: card.eff_block(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Expertise：ExpertiseAction——抽牌直到手牌 = magic
        "Expertise" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ExpertiseAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Flying Knee：Damage + EnergizedPower(1)（抽牌后 +1 能量）
        "Flying Knee" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Player,
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Energized, 1),
                    from_monster: false,
                });
            }
        }
        // Footwork：ApplyPower(Dexterity, magic)
        "Footwork" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Dexterity, card.eff_magic()),
                from_monster: false,
            });
        }
        _ => {}
    }
}
