# spire-rs 交接文档（→ ChatGPT）

> 本文档由前一个 Agent（GLM）生成，供接手者无损继续。所有结论基于真实代码与构建，无虚报。

---

## 一、项目概览

| 项 | 值 |
|---|---|
| 仓库 | `git@github.com:F070I/spire-rs.git`（SSH，HTTPS 被墙） |
| 本地路径 | `C:\Users\17366\.zcode\workspace\default\sts-rs` |
| 主分支 | `main`（commit `23978f8`） |
| UI 分支 | `ui/terminal-sop`（commit 后已推远端，**用户不认可，可参考可丢弃**） |
| 正式 tag | `v0.1.0` / `v0.1.1`（旧终端 UI，已发 Release） |
| 测试 | **316 全过**（`cargo test --workspace`） |
| clippy | 0 警告 |
| fmt | 干净 |
| Release 体积 | Windows 3.96 MB |
| 核心保护 | `sts-data` / `sts-core` **零改动**（自项目创建起未被 UI 工作触碰） |

---

## 二、crate 架构（冻结，勿重组）

```
crates/
  sts-data/       数据表、RNG、常量（375 卡/181 遗物/42 药水/68 怪/52 事件/关键词/官方中文 zhs_text）
  sts-core/       游戏状态、规则、战斗、地图、奖励、存档（纯逻辑零 UI）
  sts-interface/  UI↔核心协议：PlayerCommand 枚举 + GameView 结构化快照 + dispatch 分发
  sts-terminal/   旧终端 UI（ANSI 行协议，仍可编译运行，保留不删）
  sts-desktop/    桌面窗口 UI（egui/glow 0.27）← 本次重设计目标
  sts-tests/      全部集成测试（18 个测试文件）
```

**依赖方向**：`sts-data` ← `sts-core` ← `sts-interface` ← `sts-terminal` / `sts-desktop` / `sts-tests`

**硬约束**：
- `sts-data` / `sts-core` 的 `git diff` 从项目创建至今为空。不要改。
- `sts-interface` 只含只读 ViewModel（`GameView::capture`）和命令分发（`dispatch`），不含规则。
- `PlayerCommand` 枚举 ~45 变体，覆盖全部交互。`dispatch` 映射到 `RunState4b` 公开方法。
- UI 不得直接修改 `RunState4b` 字段，一切走 `dispatch`。

---

## 三、构建与运行

### 环境要求（Windows）

```bash
# MSYS2 binutils 必须在 PATH（dlltool/as——raw-dylib 链接依赖）
export PATH="/c/msys64/mingw64/bin:$PATH"
```

