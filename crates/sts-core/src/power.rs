//! 能力（Power）机制层 —— 批次 2.2 的第一批：伤害/格挡修正相关能力。
//!
//! 依据 docs/source-inventory/03 §1-4：
//! - amount = -1 为"不叠加"哨兵（stackPower 忽略）；本层仅建模数值语义；
//! - 伤害修正链与取整点全部在 `damage.rs`（本模块只提供单能力钩子的语义）；
//! - 全部 ~162 能力在批次 3.5 补全；此处只放伤害管线必需的子集，
//!   枚举留 `Future` 兜底变体以便逐步替换而不破坏结构。

/// 伤害类型（cards/DamageInfo.java L201）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageType {
    Normal,
    /// 无视格挡的攻击型伤害（THORNS）
    Thorns,
    /// 直接扣血（HP_LOSS）
    HpLoss,
}

/// 能力种类（批次 2.2 起逐步补全至 162 个；出处 powers/*.java）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PowerKind {
    Strength,
    Dexterity,
    Vulnerable,
    Weak,
    Frail,
    Slow,
    Focus,
    Artifact,
    Intangible,
    IntangiblePlayer,
    /// 兜底：尚未转译的能力（批次 3.5 逐个消灭；带原版 ID 以便审计）。
    Future,
}

/// 能力实例。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Power {
    pub kind: PowerKind,
    pub amount: i32,
    /// 原版 ID 字符串（硬契约，Future 兜底时必填）。
    pub id: &'static str,
}

impl Power {
    pub fn new(kind: PowerKind, amount: i32) -> Self {
        Self {
            kind,
            amount,
            id: "",
        }
    }

    pub fn future(id: &'static str, amount: i32) -> Self {
        Self {
            kind: PowerKind::Future,
            amount,
            id,
        }
    }
}

/// 单能力的 `atDamageGive` 语义（powers/*.java 同名钩子逐条对照）。
/// 全程 f32，取整只在管线末端（记录 03 §11-2）。
pub fn at_damage_give(p: &Power, dmg: f32) -> f32 {
    match p.kind {
        // StrengthPower.java: dmg += amount（加法，负值即虚弱力量）
        PowerKind::Strength => dmg + p.amount as f32,
        // WeakPower.java: dmg *= 0.75（攻击方虚弱）
        PowerKind::Weak => dmg * 0.75,
        _ => dmg,
    }
}

/// 单能力的 `atDamageReceive` 语义。
pub fn at_damage_receive(p: &Power, dmg: f32) -> f32 {
    match p.kind {
        // VulnerablePower.java: dmg *= 1.5
        PowerKind::Vulnerable => dmg * 1.5,
        // SlowPower.java: dmg *= 1.0 + 0.1 * amount
        PowerKind::Slow => dmg * (1.0 + 0.1 * p.amount as f32),
        _ => dmg,
    }
}

/// `atDamageFinalGive` / `atDamageFinalReceive`（批次 2.2 补全大部分成员）。
///
/// 注：Intangible 系的"受击钳 1"发生在 damage() 内（AbstractMonster/AbstractPlayer.damage
/// 首行），不在 final 段——保持一致。
pub fn at_damage_final_receive(_p: &Power, dmg: f32) -> f32 {
    // 批次 2.2 补全（无实体/ForceField 等成员进入此段）
    dmg
}

pub fn at_damage_final_give(_p: &Power, dmg: f32) -> f32 {
    // 批次 2.2 补全
    dmg
}

/// 单能力的 `modifyBlock` 语义（格挡链第一段）。
pub fn modify_block(p: &Power, block: f32) -> f32 {
    match p.kind {
        // DexterityPower.java: block += amount
        PowerKind::Dexterity => block + p.amount as f32,
        // FrailPower.java: block *= 0.75
        PowerKind::Frail => block * 0.75,
        _ => block,
    }
}

/// `modifyBlockLast`（格挡链第二段；与 modifyBlock 分离以保持钩子顺序）。
pub fn modify_block_last(_p: &Power, block: f32) -> f32 {
    // 批次 2.2 补全（Barricade 系无；Calipers 不在此）
    block
}
