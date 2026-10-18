//! 伤害数值管线 —— 原版三条修正链的逐位复刻（docs/source-inventory/03 §4）。
//!
//! 保真要点：
//! - 全程 f32 运算（Java float），末端一次性 floor；中途 int 截断点逐处保留（背击 ×1.5）；
//! - 修正链的**顺序**即语义：力量（加法，give 段）→ 架势倍率 → 易伤/缓慢（receive 段）→ final 段；
//! - THORNS/HP_LOSS 的旁路在 `decrement_block` 与矩阵层，不在 give/receive 段；
//! - 遗物 `atDamageModify`（卡牌链第一环）与 Endless 疫病乘算在遗物/灾祸层接入后补
//!   （TODO 批次 3.6 / 4.9），当前以空切片占位保持链形状。

use crate::power::{self, DamageType, Power};
use crate::stance::{self, StanceKind};

/// MathUtils.floor 等价：向下取整（对已钳非负的值等价于截断）。
fn floor_i(v: f32) -> i32 {
    v.floor() as i32
}

/// 卡牌伤害链（`AbstractCard.applyPowers` / `calculateCardDamage` 的数值核心，记录 03 §4b）。
///
/// 顺序：遗物 atDamageModify → 玩家 powers atDamageGive → 玩家架势 atDamageGive
/// → 目标 powers atDamageReceive → 玩家 powers atDamageFinalGive → 目标 powers atDamageFinalReceive
/// → 钳非负 → floor。
pub fn card_damage_raw(
    attacker_powers: &[Power],
    attacker_stance: StanceKind,
    target_powers: &[Power],
    base_damage: i32,
    ty: DamageType,
) -> f32 {
    let mut tmp = base_damage as f32;
    // 1. 遗物 atDamageModify —— TODO(批次3.6)
    for p in attacker_powers {
        tmp = power::at_damage_give(p, tmp);
    }
    tmp = stance::at_damage_give(attacker_stance, tmp, ty);
    for p in target_powers {
        tmp = power::at_damage_receive(p, tmp);
    }
    for p in attacker_powers {
        tmp = power::at_damage_final_give(p, tmp);
    }
    for p in target_powers {
        tmp = power::at_damage_final_receive(p, tmp);
    }
    if tmp < 0.0 {
        tmp = 0.0;
    }
    tmp
}

/// 卡牌伤害链（取整出口）。
pub fn card_damage(
    attacker_powers: &[Power],
    attacker_stance: StanceKind,
    target_powers: &[Power],
    base_damage: i32,
    ty: DamageType,
) -> i32 {
    floor_i(card_damage_raw(
        attacker_powers,
        attacker_stance,
        target_powers,
        base_damage,
        ty,
    ))
}

/// 怪物意图伤害链（`AbstractMonster.calculateDamage`，记录 03 §4c）。
///
/// 顺序：疫病（Endless，TODO 4.9）→ 怪物 powers atDamageGive → 玩家 powers atDamageReceive
/// → 玩家架势 atDamageReceive → **背击 ×1.5（中途 int 截断）** → 怪物 atDamageFinalGive
/// → 玩家 atDamageFinalReceive → 钳非负 → floor。
pub fn monster_calculate_damage_raw(
    monster_powers: &[Power],
    player_powers: &[Power],
    player_stance: StanceKind,
    base_damage: i32,
    back_attack: bool,
) -> f32 {
    let mut tmp = base_damage as f32;
    for p in monster_powers {
        tmp = power::at_damage_give(p, tmp);
    }
    for p in player_powers {
        tmp = power::at_damage_receive(p, tmp);
    }
    tmp = stance::at_damage_receive(player_stance, tmp, DamageType::Normal);
    if back_attack {
        // (int)(tmp * 1.5F) —— 中途截断（非 floor！负值不可能：base ≥ 0 且乘法非负）
        tmp = (tmp * 1.5) as i32 as f32;
    }
    for p in monster_powers {
        tmp = power::at_damage_final_give(p, tmp);
    }
    for p in player_powers {
        tmp = power::at_damage_final_receive(p, tmp);
    }
    if tmp < 0.0 {
        tmp = 0.0;
    }
    tmp
}

