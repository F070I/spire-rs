# 模块记录：sts-data::gen（机械抽取的静态数据表）

批次：1.2 / 1.3（ID 表 + 文本数据）
状态：✅ 完成（静态数据层；行为转译在批次 2-3）
实现位置：
- 抽取脚本：`tools/extract.py`（Python 3，零依赖）
- 生成产物：`crates/sts-data/src/gen/`（cards/relics/potions/powers/monsters/events/keywords/types/mod）
- 审计中间产物：`tools/gen/extracted.json`

## 覆盖范围

| 表 | 数量 | 内容 | 文本 |
|---|---|---|---|
| cards | 375 | id/class/cost/type/color/rarity/target/base数值/9 个行为标志/5 个 tags | eng + zhs 名称与描述 |
| relics | 181 | id/tier | eng + zhs 名称 |
| potions | 42 | id/rarity | eng + zhs 名称 |
| powers | 149 | id/class/包名 | eng + zhs 名称 |
| monsters | 68 | id/class/包名 | eng + zhs 名称 |
| events | 52 | id/class/包名 | （文本后续接 events.json） |
| keywords | ~45 | 键/别名数组/描述 | eng + zhs |

## 验证依据

- 卡牌：`cards/*/X.java` 的 `super(id, name, img, cost, desc, CardType, CardColor, CardRarity, CardTarget)` 构造 + 字段赋值行（`this.baseDamage = 6;` 等）。
- 数量对照盘点记录 02 §9（逐色 75/75/76/77/39/14/5/9/5 = 375，完全吻合）。
- 文本：`src/main/resources/localization/{eng,zhs}/cards.json` 等（eng 卡 423 键）。
- 测试：`crates/sts-tests/tests/extract_audit.rs`（数量锁定、ID 唯一性、抽查 Strike_R/Anger/Whirlwind/Burn/Bash/Ghostly/Vajra/Fire Potion、双语覆盖）。

## 抽查确认的正确性样例

- `Feed`：cost 1、RARE、baseDamage 10、magic 3、flags=EXHAUST、tags=HEALING ✓
- `Backstab`：cost 0、damage 11、flags=EXHAUST|INNATE ✓
- `Whirlwind` cost=-1（X 费）、`Burn` cost=-2 + Status ✓
- `Apparition` 类的规范 ID 是 **"Ghostly"**（special 无色卡）——ID 与类名不同名的又一实例 ✓
- 遗物 181 个代码 ID 全部有本地化文本（loc 有 195 键，多出的是文本侧独有）✓

## 已知差异 / 待办

1. **遗物 181 vs RelicLibrary 注册 177**：抽取含 4 个未注册类（批次 3.6 对照 `RelicLibrary.initialize()` 逐个核实，预计为 Circlet/RedCirclet 等占位或特殊遗物）。
2. **1 张卡无 eng/zhs 文本**：`Impulse`（blue）——反编译资产缺口，疑似被砍内容残留；不进卡池（同 deprecated 处理，批次 3.3 裁决）。
3. **3 个能力无 eng 文本**：`Lightning Mastery`、`RechargingCore`、`Winter`（同上，疑似开发残留）。
4. 未抽取（属行为/配置层，批次 2-4 人工处理）：遭遇表权重（MonsterHelper）、事件逻辑、商店规则、升级 delta（upgrade() 方法体）、动态费用与 misc 字段、cards/deprecated 63 张。
5. 本地化 JSON 中的 EXTENDED_DESCRIPTION 等未抽取（按需后补）。

## 使用方法

```bash
python tools/extract.py \
  "<cardcrawl>/src/main/java/com/megacrit/cardcrawl" \
  "<cardcrawl>/src/main/resources/localization" \
  "crates/sts-data/src/gen"
```

重新生成后 `cargo test`，若 `extract_audit.rs` 数量断言变化必须先审查差异原因。
