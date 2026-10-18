//! 无色卡牌效果（cards/colorless/，39 张）——批次 3.6。
//! 翻译规范见 docs/guides/card-translation.md。

use super::builder::EffectBuilder;
use super::{attack_info, damage_matrix};
use crate::action::{GameAction, RoutineCall, TargetRef};
use crate::battle::{BattleState, CardInst};
use crate::power::{DamageType, Power, PowerKind};

pub const HANDLED: &[&str] = &[
    "Apotheosis",
    "Bandage Up",
    "Bite",
    "Blind",
    "Ghostly",
    "Chrysalis",
    "Dark Shackles",
    "Deep Breath",
    "Discovery",
    "Dramatic Entrance",
    "Enlightenment",
    "Finesse",
    "Flash of Steel",
    "Forethought",
    "Good Instincts",
    "HandOfGreed",
    "Impatience",
    "Jack Of All Trades",
    "J.A.X.",
    "Madness",
    "Magnetism",
    "Master of Strategy",
    "Mayhem",
    "Metamorphosis",
    "Mind Blast",
    "Panacea",
    "Panache",
    "PanicButton",
    "Purity",
    "RitualDagger",
    "Sadistic Nature",
    "Secret Technique",
    "Secret Weapon",
    "Swift Strike",
    "The Bomb",
    "Thinking Ahead",
    "Transmutation",
    "Trip",
    "Violence",
];