MSYS2 已装在 `C:\msys64\`（binutils 2.47，含 as.exe/dlltool.exe/ld.exe）。
`libshlwapi.a` 等导入库已从 `mingw-w64-x86_64-crt-git` 包拷入 rustup self-contained 目录。

### 常用命令

```bash
cargo build --release -p sts-desktop     # 桌面 Release
cargo run --release -p sts-desktop       # 运行桌面窗口
cargo test --workspace                   # 316 测试
cargo clippy --workspace --all-targets   # 0 警告
cargo fmt --all -- --check               # 格式检查
```

### 网络

- GitHub HTTPS 被墙 → 用 SSH（`git@github.com:F070I/spire-rs.git`，SSH key 已配）
- crates.io / static.crates.io 可达
- repo.msys2.org 可达但不稳定

---

## 四、官方中文本地化（已完成，接手者直接复用）

### 来源确认

反编译项目 `C:\Users\17366\Desktop\MySlayTheSpire-master\src\main\resources\localization\zhs\` 包含**完整官方简体中文**（17 个 JSON）。这是原版正式本地化，不是 AI 翻译。

### 已提取到 `crates/sts-data/src/gen/zhs_text.rs`（649 条）

| 数据 | 官方 JSON 来源 | 结构 |
|---|---|---|
| 遗物名/描述/风味 | relics.json | `RELIC_TEXT: &[(&id, &name, &desc, &flavor)]` |
| 药水名/描述 | potions.json | `POTION_TEXT: &[(&id, &name, &desc)]` |
| 事件名/多屏正文/选项 | events.json | `EVENT_TEXT: &[(&id, &name, &body, &options)]` |
| 怪物官方名 | monsters.json | `MONSTER_NAMES: &[(&class_key, &zhs_name)]` |
| 能力官方名 | powers.json | `POWER_NAMES: &[(&power_id, &zhs_name)]` |
| 角色名 | characters.json | `CHARACTER_NAMES: &[(&id, &zhs_name)]` |
| 卡牌升级描述（104 张） | cards.json | `CARD_UPG_DESC: &[(&id, &upgrade_desc)]` |
| 卡牌名称/描述 | cards.json（早期已入 `gen::CARDS.desc_zhs`） | — |

提取脚本：`tools/extract_zhs_text.py`

### 插值引擎（`crates/sts-interface/src/text.rs`）

| 函数 | 功能 |
|---|---|
| `card_desc_raw(sid, upgraded, dmg, blk, magic)` | 替换 `!D!`/`!B!`/`!M!` → 实际值；`NL`→换行；去 `*` |
| `static_eff(sid, upgraded)` | 非战斗显示值（基础 + 升级 delta，UPF 标志守卫 -1 哨兵） |
| `combat_eff(sid, upgraded, powers)` | 战斗内显示值（+力量→攻击、+敏捷→技能格挡） |
| `power_label(kind)` | PowerKind → 官方中文名（力量/易伤/虚弱/脆弱/中毒…） |
| `monster_name_zhs(display_name)` | 显示名 → 官方中文名（Jaw Worm → 大颚虫） |
| `relic_text(id)` / `potion_text(id)` / `event_text(id)` | 官方遗物/药水/事件文本查询 |
| `stance_name_zhs(stance)` | 姿态官方名（中立/平静/愤怒/神格） |
| `character_name_zhs(id)` | 角色官方名（铁甲战士…） |

**已全路径接线到 `GameView::capture`**：手牌/奖励/商店/敌人名/遗物名/药水名/事件标题+正文/能力标签/姿态名。

### 事件选项一致性规则（§15.5）

官方选项数 ≠ 核心实装分支数时，标题/正文用官方，选项回退到与实际效果一致的接口文本。
原因：展示官方 5 个选项但核心只实装 2 个 = 假交互。

### 遗留

- 部分事件选项含官方格式码 `#r`/`#g`/`#y`/`#b`（未解析颜色标记）——UI 层应去除或渲染。
- `powers.json` 的 Weak 键实际为 `Weakened`（非 "Weak"）——已在映射中处理。
- `monsters.json` 键为原版类键（`JawWorm` 非 "Jaw Worm"）——已在映射中处理。

---

## 五、桌面 UI 现状（用户不认可，全部重做）

### 被否决的设计

用户提供了完整 SOP（`C:\Users\17366\Downloads\spire_rs_UI_SOP_v1.md`），要求"老式单色绿磷终端"风格。前一个 Agent 在 `ui/terminal-sop` 分支上做了尝试，**用户不认可任何设计**。

**分支 `ui/terminal-sop` 上的实现**（可参考/丢弃）：
- egui/glow 0.27 窗口 1440×840
- 绿磷/灰度双主题（SOP 固定 HEX）
- 自绘终端控件（terminal_button/terminal_choice_row/terminal_separator）
- 地图从下向上矢量画布（中文节点符号 战/精/?/休/商/宝/B）
- 战斗重构（药水顶栏 28×28 / 能量球底部 58px / 手牌 142px）
- 主菜单 520px 居中列 + 角色选择
- 全部场景改选项行
- 编译通过，316 测试全过，Release 3.95 MB

