// Macro arguments that parse only as a chained comparison are not expressions (rustc rejects
// `a < b > c`): they are copied unchanged, their spacing kept
fn clap_args() {
    let cmd = Command::new("prog")
        .arg(arg!(--opt <FILE> "some arg"))
        .arg(arg!(-c --config <FILE> "Sets a custom config file").required(false))
        .arg(arg!(-O --option3 <option3> "specific vals").value_parser(opt3_vals))
        .arg(arg!(--"long-option-2" <option2> "tests long options with exclusions"))
        .arg(arg!(name: -n --name <NAME> ... "names"));
}

fn chained() {
    m!(a<b>c);
    m!(a==b==c);
    m!(a<b==c);
    m!(x, a  <  b  >  c);
}

// A comparison of a parenthesized comparison is an expression, formatted
fn parenthesized() {
    m!(a<(b>c));
    m!((a<b)==c);
    m!(a<b, c>d);
}

macro_rules! chained_in_transcriber {
    ($a:expr) => {
        f($a<b>c)
    };
}
