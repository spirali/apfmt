fn main() {
    if let Some(x) = y { f(x); }
    if let Some(x) = y { f(x); } else if let Some((a, _)) = z { g(a); } else { h(); }
    let value = if let Some(x) = y { x } else { 0 };
    if let Some(x) = some_function_with_long_name(first_argument_value, second_argument_value) { f(x); }
}
