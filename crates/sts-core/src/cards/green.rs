//! 静默猎手卡牌效果（cards/green/，75 张）——批次 3.3。
//! 翻译规范见 docs/guides/card-translation.md；注释标注原版依据。

use super::builder::EffectBuilder;
use super::{attack_info, damage_matrix};
use crate::action::{GameAction, RoutineCall, TargetRef};
use crate::battle::{BattleState, CardInst};
use crate::power::{DamageType, Power, PowerKind};

pub const HANDLED: &[&str] = &[
    "Accuracy", "Acrobatics", "A Thousand Cuts", "After Image", "All Out Attack",
    "Backflip", "Backstab", "Bane", "Blade Dance", "Blur", "Bouncing Flask",
    "Bullet Time", "Burst", "Calculated Gamble", "Caltrops", "Catalyst", "Choke",
    "Cloak And Dagger", "Concentrate", "Corpse Explosion", "Crippling Poison",
    "Dagger Spray", "Dagger Throw", "Dash", "Deadly Poison", "Defend_G", "Deflect",
    "Die Die Die", "Distraction", "Dodge and Roll", "Doppelganger", "Endless Agony",
    "Envenom", "Escape Plan", "Eviscerate", "Expertise", "Finisher", "Flechettes",
    "Flying Knee", "Footwork", "Glass Knife", "Grand Finale", "Heel Hook",
    "Infinite Blades", "Leg Sweep", "Malaise", "Masterful Stab", "Neutralize",
    "Night Terror", "Noxious Fumes", "Outmaneuver", "Phantasmal Killer", "PiercingWail",
    "Poisoned Stab", "Predator", "Prepared", "Quick Slash", "Reflex",
    "Riddle With Holes", "Setup", "Skewer", "Slice",
    "Storm of Steel", "Strike_G", "Sucker Punch", "Survivor", "Tactician",
    "Tools of the Trade", "Unload", "Well Laid Plans", "Wraith Form", "Adrenaline",
    "Venomology", "Terror", "Underhanded Strike",
];

