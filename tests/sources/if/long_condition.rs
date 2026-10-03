fn main() {
    if some_very_long_function_name(first_argument, second_argument, third_argument, fourth_arg) {
        f();
    }
    if some_function_with_long_name(first_argument_value, second_argument_value, third_argument_val) { f(); }
    if some_long_condition_name_number_one && some_long_condition_name_number_two && some_other_name { f(); }
    if some_long_condition_name_number_one && some_long_condition_name_number_two && some_other_name_xx { f(); }
    if some_long_condition_name_number_one(arg) || !some_long_condition_name_number_two || (a == bbbbbbbbbbbbbb) { f(); }
    if x {
        f();
    } else if some_long_condition_name_number_one && some_long_condition_name_number_two && other_name {
        g();
    }
}
