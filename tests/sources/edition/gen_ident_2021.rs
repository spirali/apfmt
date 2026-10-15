fn gen() -> u32 { 1 }
fn main() {
    let gen = rng.gen();
    let x: u32 = rng.gen_range(0..10) + gen;
    let s = S { gen: 1 };
}
