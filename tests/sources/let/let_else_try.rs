fn f() {
    let Some(x) = g(a)? else {
        return;
    };
    let Some(x) = some_function_name_that_is_long(first_argument_value_aaaaaaaaaaaaaa, second_argument_value, third_argument_aaaaaaaaaa)?? else {
        return;
    };
    let Some(x) = m!(first_argument_value_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, second_argument_value, third_argument_aaaaaaaaaaaaaaa)? else {
        return;
    };
    let Some(x) = foo(|a| { b; c })? else {
        return;
    };
    let Some(x) = aaaa.some_function_name_that_is_long(|a| { b; c })? else {
        return;
    };
    let Some(x) = some_function_name_that_is_long(first_argument_value_aaaaaaaaaaaaaa, second_argument_value, third_argument_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa) else {
        return;
    };
    if let Some(x) = some_function_name_that_is_long(first_argument_value_aaaaaaaaaaaaaa, second_argument_value, third_argument_aaaaaaaaa)? {
        return;
    }
    while some_function_name_that_is_long(first_argument_value_aaaaaaaaaaaaaaaaaaa, second_argument_value, third_argument_aaaaaaaaaaaaaa)? {
        return;
    }
}
