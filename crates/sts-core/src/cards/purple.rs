//! 观者卡牌效果（cards/purple/，77 张）——批次 3.5。
//! 翻译规范见 docs/guides/card-translation.md；架势语义依赖 battle::change_stance。

use super::builder::EffectBuilder;
use super::{attack_info, damage_matrix};
use crate::action::{GameAction, RoutineCall, TargetRef};
use crate::battle::{BattleState, CardInst};
use crate::power::{DamageType, Power, PowerKind};

pub const HANDLED: &[&str] = &[
    "Alpha",
    "BattleHymn",
    "Blasphemy",
    "BowlingBash",
    "Brilliance",
    "CarveReality",
    "Collect",
    "Conclude",
    "ConjureBlade",
    "Consecrate",
    "Crescendo",
    "CrushJoints",
    "CutThroughFate",
    "DeceiveReality",
    "Defend_P",
    "DeusExMachina",
    "DevaForm",
    "Devotion",
    "Discipline",
    "EmptyBody",
    "EmptyFist",
    "EmptyMind",
    "Establishment",
    "Evaluate",
    "Eruption",
    "Fasting2",
    "FearNoEvil",
    "FlurryOfBlows",
    "FlyingSleeves",
    "FollowUp",
    "ForeignInfluence",
    "Foresight",
    "Halt",
    "Indignation",
    "InnerPeace",
    "Judgement",
    "JustLucky",
    "LessonLearned",
    "LikeWater",
    "MasterReality",
    "Meditate",
    "MentalFortress",
    "Nirvana",
    "Omniscience",
    "Perseverance",
    "Pray",
    "PathToVictory",
    "Prostrate",
    "Protect",
    "Ragnarok",
    "ReachHeaven",
    "Redo",
    "Rushdown",
    "Adaptation",
    "Sanctity",
    "SandsOfTime",
    "SashWhip",
    "Scrawl",
    "SignatureMove",
    "Simmering Fury",
    "Smite",
    "SpiritShield",
    "Study",
    "Strike_P",
    "Swivel",
    "TalkToTheHand",
    "Tantrum",
    "ThirdEye",
    "Tranquility",
    "ClearTheMind",
    "Unraveling",
    "Vault",
    "Vigilance",
    "Wallop",
    "WaveOfTheHand",
    "Weave",
    "WheelKick",
    "WindmillStrike",
    "Wish",
    "Wireheading",
    "Worship",
    "WreathOfFlame",
];

