fn one(x: u32) {}
fn two( a : u32 , b:&str ) {}
fn trailing_comma(a: u32, b: u32,) {}
fn patterns(mut a: u32, _: u32, ref b: u32, (c, d): Pair, Wrapper(e): Wrapper) {}
fn references(a: &u32, b: &mut Vec<u32>, c: &'a str, d: &'a mut String) {}
fn paths(a: std::string::String, b: HashMap<String, Vec<u32>>, c: <T as Trait>::Output) {}
fn vertical(
    a: u32,
    b: u32,
) {
    a;
}
fn exactly_fitting_parameter_list(first_parameter: u32, second_parameter: &str, xyz_abcdefgh: u64) {
    first_parameter;
}
fn one_over_the_limit_parameter_list(first_parameter: u32, second_parameter: &str, xyz_abcdefg: u64) {
    first_parameter;
}
fn very_long_function_name_with_parameters(first_parameter: u32, second_parameter: &str, third_parameter: u64) {
    first_parameter;
}
