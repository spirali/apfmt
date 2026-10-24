// A loop, `if` or `match` with a long head hugs the call whatever the width of its first line;
// rustfmt hugs it only within `fn_call_width`

fn f() {
    h(while condition_that_is_rather_long(first_argument, second_argument).is_some_and(|x| x > 0) {
        y();
    });
    h(for item in collection_that_is_rather_long(first_argument, second_argument).into_iter() {
        y(item);
    });
    h(if condition_that_is_rather_long(first_argument, second_argument).is_some_and(|x| x > 0) {
        y();
    } else {
        z();
    });
    h(match scrutinee_that_is_rather_long(first_argument, second_argument).unwrap_or_default() {
        A => 1,
        B => 2,
    });
}
