# 模块记录：sts-data::rng（随机数算法层）

批次：1.1（批次 1 的一部分，随阶段 0 骨架先行落地）
状态：✅ 完成（算法层）
实现位置：`crates/sts-data/src/rng.rs`

## 实现内容

| 类型 | 对应原版 | 验证 |
|---|---|---|
| `RandomXS128` | libgdx `com.badlogic.gdx.math.RandomXS128`（xorshift128+ + murmurHash3 setSeed） | 基线序列 + 独立 Python 转写逐位比对 |
| `JdkRandom` | `java.util.Random`（48 位 LCG） | 基线序列 + 已知参考值 `Random(42).nextInt() == -1170105035` 吻合 |
| `shuffle_jdk` | `java.util.Collections.shuffle(List, Random)` | 形状测试（Fisher-Yates 交换方向、单元素不变） |
| `StsRandom` | `com.megacrit.cardcrawl.random.Random`（counter 语义、双闭区间、快进 random(999)、setCounter randomBoolean 补进） | 单元测试（counter 递增、快进等价、copy、set_counter 独立流） |

## 验证依据（原版源码）

- `random/Random.java`（123 行，全方法清单与 counter 语义）→ docs/source-inventory/07 §1
- libgdx RandomXS128 源（nextLong xorshift128+ 公式、murmurHash3、next(bits) 覆写）
- JDK 拒绝采样算法（nextInt(bound)）、Collections.shuffle 实现

## 测试结果

- `cargo test`：14 通过（rng 单测 8 + 基线 4 + state/flow 2）。
- 基线固化：`crates/sts-tests/tests/rng_baseline.rs`（seed=1 前 16 个 nextLong；seed=42 前 8 个 next(31)）。
- 交叉验证：Rust 输出与独立 Python 转写完全一致（捕获并修复了一处真 bug：Java `>>>` 逻辑右移被误写为 Rust 算术右移，影响 murmurHash3 与 nextLong 两处——已修复并有回归基线守护）。
- clippy：0 警告。

## 已知差异 / 待办

- [待 L8] 与原版游戏运行时的端到端对照（例如已知种子下的具体事件序列）尚未采集；G1 门（批次 1 完成）要求对照 fixture。
- `next_f64`（nextDouble）与 `next_f32`（nextFloat）按 JDK 默认实现（经由 next(bits)）移植，暂无独立参考向量（同批补）。
- `Random()` 无参构造（调试用，seed 0..9999/counter 0..99）未移植——不进入游戏路径。
