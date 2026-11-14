fn no_return() {}
fn no_return_body() {
    g()
}
fn no_return_own_line() {
    g()
}
fn with_return<T>(a: T) -> T {
    a
}
fn with_return_own_line<T>(a: T) -> T {
    a
}
fn broken_params(
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa: A,
    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb: B,
) -> T {
    a
}
struct Unit;
struct Named {}
struct Fields {
    a: u8,
}
struct Tuple();
struct TupleFields(u8);
impl A for B {}
impl A for C {
    fn f() {}
}
trait D {}
trait E: F {
    fn f();
}
enum G {}
enum H {
    I,
}
type J = u8;
const fn l() {}

mod inner {
    pub(crate) fn build(self, layers: Vec<BoxedConnectorLayer>) -> Connector {
        1
    }
}

impl M {
    fn method(&self) {}
    fn method_return(&self) -> T {
        a
    }
    type N = u8;
}

// With predicates, it stays
fn with_predicate<T>(a: T)
where
    T: Clone,
{}

// A comment stays, `where` goes
fn comment_inline() /* c */ {}
fn comment_line() // d
{}
fn comment_own_line()
// e
{}

// A comment in front of it stays
fn comment_before() /* f */ {}

// Skipped code stays as it is
#[rustfmt::skip]
fn skipped() where {}

// In a macro call, its tokens are not ours to change
m! { fn in_macro() where {} }
