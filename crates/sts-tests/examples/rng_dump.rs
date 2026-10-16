use sts_data::rng::{RandomXS128, JdkRandom, shuffle_jdk};
fn main() {
    let mut r = RandomXS128::new(1);
    let out: Vec<i64> = (0..16).map(|_| r.next_long()).collect();
    println!("{:?}", out);
    let mut j = JdkRandom::new(42);
    let out2: Vec<i32> = (0..8).map(|_| j.next_bits(31)).collect();
    println!("{:?}", out2);
    let mut v: Vec<u32> = (0..10).collect();
    let mut rng = JdkRandom::new(31415);
    shuffle_jdk(&mut v, &mut rng);
    println!("{:?}", v);
}
