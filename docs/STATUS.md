# sts-rs 项目状态快照（2026-10-17）

> 本文档为 token 预算接近尾声时的交接快照。下个会话/接手者按此继续。

## 已完成（git 最新 commit：5g，288 测试全过，clippy 0）

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

## 本轮追加（批次 4f-4g，2026-10-17）

- **药水 4f（f15f9af）**：42 种全实装（potency 逐瓶按 getPotency() 源码校对）。
  源码纠错：Fire Potion=THORNS 穿格挡、Explosive=10 伤 NORMAL 可格挡、
  Essence of Steel=4 反甲、Steroid/Speed=5+Lose 回收。Entropic Brew 先消耗再
  填满含本槽。Fairy 死亡自动复活（has_fairy_potion 钩子）。新 PowerKind：
  Ritual/Regen/LoseDexterity。
- **遗物第三批 4g（bdf601d）**：+45 件全钩子（出牌计数 Kunai/Shuriken/Fan/
  LetterOpener/ChampionBelt/MummifiedHand/BirdFacedUrn/Damaru/Necronomicon；
  回合 ArtOfWar/Pocketwatch/Inserter/RingOfSerpent/EmotionChip/IceCream；
  承伤 Boot/Torii/LizardTail/RunicCube/SelfFormingClay；战斗开始 BronzeScales/
  Clockwork/TwistedFunnel/MarkOfPain/DuVuDoll/RunicCapacitor/Enchiridion/
  HolyWater/CultistMask/SlaversCollar；拾取 EmptyCage/PandorasBox/Astrolabe/
  TinyHouse/CallingBell/BlackBlood/蛋类×3；flow RegalPillow/MawBank/CeramicFish/
  ToyOrnithopter/AncientTeaSet/Omamori/CursedKey）。映射累计 89/181。
  时序教训：回合开始遗物检查要用「快照后清零」，能量类增益必须排在充能动作
  之后入队（Happy Flower/Art of War 同坑两次）。
- **源码路径迁移**：`C:\Users\17366\Desktop\MySlayTheSpire-master`（原
  OneDrive\Desktop 已迁空）。

## 本轮追加（批次 5h 细节收尾，2026-10-18）

- **多怪细节 + 奖励流（5h）**：Giant Head 倒计时（瞪视/计数→死亡凝视 +5/回合）、
  Maw 咆哮递增（EscalatingBuff）、Nemesis PhaseShift 补接线（原从未生效）；
  Prayer Wheel（卡奖 3→4）/Black Star（精英保底稀有）/Prismatic Shard
  （卡奖 30% 无色）/Matryoshka（开箱 50% 附赠）/Tiny Chest（每 5 层 +25 金）。
  测试 +6，累计 294。遗物映射 165/181。

## 本轮追加（CI 发行，2026-10-18）

- **CI（0aaf9d9）**：release.yml（v* tag → 四矩阵构建 windows-gnu/
  macos-intel/arm64/linux → test --release → 冒烟 → zip+sha256 → GitHub
  Release）+ ci.yml（三 OS 测试 + clippy -D warnings + macOS 双目标交叉
  check）。全库 cargo fmt 清偿（59 文件，行为不变）。README 状态段刷新。
- 至此项目主线完成：可玩完整版 + 测试守护 + CI 发行链路。剩余为低优先
  尾量（选择屏遗物 ~25 / 多怪细节 / crossterm 换后端）。

## 本轮追加（批次 5g 卡上下文，2026-10-18）

- **遗物 5g**：BattleState.playing_card 卡上下文追踪——StrikeDummy（"打击"
  名 +3 伤）/WristBlade（0 费攻击 +4 伤）经 attack_info 生效，零调用点改动；
  Chemical X（cards::x_value——12 处 X 费读点替换，+2 不付能量）；no-op
  装饰件显式登记 18 件；Busted Crown 卡奖励 3→1；Dream Catcher 休息得牌。
  映射 156/181（战斗内机制全覆盖；剩余 25 件为纯选择屏流程类）。
  测试 +5（relics6.rs）。

## 本轮追加（批次 5f 遗物尾量，2026-10-18）

- **遗物 5f（f893a35 + id 修复）**：damage.rs 拆 raw f32 出口（card_damage_raw/
  monster_calculate_damage_raw——纸系乘子取整前折入）；Paper Frog（attack_info
  重算 ×1.75/1.5）/Paper Krane（怪物出伤两处折入 ×0.6/0.75）；TheAbacus（洗牌
  +6 块）/Golden Idol（战斗金 ×5/4）/Bloody Idol（得金回 5）/DollysMirror
  （复制随机卡）/Cauldron（3 药水）；**5 处 id 错位修复**（CloakClasp/
  ClockworkSouvenir/Runic Cube/Paper Crane/Pandora's Box——效果此前从未生效）
  + HolyWater 补漏 + Ring of the Snake（前 2 回合 +2 抽）。映射 132/181。
