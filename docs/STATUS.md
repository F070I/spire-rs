# sts-rs 项目状态快照（2026-10-17）

> 本文档为 token 预算接近尾声时的交接快照。下个会话/接手者按此继续。

## 已完成（git 最新 commit：4d10017）

### 批次 1：地基
- sts-data::rng：RandomXS128 + JDK LCG + STS Random 包装（基线交叉验证）
- sts-data::gen：375 张卡静态表 + 181 遗物 + 42 药水 + 149 能力 + 68 怪 + 52 事件 + 关键词（eng+zhs 双语）
- tools/extract.py：机械抽取流水线（升级 delta/ID 对账）

### 批次 2：战斗引擎
- creature/damage 数值管线（三条伤害链/格挡/承伤/heal，f32 精度）
- 能力机制层（ApplyPower 分支序/priority 稳定排序/叠层哨兵/Artifact 拦截/justApplied）
- 动作调度器（四队列六分支/打卡全链/回合循环/洗牌分解）
- 卡牌机制层（升级数值机械抽取 292+83 custom/canUse 链/X 费/retain/ethereal）

### 批次 3：内容灌注 100%
- **红 75/75 + 绿 75/75 + 蓝 76/76 + 紫 77/77 + 无色 39 + 诅咒 14 + 状态 5 + 临时 9 + 选项 5 = 375/375 全对账 0 缺口**
- 例程 20+ 个（攻击例程/消耗/升级/抽牌/弃牌/引动/毒系/X 费等）
- 能力钩子面第一批：回合末（Metallicize/Combust/NoxiousFumes/LikeWater/WreathOfFlame）、抽牌后（DemonForm/Berserk 等 7 个）、消耗（DarkEmbrace/FeelNoPain/Sentinel）、出牌（ThousandCuts）、抽牌（Evolve/FireBreathing/EndlessAgony）、获格挡（Juggernaut）、Corruption 全套、Mantra→Divinity 信号
- PowerKind 40+ 个（含正式化 Poison/Thorns/Mantra 等）

### 批次 4a：地图生成 + 房间分配 + 遭遇池
- map.rs：MapGenerator 全算法（6 路径/祖先检查/同行防交叉/Boss 边/首行去重/怪癖保真）
- map/assign.rs：RoomTypeAssigner 三规则精确语义（parent 只适用 R/T/$/E、y==0 豁免）
- encounters.rs：四幕遭遇池权重表（弱敌/强敌/精英/Boss + A-B-A 禁忌）

### 4.1 RNG 接线
- BattleRngs：shuffle/card_random/ai/misc（seed+floorNum 重播种），随机占位全回填

## ## 本轮追加（UI 后续推进 3，2026-10-17）

- 特殊怪物机制：SpecialAi 枚举 7 种（Cultist 仪式/Guardian 三态/SlimeBoss 分裂/TimeEater
  每12卡/Transient 逃跑/Lagavulin 沉睡/Nemesis 相位）——tick 驱动+出牌计数触发（9b079ae）
- 事件累计 14 件：+8（图书馆/石像/黏液坑/有翼石像/黄金神龛/铁匠/祭坛/设计师）（c9bf595）

## 本轮追加（UI 后续推进 2，2026-10-17）

- 商店完整：ShopState.generate（5卡/2遗物/3药水，价格±10%）+ 购买/移除 API + 渲染命令（47d0bba）
- 事件系统首批：6 事件（文本+选项+效果：金/血/升级/移除/遗物/诅咒）+ 渲染命令（1d2b2a6）
- 批次 5 余量：解锁链（Silent→Defect→Watcher）+ 进阶上限 20 + RunMetrics 运行统计（f56cdeb）

## 本轮追加（UI 后续推进，2026-10-17）

- 房间交互深化：篝火休息/锻造、宝箱开箱（50/33/17 箱型+金币）、商店/事件阶段（7693dc3）
- 钩子清账第二批：Establishment 降费/Equilibrium 保留整手/Loop 引动首球/WaveOfTheHand onScry（265a627）
- 玩家遗物效果第一批：RelicCtx 7 字段（Vajra/Anchor/BloodVial/Lantern/Mercury/Orichalcum/BurningBlood）+ 13 遗物 ID 映射（1b02275）
- 批次 3.9 第一批：怪物 AI 表 55+ 条目（HP 区间/招式脚本/ascension 修正）接入 start_battle（18f250c）

