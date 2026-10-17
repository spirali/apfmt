// A lifetime of a reference type is followed by a space, also in front of a delimiter
struct S<'a, T> {
    slice: &'a [u8],
    slice_mut: &'a mut [u8],
    tuple: &'a (u8, u16),
    array: &'a [T; 4],
    unit: &'a (),
    path: &'a T,
    nested: &'a &'b [T],
    trait_object: &'a dyn Fn(&'a [u8]) -> &'a [u8],
}

fn body<'a>(items: &'a [(f64, f64)]) -> &'a [(f64, f64)] {
    items
}

impl<'a> Iterator for Items<'a> {
    type Item = &'a [u8];
}
