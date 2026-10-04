struct T(u32 /* x */, String);
struct V(
    first_field_type_name_aaaaaaaaaaaaaaaaaa /* x */,
    second_field_type_name_aaaaaaaaaaaaaaaaa,
);
struct W(
    first_field_type_name_aaaaaaaaaaaaaaaaaa // x
    ,
    second_field_type_name_aaaaaaaaaaaaaaaaa,
);
struct X(u32 /* x */ /* y */, String /* z */);
fn f(a: u32 /* x */, b: u32) {}
fn main() {
    f(1 /* c */, 2);
    g(a /* c */, b);
    f(first_argument_value_aaaaaaaaaaaaaaaaaaaaaaa /* c */, second_argument_value_aaaaaaaaaaaaaaaaa);
}
fn patterns() {
    let (a,) = x;
    let (a /* c */,) = x;
    let (a /* c */, b) = x;
    let S(a /* c */, b,) = x;
    f(a /* c */,);
    f::<A /* c */, B>();
}
fn g(&self /* s */, a: u32) {}
