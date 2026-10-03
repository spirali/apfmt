fn main() {
    let a = if x { 1 } else { 2 };
    let a = if x { f(1) } else if y { 2 } else { 3 };
    let a = if x { f(); 1 } else { 2 };
    let a = if x { 1 };
    let value = if a == b { first_value_name } else { second_value_name_long };
    let value = if a == b { first_value_name } else { second_value_name_that_is_long };
    let a = if some_long_condition_name { some_function(argument) } else { other_function(argument) };
    g(if x { 1 } else { 2 });
    if x { if y { 1 } else { 2 } } else { 3 };
    if x { 1 } else { 2 }
}
fn tail() { if x { 1 } else { 2 } }
