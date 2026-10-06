fn line_comment() // c
-> u32 {
    1
}
fn own_line()
    // c
    -> u32 {
    1
}
fn main() {
    let x = a // c
    + b;
    let y = a /* c */
    + b;
    f // c
    (1);
    let z = // c
        value;
    g(a, // c
      b);
}
fn conditions() {
    if first_condition_value_long // c
        && second_condition_value_long
        && third_condition_value_long_xx
    {
        a();
    }
    if x // c
    {
        a();
    }
}
