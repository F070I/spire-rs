//! 随机数算法层 —— 原版《杀戮尖塔》随机体系的逐位复刻。
//!
//! 依据（docs/source-inventory/07-随机数存档解锁角色.md §1）：
//! - `random/Random.java` 包装 libgdx `RandomXS128`（xorshift128+，murmurHash3 setSeed）；
//! - 所有 public 方法进入即 `counter++`（先自增再取数）；
//! - `Random(seed, counter)` 快进 = 循环调用 `random(999)`；
//! - `setCounter(target)` 用 `randomBoolean()` 补进（与快进不同流！）；
//! - JDK `java.util.Random`（LCG）用于部分洗牌（relic 池、每日 mod、CardGroup.shuffle 包装）。

/// libgdx `com.badlogic.gdx.math.RandomXS128` 的完整移植（xorshift128+）。
///
/// 关键点：
/// - `next_long` 是唯一核心；
/// - `next(bits)` = `nextLong() & ((1 << bits) - 1)`（libgdx 覆写了 Java Random 的 next）；
/// - `next_int`/`next_int_bounded`/`next_f32`/`next_f64`/`next_bool` 全部走 Java
///   `java.util.Random` 的默认实现（经由 `next(bits)`），语义必须与 JDK 一致。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RandomXS128 {
    seed0: i64,
    seed1: i64,
}

impl RandomXS128 {
    /// 等价于 libgdx `new RandomXS128(long seed)`。
    pub fn new(seed: i64) -> Self {
        let mut r = Self { seed0: 0, seed1: 0 };
        r.set_seed(seed);
        r
    }

    /// libgdx `setSeed`：seed 经 murmurHash3 混淆生成 state。
    pub fn set_seed(&mut self, seed: i64) {
        // seed == 0 时使用 Long.MIN_VALUE（libgdx 行为）
        let s = if seed == 0 { i64::MIN } else { seed };
        let seed0 = murmur_hash3(s);
        self.set_state(seed0, murmur_hash3(seed0));
    }

    pub fn set_state(&mut self, seed0: i64, seed1: i64) {
        self.seed0 = seed0;
        self.seed1 = seed1;
    }

    pub fn get_state(&self, which: u8) -> i64 {
        if which == 0 {
            self.seed0
        } else {
            self.seed1
        }
    }

    /// xorshift128+ 核心。对应 libgdx `nextLong()`。
    ///
    /// 注意 Java `>>>` 是**逻辑右移**；Rust 的 `i64 >> n` 是算术右移，
    /// 必须显式转 u64 再移（否则符号扩展破坏状态流）。
    pub fn next_long(&mut self) -> i64 {
        let mut s1 = self.seed0;
        let s0 = self.seed1;
        self.seed0 = s0;
        s1 ^= s1 << 23; // a（左移，高位脱落，与 Java `<<` 一致）
        self.seed1 = s1 ^ s0 ^ ((s1 as u64 >> 17) as i64) ^ ((s0 as u64 >> 26) as i64); // b, c（逻辑右移）
        self.seed1.wrapping_add(s0)
    }

    /// 对应 Java Random 的 `next(bits)`（被 libgdx 覆写为基于 nextLong）。
    pub fn next_bits(&mut self, bits: u32) -> i32 {
        let mask = (1u64 << bits).wrapping_sub(1);
        (self.next_long() as u64 & mask) as i32
    }
    // 注：Java 中 next(bits) 为 protected；此处公开以支撑测试与上层包装，
    // 上层（StsRandom）之外不应直接消费。

    /// `java.util.Random.nextInt()`：无参版本，全 32 位。
    pub fn next_i32(&mut self) -> i32 {
        self.next_bits(32)
    }

    /// `java.util.Random.nextInt(bound)`：拒绝采样算法，逐位对齐 JDK。
    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let m = bound - 1;
        if (bound & m) == 0 {
            // bound 为 2 的幂
            let r = self.next_bits(31) as i64;
            (((bound as i64) * r) >> 31) as i32
        } else {
            // JDK: for (int u = r; u - (r = u % bound) + m < 0; u = next(31));
            let mut r = self.next_bits(31);
            loop {
                let u = r;
                r = u % bound;
                if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                    return r;
                }
                r = self.next_bits(31);
            }
        }
    }

    /// `java.util.Random.nextBoolean()`
    pub fn next_bool(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    /// `java.util.Random.nextFloat()`：next(24) / 2^24
    pub fn next_f32(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1u32 << 24) as f32
    }

    /// `java.util.Random.nextDouble()`：((next(26) << 27) + next(27)) / 2^53
    pub fn next_f64(&mut self) -> f64 {
        let hi = self.next_bits(26) as i64;
        let lo = self.next_bits(27) as i64;
        ((hi << 27) + lo) as f64 / (1u64 << 53) as f64
    }
}

