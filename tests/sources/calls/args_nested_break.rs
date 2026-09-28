fn main() {
    outer(inner_call(first_long_argument, second_long_argument, third_long_argument_x), b);
    outer(a, inner_call(first_long_argument, second_long_argument, third_long_argument_x));
}