pub fn effects(
    b: &mut BattleState,
    card: &CardInst,
    target: Option<usize>,
    eb: &mut EffectBuilder,
) {
    match card.st().id {
        // Apotheosis：ApotheosisAction——全牌升级
        "Apotheosis" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ApotheosisAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Bandage Up：HealAction(magic 4/6)（消耗：静态）
        "Bandage Up" => {
            eb.bot(GameAction::Heal {
                target: TargetRef::Player,
                amount: card.eff_magic(),
            });
        }
        // Bite：Damage 7（吸血：Vampire 语义 TODO 3.5——原版 BiteAction heal）
        "Bite" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Blind：Weak(magic 2) 单目标 / 升级全体
        "Blind" => {
            if card.upgraded {
                for mi in 0..b.monsters.len() {
                    if !b.monsters[mi].creature.is_dying && !b.monsters[mi].creature.is_dead {
                        eb.bot(GameAction::ApplyPower {
                            target: TargetRef::Monster(mi),
                            source: Some(TargetRef::Player),
                            power: Power::new(PowerKind::Weak, card.eff_magic()),
                            from_monster: false,
                        });
                    }
                }
            } else if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Weak, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Chrysalis：随机技能进抽牌堆 ×magic（0 费化）——TODO 4.1 池随机（例程占位）
        "Chrysalis" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ChrysalisAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Dark Shackles：Strength -magic（9/15）+ 回合末收回（LoseStrength 配对——原版有 StrengthPower -9 无 LoseStrength：临时力量 TODO 3.5）
        "Dark Shackles" => {
            if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Strength, -card.eff_magic()),
                    from_monster: false,
                });
                // 原版回合末收回：StrengthPower -9 配 GainStrengthPower——钩子 TODO 3.5
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::GainStrength, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Deep Breath：洗牌（弃牌堆非空时）+ DrawCard(magic 2/3)
        "Deep Breath" => {
            if !b.discard_pile.group.is_empty() {
                eb.bot(GameAction::EmptyDeckShuffle);
            }
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        // Discovery：DiscoveryAction——三选一临时卡（选择屏 TODO 6；例程占位）
        "Discovery" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "DiscoveryAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Dramatic Entrance：DamageAll（固有：静态标志）
        "Dramatic Entrance" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
        }
        // Enlightenment：EnlightenmentAction——手牌费用减半（升级=永久）
        "Enlightenment" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "EnlightenmentAction",
                    target: None,
                    amount: if card.upgraded { 1 } else { 0 },
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Finesse：GainBlock 2 + DrawCard 1
        "Finesse" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::DrawCard {
                amount: 1,
                end_turn_draw: false,
            });
        }
        // Flash of Steel：Damage 3 + DrawCard 1
        "Flash of Steel" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::DrawCard {
                    amount: 1,
                    end_turn_draw: false,
                });
            }
        }
        // Forethought：ForethoughtAction——手牌置抽牌堆底（X 回合后抽：选择屏 TODO 6）
        "Forethought" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ForethoughtAction",
                    target: None,
                    amount: if card.upgraded { 1 } else { 0 },
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Good Instincts：GainBlock 7/12
        "Good Instincts" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Hand of Greed：GreedAction——伤害 + 斩杀回金币（例程：金币 TODO 4.x 奖励层）
        "HandOfGreed" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "GreedAction",
                        target: Some(mi),
                        amount: card.eff_damage(),
                        amount2: card.eff_magic(),
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Impatience：ConditionalDrawAction——本回合未打攻击 → 抽 magic 2/3
        "Impatience" => {
            if b.attacks_played_this_turn == 0 {
                eb.bot(GameAction::DrawCard {
                    amount: card.eff_magic(),
                    end_turn_draw: false,
                });
            }
        }
        // Jack Of All Trades：随机无色卡进手 ×1(2)——TODO 4.1（例程占位）
        "Jack Of All Trades" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "JackOfAllTradesAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // J.A.X.：LoseHP 3 + Strength +magic 2/3
        "J.A.X." => {
            eb.bot(GameAction::LoseHp {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                amount: 3,
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Strength, card.eff_magic()),
                from_monster: false,
            });
        }
        // Madness：MadnessAction——随机手牌费用变 1（升级=0）
        "Madness" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "MadnessAction",
                    target: None,
                    amount: if card.upgraded { 0 } else { 1 },
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Magnetism：ApplyPower(Magnetism, magic)（每回合随机无色：TODO 4.1）
        "Magnetism" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::future("Magnetism", card.eff_magic()),
                from_monster: false,
            });
        }
        // Master of Strategy：DrawCard(magic 3/4)（消耗：静态）
        "Master of Strategy" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        // Mayhem：ApplyPower(Mayhem, magic)（每回合随机打出：TODO 4.1）
        "Mayhem" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::future("Mayhem", card.eff_magic()),
                from_monster: false,
            });
        }
        // Metamorphosis：随机攻击进手 ×magic（0 费化）——TODO 4.1（例程占位）
        "Metamorphosis" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "MetamorphosisAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Mind Blast：伤害 = 抽牌堆张数（use 内动态：drawPile.size × magic 1）
        "Mind Blast" => {
            if let Some(mi) = target {
                let base = b.draw_pile.group.len() as i32 * card.eff_magic();
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, base, DamageType::Normal),
                });
            }
        }
        // Panacea：Artifact(magic 1/2)
        "Panacea" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Artifact, card.eff_magic()),
                from_monster: false,
            });
        }
        // Panache：ApplyPower(Panache, magic 10)（下回合末有攻击伤 10：钩子 TODO 3.5）
        "Panache" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Future, 0),
                from_monster: false,
            });
        }
        // Panic Button：GainBlock 25 + NoBlockPower（本回合不再获格挡）
        "PanicButton" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::future("No Block", 1),
                from_monster: false,
            });
        }
        // Purity：ExhaustAction(magic, 非随机任意张)（选择屏 TODO 6）
        "Purity" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ExhaustAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Ritual Dagger：RitualDaggerAction——伤害 15 + 斩杀 +misc 3
        "RitualDagger" => {
            if let Some(mi) = target {
                let base = card.eff_damage();
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, base, DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "RitualDaggerAction",
                        target: Some(mi),
                        amount: 3,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Sadistic Nature：ApplyPower(Sadistic, magic)（被施加 debuff 后 +伤：钩子 TODO 3.5）
        "Sadistic Nature" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::future("Sadistic Nature", card.eff_magic()),
                from_monster: false,
            });
        }
        // Secret Technique：SkillFromDeckToHandAction(1)——抽牌堆技能取 1（选择屏 TODO 6）
        "Secret Technique" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "SkillFromDeckToHandAction",
                    target: None,
                    amount: 1,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Secret Weapon：AttackFromDeckToHandAction(1)（选择屏 TODO 6）
        "Secret Weapon" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "AttackFromDeckToHandAction",
                    target: None,
                    amount: 1,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Swift Strike：Damage 7
        "Swift Strike" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // The Bomb：TheBombPower——3 回合后全体 magic 伤
        "The Bomb" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Future, 0),
                from_monster: false,
            });
        }
        // Thinking Ahead：DrawCard 2 + PutOnDeckAction（手牌 1 置抽牌堆顶：选择屏 TODO 6）
        "Thinking Ahead" => {
            eb.bot(GameAction::DrawCard {
                amount: 2,
                end_turn_draw: false,
            });
            if !b.hand.group.is_empty() {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "PutOnDeckAction",
                        target: None,
                        amount: 1,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Transmutation：TransmutationAction 例程（X 费：随机状态进手 X 张）
        "Transmutation" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "TransmutationAction",
                    target: None,
                    amount: 0,
                    amount2: crate::cards::x_value(b),
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Trip：Vulnerable(magic 2/3)
        "Trip" => {
            if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Vulnerable, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Violence：DrawPileToHandAction(magic, ATTACK)——抽牌堆随机攻击进手（选择屏 TODO 6）
        "Violence" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "DrawPileToHandAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Apparition（数据 ID "Ghostly"）：获得 1 无实体（消耗：静态）
        "Ghostly" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::IntangiblePlayer, 1),
                from_monster: false,
            });
        }
        _ => {}
    }
}
