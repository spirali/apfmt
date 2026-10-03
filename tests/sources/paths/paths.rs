fn main() {
    let a = std :: mem :: take(x);
    let b = :: std::vec::Vec::new();
    let c = Vec :: < u32 > :: with_capacity(10);
    let d = std::mem::size_of::<u64>();
    let e: Vec < u32 > = Vec::new();
    let f: std::collections::HashMap<String,Vec<u32>> = HashMap::new();
    let g: &'a Foo<'a, T> = x;
    let h: Box<dyn_like::Iterator<Item=u32>> = y;
    let i = < T as Default > :: default();
    let j: <T as Iterator>::Item = k;
    let l = self::helper(super::value, crate::CONST);
    let m = Self::new();
    let n = Array::<u8, 4>::new();
    let o: Box<Fn(u32, &str) -> Option<u32>> = p;
    if let Some(q) = std::env::var_os(KEY) { f(q); }
}
