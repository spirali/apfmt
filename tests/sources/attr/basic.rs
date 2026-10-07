#[test]
#[derive( Debug,Clone ,PartialEq)]
#[cfg(any(unix,  feature = "x"))]
#[doc = "text"]
#[cfg_attr(test, derive(Debug), allow(dead_code))]
#[unsafe(no_mangle)]
#[allow(clippy::needless_return)]
#[doc(alias = "y", hidden)]
#[macro_export(local_inner_macros)]
fn f() {}

#[foo(a + b)]
#[bar(crate::x)]
#[baz { a b }]
fn g() {}