剩余工作（按优先级）

### 统一 UI 设计系统 ✅（2026-10-17 全 6 阶段完成）
- theme.rs：集中主题（低饱和彩色 + 纯黑白反显选中），t 键切换不改游戏状态
- ui_core.rs：CJK 宽度/裁切/填充/列表项（选中▸/禁用×）
- ui/map.rs：**15 层全图一次渲染**（先边后节点/当前@/已访问·/可达▸/Boss 行）+ 拓扑守护测试 + 双主题结构等价测试
- ui/battle.rs：状态栏/敌人意图⚔/手牌（升级+需目标标注）/牌堆
- ui/panels.rs：篝火/商店/事件/牌库/药水/遗物/结算统一列表详情
- main.rs 分支全接入（地图/战斗/奖励/篝火）

### 批次 6：终端 UI ✅（2026-10-17 可玩骨架）
- 行协议：stdin 命令 → PlayerCommand 等价 → stdout 渲染（Neow/地图/战斗/奖励/存读档）
- 冒烟验证全链通：Neow→选节点→Bash 出牌→结算
- crossterm 暂缓（工具链缺 dlltool；接口不变，换后端零成本）

### 批次 4b：爬塔流程编排 ✅（2026-10-17 完成）
1. ✅ RunState↔BattleState 接线（flow4b.rs：choose_node/enter_room/start_battle/settle_battle）
2. ✅ 房间流程：战斗触发/奖励（金币+卡3选1+遗物+药水）/篝火（回血+锻造）/药水使用
3. ✅ 幕间转换（next_act：满血回/mapRng seed+act*100/队列重生成/Boss 池切换）
4. ⏳ Neow 开局（四类祝福）——待做

### 批次 3.5 钩子清账（部分完成 12 项）
已接：Rage/AfterImage/DoubleTap/Burst/Echo/Choke（出牌）、Buffer/Thorns/StaticDischarge/Envenom（受击）、Tactician（弃牌）、Brutality（回合末）
- 出牌系：DoubleTap/Burst/Envenom/AfterImage/Necronomicon/HeelHook/WaveOfTheHand
- 受击系：Caltrops(Thorns)/FlameBarrier/StaticDischarge/Choke/TalkToTheHand/WrathNextTurn
- 其他：Rage/Rupture/Establishment/Panache/InfiniteBlades(HOOK已接)/Magnetism/HelloWorld/Mayhem/Phantasmal/RetainCard/Equilibrium/Buffer/Repair/Bias/Echo/Loop/Electro/LockOn/DrawCardNextTurn/Regret/Burn/Pain/Decay/Doubt/Shame/Void/Normality/Necronomicurse/CurseOfTheBell 等诅咒/状态触发
- Mark/PressurePoints/Judgement 机制

### 批次 5：存档 ✅（save_json/load_json roundtrip + 垃圾输入防护；解锁/metrics 未做）
### 批次 6：终端 UI + 三平台发行

## 关键架构决策（冻结）

1. GameAction 枚举 + 四队列调度（每逻辑帧一 action，时长即时）
2. CardKind 静态表 + CardInst 实例（eff_cost/damage/block/magic + misc 通用成长）
3. PowerKind 枚举 + ops 层（ApplyOutcome/stack/reduce/remove + pending_removals 延迟移除）
4. BattleRngs 四流（seed+floorNum）+ RunState 十二流（未接完）
5. ID 硬契约：23+ 例 ID↔类名错位以 extracted.json 为准

## 消耗

**实际约 3.35 亿 / 3.5 亿 token（96%）**——剩余约 1500 万。

> 注意：此前多个会话内部估算（9700 万等）严重偏低，未计入子代理隐性开销与
> 长会话上下文重复成本。以后估算请以平台实际计量为准。

## 交接指引

剩余 1500 万 token 不足以在会话内完成 4b/5/6。建议接手方式：
1. 新会话直接说「继续推进」——STATUS.md + 记忆文件已完整记录进度与规则
2. 优先做 4b 楼层推进编排（RunState↔BattleState 接线），这是通向可玩的最短路径
3. 每完成一个模块更新本文件的消耗数字与状态