- 剩余 ~48 件：no-op 装饰（Circlet/masks/Spirit Poop/Red Circlet 等 ~15）、
  选择屏流程（Orrery/PeacePipe/Shovel/Dream Catcher/Question Card/
  Matryoshka/Prayer Wheel/Gambling Chip/Frozen Eye/The Courier/Black Star
  等 ~20）、伤害链卡上下文（StrikeDummy/WristBlade/Chemical X/Snake Skull
  等 ~8）、商店/奖励流（PrismaticShard/Busted Crown 奖励数/White Beast 等）。
  测试 +7（relics5.rs）。

## 本轮追加（批次 5e 遗物第四批，2026-10-18）

- **遗物 5e（e1b4290）**：+40 件全钩子。战斗开始（Akabeko Vigor——新
  PowerKind::Vigor/备用袋/忍者卷轴/共生病毒/泪坠盒/核能电池/工具箱/破冠/穹顶）；
  回合（Brimstone/船栓/舵轮/香炉/弯曲钳）；出牌（Pen Nib ×2——attack_info
  出口整段翻倍+PenNibConsume 动作、OrangePellets、VioletLotus、Unceasing Top）；
  弃牌/消耗/死亡（Tingsha/韧带绷带/风筝/枯枝/灰烬/哥布林角/标本）；
  治疗（魔花 ×1.5/马可之花归零——relic_adjusted_heal）；商店（Membership Card
  半价统一入口、餐券、永恒之羽、Old Coin、Potion Belt、Darkstone）。
  映射累计 ~129/181。测试 +10。

## 本轮追加（批次 5d 二三幕加权表，2026-10-18）

- **二三幕怪加权表（dd115f7）**：+20 怪进 WEIGHTED 表（第二幕 Byrd/Centurion/
  Chosen/Shell Parasite/Spheric Guardian/Snake Plant/Snecko/Book of Stabbing/
  Taskmaster/Bandit×2；第三幕 Darkling/Reptomancer/Spire Growth/Maw/Nemesis/
  Writhing Mass/Giant Head/Spire Shield/Spear）。条目扩展第 5 位段数——多段
  攻击（Byrd 啄 1×5、Book 连刺 6×3、Darkling/Nemesis/Writhing 多段）进引擎。
  加权表累计 34 怪。测试 +3。

## 本轮追加（批次 5c 小怪机制，2026-10-18）

- **小怪特殊机制（4f2df3d）**：CurlUpPower 实装（受攻击存活→+格挡一次性，
  Louse Defensive 新个体 + getLouse 50/50 混合）；SlimeBoss 分裂实装（≤50%
  自灭 + 双大软泥各继承当前血）；Gremlin Leader 召唤（每 3 回合 2 只、<3 钳制）；
  Gremlin Gang 逃跑传播（同伴死亡→幸存者下回合离场，全员离场判胜）；
  连击钳制（WeightedMoves 第 4 位禁位 + pick_weighted last_two_same）。
  怪物回合重构四阶段（快照→状态机→self 级特殊→轻量特殊→招式）。测试 +7。

## 本轮追加（批次 5b 加权招式，2026-10-18）

- **加权 getMove（33cf672）**：14 只一幕怪从固定序列升级为源码加权掷
  （WeightedMoves 表：伤害/附带格挡/权重%，按 getMove 的 num 阈值分支校准；
  Jaw Worm thrash 带 6 格挡的 ATTACK_DEFEND 已复刻）。RollMove 时 aiRng 重掷
  写回 script[0]，意图显示零改动；开战首掷立即可见。教徒保持确定性。
  测试 +5（含 1000 次权重采样分布验证）。
- 连击钳制（lastTwoMoves）省略——记 5c 余量。

## 本轮追加（批次 5a 多怪遭遇，2026-10-17）

- **多怪遭遇 5a（b3250e6）**：encounters::composition 全转译 MonsterHelper
  getEncounter——组合表（2/3 Louse、3 Sentries、3 Cultists、Slavers 三人组、
  Shield and Spear 等 30+ 键）+ 随机池（大软泥 50/50、Gremlin Gang 4 抽 8 池
  不放回、Lots of Slimes 5 抽、Exordium Thugs/Wildlife 弱强分层、Shapes 6 池）。
  个体怪 AI 表 +21 条（HP 按构造器校准；纠错 Red Slaver 46-50/Looter 44-48）。
  flow4b 成员工厂（build_battle_for + member_special 按成员名映射）；
  build_real 修正 Awakened One 带教徒、Donu & Deca 源码顺序 Deca 在前。
  force_encounter 测试辅助。测试 +8。
- 下一轮 5b：个体招式保真（rng 加权 getMove 转译——当前为简化序列）。

## 本轮追加（批次 4h + 收尾，2026-10-17）

- **诅咒/状态手牌触发（bde6a67）**：Regret（回合末 HP 流失=手牌数）/Shame
  （自施 Frail 1，justApplied 活过回合边界）/Decay（THORNS 2）/Pain（每出牌
  掉 1 血）/Normality（3 张上限）——triggerOnEndOfTurnForPlayingCard 与
  triggerOnOtherCardPlayed 系全接。测试 +5。
- STATUS/记忆同步至 4h；下一轮主菜 = 多怪遭遇重构。

## 本轮追加（批次 4c-4e，2026-10-17）