pub fn effects(
    b: &mut BattleState,
    card: &CardInst,
    target: Option<usize>,
    eb: &mut EffectBuilder,
) {
    match card.st().id {
        // Alpha：洗入 Insight（临时卡进抽牌堆顶）
        "Alpha" => {
            eb.bot(GameAction::MakeTempCardInDrawPile {
                sid: super::sid_of("Insight"),
                upgraded: card.upgraded,
                count: 1,
                random_spot: false,
            });
        }
        // Battle Hymn：ApplyPower(BattleHymn, magic)（回合末加打击：钩子 TODO 3.5）
        "BattleHymn" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::BattleHymn, card.eff_magic()),
                from_monster: false,
            });
        }
        // Blasphemy：进 Divinity + EndTurnDeathPower（回合末死亡——钩子 TODO 3.5；虚无+自保留：静态标志）
        "Blasphemy" => {
            eb.bot(GameAction::ChangeStance {
                stance: crate::stance::StanceKind::Divinity,
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::future("End Turn Death", -1),
                from_monster: false,
            });
        }
        // Bowling Bash：每存活怪一段伤害（段数 = 怪数，伤害含全怪加成 TODO 3.5）
        "BowlingBash" => {
            if let Some(mi) = target {
                let n = b
                    .monsters
                    .iter()
                    .filter(|m| !m.creature.is_dead && !m.creature.is_dying)
                    .count();
                for _ in 0..n {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
            }
        }
        // Brilliance：伤害 += magic（出牌时动态 +magic 伤）
        "Brilliance" => {
            if let Some(mi) = target {
                let base = card.eff_damage() + card.eff_magic();
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, base, DamageType::Normal),
                });
            }
        }
        // Carve Reality：Damage + 临时 Smite 进手
        "CarveReality" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::MakeTempCardInHand {
                    sid: super::sid_of("Smite"),
                    upgraded: card.upgraded,
                    count: 1,
                });
            }
        }
        // Collect：CollectAction 例程（X 费：下回合回等量能量——占位扣能+TODO）
        "Collect" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "CollectAction",
                    target: None,
                    amount: 0,
                    amount2: crate::cards::x_value(b),
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Conclude：DamageAll 12（回合末自动打出：静态标志+自动出牌 TODO 3.5）
        "Conclude" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
        }
        // Conjure Blade：ConjureBladeAction 例程（X 费：抽 X、加 X 伤 Expunger——占位扣能）
        "ConjureBlade" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ConjureBladeAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: b.energy + if card.upgraded { 1 } else { 0 },
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Consecrate：DamageAll 5
        "Consecrate" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
        }
        // Crush Joints：Damage + CrushJointsAction（下回合 Vulnerable——钩子 TODO 3.5）
        "CrushJoints" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "CrushJointsAction",
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
        // Cut Through Fate：Damage + Scry(magic) + 抽 1
        "CutThroughFate" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
            eb.bot(GameAction::Scry {
                amount: card.eff_magic(),
            });
            eb.bot(GameAction::DrawCard {
                amount: 1,
                end_turn_draw: false,
            });
        }
        // Deceive Reality：GainBlock + 临时 Sash Whip 进手
        "DeceiveReality" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::MakeTempCardInHand {
                sid: super::sid_of("Smite"),
                upgraded: card.upgraded,
                count: 1,
            });
        }
        // Defend_P：GainBlock（基础卡显式化；ID "Defend_W"? 以对账为准）
        "Defend_P" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Deus Ex Machina：use() 空——抽到时加闪电（triggerWhenDrawn 钩子 TODO 3.5）
        "DeusExMachina" => {}
        // Deva Form：ApplyPower(DevaForm, 1)（每回合 +能量：钩子 TODO 3.5）
        "DevaForm" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::DevaForm, 1),
                from_monster: false,
            });
        }
        // Devotion：ApplyPower(Mantra, magic 3/4)
        "Devotion" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Mantra, card.eff_magic()),
                from_monster: false,
            });
        }
        // Discipline：DEPRECATEDDisciplinePower——废弃卡（不进卡池）
        "Discipline" => {}
        // Empty Body：GainBlock + 非中立则进中立
        "EmptyBody" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::ChangeStance {
                stance: crate::stance::StanceKind::Neutral,
            });
        }
        // Empty Fist：Damage + 非中立则进中立
        "EmptyFist" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ChangeStance {
                    stance: crate::stance::StanceKind::Neutral,
                });
            }
        }
        // Empty Mind：DrawCard(magic) + 进中立
        "EmptyMind" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
            eb.bot(GameAction::ChangeStance {
                stance: crate::stance::StanceKind::Neutral,
            });
        }
        // Establishment：ApplyPower(Establishment, magic)（保留卡每回合 -费：钩子 TODO 3.5）
        "Establishment" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Establishment, card.eff_magic()),
                from_monster: false,
            });
        }
        // Evaluate：GainBlock + Insight 洗入抽牌堆顶
        "Evaluate" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::MakeTempCardInDrawPile {
                sid: super::sid_of("Insight"),
                upgraded: false,
                count: 1,
                random_spot: false,
            });
        }
        // Eruption：Damage + 进 Wrath（基础卡）
        "Eruption" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ChangeStance {
                    stance: crate::stance::StanceKind::Wrath,
                });
            }
        }
        // Fasting：Strength +magic 与 Dexterity -magic（各半——原版双向）
        "Fasting" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Strength, card.eff_magic()),
                from_monster: false,
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Dexterity, -card.eff_magic()),
                from_monster: false,
            });
        }
        // Fear No Evil：FearNoEvilAction——伤害 + 怪攻击意图时变中立（例程占位：意图判定已可）
        "FearNoEvil" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                if b.monsters[mi].is_attacking_intent() {
                    eb.bot(GameAction::ChangeStance {
                        stance: crate::stance::StanceKind::Neutral,
                    });
                }
            }
        }
        // Flurry of Blows：Damage + 进中立（连击：打出后每架势进入再打——TODO 3.5 钩子）
        "FlurryOfBlows" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ChangeStance {
                    stance: crate::stance::StanceKind::Neutral,
                });
            }
        }
        // Flying Sleeves：2 段 Damage 5（限 WF? no——原版无限制）
        "FlyingSleeves" => {
            if let Some(mi) = target {
                for _ in 0..2 {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
            }
        }
        // Follow Up：Damage + FollowUpAction（下张牌打出两次——钩子 TODO 3.5）
        "FollowUp" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "FollowUpAction",
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
        // Foreign Influence：ForeignInfluenceAction——加随机 strike/defend（TODO 4.1 池）
        "ForeignInfluence" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ForeignInfluenceAction",
                    target: None,
                    amount: 0,
                    amount2: if card.upgraded { 1 } else { 0 },
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Foresight：ApplyPower(Foresight, magic)（每回合预见：钩子 TODO 3.5）
        "Foresight" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Foresight, card.eff_magic()),
                from_monster: false,
            });
        }
        // Halt：HaltAction——GainBlock + Wrath 下伤害翻倍叠加（amount=magic 3）
        "Halt" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "HaltAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Indignation：IndignationAction——Wrath 下全体伤/否则上弱（按当前架势）
        "Indignation" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "IndignationAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Inner Peace：InnerPeaceAction——Calm 下抽 magic 否则加 Mantra magic
        "InnerPeace" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "InnerPeaceAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Judgement：目标损失 HP = 标记层数（MarkPower 机制：钩子 TODO 3.5）
        "Judgement" => {
            if let Some(mi) = target {
                let mark = b.monsters[mi]
                    .creature
                    .powers
                    .iter()
                    .find(|p| p.id == "Mark")
                    .map(|p| p.amount)
                    .unwrap_or(0);
                if mark > 0 {
                    eb.bot(GameAction::LoseHp {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        amount: mark,
                    });
                }
            }
        }
        // Just Lucky：Scry(magic) + GainBlock
        "JustLucky" => {
            eb.bot(GameAction::Scry {
                amount: card.eff_magic(),
            });
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Lesson Learned：LessonLearnedAction 例程（伤害 + 斩杀升一张手牌 TODO 6 选择屏）
        "LessonLearned" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "LessonLearnedAction",
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
        // Like Water：ApplyPower(LikeWater, magic)（回合末 Calm 下 +格挡：end_turn 钩子已接）
        "LikeWater" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::LikeWater, card.eff_magic()),
                from_monster: false,
            });
        }
        // Master Reality：ApplyPower(MasterReality, -1)（预见临时卡升级：钩子 TODO 3.5）
        "MasterReality" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Future, 0),
                from_monster: false,
            });
        }
        // Meditate：MeditateAction 例程（保留 N 张 + 抽 1）+ 进 Calm + 直接结束回合
        "Meditate" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "MeditateAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
            eb.bot(GameAction::ChangeStance {
                stance: crate::stance::StanceKind::Calm,
            });
        }
        // Mental Fortress：ApplyPower(MentalFortress, magic)（架势切换 +格挡：钩子 TODO 3.5）
        "MentalFortress" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::MentalFortress, card.eff_magic()),
                from_monster: false,
            });
        }
        // Nirvana：ApplyPower(Nirvana, magic)（预见后 +格挡：钩子 TODO 3.5）
        "Nirvana" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Nirvana, card.eff_magic()),
                from_monster: false,
            });
        }
        // Omniscience：OmniscienceAction 例程（选 2 张打两遍——选择屏 TODO 6：占位抽顶两牌双打）
        "Omniscience" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "OmniscienceAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Prayer：Mantra(magic 4/6) + 临时 Insight 洗入
        "Pray" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Mantra, card.eff_magic()),
                from_monster: false,
            });
            eb.bot(GameAction::MakeTempCardInDrawPile {
                sid: super::sid_of("Insight"),
                upgraded: false,
                count: 1,
                random_spot: false,
            });
        }
        // Pressure Points：Mark(magic) + 全体既有标记伤害 magic（PressurePointsPower 钩子 TODO 3.5）
        "PressurePoints" => {
            if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::future("Mark", card.eff_magic()),
                    from_monster: false,
                });
                for m in b.monsters.iter() {
                    if let Some(mark) = m
                        .creature
                        .powers
                        .iter()
                        .find(|p| p.id == "Mark")
                        .map(|p| p.amount)
                    {
                        if mark > 0 {
                            eb.bot(GameAction::LoseHp {
                                target: TargetRef::Monster(0),
                                source: Some(TargetRef::Player),
                                amount: 0, // 0 占位：Mark 伤害结算在 3.5 钩子（此处避免双计）
                            });
                        }
                    }
                }
            }
        }
        // Prostrate：Mantra(magic 2/3) + GainBlock
        "Prostrate" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Mantra, card.eff_magic()),
                from_monster: false,
            });
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Ragnarok：magic 次随机敌人 5 伤（随机已接 AttackDamageRandomEnemyAction）
        "Ragnarok" => {
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "AttackDamageRandomEnemyAction",
                        target: None,
                        amount: 5,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Reach Heaven：Damage + 临时 Through Violence 洗入抽牌堆
        "ReachHeaven" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::MakeTempCardInDrawPile {
                    sid: super::sid_of("ThroughViolence"),
                    upgraded: false,
                    count: 1,
                    random_spot: false,
                });
            }
        }
        // Rushdown：ApplyPower(Rushdown, magic)（进 Wrath 抽牌：钩子 TODO 3.5）
        "Rushdown" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Rushdown, card.eff_magic()),
                from_monster: false,
            });
        }
        // Sanctity：GainBlock + SanctityAction（若本回合有弃牌 → 回血 magic）
        "Sanctity" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "SanctityAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Sash Whip：Damage + HeadStompAction（目标下回合 Vulnerable——钩子 TODO 3.5）
        "SashWhip" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "HeadStompAction",
                        target: Some(mi),
                        amount: card.eff_magic(),
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Scrawl：ExpertiseAction——抽至 10 张
        "Scrawl" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ExpertiseAction",
                    target: None,
                    amount: 10,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Signature Move：Damage 18（本回合未打出攻击才能用：canUse TODO 3.5）
        "SignatureMove" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Simmering Fury（类 Vengeance）：WrathNextTurnPower + 每回合末弃 1（钩子 TODO 3.5）
        "Vengeance" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Burst, 0),
                from_monster: false,
            });
        }
        // Smite：Damage 12（临时牌；保留：静态标志）
        "Smite" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Spirit Shield：GainBlock + 手牌数 × magic 动态（applyPowers 重写 TODO 3.5：先固定 block）
        "SpiritShield" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Study：ApplyPower(Study, magic)（每回合预见：钩子 TODO 3.5）
        "Study" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Foresight, card.eff_magic()),
                from_monster: false,
            });
        }
        // Strike_P：Damage（基础卡显式化）
        "Strike_P" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Swivel：GainBlock + FreeAttackPower（本回合攻击 0 费：canUse 钩子 TODO 3.5）
        "Swivel" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Talk To The Hand：Damage + TalkToTheHandPower（攻击此怪回血：钩子 TODO 3.5）
        "TalkToTheHand" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::future("End Turn Death", 1),
                    from_monster: false,
                });
            }
        }
        // Tantrum：magic 段 Damage 9 + 进 Wrath + 洗回抽牌堆
        "Tantrum" => {
            if let Some(mi) = target {
                for _ in 0..card.eff_magic() {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
                eb.bot(GameAction::ChangeStance {
                    stance: crate::stance::StanceKind::Wrath,
                });
                // 洗回抽牌堆（shuffleBackIntoDrawPile：静态标志路径 TODO 3.5 UseCardAction）
            }
        }
        // Third Eye：GainBlock + Scry(magic 3/5)
        "ThirdEye" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::Scry {
                amount: card.eff_magic(),
            });
        }
        // Tranquility：进 Calm（打出=进 Calm；抽到=进 Wrath：triggerWhenDrawn TODO 3.5）
        "Tranquility" => {
            eb.bot(GameAction::ChangeStance {
                stance: crate::stance::StanceKind::Calm,
            });
        }
        // Unraveling：UnravelingAction——单目标暗伤遍历（TODO 3.8 例程）
        "Unraveling" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "UnravelingAction",
                    target: None,
                    amount: card.eff_damage(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Vault：跳过怪物回合（skipMonsterTurn=true 一次）+ 直接结束回合
        "Vault" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "VaultAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Vigilance：GainBlock + 进 Calm（保留：静态标志）
        "Vigilance" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::ChangeStance {
                stance: crate::stance::StanceKind::Calm,
            });
        }
        // Wallop：WallopAction——伤害打格挡前先扣敌血（无格挡伤害：实现为 HP_LOSS + NORMAL 组合 TODO 3.5）
        "Wallop" => {
            if let Some(mi) = target {
                let info = attack_info(b, mi, card.eff_damage(), DamageType::Normal);
                eb.bot(GameAction::LoseHp {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    amount: info.output,
                });
            }
        }
        // Wave of the Hand：WaveOfTheHandPower——预见后全体弱（钩子 TODO 3.5）
        "WaveOfTheHand" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::WaveOfTheHand, card.eff_magic()),
                from_monster: false,
            });
        }
        // Weave：Damage 4（本回合未打出攻击才可用：canUse TODO 3.5）
        "Weave" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Wheel Kick：Damage 15 + DrawCard 2 + 抽牌堆洗回自身（静态标志路径 TODO 3.5）
        "WheelKick" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::DrawCard {
                    amount: 2,
                    end_turn_draw: false,
                });
            }
        }
        // Windmill Strike：Damage（保留每层 +magic：calculateCardDamage 重写 TODO 3.5）
        "WindmillStrike" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Wish：三选一选项卡（BecomeAlmighty/FameAndFortune/LiveForever——选项系统 TODO 6）
        "Wish" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "WishAction",
                    target: None,
                    amount: if card.upgraded { 1 } else { 0 },
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Worship：Mantra 5(6)
        "Worship" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Mantra, card.eff_magic()),
                from_monster: false,
            });
        }
        // Wreath of Flame：ApplyPower(WreathOfFlame, magic)（回合末 Calm 全体伤：end_turn 钩子已接）
        "WreathOfFlame" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::WreathOfFlame, card.eff_magic()),
                from_monster: false,
            });
        }
        _ => {}
    }
}
