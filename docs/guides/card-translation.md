# 卡牌转译指南（批次 3 灌注流水线）

> 本文档是子代理翻译卡牌 use() 的工作规范。翻译前必读：docs/00 → docs/source-inventory/02（卡牌系统）→ 09（打卡链）。

## 任务定义

把原版 `cards/<color>/<X>.java` 的 `use()`（与相关重写方法）语义转译为 Rust 动作序列，写入对应颜色模块 `crates/sts-core/src/cards/<color>.rs` 的 `effects()` match。

**只翻译行为**：数值已由 extract.py 静态抽取（`card.eff_damage()` 等）；文本、贴图、动画不移植。

## 工作流程（每张卡）

1. 读原版 `<X>.java`：`use(AbstractPlayer p, AbstractMonster m)` 全文 + `upgrade()`（若 custom_upgrade）+ 重写的 `applyPowers/calculateCardDamage/canUse`。
2. 写 match arm（见模板）。注释标注原版依据（文件:行为）。
3. 把卡 ID 加入 `HANDLED` 列表（保持与 match arm 同步——两者缺一冒烟测试会暴露）。
4. 需要选择屏/例程的卡：入队 `GameAction::CardRoutine`，在 `routines.rs` 注册执行体（或留 `// TODO 例程` 注释并在批次 3.10 清账）。

## match arm 模板

```rust
// cards/red/Consume.java use()：GainBlockAction + LoseHPAction(2)
"Consume" => {
    eb.bot(GameAction::GainBlock { target: TargetRef::Player, amount: card.eff_block() });
    eb.bot(GameAction::LoseHp { target: TargetRef::Player, source: Some(TargetRef::Player), amount: 2 });
}
```

## EffectBuilder 顺序语义

- `eb.bot(a)` = `addToBottom(action)`（绝大多数卡）
- `eb.top(a)` = `addToTop(action)`（后 top 的先执行）
- 语句顺序 = 原版 use() 内调用顺序；禁止重排。

## 动作映射表（高频）

| 原版 | Rust |
|---|---|
| `new DamageAction(m, new DamageInfo(p, this.damage, type))` | `eb.bot(GameAction::Damage { target: TargetRef::Monster(mi), info: attack_info(b, mi, card.eff_damage(), DamageType::Normal) })` |
| `new DamageAllEnemiesAction(p, this.multiDamage, ...)` | 先 `damage_matrix(b, base, ty)` 预计算，再 `GameAction::DamageAll { matrix, ty }` |
| `new GainBlockAction(p, this.block)` | `GameAction::GainBlock { target: TargetRef::Player, amount: card.eff_block() }` |
| `new DrawCardAction(p, n)` | `GameAction::DrawCard { amount: n, end_turn_draw: false }` |
| `new GainEnergyAction(n)` | `GameAction::GainEnergy { amount: n }` |
| `new LoseHPAction(p, n, ...)` | `GameAction::LoseHp { target: TargetRef::Player, source: Some(TargetRef::Player), amount: n }` |
| `new HealAction(p, n)` | `GameAction::Heal { target: TargetRef::Player, amount: n }` |
| `new ApplyPowerAction(new XPower(t, n), t, p)` | `GameAction::ApplyPower { target, source, power: Power::new(PowerKind::X, n), from_monster: false }` |
| `new ApplyPowerAction(new PoisonPower(...))` 等 PowerKind 未有者 | 先加 PowerKind（power/mod.rs）再用于此处 |
| `new MakeTempCardInHandAction(new XCard(), n)` | `GameAction::MakeTempCardInHand { sid: super::sid_of("X"), upgraded: false, count: n }` |
| `new DiscardAction(p, m, n, true/false, any)` | `GameAction::Discard { amount: n, is_random: .., any_number: .. }` |
| `new ChannelAction(new FrostOrb())` | `GameAction::ChannelOrb { kind: OrbKind::Frost }` |
| `new EvokeOrbAction(n)` | `GameAction::EvokeOrb { count: n }` |
| `new ChangeStanceAction("Wrath")` | `GameAction::ChangeStance { stance: StanceKind::Wrath }` |
| `new VFXAction / SFXAction / WaitAction` | **跳过**（纯表现；若夹带逻辑副作用则拆出逻辑部分） |
| 卡牌专属 Action 类（WhirlwindAction 等） | `GameAction::CardRoutine { call: RoutineCall { id: "WhirlwindAction", ... } }` + routines.rs 执行体 |

## 数值取值规则

- 基础数值：`card.eff_damage()/eff_block()/eff_magic()`（已含升级 delta；无升级字段时等值 base）
- **magicNumber 语义每卡不同**：先读原版 use() 用的是 damage/block/magic 哪个，再选对应 eff_*
- 动态基数卡（PerfectedStrike/Heavy Blade/Body Slam 等重写 applyPowers/calculateCardDamage）：在 arm 内按原版公式现算（参考 red.rs 已有实现）
- 升级分支行为（`if (this.upgraded)`）：读 `card.upgraded`；升级改变文本/标志的卡已在 custom_upgrade 名单，按原版转译

## 常见陷阱（全部来自实读记录）

1. **THORNS/HP_LOSS 不吃力量/易伤修正**——直接用 `DamageInfo::pure`（如毒素伤害）
2. **DamageInfo 在打出瞬间计算**——用 `attack_info`，不要存 base 让执行时再算
3. **多段攻击 = 多个 Damage 动作**（每段独立 applyPowers？否——原版单卡内多段共用一次 calculateCardDamage，各段同值；但 DamageAction 执行时各自过承伤管线）
4. **目标可为 null 的攻击**（ALL_ENEMY 走 None）——`card.st().target` 是 `AllEnemy` 时用 matrix
5. **isMultiDamage**：matrix 版已处理
6. **PowerKind 缺失**：在 `power/mod.rs` 的 PowerKind 加变体 + priority 表（有覆写才加）+ 伤害/格挡钩子（若有）+ is_debuff 判定
7. **例程执行体拿到 `&mut BattleState`**：需要随机数的地方标注 `// TODO 4.1 RNG 接线`，先用确定性占位并在模块记录登记

## 禁止事项

- 禁止改 GameActionManager/队列语义/钩子顺序（动架构先走 docs/03 §7 变更流程）
- 禁止"顺手简化"（如把多段攻击合并成一次伤害）
- 禁止在 effects() 里直接改状态（只读 b，产出动作；例程执行体除外——它在动作层运行）
- 禁止跳过 HANDLED 列表同步（冒烟测试按列表遍历）

## 完成定义（每批卡）

1. `cargo build` 无错
2. 新增测试：每张卡至少 1 个行为断言（伤害/格挡/能力/牌堆去向）
3. `cargo test` 全过 + clippy 清零
4. 模块记录 `docs/modules/cards-<color>.md` 追加批次小节（卡清单 + 测试结果 + 遗留 TODO）
