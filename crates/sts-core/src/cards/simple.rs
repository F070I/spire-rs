//! 诅咒/状态/临时/选项卡效果（批次 3.6 收尾）。
//! 诅咒/状态大多不可打出（use 空），触发语义在 3.5 钩子清账（Power/future 占位）。

use super::builder::EffectBuilder;
use super::attack_info;
use crate::action::{GameAction, TargetRef};
use crate::battle::{BattleState, CardInst};
use crate::power::{DamageType, Power};

pub const HANDLED_CURSES: &[&str] = &[
    "AscendersBane", "Clumsy", "Curse of the Bell", "Decay", "Doubt",
    "Injury", "Necronomicurse", "Normality", "Pain", "Regret",
    "Shame", "Slimed", "Void", "Wound", "Parasite", "Pride", "Writhe", "CurseOfTheBell",
];

pub const HANDLED_STATUS: &[&str] = &["Burn", "Dazed", "Slimed", "Void", "Wound"];

pub const HANDLED_OPTIONS: &[&str] = &["BecomeAlmighty", "Calm", "Wrath", "FameAndFortune", "LiveForever"];

pub const HANDLED_TEMP: &[&str] = &[
    "Shiv", "Insight", "Miracle", "Smite", "Safety", "Expunger",
    "Beta", "Omega", "Through Violence",
];

pub fn effects_curses(_b: &mut BattleState, card: &CardInst, _target: Option<usize>, _eb: &mut EffectBuilder) {
    match card.st().id {
        // Slimed：打出效果空（消耗一层软泥由 UseCardAction 去向处理）
        "Slimed" => {}
        // Void：消耗时少 1 抽——钩子 TODO 3.5；打出效果空
        "Void" => {}
        "Wound" => {}
        // Burn：回合末触发伤害=2（钩子 TODO 3.5）；打出空
        "Burn" => {}
        // Decay：回合末掉 2 血（钩子 TODO 3.5）
        "Decay" => {}
        // Regret：回合末按手牌数掉血（钩子 TODO 3.5）
        "Regret" => {}
        // Pain：每打出一张牌掉 1 血（钩子 TODO 3.5）
        "Pain" => {}
        // Shame：回合末获羞耻（钩子 TODO 3.5）
        "Shame" => {}
        // Doubt：每回合 +1 虚弱（钩子 TODO 3.5）
        "Doubt" => {}
        "Injury" => {}
        // Normality：本战斗最多打 3 张牌（canPlay 钩子 TODO 3.5）
        "Normality" => {}
        // Necronomicurse：消耗后回手（钩子 TODO 3.5）
        "Necronomicurse" => {}
        // Curse of the Bell：打出时加诅咒（钩子 TODO 3.5）
        "CurseOfTheBell" => {}
        "Clumsy" => {}
        // Parasite：宿主死亡移除（钩子 TODO 3.5）；打出空
        "Parasite" => {}
        // Pride：回合末获 Regen（钩子 TODO 3.5）
        "Pride" => {}
        // Writhe：每打出能力牌掉 1 血（钩子 TODO 3.5）
        "Writhe" => {}
        // AscendersBane：战斗内无效果
        "AscendersBane" => {}
        _ => {}
    }
}

pub fn effects_status(_b: &mut BattleState, card: &CardInst, _target: Option<usize>, _eb: &mut EffectBuilder) {
    match card.st().id {
        "Burn" | "Dazed" | "Slimed" | "Void" | "Wound" => {}
        _ => {}
    }
}

pub fn effects_temp(b: &mut BattleState, card: &CardInst, target: Option<usize>, eb: &mut EffectBuilder) {
    match card.st().id {
        // Shiv：Damage 4（0 费攻击）
        "Shiv" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Miracle：无打出效果（能量由获得时生效）
        "Miracle" => {}
        // Smite：Damage 12（观者 Reach Heaven 产出）
        "Smite" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Safety：GainBlock 12（观者 Protect 产出）
        "Safety" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Through Violence：Damage 20（观者 Reach Heaven 产出）
        "ThroughViolence" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Insight：无打出效果（抽牌堆内触发回合抽牌：钩子 TODO 3.5）
        "Insight" => {}
        // Expunger：Damage（观者 Conjure Blade 产出）
        "Expunger" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Beta：能力牌（Alpha 产出——打出变为 Ritual 钩子）
        "Beta" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::future("Ritual", 1),
                from_monster: false,
            });
        }
        // Omega：能力牌（打出后每回合全体 60 伤：钩子 TODO 3.5）
        "Omega" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::future("Omega", 1),
                from_monster: false,
            });
        }
        _ => {}
    }
}

pub fn effects_option(_b: &mut BattleState, _card: &CardInst, _target: Option<usize>, _eb: &mut EffectBuilder) {
    // 选项卡（cost=-2 不可打出；onChoseThisOption 钩子 TODO 6 选项系统）——效果全部空
}
