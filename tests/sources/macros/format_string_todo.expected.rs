// `todo!` and `unimplemented!` take a format string too (Style Guide); rustfmt does not list them
fn f() {
    todo!(
        "a very long format string with {} and {} and {} placeholders xxxxxxxxxxxxxxxxxxx",
        e, g
    );
    unimplemented!(
        "a very long format string with {} and {} and {} placeholders xxxxxxxxxx",
        e, g
    );
}
