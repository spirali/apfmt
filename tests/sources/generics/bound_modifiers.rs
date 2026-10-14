fn h() -> impl Iterator<Item = u8> + use<'_> { todo!() }
fn h2<'a, T>() -> impl Sized + use<'a,T> { todo!() }
fn h3() -> impl Sized + use<> { todo!() }
fn k<T: [const] Default + const PartialEq, F: async Fn(), G: for<'a> async FnMut(&'a u8)>() { a }
