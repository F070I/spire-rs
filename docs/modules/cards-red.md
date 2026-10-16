# 模块记录：批次 3.2 铁甲战士卡牌灌注

状态：🚧 进行中——29/75 张已转译（含批次 2.4 的 Bash），本记录跟踪进度
实现位置：`crates/sts-core/src/cards/red.rs` + `routines.rs` + `tests/cards_red.rs`

## 已转译清单（29 张）

批次 A（基建样板）：Bash, Body Slam, Clash*, Clothesline, Headbutt*, Heavy Blade, Iron Wave, Perfected Strike, Pommel Strike, Shrug It Off, Thunderclap, Twin Strike, Warcry*, Wild Strike（*含选择屏/例程 TODO）
批次 B（本轮试点）：Anger, Armaments, Blood for Blood, Bloodletting, Burning Pact, Cleave, Dropkick, Flex, Havoc*, Inflame, Intimidate, Rage, Rampage, True Grit, Whirlwind

## 机制新增（随试点落地）

- GameAction 变体补齐：MakeTempCardInDiscard、DamageAll（预计算矩阵）、Discard、MakeTempCardInHand/InDrawPile、ChannelOrb/EvokeOrb/ChangeStance、CardRoutine
- 例程注册表 4 项：WhirlwindAction（X 费×全体）、ArmamentsAction（升级手牌）、ExhaustAction（随机占位）、ModifyDamageAction（Rampage misc 成长）
- PowerKind 新增：LoseStrength（**原版 ID 恒为 "Flex"**——ID 契约特例，回合末收回力量钩子已实现）、Rage（onPlayCard 钩子 TODO 3.5）
- Blood for Blood 费用联动：消耗时手/抽/弃牌堆 B4B -1（挂接点核对 TODO 3.10）

## 数值纠错记录（源码 > 记忆）

- Bloodletting 是 **0 费**卡（回 2/3 能）
- True Grit baseBlock=**7**（非 5）
- Rampage misc 成长 +**5**（升级 +3 → 8）
- Weak 只在攻击方 give 段 ×0.75——被打方身上的 Weak 不影响来伤害（第二次踩同一坑，指南需强调）

## 选择屏 / 随机占位清单（批次 6 / 4.1 清账）

1. Headbutt（弃牌堆选 1 置顶）—— 例程未实现
2. Warcry（抽 1 弃任意 + 置抽牌堆位）—— 例程未实现
3. Havoc（PlayTopCard：抽牌堆顶免费打出+消耗；随机目标 TODO 4.1）—— 例程未实现
4. Burning Pact / True Grit / Armaments 未升级分支的**选择屏**——占位（取顶/首张）
5. True Grit 未升级的随机消耗——占位（取顶）
6. Clash 的 canUse 限制（本回合未打出非攻击牌）——未实现（canUse 链 3.5）
7. Whirlwind/Rampage 随机——无

## 测试结果

75 测试通过（本轮新增 13），clippy 0。

## 下一批

红卡剩余 46 张（含 Feed/Reaper/Corruption/Impervious/Demon Form/Bludgeon/Offering/Immolate/Exhume 等复杂卡）——建议子代理分 2 批（每批 ~23 张）转译；代理通道恢复后按 docs/guides/card-translation.md 执行。
