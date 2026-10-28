// A repetition on one line is one unbreakable piece: the transcriber around it is formatted (rustfmt
// copies the rule). No separator is added after a repetition, which may end with its own or expand
// to nothing; one in the source stays when the list is broken.
macro_rules! log_all {
    ($($arg:expr),*) => {
        println!("{}", [$($arg.to_string()),*].join(", "))
    };
    ($name:ident, $($arg:expr),+ $(,)?) => {
        foo($($arg),+)
    };
    ($($id:ident),+) => {
        pub(crate) enum SlhDsaVariantIdWithAVeryLongName {
            $($id,)+
        }
    };
    ($($a:expr),*) => {
        some_function_with_a_long_name(
            first_argument_long_name,
            second_argument_long_name,
            $($a.to_string()),*
        )
    };
    ($($a:expr),*) => {
        some_function_with_a_long_name(
            first_argument_long_name,
            second_argument_long_name,
            $($a.to_string()),*,
        )
    };
    ($($a:expr),*) => {
        f($($a),*)
    };
    // Not formatted: a repetition of statements, and one spanning lines
    ($($k:expr => $v:expr),*) => { { let mut m = Map::new(); $( m.insert($k, $v); )* m } };
    ($($k:expr),*) => { foo($(
        $k
    ),*) };
}
