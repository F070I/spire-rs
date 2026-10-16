# 模块记录：sts-core::{action, battle}（批次 2.3 动作调度器与战斗循环）

批次：2.3（四队列调度 + 打卡链 + 回合循环 + G2 沙盘）
状态：✅ 完成（骨架 + 核心动作集）；~270 个动作类与全钩子面在批次 3.10 补全
实现位置：
- `crates/sts-core/src/action.rs`：GameAction 枚举（20 变体）、CardQueueItem、ActionManager（四队列 + 标志 + clearPostCombat 白名单）
- `crates/sts-core/src/battle.rs`：CardInst/CardGroup 数值核心、Monster（脚本怪）、BattleState（tick 六分支 + 动作执行 + 打卡链 + 回合循环）

## 保真点（对照记录）

| 语义 | 出处 | 测试 |
|---|---|---|
| getNextAction 六分支优先级（actions→preTurn→cardQueue→怪物排队→怪物回合→回合推进），顺序不得改 | 记录 01 §3 | 沙盘全部测试依赖此序 |
| 打卡链：入队不扣能 → 消费时扣能/离手/效果入队 → UseCardAction 去向（POWER 不入堆/exhaust→消耗堆/其余→弃牌堆） | 记录 09 §1-2 | strike_via_full_play_chain、bash_applies_vulnerable |
| calculateCardDamage 在打出瞬间重算（含目标侧 receive） | 记录 09 §1 阶段C | bash_then_strike（易伤 9 伤） |
| endTurn 匿名动作序：EndTurn(turnHasEnded) → MonsterStartTurn(怪掉格挡) → monsterAttacksQueued=false → queueMonsters → 逐怪 takeTurn | 记录 01 §8、09 §9 | defend_blocks、monster_block_wiped_at_its_turn |
| 新回合序：双方 atEndOfRound（**先玩家后怪物**）→ turn++ → 掉格挡 → 抽牌(endTurnDraw=充能) → EnableEndTurnButton | 记录 01 §3-6、03 §10 | monster_debuffs_just_applied、defend_blocks（能量/手牌/turn=2） |
| 抽牌：手满钳 10、堆空洗弃牌堆（全部移入+JDK shuffle）、抽 1 语义 | 记录 09 §4 | turn_loop（牌循环）、battle_start |
| 怪物回合开始掉格挡（applyPreTurnLogic） | 记录 01 §8 | monster_block_wiped_at_its_turn |
| clearPostCombatActions 白名单（Heal/GainBlock/Damage/LoseHp 保留） | 记录 01 §3 | battle_over_when_monster_dies |
| 战斗结束判定：全体怪 dying → is_battle_over | 记录 09 §9 | 同上 |
| 毒：atEndOfRound 伤害（HP_LOSS）+ 递减，玩家与怪物双侧 | 记录 03 §2 | poison_ticks_at_round_end |
| justApplied：怪施加的易伤本回合结束不递减 | 记录 03 §2 | monster_debuffs_just_applied |

## G2 沙盘结果（crates/sts-tests/tests/g2_sandbox.rs，11 项全过）

开场序列（能量/抽5/turn=1）、Strike 全链（入队不扣能→6 伤→弃牌堆）、Bash 转译（8 伤+易伤 2 → 连击 9 伤）、Defend 格挡→怪 10 攻穿透 5→新回合清挡回能、双回合牌循环（弃→洗回→重抽）、毒的 HP_LOSS 与递减、justApplied 时序、力量/虚弱 give 段、Wrath ×2、怪死战斗结束。

全仓 51 测试通过，clippy 0。

## 已知差异 / 待办（关键）

1. **RNG 流未接线（必须记入 4.1）**：tick/shuffle 的 `rng` 参数当前由测试传入；最终形态必须是 RunState 的 shuffleRng 流（seed+floorNum 每层重播种），洗牌消耗点已按原版位置落位（initializeDeck/EmptyDeckShuffle 等价处），批次 4.1 接线后由种子回归验证消耗序列。
2. 卡牌效果仅转译 Bash（首张样板）+ 通用攻击/防御；375 卡行为在批次 3。
3. onPlayCard/onUseCard/onAfterUseCard/onExhaust/triggerWhenDrawn 等钩子链结构已定位，调用点全部标注 TODO 3.5/3.6。
4. retain/ethereal 的回合末弃牌分支、 innate/瓶装置顶、X 费、单目标校验（canUse 完整链）随 2.4 卡牌层补全。
5. 怪物为脚本怪；3.9 换真实 73 种（含 aiRng rollMove）。
6. 能量充值位置修正说明：原版 recharge 寄生在 PlayerTurnEffect（渲染层），本实现挂在 DrawCard(endTurnDraw=true) 首帧——语义等价（记录 01 风险3 的既定处理）。