pub fn monster_calculate_damage(
    monster_powers: &[Power],
    player_powers: &[Power],
    player_stance: StanceKind,
    base_damage: i32,
    back_attack: bool,
) -> i32 {
    floor_i(monster_calculate_damage_raw(
        monster_powers,
        player_powers,
        player_stance,
        base_damage,
        back_attack,
    ))
}

/// `DamageInfo.applyPowers(owner, target)`（记录 03 §4a）——非卡牌来源（怪咬人、荆棘反伤等）。
///
/// owner 非玩家：owner give → target receive → 玩家架势 receive → owner finalGive → target finalReceive
/// owner 玩家：  owner give → 玩家架势 give → target receive → owner finalGive → target finalReceive
/// （两条链都不含遗物钩子——原版如此。）
pub fn info_apply_powers(
    owner_powers: &[Power],
    owner_is_player: bool,
    target_powers: &[Power],
    player_stance: StanceKind,
    base_damage: i32,
    ty: DamageType,
) -> i32 {
    let mut tmp = base_damage as f32;
    // Endless 疫病 DeadlyEnemies 乘算 —— TODO(批次4.9)
    for p in owner_powers {
        tmp = power::at_damage_give(p, tmp);
    }
    if owner_is_player {
        tmp = stance::at_damage_give(player_stance, tmp, ty);
    }
    for p in target_powers {
        tmp = power::at_damage_receive(p, tmp);
    }
    if !owner_is_player {
        tmp = stance::at_damage_receive(player_stance, tmp, ty);
    }
    for p in owner_powers {
        tmp = power::at_damage_final_give(p, tmp);
    }
    for p in target_powers {
        tmp = power::at_damage_final_receive(p, tmp);
    }
    if tmp < 0.0 {
        tmp = 0.0;
    }
    floor_i(tmp)
}

/// 格挡链（`AbstractCard.applyPowersToBlock`，记录 03 §4b 末）。
/// 顺序：遗物（TODO）→ powers modifyBlock → powers modifyBlockLast → floor → 钳非负。
pub fn block_value(player_powers: &[Power], base_block: i32) -> i32 {
    let mut tmp = base_block as f32;
    for p in player_powers {
        tmp = power::modify_block(p, tmp);
    }
    for p in player_powers {
        tmp = power::modify_block_last(p, tmp);
    }
    let v = floor_i(tmp);
    if v < 0 {
        0
    } else {
        v
    }
}

/// `DamageInfo`（cards/DamageInfo.java）——output 在管线中赋值；THORNS 不跑修正（isPureDamage）。
#[derive(Clone, Debug)]
pub struct DamageInfo {
    pub owner_is_player: bool,
    pub base: i32,
    pub output: i32,
    pub ty: DamageType,
    pub modified: bool,
}

impl DamageInfo {
    /// 普通构造：立即按卡牌链求 output（对应原版 `new DamageInfo(p, this.damage, type)` 前的 applyPowers）。
    pub fn from_card(
        attacker_powers: &[Power],
        attacker_stance: StanceKind,
        target_powers: &[Power],
        base: i32,
        ty: DamageType,
    ) -> Self {
        let output = card_damage(attacker_powers, attacker_stance, target_powers, base, ty);
        Self {
            owner_is_player: true,
            base,
            output,
            ty,
            modified: output != base,
        }
    }

