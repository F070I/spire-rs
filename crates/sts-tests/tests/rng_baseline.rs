//! L4 种子回归（docs/06 §1）：RNG 层的确定性基线。
//!
//! 本文件为首个回归基线：固定种子下断言完整随机序列。
//! 与原版运行时的对照向量（L8）采集后补入 tests/fixtures/。

use sts_data::rng::{shuffle_jdk, JdkRandom, RandomXS128, StsRandom};

#[test]
fn xorshift_sequence_baseline_seed_1() {
    // 固定基线：任何对 rng.rs 的改动若改变此序列，必须是有意的并更新基线（diff 审查）。
    // 验证：与 libgdx RandomXS128 的独立 Python 转写逐位比对一致（阶段 0，2026-10-16）；
    // 与原版游戏的端到端对照（L8/G1 门）待批次 2 的战斗沙盘建立后补充。
    let mut r = RandomXS128::new(1);
    let out: Vec<i64> = (0..16).map(|_| r.next_long()).collect();
    assert_eq!(
        out,
        vec![
            3787875997830008111,
            7110081793310507210,
            -2833066636294175240,
            6427185811373906065,
            -857044592549396973,
            -3200417760416941377,
            -7392127685171910489,
            4775766905037017768,
            2263543422828414761,
            4961636499666507816,
            -4225648566501127036,
            8242634486574294640,
            2872287370042022446,
            -2023414454167958007,
            -7872591797522791567,
            5201765523838695903,
        ]
    );
}

#[test]
fn jdk_lcg_sequence_baseline_seed_42() {
    // JDK LCG 基线：seed=42 依次 next(31) 的前 8 个值。
    // 验证：独立 Python 转写逐位一致，且 Random(42).nextInt() == -1170105035
    // 与公开参考值吻合（阶段 0，2026-10-16）。
    let mut r = JdkRandom::new(42);
    let out: Vec<i32> = (0..8).map(|_| r.next_bits(31)).collect();
    assert_eq!(
        out,
        vec![1562431130, 117392763, 1467211248, 102948884, 662969970, 2023087525, 595021505, 1519796918]
    );
    // 已知参考值：new java.util.Random(42).nextInt() == -1170105035
    let mut r2 = JdkRandom::new(42);
    assert_eq!(r2.next_i32(), -1170105035);
}

#[test]
fn sts_random_int_bounds_hold() {
    // random(int range) 双闭区间：大量采样只落在 [0, range]。
    let mut r = StsRandom::new(20261016);
    for _ in 0..100_000 {
        let v = r.random_int_inclusive(6);
        assert!((0..=6).contains(&v));
    }
    for _ in 0..100_000 {
        let v = r.random_int_range(-3, 3);
        assert!((-3..=3).contains(&v));
    }
}

#[test]
fn jdk_shuffle_is_fisher_yates_form() {
    // JDK shuffle 形状检查：单元素不变；两元素以 50% 交换（同种子确定性）。
    let mut one = vec![1u8];
    let mut rng = JdkRandom::new(1);
    shuffle_jdk(&mut one, &mut rng);
    assert_eq!(one, vec![1]);

    let outcomes: Vec<Vec<u32>> = (0..8)
        .map(|seed| {
            let mut v = vec![0u32, 1];
            let mut rng = JdkRandom::new(seed * 1000 + 7);
            shuffle_jdk(&mut v, &mut rng);
            v
        })
        .collect();
    // 8 个种子应同时出现 [0,1] 与 [1,0]（否则 nextInt(2) 恒 0，形状错误）
    assert!(outcomes.contains(&vec![0, 1]));
    assert!(outcomes.contains(&vec![1, 0]));
}
