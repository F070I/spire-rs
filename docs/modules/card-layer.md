# 模块记录：批次 2.4 卡牌机制层

批次：2.4（升级数值机械抽取 + 卡实例生效值 + canUse 链 + 回合末弃牌语义）
状态：✅ 完成（机制层）；375 张卡的 use() 行为转译在批次 3（83 张 custom_upgrade 需人工转译）
实现位置：
- `tools/extract.py`：parse_upgrade()（升级 delta 抽取 + custom 启发式）
- `crates/sts-data/src/gen/cards.rs`：新增 up_damage/up_block/up_magic/up_cost/up_flags/custom_upgrade
- `crates/sts-core/src/battle.rs`：CardInst::eff_cost/eff_damage/eff_block/eff_magic/can_upgrade、
  BattleState::can_use、play_card X 费快照、exec_discard_at_end_of_turn（retain/ethereal/Runic Pyramid）
- `crates/sts-tests/tests/card_layer.rs`：10 项测试

## 抽取结果

- **292 张卡机械升级**（upgradeDamage/Block/MagicNumber/BaseCost 字面量）；**83 张 custom_upgrade**
  （SearingBlow 的 4+timesUpgraded、Coolheaded 的文本改写、Blasphemy 的标志改写等——批次 3 逐张转译）。
- 已知断言抽查全过：Strike_R +3、Defend_R +3、Bash +2/+1、Corruption upgradeBaseCost 2、Eruption upgradeBaseCost 1。
- ID 命名再确认："Searing Blow"（带空格）——ID 与类名不同名的第 4 例。

## 保真点

| 语义 | 出处 | 测试 |
|---|---|---|
| upgradeBaseCost 为绝对值（非 delta），costForTurn 同步 | 记录 02 §3 | upgraded_card_uses_effective_values |
| canUse：STATUS 需 MedicalKit、CURSE 需 BlueCandle（cost < -1）、能量段 | 记录 02 §5 | status/curse_unplayable |
| X 费：能量校验恒过、energyOnUse 快照当前能量 | 记录 09 §1 | x_cost_snapshot |
| 回合末弃牌序：retain/selfRetain→limbo→ethereal 消耗→其余弃牌→limbo 回手 | 记录 09 §5 | ethereal_exhausts、self_retain_returns |
| Runic Pyramid：回合末不弃牌 | 记录 09 §5 | runic_pyramid_skips |
| 升级后 Bash：10 伤 + 易伤 3（magic 升级联动效果数值） | cards/red/Bash.java | upgraded_strike_and_bash_in_battle |

## 测试结果

61 测试通过（本次新增 10），clippy 0。

## 过程中修复

1. CardInst::new 的 cost_for_turn 未用生效费用（升级费用卡 Eruption+ 显示 2 费）——真 bug。
2. extract.py custom 启发式两轮误报（355→20→373→292 定格）：根因是反编译残留 `;` 与块注释干扰
   if 包装块的识别；最终顺序 = 删标准调用 → 删分号空白 → 删空 if 包装 → 残留即 custom。
3. 测试断言时机：回合末弃牌断言被新回合抽牌回填干扰——用 game_hand_size=0 隔离（牌循环本身已由
   G2 沙盘 turn_loop 测试覆盖）。

## 已知差异 / 待办

1. 83 张 custom_upgrade 卡的 upgrade() 需批次 3 人工转译（含文本改写类——即使数值无变化）。
2. canUse 尚缺：cardPlayable 目标存活校验、powers canPlayCard、Entangled、手牌 canPlay 链（TODO 3.5）。
3. 卡牌展示值（damage/block/magic 的手牌重算 applyPowers）属 UI 层（批次 6）；核心层在打出瞬间计算
   （calculateCardDamage 语义），与原版效果等价。
4. Wound 打出后入消耗堆依赖静态 exhaust 标志——消耗钩子序（relics→powers→card）在 3.x 挂接。