**用户否决原因**：设计质量不满足 SOP 验收标准。具体哪不合格由用户判断，接手者应重新实现而非修补。

### SOP 文件位置

`C:\Users\17366\Downloads\spire_rs_UI_SOP_v1.md`

SOP 核心要求摘要：
- **§2.1** 复古绿磷终端 / 黑底单色 / 细线框 / 文字为主
- **§3** 固定 HEX 令牌（绿磷 + 黑白双主题）、字号表、线宽/间距表
- **§4** 1440×840 客户区：顶栏 40px / 主区 / 底栏 92px / 右侧详情 320px（仅地图/战斗显示）
- **§5** 自绘终端控件：透明底 + 1px 边框 + 直角；禁止 GUI 默认浅灰按钮
- **§6** 页面模板：主菜单 520px 列 / 角色四列 / 选项页 820px 列
- **§7** 地图：矢量线条、从下向上、真实节点/边、中文符号、Boss 顶部居中
- **§8** 战斗：药水顶栏 28×28、能量球底部 58px 左侧、手牌横排 142px 高
- **§12** 必须提供真实运行截图 + 逐条验收
- **§13** 一票否决项（浅灰按钮 / 左→右地图 / ASCII 路线 / 占位符残留 / 假数据 / 机制修改…）

### 接手者应该做的

1. **阅读 SOP 全文**（上面的摘要不够——必须逐条对照）
2. 决定是重写 `sts-desktop` 还是修补 `ui/terminal-sop` 分支
3. 按阶段 A→F 顺序实施（SOP §11），每阶段提供截图
4. 核心逻辑不动；接口只增只读 DTO
5. **不要发布 Release 直到用户明确验收 UI**

---

## 六、游戏内容完成度

| 类别 | 数量 | 状态 |
|---|---|---|
| 卡牌 | 375/375 全灌注 | ✅（红75+绿75+蓝76+紫77+无色39+诅咒14+状态5+临时9+选项5） |
| 事件 | 52/52 全覆盖 | ✅ |
| 药水 | 42 种全实装 | ✅ |
| 遗物 | 181 全机制（映射 174 有效果+7 纯文本） | ✅ |
| 怪物 | 组合遭遇 + 34 怪加权 AI + 个体特殊（CurlUp/分裂/召唤/逃跑） | ✅ |
| 官方中文 | 649 条 zhs_text + 插值引擎 | ✅ |
| 流程 | Neow→地图→战斗→奖励→篝火→商店→事件→宝箱→幕间→Boss→心脏→胜利/死亡 | ✅ |
| 存档 | JSON roundtrip + 垃圾输入防护 | ✅ |

### 战斗引擎细节

- `BattleRngs` 四流（shuffle/card_random/ai/misc，seed+floorNum 重播种）
- 怪物回合四阶段化（快照→真怪状态机→self 级特殊→轻量特殊→招式执行）
- 加权 getMove（14+20 怪 WeightedMoves 表 + 连击钳制 + 多段支持）
- 真怪状态机：Hexaghost 六火/Orb Walker/Donu & Deca 反相/Awakened One 双形态
- 小怪特殊：CurlUp 蜷缩/SlimeBoss 分裂/GremlinLeader 召唤/GremlinGang 逃跑
- 回合开始遗物时序：先快照后清零；能量类增益排充能动作之后

---

## 七、工具链与已知坑

| 问题 | 解法 |
|---|---|
| gnu 缺 dlltool → raw-dylib 链接失败 | MSYS2 binutils 落位 `C:\msys64\mingw64\bin`，PATH 加此目录 |
| rustup 自带 dlltool 缺 as.exe | 不可用；必须用 MSYS2 的 |
| 链接缺 libshlwapi.a | 从 `mingw-w64-x86_64-crt-git` 包拷入 rustup self-contained 目录 |
| Git Bash heredoc 超 ~150 行截断 | **会损坏目标文件！**先 `git checkout` 恢复再分小块 |
| heredoc `\\n` 转义不可靠 | 长内容用 Write 工具写 .py 文件再执行 |
| github HTTPS 被墙 | SSH 可达（key 已配）；crates.io 可达 |
| api.github.com 间歇超时 | 重试即可 |
| 回合开始遗物时序 | 先快照上回合状态再清零；能量类增益排充能动作**之后**入队 |
| Python 脚本字符串含 `\U` | 用 raw string `r'...'` |

