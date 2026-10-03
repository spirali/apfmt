fn main() {
    f(if a {
        if b { 1 } else { 2 }
    } else {
        if c { 3 } else { 4 }
    });
    some_function_name(if some_condition_value {
        first_function(argument)
    } else {
        second_function(argument)
    });
    some_function_name(if x {
        g();
    } else {
        h();
    });
    some_function_name({
        first_statement_call(argument_one);
        second_statement_call(argument_two)
    });
    some_function_name(
        !inner_function_call(first_long_argument, second_long_argument, third_argument_x),
    );
    some_function_name(
        if x {
            g();
        } else {
            h();
        },
        second_argument,
    );
    f(a == b);
    some_function_name(
        first_long_argument_value == second_long_argument_value && third_long_argument,
    );
}
