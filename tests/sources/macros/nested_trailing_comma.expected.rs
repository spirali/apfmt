// A broken list nested in the arguments of a macro call gets a trailing comma, as outside macros
// (rustfmt omits it); the last argument of the macro itself gets none, as in rustfmt
fn f() {
    assert_eq!(
        x,
        Some(TxnState::Ongoing {
            min_commit_ts: 3.into(),
            aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa: 1,
        })
    );
    assert_eq!(Some(TxnState::Ongoing {
        min_commit_ts: 3.into(),
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa: 1,
    }));
    assert_eq!(
        r,
        vec![
            String::from("aaaaaaaaaaaaaaaaaaaaa"),
            items[0].path.clone(),
            items[1].path.clone(),
        ]
    );
    assert_eq!(
        r,
        f(
            String::from("aaaaaaaaaaaaaaaaaaaaaaaaa"),
            items[0].path.clone(),
            items[1].path.clone(),
        )
    );
}
