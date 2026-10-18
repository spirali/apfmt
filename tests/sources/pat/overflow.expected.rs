fn f() {
    let x = match opts {
        SubCommand::Server(ServerOpts {
            subcmd: ServerCommand::Start(_),
        }) => false,
        SubCommand::Worker(WorkerOpts {
            subcmd: WorkerCommand::Start(_),
            other: Other::Thing(_),
        }) => false,
        Some(Foo { a, b }) => 1,
        Some(&Fooooooooooooooooooooo {
            aaaaaaaaaaaaaaaaa,
            bbbbbbbbbbbbbbbbbbbbb,
            cccccccccccccccccccc,
        }) => 1,
        Some(Baaaaaaaaaaaaaaaaaaaaaaaar(
            aaaaaaaaaaaaaaaaaaaa,
            bbbbbbbbbbbbbbbbbbbbbbbbbbb,
            cccccccccccccc,
        )) => 1,
        Some((
            aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
            bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
            cccccccccccccccccccccc,
        )) => 1,
        (Fooooooooooooooooooooooooooo {
            aaaaaaaaaaaaaaaaa,
            bbbbbbbbbbbbbbbbbbbbb,
            ccccccccccccccc,
        },) => 1,
        Two(
            x,
            Fooooooooooooooooooooooooooo {
                aaaaaaaaaaaaaaaaa,
                bbbbbbbbbbbbbbbbbbbbb,
                ccccccccc,
            },
        ) => 1,
        Some(
            [
                aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
                bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
                cccccccccccccccc,
            ],
        ) => 1,
        _ => true,
    };
    let Some(Configuration {
        aaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        cccccccccc,
    }) = x;
    for Some(Configuration {
        aaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        cccccccccc,
    }) in x
    {}
}
