fn f() {
    {
        {
            {
                let a = cmd
                    .hide_default_value(true)
                    .help(
                        "Specify when to use the pager, or use `-P` to disable (*auto*, never, always).",
                    )
                    .long_help("short");
            }
        }
    }
}