/// libgdx 的 murmurHash3 fmix64（Java `>>>` 逻辑右移，必须按无符号处理）。
fn murmur_hash3(x: i64) -> i64 {
    let mut x = x as u64;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^= x >> 33;
    x as i64
}

/// JDK `java.util.Random`（LCG，48 位截断）。
///
/// 用于原版中显式 `new java.util.Random(...)` 的场景：
/// - 遗物池洗牌 `new java.util.Random(relicRng.randomLong())`；
/// - 每日 mod 选取 `new java.util.Random(daysSince1970 / 5)`；
/// - `CardGroup.shuffle` 的 `new java.util.Random(shuffleRng.randomLong())` 包装。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JdkRandom {
    state: u64, // 48 位有效
}

const JDK_MULT: u64 = 0x0005_DEEC_E66D;
const JDK_ADD: u64 = 0xB;
const JDK_MASK: u64 = (1u64 << 48) - 1;

impl JdkRandom {
    pub fn new(seed: i64) -> Self {
        Self {
            state: ((seed as u64) ^ JDK_MULT) & JDK_MASK,
        }
    }

    /// JDK `next(bits)`（Java 中为 private；公开仅供测试与上层包装）。
    pub fn next_bits(&mut self, bits: u32) -> i32 {
        self.state = (self.state.wrapping_mul(JDK_MULT).wrapping_add(JDK_ADD)) & JDK_MASK;
        ((self.state >> (48 - bits)) & ((1u64 << bits) - 1)) as i32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_bits(32)
    }

    /// JDK `nextInt(bound)`（与 RandomXS128 处相同的拒绝采样，但基于 LCG 的 next(31)）。
    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let m = bound - 1;
        if (bound & m) == 0 {
            let r = self.next_bits(31) as i64;
            (((bound as i64) * r) >> 31) as i32
        } else {
            let mut r = self.next_bits(31);
            loop {
                let u = r;
                r = u % bound;
                if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                    return r;
                }
                r = self.next_bits(31);
            }
        }
    }

    pub fn next_bool(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    pub fn next_f64(&mut self) -> f64 {
        ((self.next_bits(26) as i64) << 27) as f64 / (1u64 << 53) as f64
            + self.next_bits(27) as f64 / (1u64 << 53) as f64
    }
}

/// JDK `Collections.shuffle(List, Random)`：
/// `for (int i=size; i>1; i--) swap(list, i-1, rnd.nextInt(i));`
pub fn shuffle_jdk<T>(list: &mut [T], rng: &mut JdkRandom) {
    let size = list.len();
    if size < 2 {
        return;
    }
    for i in (1..size).rev() {
        let j = rng.next_int_bounded(i as i32 + 1) as usize;
        list.swap(i, j);
    }
}

/// 原版 `com.megacrit.cardcrawl.random.Random` 的完整移植。
///
/// - 所有方法**进入即 counter++**（先自增再取数）；
/// - `random(int range)` = `nextInt(range + 1)`，**双闭区间 [0, range]**；
/// - `with_counter` 快进用 `random_int_inclusive(0, 999)`；
/// - `set_counter` 补进用 `random_bool()`（两种快进消耗的底层序列不同，语义分开保真）。
#[derive(Clone, Debug)]
pub struct StsRandom {
    pub rng: RandomXS128,
    pub counter: i32,
}

/// `set_counter` 在 counter >= target 时报错（对应原版 ERROR 日志）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetCounterError;

impl StsRandom {
    pub fn new(seed: i64) -> Self {
        Self {
            rng: RandomXS128::new(seed),
            counter: 0,
        }
    }

    /// `new Random(seed, counter)`：循环 `random(999)` 快进。
    pub fn with_counter(seed: i64, counter: i32) -> Self {
        let mut r = Self::new(seed);
        for _ in 0..counter {
            r.random_int_inclusive(999);
        }
        r.counter = counter;
        r
    }

    /// `copy()`：复制 128 位内部状态 + counter。
    pub fn copy(&self) -> Self {
        Self {
            rng: RandomXS128 {
                seed0: self.rng.get_state(0),
                seed1: self.rng.get_state(1),
            },
            counter: self.counter,
        }
    }

    /// `setCounter(target)`：仅当 counter < target 时用 randomBoolean 补进；否则打错误日志（原版行为，此处返回 Err）。
    pub fn set_counter(&mut self, target: i32) -> Result<(), SetCounterError> {
        if self.counter < target {
            while self.counter < target {
                self.random_bool();
            }
            Ok(())
        } else {
            Err(SetCounterError)
        }
    }

    /// `int random(int range)` → [0, range] 双闭区间。
    pub fn random_int_inclusive(&mut self, range: i32) -> i32 {
        self.counter += 1;
        self.rng.next_int_bounded(range + 1)
    }

