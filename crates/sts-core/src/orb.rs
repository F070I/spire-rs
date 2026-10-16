//! 充能球（批次 3.8 的机制层；当前覆盖 ChannelOrb/EvokeOrb 动作所需的最小面）。
//! 完整 6 球钩子（onStartOfTurn/onEndOfTurn/onEvoke/applyFocus）随批次 3.8 补全。

use crate::creature::Creature;
use crate::power::DamageType;

/// 球种类（orbs/*.java）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrbKind {
    Lightning,
    Frost,
    Dark,
    Plasma,
    EmptySlot,
}

/// 球实例（Focus 修正在批次 3.8 的 applyFocus 语义接入）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Orb {
    pub kind: OrbKind,
    pub passive_amount: i32,
    pub evoke_amount: i32,
}

/// 引动球（onEvoke）的即时效果（记录 03 §8 数值表；Electro 全体分支 TODO 3.8）。
/// 返回 (随机敌人 THORNS 伤害, 全体格挡, 暗球累计伤害, 能量)。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvokeOutcome {
    pub thorns_damage: i32,
    pub block: i32,
    pub dark_damage: i32,
    pub energy: i32,
}

pub fn evoke_effect(orb: &Orb) -> EvokeOutcome {
    match orb.kind {
        OrbKind::Lightning => EvokeOutcome {
            thorns_damage: orb.evoke_amount,
            ..Default::default()
        },
        OrbKind::Frost => EvokeOutcome {
            block: orb.evoke_amount,
            ..Default::default()
        },
        OrbKind::Dark => EvokeOutcome {
            dark_damage: orb.evoke_amount,
            ..Default::default()
        },
        OrbKind::Plasma => EvokeOutcome {
            energy: orb.evoke_amount,
            ..Default::default()
        },
        OrbKind::EmptySlot => Default::default(),
    }
}

/// 被动触发（onEndOfTurn / onStartOfTurn for Plasma）。
pub fn passive_effect(orb: &Orb) -> EvokeOutcome {
    match orb.kind {
        OrbKind::Dark => EvokeOutcome {
            dark_damage: 0, // 暗球被动是 evoke_amount += passive_amount（滚雪球），由持有层处理
            ..Default::default()
        },
        _ => evoke_effect_with_passive(orb),
    }
}

fn evoke_effect_with_passive(orb: &Orb) -> EvokeOutcome {
    match orb.kind {
        OrbKind::Plasma => EvokeOutcome {
            energy: orb.passive_amount,
            ..Default::default()
        },
        _ => evoke_effect(orb),
    }
}

/// 球的伤害类型统一 THORNS（记录 03 §8）。
pub const ORB_DAMAGE_TYPE: DamageType = DamageType::Thorns;

#[allow(dead_code)]
fn _unused(_c: &Creature) {}
