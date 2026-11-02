// An own-line comment after an operator moves in front of it, as the Style Guide breaks before the
// operator and a comment on the line of the operator moves too; rustfmt keeps the source
fn f(c: char) -> bool {
    let a = aaaaaaaa
        // own line
        && bbbbbbbb;
    let a = aaaaaaaa
        // In front of the operator in the source
        && bbbbbbbb;
    let a = aaaaaaaa // on the line of the operator
        // own line
        && bbbbbbbb;
    let a = aaaaaaaa
        // one
        // two
        && bbbbbbbb;
    let a = aaaaaaaa
        /* a block comment */
        && bbbbbbbb;
    let a = aaaaaaaa
        // before
        && bbbbbbbb
        // after
        && cccccccc;
    let a = (!lhs & lhs_v)
        // A = T & B = T
        | (lhs & lhs_v) & rhs;
    // A short first operand stays on its own line, also in a block
    let a = {
        lhs
            // B = T
            | (rhs & rhs_v)
            // A = F
            | !lhs
    };
    let a = 1
        // A short first operand of a broken call
        + call(
            aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
            bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        );
    let a = f(
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
    )
        // After a closing delimiter
        + 1;
    let a = aaaaaaaa
        && bbbbbbbb
        // A block as the last operand
        && { cccccccc };
    if aaaaaaaa
        // In a head
        && bbbbbbbb
    {
        1;
    }
    assert!(
        aaaaaaaaaaaaaaaaaaaaaaaaaaa
            // In a call
            || bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
    );
    match c {
        'a'
        // In a pattern
        | 'b' => true,
        Aaaaaaa(_)
        // In a pattern of calls
        | Bbbbbbb(_) => true,
        _ if aaaaaaaa
            // In a guard
            || bbbbbbbb =>
        {
            true
        }
        _ => false,
    }
}

// Left alone: comments followed by the operand on their line
fn g() {
    let a = aaaaaaaa && /* on one line */ bbbbbbbb;
}
