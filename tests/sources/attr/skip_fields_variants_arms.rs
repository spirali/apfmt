struct Permissions {
    #[rustfmt::skip]
    user_read:   bool,
    user_write:  bool,
}

enum E {
    #[rustfmt::skip]
    A {   x:u32 },
    B(u32),
    C,
}

fn plain() -> UiStyles {
    UiStyles {
        colourful: Some(false),

        #[rustfmt::skip]
        perms: Some(Permissions {
            user_read:           Some(Style::default()),
            user_write:          Some(Style::default()),
            user_execute_file:   Some(Style::default()),
        }),

        size:  Some(Size::default()),
    }
}

fn f(x: u32) -> u32 {
    match x {
        #[rustfmt::skip]
        1   =>   2,
        _ =>  3,
    }
}
