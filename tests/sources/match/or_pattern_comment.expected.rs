// An or-pattern of small alternatives with a comment has one alternative per line; rustfmt leaves it
// as written
fn f() {
    match x {
        // An own-line comment
        "_"
        // c
        | "a"
        | "b"
        | "c"
        | "d" => 1,
        // A comment ending the line of an alternative
        'a'
        | 'b' // t
        | 'c' => 2,
        // Behind the operator
        'd' // t
        | 'e'
        | 'f' => 3,
        'g'
        // c
        | 'h'
        | 'i' => 4,
        // After every alternative
        'j' // j
        | 'k' // k
        | 'l' => 5,
        // On one line where it fits, also with a block comment
        'm' /* c */ | 'n' | 'o' => 6,
        // Without comments, filled
        "abstract" | "as" | "async" | "await" | "become" | "box" | "break" | "const" | "continue"
        | "crate" | "do" => 7,
        _ => 8,
    }
}