    /// THORNS/矩阵用：不改数值（原版 isPureDamage 跳过修正）。
    pub fn pure(owner_is_player: bool, amount: i32, ty: DamageType) -> Self {
        Self {
            owner_is_player,
            base: amount,
            output: amount,
            ty,
            modified: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(kind: crate::power::PowerKind, amount: i32) -> Power {
        Power::new(kind, amount)
    }

    #[test]
    fn strength_then_vulnerable_floor() {
        // (6 + 3 力量) × 1.5 易伤 = 13.5 → floor 13
        let atk = [p(crate::power::PowerKind::Strength, 3)];
        let tgt = [p(crate::power::PowerKind::Vulnerable, 2)];
        assert_eq!(
            card_damage(&atk, StanceKind::Neutral, &tgt, 6, DamageType::Normal),
            13
        );
    }

    #[test]
    fn weak_multiplier_floor() {
        // (10 + 2 力量) × 0.75 虚弱 = 9.0 → 9
        let atk = [
            p(crate::power::PowerKind::Strength, 2),
            p(crate::power::PowerKind::Weak, 1),
        ];
        assert_eq!(
            card_damage(&atk, StanceKind::Neutral, &[], 10, DamageType::Normal),
            9
        );
        // 7 × 0.75 = 5.25 → 5（floor 验证）
        let atk2 = [p(crate::power::PowerKind::Weak, 1)];
        assert_eq!(
            card_damage(&atk2, StanceKind::Neutral, &[], 7, DamageType::Normal),
            5
        );
    }

    #[test]
    fn stance_multipliers_position() {
        // Wrath ×2 在 give 段（力量之后、易伤之前）：(5+1)×2 ×1.5 = 18
        let atk = [p(crate::power::PowerKind::Strength, 1)];
        let tgt = [p(crate::power::PowerKind::Vulnerable, 1)];
        assert_eq!(
            card_damage(&atk, StanceKind::Wrath, &tgt, 5, DamageType::Normal),
            18
        );
        // Divinity ×3：5×3=15
        assert_eq!(
            card_damage(&[], StanceKind::Divinity, &[], 5, DamageType::Normal),
            15
        );
    }

    #[test]
    fn slow_scales_receive() {
        // Slow ×(1+0.1n)：10 × 1.3 = 13
        let tgt = [p(crate::power::PowerKind::Slow, 3)];
        assert_eq!(
            card_damage(&[], StanceKind::Neutral, &tgt, 10, DamageType::Normal),
            13
        );
    }

    #[test]
    fn monster_intent_with_back_attack_truncation() {
        // 背击中途截断：base 7 → 玩家无 buff → (int)(7*1.5) = 10（floor 应为 10，但截断语义在此验证）
        assert_eq!(
            monster_calculate_damage(&[], &[], StanceKind::Neutral, 7, true),
            10
        );
        // 无背击：7 → 7
        assert_eq!(
            monster_calculate_damage(&[], &[], StanceKind::Neutral, 7, false),
            7
        );
        // Wrath 承伤：怪物 10 打 Wrath 玩家 → ×2 = 20
        assert_eq!(
            monster_calculate_damage(&[], &[], StanceKind::Wrath, 10, false),
            20
        );
        // 怪物力量：10+3 = 13，易伤玩家 ×1.5 = 19.5 → 19
        let mp = [p(crate::power::PowerKind::Strength, 3)];
        let pp = [p(crate::power::PowerKind::Vulnerable, 1)];
        assert_eq!(
            monster_calculate_damage(&mp, &pp, StanceKind::Neutral, 10, false),
            19
        );
    }

    #[test]
    fn block_chain_dexterity_frail() {
        // (5 + 3 敏捷) × 0.75 脆弱 = 6.0 → 6
        let pw = [
            p(crate::power::PowerKind::Dexterity, 3),
            p(crate::power::PowerKind::Frail, 1),
        ];
        assert_eq!(block_value(&pw, 5), 6);
        // 0 基础 + 敏捷 → 加法后钳
        let dex = [p(crate::power::PowerKind::Dexterity, 4)];
        assert_eq!(block_value(&dex, 0), 4);
        // 负敏捷到负值钳 0
        let neg = [p(crate::power::PowerKind::Dexterity, -3)];
        assert_eq!(block_value(&neg, 2), 0);
    }

    #[test]
    fn info_chain_player_owner_wrath() {
        // 玩家是 owner（荆棘等非常规来源）：give → Wrath give ×2 → receive
        // base 4, Wrath, target vuln: 4×2×1.5 = 12
        let tgt = [p(crate::power::PowerKind::Vulnerable, 1)];
        assert_eq!(
            info_apply_powers(&[], true, &tgt, StanceKind::Wrath, 4, DamageType::Normal),
            12
        );
    }
}