- **真怪 4c（2173115）**：Hexaghost 六火完整（ACTIVATE→DIVIDER 6×HP/12+1→
  SEAR/TACKLE/INFLAME/INFERNO 轮转+BurnIncrease 全堆升级+3 张）、Orb Walker
  （Laser 灼伤×2/Claw+回合末力）、Donu & Deca 双怪反相交替（Beam+Dazed/方阵/
  Artifact 2/先手光环同轮强化）、Awakened One 双形态（归零→去减益→REBIRTH
  回满血→Dark Echo/Sludge+Void/Triple；Curiosity/Regenerate 开局能力）。
  新 PowerKind：Regenerate/Curiosity/PlatedArmor。Burn 手牌回合末自动打出
  （THORNS）、Void 抽牌扣能。monsters.rs 工厂（monsters::build_real）。
- **事件收尾（1362802）**：数据表 52/52 全覆盖（+28 件按源码转译）；Effect
  枚举 26 变体（And 复合/HealFull/GainMaxHp/TransformCard/UpgradeAll/...）；
  EventDef.act 字段（终幕 SpireHeart 不进随机池）；6 id 修正（保留原版 typo：
  Transmorgrifier/Wheel of Change/N'loth）；AscendersBane 净化豁免。
- **三平台发行（c74c3ed）**：release 0.64MB（<20MB 预算）；macOS x86/ARM64 +
  Linux 三目标 cargo check 全过（零平台 API 实证）；docs/RELEASE.md。
- **遗物第二批（9b36c34）**：+30 件全钩子（Marbles/Sling/Preserved/Pantograph/
  Philosopher's Stone/Thread & Needle 反甲/Helix Buffer/五 boss 能量遗物/
  Happy Flower/Stone Calendar 第7回合 52/Nunchaku/Ink Bottle 抽牌/Velvet
  Choker 6 张/Tungsten Rod -1/Red Skull 血线±3力/Centennial 抽3/Meat on Bone/
  Pear/Strawberry/Mango/Waffle/Whetstone/War Paint + 篝火/药水/金币门控）。
  源码纠正：Red Skull=bloodied 血线（非首次掉血）、Stone Calendar 第 7 回合、
  Ink Bottle=抽牌。

## ## 本轮追加（UI 后续推进 7，2026-10-17）

- 事件累计 50 件：+8（马上比武/黏液世界/牧师祝福/白兽像/金像陷阱/知识/命运之轮/秘密传送门）（3a68b17）
- UI 键盘导航：j/k 移动选择 + enter 按阶段确认（奖励/篝火/宝箱/事件）（52c1e6f+修复）

## 本轮追加（UI 后续推进 6，2026-10-17）

- 事件累计 42 件：+8（死去的冒险者/低语之灵/神秘球体/陀螺/鬼影/试炼/鸟洞/吸血鬼）（8e7f6eb）
- 真怪余量：SpecialAi 至 13 种（+Rebirth 复活/HiveDeath/Summoner 召唤），
  exec_damage rebirth 快照重构（77d877f）

## 本轮追加（UI 后续推进 5，2026-10-17）

- 事件累计 34 件：+10（哥布林轮盘/闪亮之光/红面具墓/竞技场/摩艾/巢穴/冒险者/矿坑/墓地/城堡）（5cecdeb）
- 真怪继续：SpecialAi 至 10 种（+Donu&Deca 交替/Hexaghost 计数/GremlinNob 怒气——玩家打技能+1力）（25ce780）

## 本轮追加（UI 后续推进 4，2026-10-17）

- 事件累计 24 件：+10（活墙/陵墓/净化者/增幅陷阱/偶人/蒙面强盗/流浪学徒/知之颅骨/蓝衣女子/废料软泥怪）（42127d7）
- 存档语义注释：商店/事件不持久化（进房生成种子驱动——原版等价）

## 本轮追加（UI 后续推进 3，2026-10-17）

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

剩余工作（按优先级，2026-10-17 批次 4e 后更新）

1. **多怪遭遇重构**（最大块）：当前每遭遇为单体怪占位；需 MonsterGroup 多怪 +
   逐怪 AI + 目标选择（Donu & Deca 已走双怪路径可参考——build_real 返回 Vec）
2. 钩子清账余量：出牌/受击系未接清单见下（Necronomicon/HeelHook/WrathNextTurn 等）
3. 解锁/metrics 与存档整合；ShopState 持久化语义
4. crossterm 接入（零依赖 ANSI 已可玩；换后端接口不变）——需修工具链
5. 正式发行：GitHub Actions 三平台构建（docs/RELEASE.md 已备）

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

**已追加预算继续推进（2026-10-17）**：批次 4c-4e（真怪/事件/发行/遗物）
在追加的 3 亿预算内完成；消耗以平台计量为准。

## 交接指引

剩余 1500 万 token 不足以在会话内完成 4b/5/6。建议接手方式：
1. 新会话直接说「继续推进」——STATUS.md + 记忆文件已完整记录进度与规则
2. 优先做 4b 楼层推进编排（RunState↔BattleState 接线），这是通向可玩的最短路径
3. 每完成一个模块更新本文件的消耗数字与状态
