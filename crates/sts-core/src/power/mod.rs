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
    NoDraw,
    /// LoseStrengthPower（原版 ID 恒为 "Flex"——ID 契约特例）：回合末力量 -= amount。
    LoseStrength,
    /// RagePower（ID "Rage"）：每打出一张攻击获得 amount 格挡（onPlayCard 钩子 TODO 3.5）。
    Rage,
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
    /// justApplied 模式（记录 03 §2）：怪物施加的回合型减益在本回合结束时
    /// 跳过第一次递减（避免当回合就掉层）。仅回合型减益使用。
    pub just_applied: bool,
}

impl Power {
    pub fn new(kind: PowerKind, amount: i32) -> Self {
        Self {
            kind,
            amount,
            id: "",
            just_applied: false,
        }
    }

    /// 施加时携带 justApplied（回合型减益由怪物施加时置位）。
    pub fn new_just_applied(kind: PowerKind, amount: i32) -> Self {
        Self {
            just_applied: true,
            ..Self::new(kind, amount)
        }
    }

    pub fn future(id: &'static str, amount: i32) -> Self {
        Self {
            kind: PowerKind::Future,
            amount,
            id,
            just_applied: false,
        }
    }
}

/// priority 表（记录 03 §3）：默认 5，仅列出覆写者；排序只发生在
/// ApplyPowerAction 新增时（稳定排序），低级 addPower 不排序。
pub fn priority(kind: PowerKind) -> i32 {
    match kind {
        PowerKind::Frail => 10,
        PowerKind::Intangible | PowerKind::IntangiblePlayer => 75,
        PowerKind::Weak => 99,
        _ => 5,
    }
}

/// PowerType.BUFF/DEBUFF（固定于能力种类，不随 amount 正负改变——
/// 原 StrengthPower 负值仍为 BUFF，Artifact 不拦截，保真）。
pub fn is_debuff(kind: PowerKind) -> bool {
    matches!(kind, PowerKind::Vulnerable | PowerKind::Weak | PowerKind::Frail | PowerKind::Slow)
}

/// 回合型减益（atEndOfRound 递减 + justApplied 跳过，记录 03 §2）。
pub fn is_turn_based_decrement(kind: PowerKind) -> bool {
    matches!(
        kind,
        PowerKind::Vulnerable | PowerKind::Weak | PowerKind::Frail
        // 完整集合（NoBlock/AttackBurn/SkillBurn/DoubleDamage/DrawReduction/Intangible）
        // 随批次 3.5 补全对应 kind。
    )
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

// ======================================================================
// 施加 / 叠层 / 递减 / 移除（ApplyPowerAction / stackPower / ReducePowerAction /
// RemoveSpecificPowerAction 的语义核心，记录 09 §8、03 §1E/2）

pub mod ops;

pub use ops::{apply_power, add_power_raw, end_of_round_tick, reduce_power, remove_power, stack_power, ApplyCtx, ApplyOutcome, ReduceResult};
