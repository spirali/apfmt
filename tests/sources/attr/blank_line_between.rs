// A blank line between attributes of the same kind is removed; one between `#![...]` and `#[...]`, in
// front of the item and around a doc comment stays
#![allow(a)]

#![allow(b)]


#![allow(c)]

//! A doc comment

#![allow(d)]

#[a]

#[b]
fn  f() {
    #![allow(a)]

    #![allow(b)]

    #[a]

    #[b]
    let  x=1;

    #[a]

    #[b]
    x  ;
    match  x {
        #![allow(a)]

        #![allow(b)]
        #[a]

        #[b]
        A=>1,
        _=>2,
    }
}

#[a]

#[b]
struct  S {
    #[a]

    #[b]
    x:u8,
}

mod m {
    #![allow(a)]

    #![allow(b)]

    fn  f() {}
}

/// A doc comment

#[a]

/// A second one

fn  g(
    #[a]

    #[b]
    x:u8,
) {
    h();
}

#[a]

#[b]
enum  E {
    #[a]

    #[b]
    A,
}

#[a]

fn  h() {}

#[a]
// A comment

fn  i() {}

/// A doc comment

fn  j() {}
