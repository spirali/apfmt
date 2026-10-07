fn f(
    a: (u32,),
    b: (),
    c: &[u8],
    d: [u8; 4],
    e: impl Fn(u32) -> u32 + Send,
    g: Box<dyn A + B>,
    h: *const u8,
    i: *mut T,
    j: fn(u32, &str) -> bool,
) -> ! {}
fn g(
    x: unsafe extern "C" fn(*const u8),
    y: &(dyn Trait + 'a),
    z: Vec<_>,
) -> impl Iterator<Item = (u32, u64)> {}
fn h(x: &'a mut [(u8, u16); 3]) {}
fn tuple_ret() -> (
    AaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaAA,
    BBBBBBBBBBBBBBBBBB,
    CCCCCCCCCCCCCCCCCCC,
) {}
