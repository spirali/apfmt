fn main() {
    a(b(c(d(first_long_argument_name, second_long_argument_name, third_long_argument_name))));
    outer(inner_call(first_long_argument, second_long_argument, third_long_argument_x));
    outer(inner_call(first_long_argument, second_long_argument, third_long_argument_x), b);
}
