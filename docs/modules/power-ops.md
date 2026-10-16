# 模块记录：sts-core::power::ops（批次 2.2 能力机制层）

批次：2.2（ApplyPower/Reduce/Remove/priority 语义核心）
状态：✅ 完成（语义层）；钩子面（onApplyPower/onRemove 副作用等）随批次 3.5 能力灌注补全
实现位置：
- `crates/sts-core/src/power/mod.rs`：PowerKind（+NoDraw）、Power 实例（+just_applied）、priority 表、is_debuff、is_turn_based_decrement、伤害/格挡钩子语义（批次 2.1）
- `crates/sts-core/src/power/ops.rs`：apply_power / stack_power / reduce_power / remove_power / end_of_round_tick / add_power_raw

## 保真点（对照记录）

| 语义 | 出处 | 测试 |
|---|---|---|
| ApplyPowerAction 首帧分支序：Ginger/Turnip → Artifact → 同 ID 叠层 → NoDraw 特判 → 新增 | 记录 09 §8 | ginger_turnip、artifact_intercepts、nodraw_no_stack |
| 新增时按 priority **稳定重排**；叠层**不**重排、不走 onInitialApplication | 记录 03 §2/3 | apply_new_sorts_by_priority_stable、stack_existing_does_not_resort |
| priority 默认 5；Frail=10、Intangible/IntangiblePlayer=75、Weak=99（表随 3.5 扩充） | 记录 03 §3 | 上述排序测试 |
| Artifact 只挡 DEBUFF；Strength 负值仍为 BUFF（PowerType 固定于种类） | 记录 09 §8 | artifact_intercepts_debuff_only |
| Artifact onSpecificTrigger：-1 层，耗尽 → 延迟移除 | 同上 | 同上（pending_removals） |
| stackPower：amount==-1 哨兵不叠加；Strength/Dexterity/Focus 钳 ±999、归 0 → 延迟移除 | 记录 03 §1E/2 | stack_sentinel_and_clamp |
| ReducePowerAction 阈值：减量 < amount → 递减（钳 0）；否则整体移除 | 记录 03 §1E | reduce_threshold_semantics |
| justApplied：怪物施加的回合型减益首次 atEndOfRound 跳过递减 | 记录 03 §2 | end_of_round_just_applied_skip |
| 低级 addPower：无钩子无排序（内部代码专用） | 记录 03 §2 | add_power_raw_no_sort |
| 延迟移除建模：`pending_removals` 由动作层入队（本层不立即删除） | 时序保真 | 签名层强制 |

## 测试结果

40 测试通过（本次新增 9），clippy 0 警告。

## 已知差异 / 待办

1. **钩子面未接入**：施加者 `onApplyPower`（Sadistic 反伤）、Champion Belt 追加施加、`onRemove` 副作用（DrawPower 恢复手牌上限）——随批次 3.5 能力灌注挂接；接口形状已在（ctx / pending_removals / remove_power）。
2. **is_debuff / is_turn_based_decrement 只覆盖子集**：162 能力的 PowerType 表在批次 3.5 生成（可用 extract.py 扩展解析 powers 源码的 `PowerType.DEBUFF` 声明——已列入 3.5 计划）。
3. justApplied 完整集合（NoBlock/AttackBurn/SkillBurn/DoubleDamage/DrawReduction/Intangible）待对应 kind 加入时补表。
4. "Night Terror" 同 ID 排除特例：该能力为怪物专属非标准实例，待怪物批次处理。
