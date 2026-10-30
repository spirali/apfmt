trait T {
    /// An associated type.
    type A: Clone;
    /** An associated constant. */
    const C: u8;

    /// A method.
    #[must_use]
    fn f(&self) -> u8;
    /// With a default body.
    fn g(&self) {}
}

impl T for S {
    /// The type.
    type A = u8;
    /// The constant.
    /// On two lines.
    const C: u8 = 1;

    /// A method.
    fn f(&self) -> u8 { 1 }
    /** Block doc comment. */ fn g(&self) {}
}

impl S {
    pub const X: u8 = 1; /// Documents `new`.
    pub fn new() -> Self { S }
}
