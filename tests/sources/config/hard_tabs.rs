// With `hard_tabs`, each block indentation level is a tab and alignment is in spaces after the
// tabs (closure parameters after `|`, the lines of a block comment); widths count a tab as
// `tab_spaces` columns, so the layouts are those with spaces
fn main() {
    let f = |first_parameter_xxxxxxxxxxxxxxxxxxxxx: u32, second_parameter_yyyyyyyyyyyyyyyyyyyyyyyyy: u32, third_parameter_zzzzzzzzzzzzzzzzzzzzzzz: u32| first_parameter_xxxxxxxxxxxxx + 1;
    if x {
        foo(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccccccccc);
        match z {
            A => {
                g();
            }
            B => h(), // trailing
        }
        f(); /* block
                comment */
        defer! {
            shutdown_terminal();
        }
        let s = "a
  b";
    }
}
