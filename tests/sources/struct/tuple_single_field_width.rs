// A single tuple field is limited by `max_width` only, not by `fn_call_width`
enum E<'a> {
    DoubleSlice(iter::Chain<std::slice::Iter<'a, Node>, std::slice::Iter<'a, Node>>),
    Plain(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa),
    Pub(pub iter::Chain<std::slice::Iter<'a, Node>, std::slice::Iter<'a, Node>>),
    Attr(#[serde(skip)] iter::Chain<std::slice::Iter<'a, Node>, std::slice::Iter<'a, Node>>),
    Trailing(iter::Chain<std::slice::Iter<'a, Node>, std::slice::Iter<'a, Node>>), // c
    Discriminant(iter::Chain<std::slice::Iter<'a, Node>, std::slice::Iter<'a, Node>>) = 5,
    // two fields wider than `fn_call_width`, or a single field too wide for the line
    Two(Chain<std::slice::Iter<'a, Node>, std::slice::Iter<'a, Node>>, u32, u64),
    TooWide(iter::Chain<std::slice::Iter<'a, Nodeeeeeeeeeeeeeee>, std::slice::Iter<'a, Nodeeeeee>>),
}
struct S<'a>(iter::Chain<std::slice::Iter<'a, Node>, std::slice::Iter<'a, Nodeeeeeeeeee>>);
pub(crate) struct Wrapper(pub(crate) HashMap<String, Vec<(usize, Option<String>)>, Hasher>);
struct W<T>(iter::Chain<std::slice::Iter<'a, Node>, std::slice::Iter<'a, Nod>>)
where
    T: Copy;
