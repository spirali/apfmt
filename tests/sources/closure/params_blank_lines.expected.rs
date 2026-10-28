// Blank lines between closure parameters aligned after `|` are kept, as between function parameters
fn main() {
    let f = |first_parameter_xxxxxxxxxxxxxxxxxxxxx: u32,

             second_parameter_yyyyyyyyyyyyyyyyyyyyyyyyy: u32,
             third_parameter_zzzzzzzzzzzzzzzzzzzzzzz: u32| first_parameter_xxxxxxxxxxxxx + 1;
    let f = |first_parameter_xxxxxxxxxxxxxxxxxxxxx: u32,

             second_parameter_yyyyyyyyyyyyyyyyyyyyyyyyy: u32,
             third_parameter_zzzzzzzzzzzzzzzzzzzzzzz: u32|
             -> u32 { first_parameter_xxxxxxxxxxxxx + 1 };
    let f = |a: u32, b: u32| a + b;
}