pub fn effects(b: &mut BattleState, card: &CardInst, target: Option<usize>, eb: &mut EffectBuilder) {
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
        // A Thousand Cuts：ApplyPower(ThousandCuts, magic)（出牌全体伤：consume 钩子已接）
        "A Thousand Cuts" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::ThousandCuts, card.eff_magic()),
                from_monster: false,
            });
        }
        // After Image：ApplyPower(AfterImage, 1)（出牌 +无实体：onUseCard 钩子 TODO 3.5）
        "After Image" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::AfterImage, 1),
                from_monster: false,
            });
        }
        // Bouncing Flask：随机怪 3 次 ×magic 毒（TODO 4.1 随机目标：首活怪占位）
        "Bouncing Flask" => {
            if let Some(mi) = b.first_alive_monster() {
                for _ in 0..3 {
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Poison, card.eff_magic()),
                        from_monster: false,
                    });
                }
            }
        }
        // Bullet Time：NoDrawPower + ApplyBulletTimeAction（手牌 0 费例程）
        "Bullet Time" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::NoDraw, -1),
                from_monster: false,
            });
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ApplyBulletTimeAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Burst：ApplyPower(Burst, magic)（下张技能翻倍——onUseCard 钩子 TODO 3.5）
        "Burst" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Burst, card.eff_magic()),
                from_monster: false,
            });
        }
        // Calculated Gamble：CalculatedGambleAction 例程（弃整手抽同数）
        "Calculated Gamble" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "CalculatedGambleAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Catalyst：未升级 ×2 / 升级 ×3（CatalystAction 例程）
        "Catalyst" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "CatalystAction",
                        target: Some(mi),
                        amount: if card.upgraded { 3 } else { 2 },
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Corpse Explosion：Poison(magic) + CorpseExplosionPower（死亡传播钩子 TODO 3.5）
        "Corpse Explosion" => {
            if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Poison, card.eff_magic()),
                    from_monster: false,
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::future("Corpse Explosion", 1),
                    from_monster: false,
                });
            }
        }
        // Crippling Poison：全体 Poison(magic) + Weak 2
        "Crippling Poison" => {
            for mi in 0..b.monsters.len() {
                if !b.monsters[mi].creature.is_dying && !b.monsters[mi].creature.is_dead {
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Poison, card.eff_magic()),
                        from_monster: false,
                    });
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Weak, 2),
                        from_monster: false,
                    });
                }
            }
        }
        // Distraction：随机技能进手 0 费——TODO 4.1 池随机（例程占位）
        "Distraction" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "DistractionAction",
                    target: None,
                    amount: 1,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Doppelganger：X 费——下回合抽 X + 能量 X（DoppelgangerAction 例程占位 + 能量语义）
        "Doppelganger" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "DoppelgangerAction",
                    target: None,
                    amount: crate::cards::x_value(b),
                    amount2: crate::cards::x_value(b),
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Eviscerate：3 段伤害（本回合弃牌减费钩子 TODO 3.5）
        "Eviscerate" => {
            if let Some(mi) = target {
                for _ in 0..3 {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
            }
        }
        // Envenom：ApplyPower(Envenom, 1)（未格挡攻击伤上毒：钩子 TODO 3.5）
        "Envenom" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Envenom, 1),
                from_monster: false,
            });
        }
        // Finisher：DamagePerAttackPlayedAction 例程（每张本回合攻击一段）
        "Finisher" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "FinisherAction",
                        target: Some(mi),
                        amount: card.eff_damage(),
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Flechettes：FlechetteAction 例程（手牌技能数 × amount 伤）
        "Flechettes" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "FlechetteAction",
                        target: Some(mi),
                        amount: 4,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Glass Knife：2 段伤害 + ModifyDamageAction(uuid, -2)（misc -2 本战斗）
        "Glass Knife" => {
            if let Some(mi) = target {
                let base = card.st().base_damage + card.misc;
                for _ in 0..2 {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, base, DamageType::Normal),
                    });
                }
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "ModifyDamageAction",
                        target: None,
                        amount: -2,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Grand Finale：DamageAll（canUse：抽牌堆必须为空——can_use 特例已接）
        "Grand Finale" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
        }
        // Heel Hook：HeelHookAction 例程（本回合有弃牌 → 抽 1 回 1 能）
        "Heel Hook" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "HeelHookAction",
                        target: Some(mi),
                        amount: 0,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Infinite Blades：ApplyPower(InfiniteBlades, 1)（抽牌后 +Shiv：钩子已接）
        "Infinite Blades" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::InfiniteBlades, 1),
                from_monster: false,
            });
        }
        // Leg Sweep：ApplyPower(Weak, magic) + GainBlock
        "Leg Sweep" => {
            if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Weak, card.eff_magic()),
                    from_monster: false,
                });
            }
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Malaise：MalaiseAction 例程（X 费：Weak 与力量 -X）
        "Malaise" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "MalaiseAction",
                        target: Some(mi),
                        amount: 0,
                        amount2: crate::cards::x_value(b),
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Masterful Stab：Damage 12（对中毒目标 1 费：modifyCost 钩子 TODO 3.5）
        "Masterful Stab" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Neutralize：Damage 3 + Weak(magic 1/2)
        "Neutralize" => {
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
        // Nightmare（原版 ID "Night Terror"）：NightmareAction——复制手牌（选择屏 TODO 批次 6）
        "Night Terror" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "NightmareAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Noxious Fumes：ApplyPower(NoxiousFumes, magic)（回合末全体加毒：钩子已接）
        "Noxious Fumes" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::NoxiousFumes, card.eff_magic()),
                from_monster: false,
            });
        }
        // Outmaneuver：Energized(magic 2/3)
        "Outmaneuver" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Energized, card.eff_magic()),
                from_monster: false,
            });
        }
        // Phantasmal Killer：ApplyPower(Phantasmal, 1)（下回合弃整手按技能伤：钩子 TODO 3.5）
        "Phantasmal Killer" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Phantasmal, 1),
                from_monster: false,
            });
        }
        // PiercingWail：全体敌人 Strength -magic（本回合收回：LoseStrength 配对）
        "PiercingWail" => {
            for mi in 0..b.monsters.len() {
                if !b.monsters[mi].creature.is_dying && !b.monsters[mi].creature.is_dead {
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Strength, -card.eff_magic()),
                        from_monster: false,
                    });
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::LoseStrength, card.eff_magic()),
                        from_monster: false,
                    });
                }
            }
        }
        // Poisoned Stab：Damage 6 + Poison(magic 5/7)
        "Poisoned Stab" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Poison, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Predator：Damage 15 + DrawCardNextTurnPower(magic 2/3)（钩子 TODO 3.5）
        "Predator" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Player,
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::DrawCardNextTurn, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Prepared：DrawCard(magic 1/2) + Discard(magic, 任意张)
        "Prepared" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
            eb.bot(GameAction::Discard {
                amount: card.eff_magic(),
                is_random: false,
                any_number: true,
            });
        }
        // Quick Slash：Damage 8 + DrawCard 1
        "Quick Slash" => {
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
        // Reflex：use() 仅 DrawCard(magic 2)（弃牌触发钩子 TODO 3.5）
        "Reflex" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        // Riddle With Holes：5 段伤害
        "Riddle With Holes" => {
            if let Some(mi) = target {
                for _ in 0..5 {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
            }
        }
        // Setup：SetupAction 例程（手牌置抽牌堆顶；选择屏 TODO 批次 6）
        "Setup" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "SetupAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Skewer：SkewerAction 例程（X 费：每点 4 伤一段）
        "Skewer" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "SkewerAction",
                        target: Some(mi),
                        amount: card.eff_damage(),
                        amount2: crate::cards::x_value(b),
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Slice：Damage 6
        "Slice" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Storm of Steel：BladeFuryAction 例程（弃整手 → 每张 +1 飞刀）
        "Storm of Steel" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "BladeFuryAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Strike_G：Damage（基础卡显式化）
        "Strike_G" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Sucker Punch：Damage 3 + Weak 1
        "Sucker Punch" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Weak, 1),
                    from_monster: false,
                });
            }
        }
        // Survivor：GainBlock + Discard(1, 非随机)（弃牌触发能量 TODO 3.5）
        "Survivor" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::Discard {
                amount: 1,
                is_random: false,
                any_number: false,
            });
        }
        // Tools of the Trade：ApplyPower(ToolsOfTheTrade, 1)（抽牌后抽 1：钩子已接；弃 1 TODO）
        "Tools of the Trade" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::ToolsOfTheTrade, 1),
                from_monster: false,
            });
        }
        // Tactician：use() 为空——弃牌时 +1 能量（didDiscard 钩子 TODO 3.5）
        "Tactician" => {}
        // Unload：Damage 14 + UnloadAction（本回合不再打攻击——钩子 TODO 3.5）
        "Unload" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "UnloadAction",
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
        // Well Laid Plans：ApplyPower(RetainCard, magic)（回合末保留 N 张：钩子 TODO 3.5）
        "Well Laid Plans" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::RetainCard, card.eff_magic()),
                from_monster: false,
            });
        }
        // Wraith Form v2（类 WraithForm）：ApplyPower(WraithForm, magic)（每回合 +无实体递减：钩子已接；虚无：静态标志）
        "Wraith Form v2" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::WraithForm, card.eff_magic()),
                from_monster: false,
            });
        }
        // Adrenaline：GainEnergy(1/2) + DrawCard(2)（消耗：静态；0 费）
        "Adrenaline" => {
            eb.bot(GameAction::GainEnergy {
                amount: if card.upgraded { 2 } else { 1 },
            });
            eb.bot(GameAction::DrawCard { amount: 2, end_turn_draw: false });
        }
        // Venomology（类 Alchemize）：获得随机药水——TODO 4.1 池随机（potionRng limited=true）
        "Venomology" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ObtainPotionAction", target: None, amount: 0, amount2: 0,
                    card_uuid: card.uuid, sid: card.sid, upgraded: card.upgraded,
                },
            });
        }
        // Terror：Vulnerable 99（持续到战斗结束的脆弱施加）
        "Terror" => {
            if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Vulnerable, 99),
                    from_monster: false,
                });
            }
        }
        // Wraith Form v2（类 WraithForm）：Intangible(magic 2/3) + WraithFormPower（递减）
        "Wraith Form" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::IntangiblePlayer, card.eff_magic()),
                from_monster: false,
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::WraithForm, card.eff_magic()),
                from_monster: false,
            });
        }
        // Underhanded Strike（类 SneakyStrike）：Damage 11 + GainEnergyIfDiscard（弃牌回能钩子 TODO 3.5）
        "Underhanded Strike" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "GainEnergyIfDiscardAction", target: None, amount: 1, amount2: 0,
                        card_uuid: card.uuid, sid: card.sid, upgraded: card.upgraded,
                    },
                });
            }
        }
        _ => {}
    }
}
