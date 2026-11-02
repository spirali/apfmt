fn f() {
    // The arguments in front of the closure are wider than `fn_call_width`, with or without a string
    setup_long_function_name_here("a_rather_long_literal_argument_value_here", |dirs, sandbox| {
        let expected = 1;
    });
    setup_long_function_name_here("a_rather_long_literal_argument_value_here", |dirs, sandbox| {
        let expected = "\
line
";
    });
    setup_long_function_name_here("a_rather_long_literal_argument_value_here", |dirs, sandbox| {
        let expected = "line one";
        g("\
line
");
    });
    // Within `fn_call_width`, it hugs with or without a string
    setup(first, |dirs, sandbox| {
        let expected = 1;
    });
    setup(first, |dirs, sandbox| {
        let expected = "\
line
";
    });
    // The only argument is not limited by the arguments in front of it
    setup(|dirs, sandbox| {
        let expected = "\
line
";
    });
    setup_long_function_name_here("a_rather_long_literal_argument_value_here", async move {
        let s = "\
x";
    });
    m!(a_rather_long_literal_argument_value_here_for_a_macro_call_here, |dirs, sandbox| {
        let expected = "\
line
";
    });
}
