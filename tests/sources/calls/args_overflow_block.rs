fn main() {
    let r = tokio::time::timeout(Duration::from_secs(10), async move { let x = fetch().await; process(x).await });
    let v = with_raw_parts(pointer, length, unsafe { let slice = std::slice::from_raw_parts(pointer, length); slice.to_vec() });
    run_in_scope(context, { let value = compute_something(); value + another_computation() });
    labelled(context, 'outer: { if condition { break 'outer 1; } compute_the_remaining_value() });
    gen_block(argument, gen { first_function_call(); second_function_call(); third_one() });
    short(a, unsafe { b() });
    single_stmt_block(first_argument_which_is_long, second_argument_also, unsafe { work_function() });
    a_rather_long_function_name_here_and_more(first_argument_which_is_long, second_argument, unsafe { a(); b() });
    two_blocks(unsafe { first_function_call(); second() }, unsafe { third_function_call(); fourth() });
    two_blocks(unsafe { a() }, unsafe { third_function_call(); fourth_function_call(); fifth() });
    closures(|x| x + 1, |y| { y + 2 }, unsafe { first_function_call(); second_function_call(); third() });
    block_and_closure(unsafe { first_function_call(); second() }, |x| { x + first_function_call() });
    block_closure(unsafe { a() }, |y| { first_function_call(); second_function_call(); third() });
    method_receiver.method_call(argument, unsafe { first_function_call(); second_function_call(); third() });
    f(a, unsafe {
        // comment
        b()
    });
    f(a, unsafe { b(); c() } // comment
    );
    vec![a, unsafe { first_function_call(); second_function_call(); third(); fourth() }];
    m!(a, unsafe { first_function_call(); second_function_call(); third(); fourth_one() });
    [a, unsafe { first_function_call(); second_function_call(); third(); fourth_fifth() }];
    (a, unsafe { first_function_call(); second_function_call(); third(); fourth_fifth() });
}
