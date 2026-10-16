# sts-rs —— Rust 文字版《杀戮尖塔》

以原版反编译源码（`MySlayTheSpire-master`，desktop-1.0 完整版）为唯一实现依据的语义移植项目。
图形界面替换为纯文字终端 UI，游戏规则、触发顺序、随机体系、完整流程全部保留。

**当前状态：阶段 0（设计与源码盘点）已完成** —— 完整机制清单、模块映射、冻结架构、UI 规范、移植顺序、测试计划、风险与决策、跨平台构建规范全部就绪；workspace 骨架与 RNG 算法层已落地。

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
