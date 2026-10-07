struct S {
    /// Field doc
    #[allow(unused)]
    a: u32,
    #[cfg(test)] b: u32,
}

struct T(#[allow(unused)] u32, u64);

#[repr(u8)]
enum E {
    #[default]
    A,
    /// Variant doc
    #[allow(unused)] B(u32),
    #[cfg(test)]
    C { x: u32 },
}

impl S {
    fn f(#[allow(unused)] &self, #[cfg(test)] a: u32) {
        #[allow(unused)]
        let x = 1;
    }
}
