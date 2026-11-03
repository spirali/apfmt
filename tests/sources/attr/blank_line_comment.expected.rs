// A blank line next to a comment between attributes of the same kind stays; rustfmt removes it
#![allow(a)]

// In front of an attribute
#![allow(b)]
// Behind an attribute

#![allow(c)]

#[a]

// In front of an attribute
#[b]
// Behind an attribute

#[c]
struct S {
    #[a]

    // In a list
    #[b]
    x: u8,
}
#[a] // Ending the line

#[b]
fn f() {}
