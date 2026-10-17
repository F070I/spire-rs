# 模块记录：批次 3.2 铁甲战士卡牌灌注

状态：🚧 进行中——52/75 张已转译，本记录跟踪进度
实现位置：`crates/sts-core/src/cards/red.rs` + `routines.rs` + `tests/cards_red.rs`

## 已转译清单（52 张）

批次 A（基建样板）：Bash, Body Slam, Clash*, Clothesline, Headbutt*, Heavy Blade, Iron Wave, Perfected Strike, Pommel Strike, Shrug It Off, Thunderclap, Twin Strike, Warcry*, Wild Strike（*含选择屏/例程 TODO）
批次 B（试点）：Anger, Armaments, Blood for Blood, Bloodletting, Burning Pact, Cleave, Dropkick, Flex, Havoc*, Inflame, Intimidate, Rage, Rampage, True Grit, Whirlwind
批次 C（第 2 批）：Barricade, Battle Trance, Berserk, Bludgeon, Carnage, Combust, Corruption, Dark Embrace, Demon Form, Disarm, Double Tap, Entrench, Evolve, Feed, Feel No Pain, Fiend Fire, Flame Barrier, Ghostly Armor, Hemokinesis, Immolate, Infernal Blade*, Juggernaut, Limit Break, Metallicize

## 机制新增（随试点落地）

- GameAction 变体补齐：MakeTempCardInDiscard、DamageAll（预计算矩阵）、Discard、MakeTempCardInHand/InDrawPile、ChannelOrb/EvokeOrb/ChangeStance、CardRoutine
- 例程注册表 4 项：WhirlwindAction（X 费×全体）、ArmamentsAction（升级手牌）、ExhaustAction（随机占位）、ModifyDamageAction（Rampage misc 成长）
- PowerKind 新增：LoseStrength（**原版 ID 恒为 "Flex"**——ID 契约特例，回合末收回力量钩子已实现）、Rage（onPlayCard 钩子 TODO 3.5）
- **能力回合钩子面第一批（3.5 开工）**：Metallicize（回合末+格挡）、Combust（回合末掉血+全体 THORNS 伤）、DemonForm（抽牌后+力量）、Berserk（抽牌后+能量递减）、Barricade（双侧不掉格挡，含怪物回合）、DarkEmbrace/FeelNoPain（消耗钩子）、Evolve/FireBreathing（抽牌钩子）、Juggernaut（获格挡反伤 THORNS）、Corruption（四堆技能 0 费+免能+消耗）
- 例程新增：FiendFireAction（消耗整手按张伤害）、LimitBreakAction（力量叠加翻倍）、FeedAction（斩杀加最大生命）、InfernalBladeAction（TODO 4.1 随机池）
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

85 测试通过（第 2 批新增 10），clippy 0。

## 下一批

红卡剩余 23 张：Defend_R, Exhume, Impervious, Metallicize✓(done), Offering, Power Through, Pummel, Reaper, Reckless Charge, Rupture, Searing Blow, Second Wind, Seeing Red, Sentinel, Sever Soul, Shockwave, Spot Weakness, Strike_R, Sword Boomerang, Uppercut（以 extract 对账为准）——多数依赖选择屏（Second Wind/Sever Soul/Impervious 的消耗选择、Offering）与随机（Sword Boomerang、Reckless Charge 的 Dazed 洗入），待 4.1 RNG 接线后效率更高。
