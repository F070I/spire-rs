//! 生物实体（`core/AbstractCreature.java` 的数值核心，记录 01 §1）。
//!
//! 阶段 2.1 范围：生命/格挡/治疗/加格挡/减格挡的数值语义与钳制；
//! 遗物钩子（onPlayerHeal 等）与完整承伤管线在批次 2.2/2.3 接入
//! （`damage.rs` 已提供数值链）。

use crate::power::DamageType;

/// 原版数值边界（core/AbstractCreature.java / EnergyPanel）。
pub const MAX_BLOCK: i32 = 999;

/// 生物（玩家与怪物共用字段；完整子类字段在 player.rs / monster/ 扩展）。
#[derive(Clone, Debug)]
pub struct Creature {
    pub name: String,
    pub current_health: i32,
    pub max_health: i32,
    pub current_block: i32,
    pub is_dying: bool,
    pub is_dead: bool,
    pub half_dead: bool,
    pub powers: Vec<crate::power::Power>,
    pub is_player: bool,
    /// 上一笔"实际扣血"（lastDamageTaken = min(dmg, currentHealth)，记录 01 §1）。
    pub last_damage_taken: i32,
}

impl Creature {
    pub fn new(name: &str, hp: i32, is_player: bool) -> Self {
        Self {
            name: name.to_string(),
            current_health: hp,
            max_health: hp,
            current_block: 0,
            is_dying: false,
            is_dead: false,
            half_dead: false,
            powers: Vec::new(),
            is_player,
            last_damage_taken: 0,
        }
    }

    /// `increaseMaxHp(amount)`：maxHp 增加并同步 heal 同等量（记录 01 §1）。
    pub fn increase_max_hp(&mut self, amount: i32) {
        self.max_health += amount;
        self.heal(amount);
    }

    /// `decreaseMaxHealth`：maxHp 减，下限 1；当前血超出则夹到 maxHp。
    pub fn decrease_max_hp(&mut self, amount: i32) {
        self.max_health = (self.max_health - amount).max(1);
        if self.current_health > self.max_health {
            self.current_health = self.max_health;
        }
    }

    /// `heal(amount)`（AbstractCreature:402-446 的数值核心）：
    /// isDying 直接拒绝 → powers onHeal（TODO 2.2）→ 加血钳 maxHp → 半血以上清 isBloodied。
    /// Endless Muzzle 减半 TODO(批次4.9)。
    pub fn heal(&mut self, amount: i32) -> i32 {
        if self.is_dying {
            return 0;
        }
        let amount = amount.max(0);
        let before = self.current_health;
        self.current_health = (self.current_health + amount).min(self.max_health);
        let healed = self.current_health - before;
        // isBloodied 清除与遗物 onNotBloodied —— TODO(批次2.2 遗物层)
        healed
    }

    /// `addBlock(amount)` 的数值核心（AbstractCreature:448-495）：
    /// powers 钩子（TODO 2.2 完整化）→ floor → 钳 999。
    pub fn add_block(&mut self, amount: i32) -> i32 {
        let mut tmp = amount as f32;
        for p in &self.powers {
            tmp = crate::power::modify_block(p, tmp);
        }
        for p in &self.powers {
            tmp = crate::power::modify_block_last(p, tmp);
        }
        let v = tmp.floor() as i32;
        let v = if v < 0 { 0 } else { v };
        let before = self.current_block;
        self.current_block = (self.current_block + v).min(MAX_BLOCK);
        self.current_block - before
    }

    /// `loseBlock()`：清空全部格挡。
    pub fn lose_block(&mut self) {
        self.current_block = 0;
    }

    /// `loseBlock(amount)`：扣指定量，下限 0。
    pub fn lose_block_amount(&mut self, amount: i32) {
        self.current_block = (self.current_block - amount).max(0);
    }

    /// `decrementBlock(info, damageAmount)`（AbstractCreature:186-214）的数值核心。
    ///
    /// 返回 (穿透到生命的伤害, 是否破格挡)。
    /// - HP_LOSS 与 THORNS 完全无视格挡（decrementBlock 条件：type != HP_LOSS 且 THORNS 旁路，记录 03 §5-4）；
    /// - 伤害 > 格挡 → 全清 + 破格挡 + 剩余穿透；
    /// - 等于 → 全清 + 破格挡（"Absorbed"）；
    /// - 小于 → 部分消耗。
    pub fn decrement_block(&mut self, ty: DamageType, damage_amount: i32) -> (i32, bool) {
        if ty == DamageType::HpLoss || ty == DamageType::Thorns || self.current_block <= 0 {
            return (damage_amount, false);
        }
        let block = self.current_block;
        if damage_amount > block {
            self.lose_block();
            (damage_amount - block, true)
        } else if damage_amount == block {
            self.lose_block();
            (0, true)
        } else {
            self.lose_block_amount(damage_amount);
            (0, false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heal_clamps_and_rejects_dying() {
        let mut c = Creature::new("Ironclad", 80, true);
        c.current_health = 10;
        assert_eq!(c.heal(30), 30);
        assert_eq!(c.current_health, 40);
        assert_eq!(c.heal(999), 40); // 钳 maxHp
        c.is_dying = true;
        assert_eq!(c.heal(999), 0);
    }

    #[test]
    fn increase_max_hp_heals_same_amount() {
        let mut c = Creature::new("p", 50, true);
        c.current_health = 50;
        c.increase_max_hp(5);
        assert_eq!((c.max_health, c.current_health), (55, 55));
        // 非满血时 increase 同步回血 5
        let mut d = Creature::new("p", 50, true);
        d.current_health = 30;
        d.increase_max_hp(5);
        assert_eq!((d.max_health, d.current_health), (55, 35));
    }

    #[test]
    fn decrease_max_hp_floor_1() {
        let mut c = Creature::new("p", 10, true);
        c.current_health = 10;
        c.decrease_max_hp(50);
        assert_eq!((c.max_health, c.current_health), (1, 1));
    }

    #[test]
    fn add_block_clamps_999_and_negative_dex() {
        let mut c = Creature::new("p", 10, true);
        c.current_block = 995;
        assert_eq!(c.add_block(10), 4);
        assert_eq!(c.current_block, 999);
        let mut d = Creature::new("p", 10, true);
        d.powers.push(crate::power::Power::new(
            crate::power::PowerKind::Dexterity,
            -3,
        ));
        assert_eq!(d.add_block(2), 0);
    }

    #[test]
    fn decrement_block_three_branches() {
        let mut c = Creature::new("p", 10, true);
        c.current_block = 5;
        // 大于：穿透 + 破格挡
        assert_eq!(c.decrement_block(DamageType::Normal, 8), (3, true));
        assert_eq!(c.current_block, 0);
        // 小于：部分消耗
        c.current_block = 5;
        assert_eq!(c.decrement_block(DamageType::Normal, 3), (0, false));
        assert_eq!(c.current_block, 2);
        // 等于：Absorbed + 破格挡
        c.current_block = 5;
        assert_eq!(c.decrement_block(DamageType::Normal, 5), (0, true));
        assert_eq!(c.current_block, 0);
    }

    #[test]
    fn thorns_and_hp_loss_bypass_block() {
        let mut c = Creature::new("p", 10, true);
        c.current_block = 9;
        assert_eq!(c.decrement_block(DamageType::HpLoss, 4), (4, false));
        assert_eq!(c.current_block, 9);
        assert_eq!(c.decrement_block(DamageType::Thorns, 6), (6, false));
        assert_eq!(c.current_block, 9);
    }
}
