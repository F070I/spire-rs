# 模块记录：sts-core::{creature, damage, power, stance}（批次 2.1 数值核心）

批次：2.1（creature/damage 管线的数值核心）
状态：✅ 完成（数值层）；完整承伤钩子链与遗物接入在批次 2.2/2.3
实现位置：
- `crates/sts-core/src/power.rs`：DamageType、PowerKind（子集+Future 兜底）、Power 实例、单能力钩子语义（atDamageGive/Receive/Final、modifyBlock/Last）
- `crates/sts-core/src/stance.rs`：4 架势的 atDamageGive/Receive（Wrath ×2 双向、Divinity ×3）
- `crates/sts-core/src/damage.rs`：三条修正链 + DamageInfo + 格挡链
- `crates/sts-core/src/creature.rs`：Creature 数值核心（HP/heal/加格挡/减格挡/破格挡三分支）

## 保真点（对照记录）

| 语义 | 出处 | 实现与测试 |
|---|---|---|
| 卡牌链顺序：遗物→give→架势→receive→finalGive→finalReceive→floor | 记录 03 §4b | `card_damage`；测试 strength_then_vulnerable_floor |
| 怪物意图链含背击 (int)(×1.5) 中途截断 | 记录 03 §4c | `monster_calculate_damage`；测试 back_attack_truncation |
| Wrath 双向 ×2 / Divinity ×3 位于 give（攻）/receive（防）段 | 记录 03 §9 | 测试 stance_multipliers_position、monster Wrath 承伤 |
| f32 全程 + 末端 floor + 钳非负 | 记录 03 §11-2 | 全部测试 |
| Weak ×0.75 在 give 段、Vulnerable ×1.5 在 receive 段、Slow ×(1+0.1n) | powers/*.java | 测试 weak_multiplier_floor、slow |
| 格挡链 modifyBlock→modifyBlockLast→floor，负敏钳 0 | AbstractCard.applyPowersToBlock | 测试 block_chain_dexterity_frail |
| decrementBlock 三分支 + THORNS/HP_LOSS 无视格挡 + 破格挡标记 | 记录 01 §1、03 §5 | 测试 decrement_block_three_branches、thorns_and_hp_loss_bypass |
| heal：isDying 拒绝、钳 maxHp；increaseMaxHp 同步回血；decreaseMaxHp 下限 1 | AbstractCreature:402/216/228 | 测试 heal_clamps、increase_max_hp、decrease_max_hp_floor_1 |
| addBlock floor + 钳 999 | AbstractCreature:448 | 测试 add_block_clamps_999 |
| lastDamageTaken = min(dmg, currentHealth) | 记录 01 §1 | 字段就位（管线接入时断言） |

## 测试结果

31 测试通过（本次新增 15），clippy 0 警告。

## 已知差异 / 待办

1. 遗物 `atDamageModify`（卡牌链第一环）与 Endless 疫病（DeadlyEnemies/ToughEnemies）未接入——管线已留形状（TODO 3.6 / 4.9）。
2. 完整承伤钩子链（onAttackToChangeDamage→…→onInflictDamage 9 步，玩家与怪物两条不同顺序）在批次 2.3 的动作层接入——数值核心已就绪。
3. PowerKind 当前仅 10 种 + Future 兜底；批次 3.5 补全 162 种并把 Future 逐个消灭（审计靠 `Power.id` 原版字符串）。
4. powers 列表的 priority 排序语义（仅 ApplyPowerAction 新增时重排）在批次 2.2 实装。
