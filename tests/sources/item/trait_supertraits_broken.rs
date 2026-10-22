pub trait Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa: Bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb + Cccccccccccccccccccccccccccccccccccccccccc + Ddddddddddddddddddddddd {
    fn f(&self);
}

pub trait Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa<T>: Bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb + Cccccccccccccccccccccccccccccc + Ddddddddddddddddddddddd where T: Clone {
    fn f(&self);
}

pub trait Short: Bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb + Cccccccccccccccccccccccccccccc {
    fn f(&self);
}

fn f<T: Bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb + Cccccccccccccccccccccccccccccc + Ddddddddddddddddddddddd>() {
    x
}
