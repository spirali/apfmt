// A single string spanning lines hugs the list as in the source, as a call argument; rustfmt hugs
// `name = "..."` if its first line is at most `attr_fn_like_width` wide, and never a bare string
#[should_panic(expected = "hugged in the source \
    second line")]
fn a() {}

#[should_panic(
    expected = "on its own line in the source \
    second line",
)]
fn b() {}

#[should_panic(expected = r#"a raw string
    second line"#)]
fn c() {}

#[deprecated(note = "a first line longer than attr_fn_like_width, hugged in the source \
    second line")]
fn d() {}

#[error("a bare string hugged in the source
second line")]
struct E;

#[error(
    "a bare string on its own line in the source
second line",
)]
struct F;

// Nested in a single list
#[foo(bar(x = "a
b"))]
fn g() {}

#[foo(bar(
    x = "a
b",
))]
fn h() {}

// More arguments are not hugged
#[foo(
    x = "a
b",
    y = 1,
)]
fn i() {}

// One line does not fit after `(`, so it moves to the next one
#[should_panic(
    expected = "a first line that is far too long to fit after the opening parenthesis of the list \
    second line",
)]
fn j() {}

// The last line does not fit anywhere: the string stays as in the source
#[should_panic(expected = r#"first line
xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"#)]
fn l() {}

// The last line fits only on the line of its own: 99 columns, with `,` at column 100
#[should_panic(
    expected = r#"first line
xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"#,
)]
fn n() {}

mod m {
    #[should_panic(expected = "indented \
        second line")]
    fn k() {}
}
