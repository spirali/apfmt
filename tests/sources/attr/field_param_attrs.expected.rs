// A single attribute stays on the line of a tuple field or a parameter when it fits on one line;
// several attributes, or one that does not fit there, go on their own lines
pub struct A(#[serde(skip)] pub u8);
pub struct B(
    #[cfg(feature = "python")]
    #[serde(skip)]
    pub Arc<u8>,
);
pub struct C(
    #[cfg_attr(feature = "dsl-schema", schemars(with = "Vec<u8>"))]
    pub Arc<polars_utils::python_function::PythonFunction>,
);
pub struct D(
    #[cfg_attr(feature = "serde", serde(skip))]
    // comment
    pub u8,
);

fn d(#[values(1, 2)] a: u32) {
    a
}
fn e(
    #[values(OpScheduling::Eager, OpScheduling::Lazy, OpScheduling::Deferred)]
    scheduling: OpScheduling,
) {
    scheduling
}
fn f(
    #[case] driver: D,
    #[values(OpScheduling::Eager, OpScheduling::Lazy)]
    #[allow(unused_variables)]
    scheduling: OpScheduling,
) {
    driver
}
impl S {
    fn g(#[cfg(foo)] &mut self, #[deny(C)] b: i32) {
        b
    }
}
