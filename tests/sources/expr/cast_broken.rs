// Style Guide "Casts": break before `as`; a series breaks before the first `as` only if the rest fits
fn main() {
    let percent = (((memory_stats.total - memory_stats.free) as f64 / (memory_stats.total as f64))
        * 100.00) as u64;
    let cstr = very_long_expression_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa() as *const str;
    let x = (first_operand_value_xxxxxxxxxxxxxxxxxxxxxxxx + second_operand_value_xxxxxxxxxxxxxxx) as u64;
    let cstr = very_long_expression_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa() as *const str as *const [u8] as *const std::os::raw::c_char;
    let cstr = very_long_expression_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa() as *const some_module::very_long_type_name_aaaaaaaaaa as *const another_module::another_long_type_name_bbbbbbbbbbbbbbb;
    let y = foo(first_argument_value_xxxxxxxxxxxxxxxxxx, second_argument_value_xxxxxxxxxxxxxxxxx) as u64;
}
