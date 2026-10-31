fn f(c: char) -> bool {
    let x = first_condition_value // first
        && second_condition_value // second
        && third_condition_value;
    let y = first_operand_value
        + /* a */ second_operand_value // b
        + third_operand_value;
    match c {
        '\u{019C}' // Ɯ  [LATIN CAPITAL LETTER TURNED M]
        | '\u{1D0D}' // ᴍ  [LATIN LETTER SMALL CAPITAL M]
        | '\u{1E3E}' // Ḿ  [LATIN CAPITAL LETTER M WITH ACUTE]
            => true,
        'a' // first
        | 'b' // second
            => true,
        _ => false,
    }
}
