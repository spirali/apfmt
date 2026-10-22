// A hugging last argument: `fn_call_width` covers the arguments in front of it and its first line
fn string_to_duration(s: &str, span: Span) -> Result<i64, ShellError> {
    if let Some(Ok(expression)) = parse_unit_value(
        s.as_bytes(),
        span,
        DURATION_UNIT_GROUPS,
        Type::Duration,
        |x| x,
    ) && let Expr::ValueWithUnit(value) = expression.expr
        && let Expr::Int(x) = value.expr.expr
    {
        x
    }
    let z = parse_unit_value(
        s.as_bytes(),
        span,
        DURATION_UNIT_GROUPS,
        Type::Duration,
        |x| x,
    );
    let Some(Ok(expression)) = parse_unit_value(
        s.as_bytes(),
        span,
        DURATION_UNIT_GROUPS,
        Type::Duration,
        |x| x,
    ) else {
        return;
    };
    Playground::setup(
        "idx_status_reports_initialized_after_init",
        |dirs, sandbox| {
            sandbox.with_files(&[EmptyFile("beta.txt")]);
            test()
                .cwd(dirs.test())
                .run("idx init .; idx status | get initialized")
        },
    );
    Playground::setup("idx_status_reports_initialized", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("beta.txt")]);
        test()
            .cwd(dirs.test())
            .run("idx init .; idx status | get initialized")
    });
    foo(aaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, unsafe {
        cccccccccccccccccccccccccc(dddddddddddddddddd)
    });
    // The first line of the hug is not squeezed into the width by a layout that costs a split
    format_text(
        format_str,
        config_path,
        context,
        |variable| match variable {
            "count" => Some(count.to_string()),
            _ => None,
        },
    );
    map(
        tuple((leaf, fallible(boost))),
        |(leaf, boost_opt)| match boost_opt {
            Some(boost) => UserInputAst::Boost(Box::new(leaf), boost.into()),
            None => leaf,
        },
    );
    s.try_binary_zip_and_apply_amortized(
        n,
        offset,
        |opt_s, opt_n, opt_offset| match (opt_s, opt_n, opt_offset) {
            (Some(s), Some(n), Some(offset)) => {
                Ok(Some(s.gather_every(n as usize, offset as usize)?))
            }
            _ => Ok(None),
        },
    )?;
    // Braces of the source are kept to hug
    call_hooks(worker, test_hooks.after_each.iter().rev(), |core_error| {
        match core_error {
            CoreErrorKind::Js(err) => Ok(()),
            err => Err(err.into_box().into()),
        }
    }).await?;
}
