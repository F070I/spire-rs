# 模块记录：批次 3.2 铁甲战士卡牌灌注

> 批次 3.3 绿卡进度（2026-10-17）：**75/75 全部转译完成**（cards/green.rs，12 专项测试 + 全量 108）。
> ID 珍品：Nightmare 类 ID="Night Terror"、Alchemize 类 ID="Venomology"、SneakyStrike 类 ID="Underhanded Strike"、WraithForm 类 ID="Wraith Form v2"（第 5-8 例）。
> 新增 PowerKind：Poison（正式化）/Accuracy/Blur/Choke/DodgeAndRoll/Energized/Thorns；
> 钩子：Blur（不掉格挡）、DodgeAndRoll/Energized（抽牌后）、Endless Agony（抽到再抽）；
> 例程：EscapePlanAction/ExpertiseAction。
> 数值纠错：Footwork 敏捷 2、Dodge and Roll 块 4、Dash 双值独立于 magic、Bane 无条件首段、Reaper 4 伤/次实伤回 1、鲁莽冲锋 7 伤。
> 第二批钩子：NoxiousFumes（回合末全体加毒）、ThousandCuts（出牌全体伤）、InfiniteBlades/WraithForm/ToolsOfTheTrade（抽牌后）、计数器（totalDiscardedThisTurn/attacksPlayedThisTurn）。

状态：✅ **75/75 全部转译完成**（2026-10-17）；残留例程/选择屏占位见清账
实现位置：`crates/sts-core/src/cards/red.rs` + `routines.rs` + `tests/cards_red.rs`

## 已转译清单（52 张）

批次 A（基建样板）：Bash, Body Slam, Clash*, Clothesline, Headbutt*, Heavy Blade, Iron Wave, Perfected Strike, Pommel Strike, Shrug It Off, Thunderclap, Twin Strike, Warcry*, Wild Strike（*含选择屏/例程 TODO）
批次 B（试点）：Anger, Armaments, Blood for Blood, Bloodletting, Burning Pact, Cleave, Dropkick, Flex, Havoc*, Inflame, Intimidate, Rage, Rampage, True Grit, Whirlwind
批次 C（第 2 批）：Barricade, Battle Trance, Berserk, Bludgeon, Carnage, Combust, Corruption, Dark Embrace, Demon Form, Disarm, Double Tap, Entrench, Evolve, Feed, Feel No Pain, Fiend Fire, Flame Barrier, Ghostly Armor, Hemokinesis, Immolate, Infernal Blade*, Juggernaut, Limit Break, Metallicize
批次 D（收尾）：Defend_R, Strike_R（显式化）, Exhume*, Impervious, Offering, Power Through, Pummel, Reaper, Reckless Charge, Rupture*, Searing Blow, Second Wind, Seeing Red, Sentinel, Sever Soul, Shockwave, Spot Weakness, Sword Boomerang, Uppercut

## 机制新增（随试点落地）

- GameAction 变体补齐：MakeTempCardInDiscard、DamageAll（预计算矩阵）、Discard、MakeTempCardInHand/InDrawPile、ChannelOrb/EvokeOrb/ChangeStance、CardRoutine
- 例程注册表 4 项：WhirlwindAction（X 费×全体）、ArmamentsAction（升级手牌）、ExhaustAction（随机占位）、ModifyDamageAction（Rampage misc 成长）
- PowerKind 新增：LoseStrength（**原版 ID 恒为 "Flex"**——ID 契约特例，回合末收回力量钩子已实现）、Rage（onPlayCard 钩子 TODO 3.5）
- **能力回合钩子面第一批（3.5 开工）**：Metallicize（回合末+格挡）、Combust（回合末掉血+全体 THORNS 伤）、DemonForm（抽牌后+力量）、Berserk（抽牌后+能量递减）、Barricade（双侧不掉格挡，含怪物回合）、DarkEmbrace/FeelNoPain（消耗钩子）、Evolve/FireBreathing（抽牌钩子）、Juggernaut（获格挡反伤 THORNS）、Corruption（四堆技能 0 费+免能+消耗）
- 例程新增：FiendFireAction（消耗整手按张伤害）、LimitBreakAction（力量叠加翻倍）、FeedAction（斩杀加最大生命）、InfernalBladeAction（TODO 4.1 随机池）
- Blood for Blood 费用联动：消耗时手/抽/弃牌堆 B4B -1（挂接点核对 TODO 3.10）

## 4.1 RNG 接线（随本批完成）

- BattleRngs：shuffle/card_random/ai/misc 四条流（seed+floorNum 每层重播种；monsterHp 留 3.9）
- 回填占位：随机弃牌（DiscardAction）、addToRandomSpot、随机敌人（Sword Boomerang）、洗牌（开局/牌堆空）
- tick/run_until_idle/start_battle 签名去 rng 参数（流由 BattleState 持有）

## 数值纠错记录（源码 > 记忆）

- Bloodletting 是 **0 费**卡（回 2/3 能）
- True Grit baseBlock=**7**（非 5）
- Rampage misc 成长 +**5**（升级 +3 → 8）
- Weak 只在攻击方 give 段 ×0.75——被打方身上的 Weak 不影响来伤害（第二次踩同一坑，指南需强调）
- Reaper：4 伤全体 + 每点实扣血回 1（不是 20 伤）
- Reckless Charge：7 伤（非 14）；Sever Soul **无自身消耗**；Spot Weakness **不造成伤害**（纯力量条件卡）
- Second Wind：SW 自身无消耗标志，格挡数 = 动作执行时的手牌非攻击数

## 选择屏 / 随机占位清单（批次 6 / 4.1 清账）

1. Headbutt（弃牌堆选 1 置顶）—— 例程未实现
2. Warcry（抽 1 弃任意 + 置抽牌堆位）—— 例程未实现
3. Havoc（PlayTopCard：抽牌堆顶免费打出+消耗；随机目标 TODO 4.1）—— 例程未实现
4. Burning Pact / True Grit / Armaments 未升级分支的**选择屏**——占位（取顶/首张）
5. True Grit 未升级的随机消耗——占位（取顶）
6. Clash 的 canUse 限制（本回合未打出非攻击牌）——未实现（canUse 链 3.5）
7. Whirlwind/Rampage 随机——无

## 测试结果

96 测试通过（收尾批新增 11），clippy 0。红卡全量完成。

## Searing Blow 特例

- can_upgrade 恒 true（ID 判定）；apply_upgrade 走 misc 累进（+4+timesUpgraded）；eff_damage 路径用 base_damage + misc。83 张 custom_upgrade 其余卡在批次 3 逐个转译（extrator 已标记）。

## 下一批

红卡剩余 23 张：Defend_R, Exhume, Impervious, Metallicize✓(done), Offering, Power Through, Pummel, Reaper, Reckless Charge, Rupture, Searing Blow, Second Wind, Seeing Red, Sentinel, Sever Soul, Shockwave, Spot Weakness, Strike_R, Sword Boomerang, Uppercut（以 extract 对账为准）——多数依赖选择屏（Second Wind/Sever Soul/Impervious 的消耗选择、Offering）与随机（Sword Boomerang、Reckless Charge 的 Dazed 洗入），待 4.1 RNG 接线后效率更高。
