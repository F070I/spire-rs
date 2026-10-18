# sts-rs —— Rust 文字版《杀戮尖塔》

以原版反编译源码（`MySlayTheSpire-master`，desktop-1.0 完整版）为唯一实现依据的语义移植项目。
图形界面替换为纯文字终端 UI，游戏规则、触发顺序、随机体系、完整流程全部保留。

**当前状态：可玩完整版** —— 375 卡全灌注 / 多怪组合遭遇 + 加权 AI / 52 事件
全覆盖 / 药水 42 种实装 / 遗物机制映射 156+ / 288 测试全过、clippy 零警告。
`cargo run -p sts-terminal` 即玩（Neow → 全图选路 → 战斗 → 奖励 → 幕间 → 存读档）。

## 运行

```bash
cargo run -p sts-terminal                # 开发运行
cargo build --release -p sts-terminal    # 产物 ~0.6 MB
```

命令：`n` 开 Neow · `nn <0-3>` 选祝福 · `m <x> <y>` 选节点 · `p <手牌号> [目标]`
出牌 · `e` 结束回合 · `r <0-2>|skip` 领奖 · `po <槽>` 用药 · `s`/`l` 存读档 ·
`j`/`k` 选择 · `enter` 确认 · `t` 切主题 · `q` 退出。

## 发行

打 `v*` tag 自动触发 [.github/workflows/release.yml](.github/workflows/release.yml)
三平台构建（Windows x64 / macOS Intel+ARM64 / Linux x64）→ 冒烟 → 打包发
GitHub Release。详见 [docs/RELEASE.md](docs/RELEASE.md)。

## 阅读顺序（后续任何接手者必读）

1. `docs/00-项目总规范.md` —— 目标、原则、范围裁决规则
2. `docs/01-机制清单.md` —— 移植总范围（A-L 全条目，带状态跟踪）
3. `docs/02-模块映射.md` —— 原版模块 → Rust 模块
4. `docs/03-架构冻结.md` —— 冻结的架构与依赖（变更须走变更流程）
5. 对应批次查阅 `docs/05-移植顺序与完成条件.md` 与 `docs/source-inventory/` 的原始记录

## Crate 结构

| crate | 职责 |
|---|---|
| `sts-data` | 静态数据、ID、随机数算法（RandomXS128/JDK LCG/STS 包装）、常量 |
| `sts-core` | 游戏状态、规则、流程（零 UI/平台依赖） |
| `sts-interface` | UI↔核心协议（PlayerCommand / GameView） |
| `sts-terminal` | 终端渲染与输入（`sts` 可执行） |
| `sts-tests` | 规则测试、种子回归、对照测试、流程测试 |

## 构建

```bash
cargo build --release          # Windows: target/release/sts.exe
cargo test                     # 全部测试
```

跨平台构建命令、体积目标（<20MB）与平台测试矩阵见 `docs/08-构建与发行.md`。

## 阶段 0 交付物索引

见 `docs/00-项目总规范.md` §三。