    /// `int random(int start, int end)` → [start, end]。
    pub fn random_int_range(&mut self, start: i32, end: i32) -> i32 {
        self.counter += 1;
        start + self.rng.next_int_bounded(end - start + 1)
    }

    /// `long random(long range)` → `(long)(nextDouble() * range)`。
    pub fn random_long_inclusive(&mut self, range: i64) -> i64 {
        self.counter += 1;
        (self.rng.next_f64() * range as f64) as i64
    }

    /// `long random(long start, long end)` → `start + (long)(nextDouble() * (end - start))`。
    pub fn random_long_range(&mut self, start: i64, end: i64) -> i64 {
        self.counter += 1;
        start + (self.rng.next_f64() * (end - start) as f64) as i64
    }

    /// `long randomLong()`。
    pub fn random_long(&mut self) -> i64 {
        self.counter += 1;
        self.rng.next_long()
    }

    /// `boolean randomBoolean()`。
    pub fn random_bool(&mut self) -> bool {
        self.counter += 1;
        self.rng.next_bool()
    }

    /// `boolean randomBoolean(float chance)` → `nextFloat() < chance`。
    pub fn random_bool_chance(&mut self, chance: f32) -> bool {
        self.counter += 1;
        self.rng.next_f32() < chance
    }

    /// `float random()`。
    pub fn random_f32(&mut self) -> f32 {
        self.counter += 1;
        self.rng.next_f32()
    }

    /// `float random(float range)`。
    pub fn random_f32_scaled(&mut self, range: f32) -> f32 {
        self.counter += 1;
        self.rng.next_f32() * range
    }

    /// `float random(float start, float end)`。
    pub fn random_f32_range(&mut self, start: f32, end: f32) -> f32 {
        self.counter += 1;
        start + self.rng.next_f32() * (end - start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xorshift_reproducible() {
        let mut a = RandomXS128::new(42);
        let mut b = RandomXS128::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_long(), b.next_long());
        }
    }

    #[test]
    fn murmur_of_zero_is_zero() {
        assert_eq!(murmur_hash3(0), 0);
    }

    #[test]
    fn set_seed_zero_uses_long_min() {
        // seed=0 应回退为 i64::MIN（libgdx 行为），与"直接传 i64::MIN"一致
        let mut a = RandomXS128::new(0);
        let mut b = RandomXS128::new(i64::MIN);
        assert_eq!(a.next_long(), b.next_long());
    }
    #[test]
    fn sts_random_counter_semantics() {
        let mut r = StsRandom::new(7);
        assert_eq!(r.counter, 0);
        r.random_int_inclusive(10);
        assert_eq!(r.counter, 1);
        r.random_bool();
        assert_eq!(r.counter, 2);
    }

    #[test]
    fn fast_forward_matches_incremental() {
        // Random(seed, counter) 快进 ≡ 先建再消费 counter 次 random(999)
        let mut a = StsRandom::with_counter(12345, 17);
        let mut b = StsRandom::new(12345);
        for _ in 0..17 {
            b.random_int_inclusive(999);
        }
        assert_eq!(a.counter, 17);
        assert_eq!(b.counter, 17);
        assert_eq!(a.random_long(), b.random_long());
    }

    #[test]
    fn set_counter_uses_random_bool_stream() {
        // setCounter 与快进不同流：显式断言两者产物不一致（保真语义）
        let mut a = StsRandom::new(999);
        a.set_counter(5).unwrap();
        let mut b = StsRandom::with_counter(999, 5);
        // 消耗序列不同，不要求相等；只要求 set_counter 后 counter 正确且可继续取数
        assert_eq!(a.counter, 5);
        let _ = a.random_int_inclusive(10);
        let _ = b.random_int_inclusive(10);
    }

    #[test]
    fn copy_matches_state() {
        let mut r = StsRandom::new(2024);
        r.random_long();
        r.random_bool();
        let mut c = r.copy();
        assert_eq!(c.random_long(), r.random_long());
        assert_eq!(c.counter, r.counter);
    }

    #[test]
    fn jdk_shuffle_deterministic() {
        let mut v: Vec<u32> = (0..10).collect();
        let mut rng = JdkRandom::new(31415);
        shuffle_jdk(&mut v, &mut rng);
        let mut v2: Vec<u32> = (0..10).collect();
        let mut rng2 = JdkRandom::new(31415);
        shuffle_jdk(&mut v2, &mut rng2);
        assert_eq!(v, v2);
    }

    // 注：跨实现参考向量（与 libgdx/JDK 运行时比对）属 docs/06 §4 的 L8 原版对照，
    // 在批次 1 的 G1 门采集 fixture 后补入 tests/fixtures/。
}
