# 模块记录：sts-core::map（批次 4a 地图生成）

批次：4a（MapGenerator 全算法复刻）
状态：✅ 完成（算法层 + 房间分配；遭遇池在 4a 收尾）
实现位置：`crates/sts-core/src/map.rs`

## 保真点（对照记录 05 §1）

| 语义 | 出处 | 验证 |
|---|---|---|
| 15×7 全空节点 + 6 条路径 | MapGenerator.generateDungeon | map_structure_invariants |
| 第 2 条路径起点重摇 ≠ 第 1 条 | createPaths while 循环 | 种子确定性测试（同种子全图相等） |
| 步进方向边界（x=0 → [0,1]、x=6 → [-1,0]、其余 [-1,1]） | _createPaths | 结构测试 |
| 祖先检查 gap<3 重摇 / gap∈{3,4} 落空 | _createPaths（保真） | 算法复刻 |
| getCommonAncestor 的 node1.x<node2.y 笔误 | docs/07 台账 | 按源码复刻（未修正） |
| 同行防交叉（左 maxEdge/右 minEdge 抬压） | _createPaths | 算法复刻 |
| Boss 边 dst=(3, dstY+2) 虚拟行 | _createPaths 终点分支 | boss_edges 断言 |
| 首行去重边（仅第 0 行） | filterRedundantEdgesFromRow | 算法复刻 |
| randRange 双闭区间走 StsRandom | randRange | 种子确定性 |

## 测试结果

121 测试通过（新增 3），clippy 0。

## 房间分配（map/assign.rs，4a 收尾完成）

- count 排除 y==13 怪癖；比例池（Elite ×1.6 进阶 / ×2.5 BGH）；硬性行 0/8/14
- 三规则原文语义：parent 只适用 Rest/Treasure/Shop/Elite（Monster/Event 父子允许！）；
  sibling 适用 Rest/Monster/Event/Elite/Shop；y==0 豁免 parent+sibling
- ruleAssignableToRow：Rest 禁 y≤4 与 y≥13（14 行硬性篝火合法）、Elite 禁 y≤4
- lastMinuteNodeChecker 兜底 MonsterRoom
- 测试：全覆盖/规则校验（含 parent 适用类型）/种子确定性

## 已知差异 / 待办

1. 遭遇池三幕预生成（弱敌/强敌/精英/Boss 权重表）——4a 收尾。
2. 遭遇池三幕预生成（弱敌/强敌/精英/Boss 权重表）——4a 收尾。
3. TheEnding 固定图（generateSpecialMap）——批次 4b。
4. mapRng 种子 = seed + actNum（×100/×200/×300）——RunState 接线时对齐（决策：BattleRngs 已有先例）。
