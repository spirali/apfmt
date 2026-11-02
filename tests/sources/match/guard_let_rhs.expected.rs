fn f() {
    match x {
        // The right side does not fit on one line on the next line: both tuples break, as in `if let`
        A if let (
            Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
            Bbbbbbbbbbbbbbbbbbbbbbbbbbbb,
            Ccccccccccccccc,
        ) = (
            aaaaaaaaaaaaaaaaaaaaaaaaa,
            bbbbbbbbbbbbbbbbbbbbbb,
            cccccccccccccccc,
        ) =>
        {
            1
        }
        // It fits there: it moves after `=`, as in `if let`
        A if let Some(Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa) =
            compute(aaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbb) =>
        {
            2
        }
        // A chain breaks on the line of `=`
        A if let Some(Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa) = self
            .aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
            .compute_something(aaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbb) =>
        {
            3
        }
        // A let chain is not affected
        A if let Some(a) = b
            && let (Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, Bbbbbbbbbbbbbbbbbbbbbbbbbbbb, Cccc) = (
                aaaaaaaaaaaaaaaaaaaaaaaaa,
                bbbbbbbbbbbbbbbbbbbbbb,
                cccccccccccccccc,
            ) =>
        {
            4
        }
    }
    {
        {
            if let (
                Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
                Bbbbbbbbbbbbbbbbbbbbbbbbbbbb,
                Ccccccccccccccc,
            ) = (
                aaaaaaaaaaaaaaaaaaaaaaaaa,
                bbbbbbbbbbbbbbbbbbbbbb,
                cccccccccccccccc,
            ) {
                1
            }
            if let Some(Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa) =
                compute(aaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbb)
            {
                2
            }
        }
    }
}
