fn no_return() where {}
fn no_return_body() where { g() }
fn no_return_own_line()
where
{
    g()
}
fn with_return<T>(a: T) -> T where { a }
fn with_return_own_line<T>(a: T) -> T
where
{
    a
}
fn broken_params(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa: A, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb: B) -> T where { a }
struct Unit where;
struct Named where {}
struct Fields where { a: u8 }
struct Tuple() where;
struct TupleFields(u8) where;
impl A for B where {}
impl A for C where { fn f() {} }
trait D where {}
trait E: F where { fn f(); }
enum G where {}
enum H where { I }
type J where = u8;
const fn l() where {}

mod inner {
    pub(crate) fn build(self, layers: Vec<BoxedConnectorLayer>) -> Connector
where {
        1
    }
}

impl M {
    fn method(&self) where {}
    fn method_return(&self) -> T where { a }
    type N = u8 where;
}

// With predicates, it stays
fn with_predicate<T>(a: T) where T: Clone {}

// A comment keeps it
fn comment_inline() where /* c */ {}
fn comment_line() where // d
{}
fn comment_own_line()
where
    // e
{}

// A comment in front of it stays
fn comment_before() /* f */ where {}

// Skipped code stays as it is
#[rustfmt::skip]
fn skipped() where {}

// In a macro call, its tokens are not ours to change
m! { fn in_macro() where {} }
