// The arguments of a `[...]` macro keep the trailing comma of the source, broken and on one line:
// the pattern of the macro may reject one (`$($x:expr),*`) or require it (`$($x:expr,)*`). `vec!`
// accepts both, so it gets one when broken and none on one line, as an array
macro_rules! no_comma {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}
macro_rules! need_comma {
    ($($x:expr,)*) => (vec![$($x.to_string()),*]);
}
fn main() {
    let a: Vec<String> = no_comma!["first_argument_xxxxxxxxxx", "second_argument_xxxxxxxxx", "third_argument_xxxxxxxxxxx"];
    let b: Vec<String> = need_comma!["first_argument_xxxxxxxxxx", "second_argument_xxxxxxxxx", "third_argument_xxxxxxxxxxx",];
    let c: Vec<String> = need_comma!["a", "b",];
    let d: Vec<&str> = vec!["first_argument_xxxxxxxxxx", "second_argument_xxxxxxxxx", "third_argument_xxxxxxxxxxx"];
    let e: Vec<&str> = vec!["a", "b",];
    let f = no_comma![1; 3];
}