---

## 八、文件索引

| 文件 | 行数 | 说明 |
|---|---|---|
| `crates/sts-core/src/battle.rs` | ~2400 | 战斗引擎核心（ BattleState/怪物/dispatch/遗物钩子） |
| `crates/sts-core/src/flow4b.rs` | ~1900 | 楼层推进编排（RunState4b/商店/篝火/事件/幕间） |
| `crates/sts-core/src/power/mod.rs` | ~200 | PowerKind 枚举 + atDamageGive/Receive |
| `crates/sts-core/src/power/ops.rs` | ~220 | apply_power/stack/reduce/end_of_round_tick |
| `crates/sts-core/src/event.rs` | ~750 | 52 事件定义（Effect 枚举 26 变体） |
| `crates/sts-core/src/map/gen.rs` | ~350 | MapGenerator 全算法 |
| `crates/sts-interface/src/view.rs` | ~600 | GameView DTO + capture + dispatch |
| `crates/sts-interface/src/text.rs` | ~210 | 官方中文插值引擎 |
| `crates/sts-data/src/gen/zhs_text.rs` | 682 | 官方中文文本数据 |
| `crates/sts-desktop/src/main.rs` | ~1500 | 桌面窗口（**用户不认可，需重做**） |
| `tools/extract.py` | — | 主数据抽取流水线 |
| `tools/extract_zhs_text.py` | — | 官方中文补充文本抽取 |
| `docs/STATUS.md` | — | 项目进度快照（最新） |
| `docs/RELEASE.md` | — | 三平台发行手册 |
| `.github/workflows/ci.yml` | — | push/PR 门禁（三 OS 测试 + clippy -D warnings） |
| `.github/workflows/release.yml` | — | v* tag 四矩阵构建 → GitHub Release |
| `.github/workflows/desktop.yml` | — | desktop 测试构建（Artifacts only） |

---

## 九、Git 分支说明

| 分支 | commit | 内容 | 用户态度 |
|---|---|---|---|
| `main` | `23978f8` | 游戏核心 + 旧终端 UI + GameView/dispatch + 本地化 | ✅ 保留 |
| `ui/terminal-sop` | SOP 尝试（绿磷终端） | 用户不认可，可参考可丢弃 | ❌ |
| tag `v0.1.0` | 旧终端 UI 首版 | 已发 Release（Intel 产物要求 macOS 13+） | 保留 |
| tag `v0.1.1` | macOS 12 兼容修复 | 已发 Release（四平台过） | 保留 |

**v0.1.x 不要动**。新 UI 验收后打 `v0.2.0`。

---

## 十、接手检查清单

- [ ] `git clone git@github.com:F070I/spire-rs.git`
- [ ] `cargo test --workspace` → 316 全过
- [ ] `cargo clippy --workspace --all-targets` → 0 警告
- [ ] 读 SOP：`C:\Users\17366\Downloads\spire_rs_UI_SOP_v1.md`
- [ ] 读 `docs/STATUS.md` 最新快照
- [ ] 确认 `crates/sts-data` / `crates/sts-core` 零改动
- [ ] 理解 `GameView::capture` / `dispatch` 边界
- [ ] 理解官方 zhs_text 来源与插值引擎
- [ ] 按 SOP §11 阶段 A→F 顺序重做 UI
- [ ] 每阶段提供真实截图 + 逐条验收
- [ ] UI 通过用户验收后才打 tag 发 Release
