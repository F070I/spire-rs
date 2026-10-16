//! 架势（Stance）—— 4 个架势的数值语义（依据 docs/source-inventory/03 §9）。
//! 行为钩子（onEnterStance 出/进能量等）在批次 2.3/3.8 接入动作系统。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StanceKind {
    Neutral,
    Calm,
    Wrath,
    Divinity,
}

/// `AbstractStance.atDamageGive(float, DamageType)`。
pub fn at_damage_give(stance: StanceKind, dmg: f32, ty: DamageType) -> f32 {
    match (stance, ty) {
        // WrathStance: NORMAL 伤害 ×2（打与被打都翻倍）
        (StanceKind::Wrath, DamageType::Normal) => dmg * 2.0,
        // DivinityStance: NORMAL ×3
        (StanceKind::Divinity, DamageType::Normal) => dmg * 3.0,
        _ => dmg,
    }
}

/// `AbstractStance.atDamageReceive(float, DamageType)`。
pub fn at_damage_receive(stance: StanceKind, dmg: f32, ty: DamageType) -> f32 {
    match (stance, ty) {
        // WrathStance：被打也 ×2
        (StanceKind::Wrath, DamageType::Normal) => dmg * 2.0,
        _ => dmg,
    }
}

use crate::power::DamageType;
